mod storage;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            storage::load_todos,
            storage::save_todos,
            storage::load_ideas,
            storage::save_ideas,
            storage::create_team,
            storage::join_team,
            storage::load_team_context,
            storage::load_team_activity,
            storage::update_team_member_role,
            storage::load_team_todos,
            storage::save_team_todos,
            storage::load_team_ideas,
            storage::save_team_ideas
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
