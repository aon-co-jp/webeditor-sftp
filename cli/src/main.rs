//! webeditor-sidecar: VSCode拡張機能から`webeditor-core`を呼び出すための
//! サイドカーCLI。
//!
//! プロトコル: 標準入力から1行1リクエストのJSONを読み、標準出力へ
//! 1行1レスポンスのJSONを書く(JSON Lines)。標準エラー出力はログ用途に
//! 予約し、プロトコルには使わない。
//!
//! リクエスト: {"id": <任意の値、応答にそのまま echo>, "method": "...", "params": {...}}
//! レスポンス(成功): {"id": ..., "ok": true, "result": ...}
//! レスポンス(失敗): {"id": ..., "ok": false, "error": "..."}
//!
//! 対応method一覧は `dispatch` 関数を参照。

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(serde::Deserialize)]
struct Request {
    id: Value,
    method: String,
    #[serde(default)]
    params: Value,
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut lines = BufReader::new(stdin).lines();

    loop {
        let line = match lines.next_line().await {
            Ok(Some(l)) => l,
            Ok(None) => break, // EOF: 呼び出し元(VSCode拡張)がプロセスを終了させた
            Err(e) => {
                eprintln!("[webeditor-sidecar] stdin読み取りエラー: {e}");
                break;
            }
        };
        if line.trim().is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<Request>(&line) {
            Ok(req) => {
                let id = req.id.clone();
                match dispatch(&req.method, req.params).await {
                    Ok(result) => json!({ "id": id, "ok": true, "result": result }),
                    Err(err) => json!({ "id": id, "ok": false, "error": err }),
                }
            }
            Err(e) => json!({ "id": Value::Null, "ok": false, "error": format!("リクエストのJSON解析に失敗: {e}") }),
        };

        let mut line_out = serde_json::to_string(&response).unwrap_or_else(|_| {
            r#"{"id":null,"ok":false,"error":"応答のJSON化に失敗しました"}"#.to_string()
        });
        line_out.push('\n');
        if stdout.write_all(line_out.as_bytes()).await.is_err() {
            break; // 呼び出し元が既にパイプを閉じている
        }
        let _ = stdout.flush().await;
    }
}

async fn dispatch(method: &str, params: Value) -> Result<Value, String> {
    match method {
        "editor.blogToHtml" => {
            let text = get_str(&params, "text")?;
            webeditor_core::editor::blog_to_html(text)
                .map(Value::String)
                .map_err(|e| e.to_string())
        }
        "editor.htmlToBlog" => {
            let html = get_str(&params, "html")?;
            webeditor_core::editor::html_to_blog(html)
                .map(Value::String)
                .map_err(|e| e.to_string())
        }
        "sftp.generateKeypair" => {
            let label = get_str(&params, "label")?;
            let info = webeditor_core::sftp::generate_keypair(label).map_err(|e| e.to_string())?;
            serde_json::to_value(info).map_err(|e| e.to_string())
        }
        "sftp.deleteKeypair" => {
            let label = get_str(&params, "label")?;
            webeditor_core::sftp::delete_keypair(label).map_err(|e| e.to_string())?;
            Ok(Value::Null)
        }
        "sftp.generatePairingQr" => {
            let payload: webeditor_core::sftp::PairingPayload =
                serde_json::from_value(params.get("payload").cloned().unwrap_or(Value::Null))
                    .map_err(|e| format!("payloadが不正です: {e}"))?;
            let data_uri =
                webeditor_core::sftp::generate_pairing_qr(&payload).map_err(|e| e.to_string())?;
            Ok(Value::String(data_uri))
        }
        "sftp.forgetHost" => {
            let host = get_str(&params, "host")?;
            let port = get_u16(&params, "port")?;
            webeditor_core::sftp::forget_host(host, port).map_err(|e| e.to_string())?;
            Ok(Value::Null)
        }
        "sftp.uploadText" => {
            let host = get_str(&params, "host")?;
            let port = get_u16(&params, "port")?;
            let username = get_str(&params, "username")?;
            let key_label = get_str(&params, "keyLabel")?;
            let remote_path = get_str(&params, "remotePath")?;
            let content = get_str(&params, "content")?;
            let exec_after_upload = params.get("execAfterUpload").and_then(|v| v.as_str());
            let result = webeditor_core::sftp::upload_text(
                host,
                port,
                username,
                key_label,
                remote_path,
                content,
                exec_after_upload,
            )
            .await
            .map_err(|e| e.to_string())?;
            serde_json::to_value(result).map_err(|e| e.to_string())
        }
        "sftp.appendAuthorizedKey" => {
            let host = get_str(&params, "host")?;
            let port = get_u16(&params, "port")?;
            let username = get_str(&params, "username")?;
            let key_label = get_str(&params, "keyLabel")?;
            let public_key_line = get_str(&params, "publicKeyLine")?;
            let result = webeditor_core::sftp::append_authorized_key(
                host,
                port,
                username,
                key_label,
                public_key_line,
            )
            .await
            .map_err(|e| e.to_string())?;
            serde_json::to_value(result).map_err(|e| e.to_string())
        }
        other => Err(format!("未知のmethodです: {other}")),
    }
}

fn get_str<'a>(params: &'a Value, key: &str) -> Result<&'a str, String> {
    params
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("params.{key}(文字列)が必要です"))
}

fn get_u16(params: &Value, key: &str) -> Result<u16, String> {
    params
        .get(key)
        .and_then(|v| v.as_u64())
        .and_then(|n| u16::try_from(n).ok())
        .ok_or_else(|| format!("params.{key}(0〜65535の整数)が必要です"))
}
