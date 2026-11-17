use crate::models::CommitItem;
use chrono::Local;
use std::collections::HashMap;
use std::process::Command;

pub fn run_git(dir: &str, args: &[&str]) -> Result<String, String> {
    let mut command = Command::new("git");
    command
        .args(args)
        .current_dir(dir)
        .env("GIT_TERMINAL_PROMPT", "0");

    // Avoid a console window flashing for each Git command on Windows.
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let output = command
        .output()
        .map_err(|e| format!("Не удалось запустить Git: {}", e))?;

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

pub fn get_current_branch(dir: &str) -> String {
    run_git(dir, &["branch", "--show-current"])
        .ok()
        .filter(|branch| !branch.is_empty())
        .unwrap_or_else(|| "HEAD (нет коммитов)".to_string())
}

pub fn get_status_map(dir: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Ok(output) = run_git(dir, &["status", "--porcelain"] ) {
        for line in output.lines() {
            if line.len() < 4 {
                continue;
            }
            let code = &line[0..2];
            let file_path = line[3..].trim().trim_matches('"').to_string();
            let badge = if code.contains('?') { "U" } else { "M" };
            map.insert(file_path, badge.to_string());
        }
    }
    map
}

pub fn get_commits_data(repo_path: &str) -> Result<Vec<CommitItem>, String> {
    let raw = run_git(
        repo_path,
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

fn latest_commit(dir: &str) -> Result<CommitItem, String> {
    get_commits_data(dir)?
        .into_iter()
        .next()
        .ok_or_else(|| "Изменения сохранены локально, но коммитов пока нет".to_string())
}

#[tauri::command]
pub async fn get_commits(repo_path: String) -> Result<Vec<CommitItem>, String> {
    tauri::async_runtime::spawn_blocking(move || get_commits_data(&repo_path))
        .await
        .map_err(|e| format!("Ошибка фоновой задачи Git: {}", e))?
}

#[tauri::command]
pub async fn restore_commit(repo_path: String, hash: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_git(&repo_path, &["checkout", &hash, "--", "."])?;
        Ok(())
    })
    .await
    .map_err(|e| format!("Ошибка фоновой задачи Git: {}", e))?
}

/// Save changes as a local commit. This command deliberately does not pull or push,
/// so it works for repositories initialized locally and never requires an upstream.
#[tauri::command]
pub async fn git_sync(repo_path: String) -> Result<CommitItem, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_git(&repo_path, &["add", "--all"])?;
        let status = run_git(&repo_path, &["status", "--porcelain"])?;
        if !status.is_empty() {
            let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let commit_msg = format!("Local save: {}", now);
            run_git(
                &repo_path,
                &["-c", "core.quotepath=false", "commit", "-m", &commit_msg],
            )?;
        }
        latest_commit(&repo_path)
    })
    .await
    .map_err(|e| format!("Ошибка фоновой задачи Git: {}", e))?
}
