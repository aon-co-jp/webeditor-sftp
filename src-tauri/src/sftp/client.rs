//! SFTP接続・アップロード(russh + russh-sftpによる非同期実装)。
//!
//! ⚠️ 既知の制約(プロトタイプ段階): ホスト鍵検証が未実装(`accept-all`)。
//! 中間者攻撃を防げないため、本番投入前に必ずTOFU(Trust On First Use、
//! 既知ホスト鍵の記録・照合)を実装すること。次回再開ポイントとして
//! `PORTING.md`にも明記済み。

use std::sync::Arc;

use russh::client::{self, Handle};
use russh_keys::key::KeyPair;
use russh_sftp::client::SftpSession;

use super::SftpError;

struct AcceptAllHostKeys;

#[async_trait::async_trait]
impl client::Handler for AcceptAllHostKeys {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh_keys::key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // TODO(次回再開ポイント): 既知ホスト鍵を保存して照合するTOFU実装に置き換える。
        Ok(true)
    }
}

pub struct UploadResult {
    pub remote_path: String,
    pub bytes_written: u64,
}

/// 公開鍵認証でSSH接続し、SFTPで1ファイルをアップロードする。
pub async fn upload_file(
    host: &str,
    port: u16,
    username: &str,
    key_pair: KeyPair,
    remote_path: &str,
    content: &[u8],
) -> Result<UploadResult, SftpError> {
    let config = Arc::new(client::Config::default());
    let mut session: Handle<AcceptAllHostKeys> =
        client::connect(config, (host, port), AcceptAllHostKeys)
            .await
            .map_err(|e| SftpError::Connection(e.to_string()))?;

    let authenticated = session
        .authenticate_publickey(username, Arc::new(key_pair))
        .await
        .map_err(|e| SftpError::Connection(e.to_string()))?;
    if !authenticated {
        return Err(SftpError::Connection(
            "公開鍵認証に失敗しました(サーバー側にauthorized_keysが未登録の可能性)".into(),
        ));
    }

    let channel = session
        .channel_open_session()
        .await
        .map_err(|e| SftpError::Connection(e.to_string()))?;
    channel
        .request_subsystem(true, "sftp")
        .await
        .map_err(|e| SftpError::Connection(e.to_string()))?;

    let sftp = SftpSession::new(channel.into_stream())
        .await
        .map_err(|e| SftpError::Sftp(e.to_string()))?;

    let mut file = sftp
        .create(remote_path)
        .await
        .map_err(|e| SftpError::Sftp(e.to_string()))?;

    use tokio::io::AsyncWriteExt;
    file.write_all(content)
        .await
        .map_err(|e| SftpError::Sftp(e.to_string()))?;
    file.shutdown()
        .await
        .map_err(|e| SftpError::Sftp(e.to_string()))?;

    sftp.close().await.map_err(|e| SftpError::Sftp(e.to_string()))?;

    Ok(UploadResult {
        remote_path: remote_path.to_string(),
        bytes_written: content.len() as u64,
    })
}
