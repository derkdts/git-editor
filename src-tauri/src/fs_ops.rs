use crate::git_ops::{get_commits, get_current_branch, get_status_map, run_git};
use crate::models::{FileEntry, RepoState};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path};

#[tauri::command]
pub fn init_repo(repo_path: String) -> Result<(), String> {
    let path = Path::new(&repo_path);
    if !path.is_dir() {
        return Err("Выбранный путь не является папкой".to_string());
    }
    if path.join(".git").exists() {
        return Err("В этой папке Git-репозиторий уже существует".to_string());
    }

    run_git(&repo_path, &["init"]).map(|_| ()).map_err(|e| {
        format!("Не удалось инициализировать Git-репозиторий: {}", e)
    })
}

#[tauri::command]
pub fn open_repo(repo_path: String) -> Result<RepoState, String> {
    let path = Path::new(&repo_path);
    if !path.join(".git").exists() {
        return Err("В выбранной папке не инициализирован Git-репозиторий".to_string());
    }

    let status_map = get_status_map(&repo_path);
    let mut files = Vec::new();

    let entries = fs::read_dir(path).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.starts_with('.') {
            continue;
        }
        if entry.path().is_file() {
            let git_status = status_map.get(&file_name).cloned();
            files.push(FileEntry {
                name: file_name,
                path: entry.path().to_string_lossy().to_string(),
                git_status,
            });
        }
    }

    files.sort_by(|a, b| a.name.cmp(&b.name));
    let commits = get_commits(repo_path.clone()).unwrap_or_default();
    let branch = get_current_branch(&repo_path);

    Ok(RepoState {
        repo_path,
        branch,
        files,
        commits,
    })
}

#[tauri::command]
pub fn create_file(repo_path: String, file_name: String) -> Result<String, String> {
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
}

#[tauri::command]
pub fn read_file(file_path: String) -> Result<String, String> {
    fs::read_to_string(&file_path).map_err(|e| format!("Не удалось прочитать файл: {}", e))
}

#[tauri::command]
pub fn save_file(file_path: String, content: String) -> Result<(), String> {
    fs::write(&file_path, content).map_err(|e| format!("Не удалось сохранить файл: {}", e))
}
