use crate::models::CommitItem;
use chrono::Local;
use std::collections::HashMap;
use std::process::Command;

/// Базовый запуск системного git без зависания на запросе пароля
pub fn run_git(dir: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|e| format!("Ошибка запуска git: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if err.is_empty() {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        } else {
            err
        })
    }
}

/// Получить имя текущей ветки
pub fn get_current_branch(dir: &str) -> String {
    run_git(dir, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap_or_else(|_| "main".to_string())
}

/// Получить карту статусов файлов: { "README.md": "M", "todo.txt": "U" }
pub fn get_status_map(dir: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Ok(output) = run_git(dir, &["status", "--porcelain"]) {
        for line in output.lines() {
            if line.len() < 4 {
                continue;
            }
            let code = &line[0..2];
            let file_path = line[3..].trim().trim_matches('"').to_string();
            let badge = if code.contains('?') {
                "U"
            } else {
                "M"
            };
            map.insert(file_path, badge.to_string());
        }
    }
    map
}

/// Получить список последних 20 коммитов
#[tauri::command]
pub fn get_commits(repo_path: String) -> Result<Vec<CommitItem>, String> {
    // Формат: полный_хеш|короткий_хеш|дата|сообщение
    let raw = run_git(
        &repo_path,
        &[
            "log",
            "-n",
            "20",
            "--pretty=format:%H|%h|%ad|%s",
            "--date=format:%Y-%m-%d %H:%M",
        ],
    )?;

    let mut commits = Vec::new();
    for line in raw.lines() {
        let parts: Vec<&str> = line.splitn(4, '|').collect();
        if parts.len() == 4 {
            commits.push(CommitItem {
                hash: parts[0].to_string(),
                short_hash: parts[1].to_string(),
                date: parts[2].to_string(),
                message: parts[3].to_string(),
            });
        }
    }
    Ok(commits)
}

/// Подгрузить состояние файлов из выбранного коммита без Detached HEAD
#[tauri::command]
pub fn restore_commit(repo_path: String, hash: String) -> Result<(), String> {
    run_git(&repo_path, &["checkout", &hash, "--", "."])?;
    Ok(())
}

/// Главная кнопка Sync: add -> commit (с датой/временем) -> pull --rebase -> push
#[tauri::command]
pub async fn git_sync(repo_path: String) -> Result<CommitItem, String> {
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let commit_msg = format!("Auto-sync: {}", now);

    // 1. Добавляем все изменения
    run_git(&repo_path, &["add", "."])?;

    // 2. Создаем коммит (если изменений не было, git вернет ошибку — игнорируем её)
    let _ = run_git(&repo_path, &["commit", "-m", &commit_msg]);

    // 3. Подтягиваем изменения с удаленного репозитория поверх локальных
    if let Err(e) = run_git(&repo_path, &["pull", "--rebase"]) {
        return Err(format!("Ошибка при получении изменений (pull): {}", e));
    }

    // 4. Отправляем в удаленный репозиторий
    if let Err(e) = run_git(&repo_path, &["push"]) {
        return Err(format!("Сохранено локально, но ошибка отправки (push): {}", e));
    }

    // Возвращаем данные свежего коммита для обновления статус-бара
    let commits = get_commits(repo_path)?;
    commits
        .into_iter()
        .next()
        .ok_or_else(|| "Коммит создан, но не удалось прочитать историю".to_string())
}
