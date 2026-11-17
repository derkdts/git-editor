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

    // Prevent a console window from flashing for every Git command on Windows.
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
    if let Ok(output) = run_git(dir, &["status", "--porcelain"]) {
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

pub fn latest_commit(dir: &str) -> Result<CommitItem, String> {
    let raw = run_git(
        dir,
        &[
            "log",
            "-1",
            "--pretty=format:%H|%h|%ad|%s",
            "--date=format:%Y-%m-%d %H:%M",
        ],
    )?;
    let parts: Vec<&str> = raw.splitn(4, '|').collect();
    if parts.len() != 4 {
        return Err("В репозитории пока нет коммитов".to_string());
    }
    Ok(CommitItem {
        hash: parts[0].to_string(),
        short_hash: parts[1].to_string(),
        date: parts[2].to_string(),
        message: parts[3].to_string(),
    })
}

#[tauri::command]
pub fn get_commits(repo_path: String) -> Result<Vec<CommitItem>, String> {
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

#[tauri::command]
pub fn restore_commit(repo_path: String, hash: String) -> Result<(), String> {
    run_git(&repo_path, &["checkout", &hash, "--", "."])?;
    Ok(())
}

#[tauri::command]
pub async fn git_sync(repo_path: String) -> Result<CommitItem, String> {
    // Always save changes to the local repository first. No remote is required.
    run_git(&repo_path, &["add", "--all"])?;
    let status = run_git(&repo_path, &["status", "--porcelain"])?;
    if !status.is_empty() {
        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let commit_msg = format!("Auto-sync: {}", now);
        run_git(&repo_path, &["-c", "core.quotepath=false", "commit", "-m", &commit_msg])?;
    }

    let commit = latest_commit(&repo_path)
        .map_err(|_| "Изменения сохранены локально; пока нет коммитов для синхронизации".to_string())?;

    // Only contact a remote when this local branch explicitly tracks one.
    // Freshly initialized local repositories therefore work without upstream errors.
    let has_upstream = run_git(
        &repo_path,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{upstream}"],
    )
    .is_ok();

    if has_upstream {
        run_git(&repo_path, &["pull", "--rebase", "--autostash"])
            .map_err(|e| format!("Локальный коммит сохранён, ошибка получения изменений: {}", e))?;
        run_git(&repo_path, &["push"])
            .map_err(|e| format!("Локальный коммит сохранён, ошибка отправки: {}", e))?;
        latest_commit(&repo_path)
    } else {
        Ok(commit)
    }
}
