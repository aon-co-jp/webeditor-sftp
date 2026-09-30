// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod commands;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::editor_blog_to_html,
            commands::editor_html_to_blog,
            commands::sftp_generate_keypair,
            commands::sftp_delete_keypair,
            commands::sftp_generate_pairing_qr,
            commands::sftp_forget_host,
            commands::sftp_upload_text,
            commands::sftp_append_authorized_key,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
