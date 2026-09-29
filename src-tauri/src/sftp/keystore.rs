//! OSセキュアストレージ(Windows Credential Manager / macOS Keychain /
//! Linux Secret Service / Android Keystore)への秘密鍵の保管。
//!
//! `keyring`クレートがプラットフォームごとのネイティブAPIを抽象化する。
//! 秘密鍵の値そのものは呼び出し元(コマンド層)からもディスクにも一切
//! 書き出さない — このモジュールを通じてのみ読み書きする。

use keyring::Entry;

use super::SftpError;

const SERVICE: &str = "webeditor-sftp";

fn entry(label: &str) -> Result<Entry, SftpError> {
    Entry::new(SERVICE, label).map_err(|e| SftpError::KeyStore(e.to_string()))
}

/// 秘密鍵(OpenSSH PEM形式の文字列)をOSセキュアストレージへ保存する。
pub fn store_private_key(label: &str, private_key_pem: &str) -> Result<(), SftpError> {
    entry(label)?
        .set_password(private_key_pem)
        .map_err(|e| SftpError::KeyStore(e.to_string()))
}

/// 秘密鍵をOSセキュアストレージから読み出す。
pub fn load_private_key(label: &str) -> Result<String, SftpError> {
    entry(label)?
        .get_password()
        .map_err(|e| SftpError::KeyStore(e.to_string()))
}

/// 秘密鍵をOSセキュアストレージから削除する。
pub fn delete_private_key(label: &str) -> Result<(), SftpError> {
    match entry(label)?.delete_credential() {
        Ok(()) => Ok(()),
        // 既に無い場合は削除成功と同義として扱う(冪等性)。
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(SftpError::KeyStore(e.to_string())),
    }
}
