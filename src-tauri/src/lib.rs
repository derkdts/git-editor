mod fs_ops;
mod git_ops;
mod models;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            fs_ops::open_repo,
            fs_ops::read_file,
            fs_ops::save_file,
            git_ops::get_commits,
            git_ops::restore_commit,
            git_ops::git_sync
        ])
        .run(tauri::generate_context!())
        .expect("Ошибка запуска Tauri приложения");
}
