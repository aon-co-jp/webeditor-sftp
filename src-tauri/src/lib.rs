// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod editor;
mod sftp;

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
            editor::editor_blog_to_html,
            editor::editor_html_to_blog,
            sftp::sftp_generate_keypair,
            sftp::sftp_delete_keypair,
            sftp::sftp_generate_pairing_qr,
            sftp::sftp_upload_text,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
