use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub git_status: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CommitItem {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub date: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RepoState {
    pub repo_path: String,
    pub branch: String,
    pub files: Vec<FileEntry>,
    pub commits: Vec<CommitItem>,
}