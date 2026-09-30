//! 鍵ペア生成・OSセキュアストレージへの保管・SFTP接続/アップロード。
//! 設計方針は `CLAUDE.md` 「鍵の安全な受け渡し」節を参照。
//!
//! ここに置く関数はTauri/CLIどちらの呼び出し元にも依存しない、
//! 純粋なコアロジックのみ。呼び出し側固有のラッピング
//! (`#[tauri::command]`属性、CLIのstdio JSONディスパッチ)は
//! `src-tauri`側/`cli`側にそれぞれ置く。

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

pub fn generate_keypair(label: &str) -> Result<KeyInfo, SftpError> {
    keys::generate_keypair(label)
}

pub fn delete_keypair(label: &str) -> Result<(), SftpError> {
    keys::delete_keypair(label)
}

pub fn generate_pairing_qr(payload: &PairingPayload) -> Result<String, SftpError> {
    pairing::generate_pairing_qr(payload)
}

pub fn forget_host(host: &str, port: u16) -> Result<(), SftpError> {
    known_hosts::forget_host(host, port)
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecOutputDto {
    pub exit_status: Option<u32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UploadResultDto {
    pub remote_path: String,
    pub bytes_written: u64,
    /// "trusted_first_time" | "known" — 呼び出し側でホスト鍵を初めて
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

pub async fn upload_text(
    host: &str,
    port: u16,
    username: &str,
    key_label: &str,
    remote_path: &str,
    content: &str,
    exec_after_upload: Option<&str>,
) -> Result<UploadResultDto, SftpError> {
    let key_pair = keys::load_keypair(key_label)?;
    let result = client::upload_file(
        host,
        port,
        username,
        key_pair,
        remote_path,
        content.as_bytes(),
        exec_after_upload,
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
pub async fn append_authorized_key(
    host: &str,
    port: u16,
    username: &str,
    key_label: &str,
    public_key_line: &str,
) -> Result<UploadResultDto, SftpError> {
    let key_pair = keys::load_keypair(key_label)?;
    let result =
        client::append_authorized_key(host, port, username, key_pair, public_key_line).await?;
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
