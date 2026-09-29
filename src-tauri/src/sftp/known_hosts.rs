//! TOFU(Trust On First Use)によるホスト鍵検証。
//!
//! `~/.ssh/known_hosts`と同じ考え方: 初回接続時にサーバーの公開鍵の
//! フィンガープリントを記録し、以後の接続では記録済みの値と一致する
//! ことを必須とする。一致しない場合は接続を拒否する(サーバー側の
//! 鍵が変わった/中間者攻撃、いずれの可能性も呼び出し元に見分けが
//! つかないため、安全側に倒して拒否し、ユーザーに判断を委ねる)。
//!
//! 秘密情報ではないため`keyring`ではなくアプリ設定ディレクトリ配下の
//! JSONファイルに平文で保存する(通常のOpenSSHの`known_hosts`と同様の
//! 扱い)。

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::SftpError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustOutcome {
    /// 初回接続。フィンガープリントを新規記録した。
    TrustedOnFirstUse,
    /// 記録済みのフィンガープリントと一致した。
    Known,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct KnownHostsFile {
    /// キー: "host:port"、値: フィンガープリント文字列
    hosts: HashMap<String, String>,
}

fn known_hosts_path() -> Result<PathBuf, SftpError> {
    let dir = dirs::config_dir()
        .ok_or_else(|| SftpError::KeyStore("設定ディレクトリを取得できませんでした".into()))?
        .join("webeditor-sftp");
    std::fs::create_dir_all(&dir).map_err(|e| SftpError::KeyStore(e.to_string()))?;
    Ok(dir.join("known_hosts.json"))
}

fn load() -> Result<KnownHostsFile, SftpError> {
    let path = known_hosts_path()?;
    if !path.exists() {
        return Ok(KnownHostsFile::default());
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| SftpError::KeyStore(e.to_string()))?;
    if raw.trim().is_empty() {
        return Ok(KnownHostsFile::default());
    }
    serde_json::from_str(&raw).map_err(|e| SftpError::KeyStore(e.to_string()))
}

fn save(file: &KnownHostsFile) -> Result<(), SftpError> {
    let path = known_hosts_path()?;
    let raw = serde_json::to_string_pretty(file).map_err(|e| SftpError::KeyStore(e.to_string()))?;
    std::fs::write(&path, raw).map_err(|e| SftpError::KeyStore(e.to_string()))
}

/// ホストの公開鍵フィンガープリントを照合する。
/// 初回はフィンガープリントを記録して`TrustedOnFirstUse`を返す。
/// 記録済みと一致すれば`Known`を返す。不一致なら`Err`(接続拒否)。
pub fn verify_or_trust(host: &str, port: u16, fingerprint: &str) -> Result<TrustOutcome, SftpError> {
    let key = format!("{host}:{port}");
    let mut file = load()?;

    match file.hosts.get(&key) {
        Some(known) if known == fingerprint => Ok(TrustOutcome::Known),
        Some(known) => Err(SftpError::Connection(format!(
            "ホスト鍵が記録済みのものと一致しません(中間者攻撃、またはサーバー側の鍵再生成の可能性)。\
             記録済み: {known} / 今回: {fingerprint}。\
             意図した変更であれば known_hosts.json から該当行を削除してから再接続してください。"
        ))),
        None => {
            file.hosts.insert(key, fingerprint.to_string());
            save(&file)?;
            Ok(TrustOutcome::TrustedOnFirstUse)
        }
    }
}

/// 記録済みホスト鍵を削除する(意図したサーバー鍵変更時のリセット用)。
pub fn forget_host(host: &str, port: u16) -> Result<(), SftpError> {
    let key = format!("{host}:{port}");
    let mut file = load()?;
    file.hosts.remove(&key);
    save(&file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Mutex;

    // 実際のknown_hosts.json(ユーザーのOS設定ディレクトリ)を複数テストが
    // 同時に読み書きしないよう直列化する。ホスト名もテストごとに一意化して
    // 相互汚染を避ける(テスト用データは`example.test.<連番>`)。
    static TEST_LOCK: Mutex<()> = Mutex::new(());
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn unique_host() -> String {
        format!("example.test.{}", COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    #[test]
    fn first_connection_is_trusted_and_recorded() {
        let _guard = TEST_LOCK.lock().unwrap();
        let host = unique_host();
        let outcome = verify_or_trust(&host, 22, "SHA256:abc").unwrap();
        assert_eq!(outcome, TrustOutcome::TrustedOnFirstUse);
        forget_host(&host, 22).unwrap();
    }

    #[test]
    fn matching_fingerprint_is_known() {
        let _guard = TEST_LOCK.lock().unwrap();
        let host = unique_host();
        verify_or_trust(&host, 22, "SHA256:abc").unwrap();
        let outcome = verify_or_trust(&host, 22, "SHA256:abc").unwrap();
        assert_eq!(outcome, TrustOutcome::Known);
        forget_host(&host, 22).unwrap();
    }

    #[test]
    fn mismatched_fingerprint_is_rejected() {
        let _guard = TEST_LOCK.lock().unwrap();
        let host = unique_host();
        verify_or_trust(&host, 22, "SHA256:abc").unwrap();
        let result = verify_or_trust(&host, 22, "SHA256:DIFFERENT");
        assert!(result.is_err(), "ホスト鍵の不一致は拒否されるべき");
        forget_host(&host, 22).unwrap();
    }

    #[test]
    fn forgetting_host_allows_re_trust() {
        let _guard = TEST_LOCK.lock().unwrap();
        let host = unique_host();
        verify_or_trust(&host, 22, "SHA256:abc").unwrap();
        forget_host(&host, 22).unwrap();
        let outcome = verify_or_trust(&host, 22, "SHA256:NEWKEY").unwrap();
        assert_eq!(outcome, TrustOutcome::TrustedOnFirstUse);
        forget_host(&host, 22).unwrap();
    }
}
