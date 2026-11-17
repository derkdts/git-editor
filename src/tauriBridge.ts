import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

export interface FileEntry {
  name: string;
  path: string;
  git_status: 'M' | 'U' | null;
}

export interface CommitItem {
  hash: string;
  short_hash: string;
  message: string;
  date: string;
}

export interface RepoState {
  repo_path: string;
  branch: string;
  files: FileEntry[];
  commits: CommitItem[];
}

// Открыть диалог выбора папки и загрузить репозиторий
export async function pickAndOpenRepo(): Promise<RepoState | null> {
  const selected = await open({ directory: true, multiple: false });
  if (!selected || typeof selected !== 'string') return null;
  return await invoke<RepoState>('open_repo', { repoPath: selected });
}

// Чтение и автосохранение файла
export const readFile = (filePath: string) =>
  invoke<string>('read_file', { filePath });

export const saveFile = (filePath: string, content: string) =>
  invoke<void>('save_file', { filePath, content });

// Синхронизация (кнопка Sync)
export const syncRepository = (repoPath: string) =>
  invoke<CommitItem>('git_sync', { repoPath });

// Подгрузка старого коммита для редактирования
export const restoreCommit = (repoPath: string, hash: string) =>
  invoke<void>('restore_commit', { repoPath, hash });
