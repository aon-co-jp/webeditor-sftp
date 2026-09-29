//! 鍵ペア生成・OSセキュアストレージへの保管・SFTP接続/アップロード。
//! 設計方針は `F:\webeditor-sftp\CLAUDE.md` 「鍵の安全な受け渡し」節を参照。

mod client;
mod keys;
mod keystore;
mod pairing;

use serde::Serialize;
use thiserror::Error;

pub use keys::KeyInfo;
pub use pairing::PairingPayload;

#[derive(Debug, Error, Serialize)]
pub enum SftpError {
    #[error("入力エラー: {0}")]
    InvalidInput(String),
    #[error("鍵生成エラー: {0}")]
    KeyGeneration(String),
    #[error("セキュアストレージエラー: {0}")]
    KeyStore(String),
    #[error("接続エラー: {0}")]
    Connection(String),
    #[error("SFTPエラー: {0}")]
    Sftp(String),
}

// --- Tauriコマンド(フロントエンドから`invoke`で呼び出す) ---

#[tauri::command]
pub fn sftp_generate_keypair(label: String) -> Result<KeyInfo, SftpError> {
    keys::generate_keypair(&label)
}

#[tauri::command]
pub fn sftp_delete_keypair(label: String) -> Result<(), SftpError> {
    keys::delete_keypair(&label)
}

#[tauri::command]
pub fn sftp_generate_pairing_qr(payload: PairingPayload) -> Result<String, SftpError> {
    pairing::generate_pairing_qr(&payload)
}

#[derive(Debug, Serialize)]
pub struct UploadResultDto {
    pub remote_path: String,
    pub bytes_written: u64,
}

#[tauri::command]
pub async fn sftp_upload_text(
    host: String,
    port: u16,
    username: String,
    key_label: String,
    remote_path: String,
    content: String,
) -> Result<UploadResultDto, SftpError> {
    let key_pair = keys::load_keypair(&key_label)?;
    let result = client::upload_file(
        &host,
        port,
        &username,
        key_pair,
        &remote_path,
        content.as_bytes(),
    )
    .await?;
    Ok(UploadResultDto {
        remote_path: result.remote_path,
        bytes_written: result.bytes_written,
    })
}
