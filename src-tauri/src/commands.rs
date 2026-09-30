//! Tauriコマンド(フロントエンドから`invoke`で呼び出す)の薄いラッパー。
//! 実装本体は全て `webeditor_core` クレートにあり、ここでは
//! `#[tauri::command]`属性を付けて中継するのみ。デスクトップアプリ
//! (このsrc-tauri)とVSCode拡張向けサイドカーCLI(`cli/`)の両方が
//! `webeditor_core`を参照することで、ロジックの実装は1箇所に保つ。

use webeditor_core::editor::{self, EditorError};
use webeditor_core::sftp::{self, KeyInfo, PairingPayload, SftpError, UploadResultDto};

#[tauri::command]
pub fn editor_blog_to_html(text: String) -> Result<String, EditorError> {
    editor::editor_blog_to_html(text)
}

#[tauri::command]
pub fn editor_html_to_blog(html: String) -> Result<String, EditorError> {
    editor::editor_html_to_blog(html)
}

#[tauri::command]
pub fn sftp_generate_keypair(label: String) -> Result<KeyInfo, SftpError> {
    sftp::generate_keypair(&label)
}

#[tauri::command]
pub fn sftp_delete_keypair(label: String) -> Result<(), SftpError> {
    sftp::delete_keypair(&label)
}

#[tauri::command]
pub fn sftp_generate_pairing_qr(payload: PairingPayload) -> Result<String, SftpError> {
    sftp::generate_pairing_qr(&payload)
}

#[tauri::command]
pub fn sftp_forget_host(host: String, port: u16) -> Result<(), SftpError> {
    sftp::forget_host(&host, port)
}

#[tauri::command]
pub async fn sftp_upload_text(
    host: String,
    port: u16,
    username: String,
    key_label: String,
    remote_path: String,
    content: String,
    exec_after_upload: Option<String>,
) -> Result<UploadResultDto, SftpError> {
    sftp::upload_text(
        &host,
        port,
        &username,
        &key_label,
        &remote_path,
        &content,
        exec_after_upload.as_deref(),
    )
    .await
}

#[tauri::command]
pub async fn sftp_append_authorized_key(
    host: String,
    port: u16,
    username: String,
    key_label: String,
    public_key_line: String,
) -> Result<UploadResultDto, SftpError> {
    sftp::append_authorized_key(&host, port, &username, &key_label, &public_key_line).await
}
