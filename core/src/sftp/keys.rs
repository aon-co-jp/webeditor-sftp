//! 鍵ペア生成(ed25519)。

use russh_keys::key::KeyPair;
use russh_keys::PublicKeyBase64;
use serde::{Deserialize, Serialize};

use super::keystore;
use super::SftpError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyInfo {
    /// このアプリ内で鍵を識別するラベル(例: "audiocafe.tokyo-root")
    pub label: String,
    pub algorithm: String,
    /// `ssh-ed25519 AAAA... label` 形式(authorized_keysに追記可能)
    pub public_key_openssh: String,
    pub fingerprint: String,
}

/// ed25519鍵ペアを新規生成し、秘密鍵をOSセキュアストレージへ保存する。
/// 呼び出し元(フロントエンド)には公開鍵とフィンガープリントのみを返す。
pub fn generate_keypair(label: &str) -> Result<KeyInfo, SftpError> {
    if label.trim().is_empty() {
        return Err(SftpError::InvalidInput("ラベルが空です".into()));
    }

    let key_pair = KeyPair::generate_ed25519().ok_or_else(|| {
        SftpError::KeyGeneration("ed25519鍵ペアの生成に失敗しました".into())
    })?;

    let mut private_pem_bytes: Vec<u8> = Vec::new();
    russh_keys::encode_pkcs8_pem(&key_pair, &mut private_pem_bytes)
        .map_err(|e| SftpError::KeyGeneration(e.to_string()))?;
    let private_pem = String::from_utf8(private_pem_bytes)
        .map_err(|e| SftpError::KeyGeneration(e.to_string()))?;

    let public_b64 = key_pair.public_key_base64();
    let public_key_openssh = format!("ssh-ed25519 {public_b64} {label}");
    let fingerprint = russh_keys::key::PublicKey::fingerprint(&key_pair.clone_public_key()
        .map_err(|e| SftpError::KeyGeneration(e.to_string()))?);

    keystore::store_private_key(label, &private_pem)?;

    Ok(KeyInfo {
        label: label.to_string(),
        algorithm: "ed25519".to_string(),
        public_key_openssh,
        fingerprint,
    })
}

/// 保存済み秘密鍵を読み込み、SFTP接続に使う`KeyPair`を復元する。
pub fn load_keypair(label: &str) -> Result<KeyPair, SftpError> {
    let pem = keystore::load_private_key(label)?;
    russh_keys::decode_secret_key(&pem, None).map_err(|e| SftpError::KeyStore(e.to_string()))
}

/// 秘密鍵を削除する(公開鍵の記録はフロント側/呼び出し元の責任で管理)。
pub fn delete_keypair(label: &str) -> Result<(), SftpError> {
    keystore::delete_private_key(label)
}
