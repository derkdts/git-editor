use crate::git_ops::{get_commits_data, get_current_branch, get_status_map, run_git};
use crate::models::{FileEntry, RepoState};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path};

#[tauri::command]
pub async fn init_repo(repo_path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = Path::new(&repo_path);
        if !path.is_dir() {
            return Err("Выбранный путь не является папкой".to_string());
        }
        if path.join(".git").exists() {
            return Err("В этой папке Git-репозиторий уже существует".to_string());
        }
        run_git(&repo_path, &["init"]).map(|_| ())
    })
    .await
    .map_err(|e| format!("Ошибка фоновой задачи Git: {}", e))?
}

#[tauri::command]
pub async fn clone_repo(repo_url: String, destination_path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let url = repo_url.trim();
        if url.is_empty() || url.chars().any(char::is_control) {
            return Err("Укажите корректный URL Git-репозитория".to_string());
        }
        if !(url.starts_with("https://")
            || url.starts_with("http://")
            || url.starts_with("ssh://")
            || url.starts_with("git@"))
        {
            return Err("Поддерживаются URL HTTPS, HTTP, SSH и git@host:path".to_string());
        }

        let destination = Path::new(&destination_path);
        if !destination.is_dir() {
            return Err("Папка назначения не существует".to_string());
        }
        let mut entries = fs::read_dir(destination).map_err(|e| e.to_string())?;
        if entries.next().is_some() {
            return Err("Для клонирования выберите пустую папку".to_string());
        }

        run_git(&destination_path, &["clone", "--", url, "."])
            .map(|_| ())
            .map_err(|e| format!("Не удалось клонировать репозиторий: {}", e))
    })
    .await
    .map_err(|e| format!("Ошибка фоновой задачи Git: {}", e))?
}

/// Recursively collect regular files without following symlinks. Hidden files
/// and directories (including .git) remain excluded as in the original listing.
fn collect_files(root: &Path, directory: &Path, status_map: &std::collections::HashMap<String, String>, files: &mut Vec<FileEntry>) -> Result<(), String> {
    let entries = fs::read_dir(directory).map_err(|e| format!("Не удалось прочитать папку: {}", e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Не удалось прочитать элемент папки: {}", e))?;
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.starts_with('.') {
            continue;
        }
        let file_type = entry.file_type().map_err(|e| format!("Не удалось определить тип файла: {}", e))?;
        if file_type.is_dir() {
            collect_files(root, &entry.path(), status_map, files)?;
        } else if file_type.is_file() {
            let relative = entry.path().strip_prefix(root).map_err(|e| e.to_string())?;
            let git_path = relative.to_string_lossy().replace('\\', "/");
            files.push(FileEntry {
                git_status: status_map.get(&git_path).cloned(),
                name: file_name,
                path: entry.path().to_string_lossy().to_string(),
            });
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn open_repo(repo_path: String) -> Result<RepoState, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = Path::new(&repo_path);
        if !path.join(".git").exists() {
            return Err("В выбранной папке не инициализирован Git-репозиторий".to_string());
        }

        let status_map = get_status_map(&repo_path);
        let mut files = Vec::new();
        collect_files(path, path, &status_map, &mut files)?;
        files.sort_by(|a, b| {
            let a_relative = Path::new(&a.path).strip_prefix(path).unwrap_or(Path::new(&a.path));
            let b_relative = Path::new(&b.path).strip_prefix(path).unwrap_or(Path::new(&b.path));
            a_relative.cmp(b_relative)
        });

        let commits = get_commits_data(&repo_path).unwrap_or_default();
        let branch = get_current_branch(&repo_path);
        Ok(RepoState { repo_path, branch, files, commits })
    })
    .await
    .map_err(|e| format!("Ошибка фоновой задачи репозитория: {}", e))?
}

/// Read a historical file version without checking out the commit or changing
/// the working tree. This keeps the user's current file and Git state intact.
#[tauri::command]
pub async fn read_file_at_commit(
    repo_path: String,
    file_path: String,
    hash: String,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if hash.len() < 7 || hash.len() > 40 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("Некорректный идентификатор коммита".to_string());
        }
        let repo = Path::new(&repo_path)
            .canonicalize()
            .map_err(|e| format!("Не удалось определить путь репозитория: {}", e))?;
        let file = Path::new(&file_path)
            .canonicalize()
            .map_err(|e| format!("Не удалось определить путь файла: {}", e))?;
        let relative = file
            .strip_prefix(&repo)
            .map_err(|_| "Файл находится за пределами репозитория".to_string())?;
        if relative.components().any(|component| !matches!(component, Component::Normal(_))) {
            return Err("Некорректный путь файла".to_string());
        }
        let git_path = relative.to_string_lossy().replace('\\', "/");
        run_git(&repo.to_string_lossy(), &["cat-file", "blob", &format!("{}:{}", hash, git_path)])
            .map_err(|e| format!("Не удалось прочитать версию файла из коммита: {}", e))
    })
    .await
    .map_err(|e| format!("Ошибка фоновой задачи истории: {}", e))?
}

#[tauri::command]
pub async fn create_file(repo_path: String, file_name: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let name = file_name.trim();
        let relative_path = Path::new(name);
        if name.is_empty()
            || name.starts_with('.')
            || relative_path.components().count() != 1
            || !matches!(relative_path.components().next(), Some(Component::Normal(_)))
            || name.chars().any(char::is_control)
        {
            return Err("Введите имя файла без пути и без управляющих символов".to_string());
        }

        let repo = Path::new(&repo_path);
        if !repo.join(".git").exists() {
            return Err("В выбранной папке не инициализирован Git-репозиторий".to_string());
        }

        let path = repo.join(relative_path);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    format!("Файл «{}» уже существует", name)
                } else {
                    format!("Не удалось создать файл: {}", e)
                }
            })?;
        file.write_all(b"")
            .map_err(|e| format!("Не удалось создать файл: {}", e))?;
        Ok(path.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| format!("Ошибка фоновой задачи файла: {}", e))?
}

#[tauri::command]
pub async fn read_file(file_path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        fs::read_to_string(&file_path)
            .map_err(|e| format!("Не удалось прочитать файл: {}", e))
    })
    .await
    .map_err(|e| format!("Ошибка фоновой задачи файла: {}", e))?
}

#[tauri::command]
pub async fn save_file(file_path: String, content: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        fs::write(&file_path, content)
            .map_err(|e| format!("Не удалось сохранить файл: {}", e))
    })
    .await
    .map_err(|e| format!("Ошибка фоновой задачи файла: {}", e))?
}
