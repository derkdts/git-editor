use crate::git_ops::{get_commits, get_current_branch, get_status_map};
use crate::models::{FileEntry, RepoState};
use std::fs;
use std::path::Path;

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
            continue; // Пропускаем .git, .DS_Store и т.д.
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
pub fn read_file(file_path: String) -> Result<String, String> {
    fs::read_to_string(&file_path).map_err(|e| format!("Не удалось прочитать файл: {}", e))
}

#[tauri::command]
pub fn save_file(file_path: String, content: String) -> Result<(), String> {
    fs::write(&file_path, content).map_err(|e| format!("Не удалось сохранить файл: {}", e))
}
