//! 鍵ペア生成・OSセキュアストレージへの保管・SFTP接続/アップロード。
//! 設計方針は `F:\webeditor-sftp\CLAUDE.md` 「鍵の安全な受け渡し」節を参照。

mod client;
mod keys;
mod keystore;
mod known_hosts;
mod pairing;

use serde::Serialize;
use thiserror::Error;

pub use keys::KeyInfo;
pub use known_hosts::TrustOutcome;
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

#[tauri::command]
pub fn sftp_forget_host(host: String, port: u16) -> Result<(), SftpError> {
    known_hosts::forget_host(&host, port)
}

#[derive(Debug, Serialize)]
pub struct ExecOutputDto {
    pub exit_status: Option<u32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Serialize)]
pub struct UploadResultDto {
    pub remote_path: String,
    pub bytes_written: u64,
    /// "trusted_first_time" | "known" — フロント側でホスト鍵を初めて
    /// 信頼したことをユーザーに明示するためのフラグ。
    pub host_key_trust: &'static str,
    pub exec_output: Option<ExecOutputDto>,
}

fn trust_label(outcome: TrustOutcome) -> &'static str {
    match outcome {
        TrustOutcome::TrustedOnFirstUse => "trusted_first_time",
        TrustOutcome::Known => "known",
    }
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
    let key_pair = keys::load_keypair(&key_label)?;
    let result = client::upload_file(
        &host,
        port,
        &username,
        key_pair,
        &remote_path,
        content.as_bytes(),
        exec_after_upload.as_deref(),
    )
    .await?;
    Ok(UploadResultDto {
        remote_path: result.remote_path,
        bytes_written: result.bytes_written,
        host_key_trust: trust_label(result.host_key_trust),
        exec_output: result.exec_output.map(|e| ExecOutputDto {
            exit_status: e.exit_status,
            stdout: e.stdout,
            stderr: e.stderr,
        }),
    })
}

/// QRコードで読み取った他端末の公開鍵を、既存の鍵で接続した
/// サーバーの`~/.ssh/authorized_keys`へ追記する。
#[tauri::command]
pub async fn sftp_append_authorized_key(
    host: String,
    port: u16,
    username: String,
    key_label: String,
    public_key_line: String,
) -> Result<UploadResultDto, SftpError> {
    let key_pair = keys::load_keypair(&key_label)?;
    let result =
        client::append_authorized_key(&host, port, &username, key_pair, &public_key_line).await?;
    Ok(UploadResultDto {
        remote_path: result.remote_path,
        bytes_written: result.bytes_written,
        host_key_trust: trust_label(result.host_key_trust),
        exec_output: result.exec_output.map(|e| ExecOutputDto {
            exit_status: e.exit_status,
            stdout: e.stdout,
            stderr: e.stderr,
        }),
    })
}
