//! 端末間の鍵ペアリング(QRコード方式)。
//!
//! 現状の方式: 「公開鍵」と接続先ヒントをQRコード化して画面表示し、
//! 別端末のカメラで読み取って`authorized_keys`への追記や接続先入力を
//! 補助する、**公開鍵の受け渡しのみ**を対象とする片方向フロー。
//!
//! 秘密鍵そのものをQRコード化することは行わない(QRコードは画面越しに
//! 盗み見・写真撮影されうるため、秘密鍵の伝送経路としては不適切)。
//! 秘密鍵は各端末でそれぞれ生成し、公開鍵だけを機器間・サーバー間で
//! 共有する運用を前提とする。
//!
//! 同一LAN内ペアリング(直接転送)は未実装(次回再開ポイント、
//! `PORTING.md`参照)。

use base64::Engine;
use image::{ImageEncoder, Luma};
use qrcode::QrCode;
use serde::{Deserialize, Serialize};

use super::SftpError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingPayload {
    pub label: String,
    pub public_key_openssh: String,
    pub fingerprint: String,
}

/// 公開鍵ペアリング情報をQRコード化し、`data:image/png;base64,...`
/// のData URIとして返す(フロント側で`<img>`にそのまま埋め込める)。
pub fn generate_pairing_qr(payload: &PairingPayload) -> Result<String, SftpError> {
    let json = serde_json::to_string(payload)
        .map_err(|e| SftpError::InvalidInput(e.to_string()))?;

    let code = QrCode::new(json.as_bytes())
        .map_err(|e| SftpError::KeyGeneration(format!("QRコード生成に失敗: {e}")))?;

    let image_buf = code.render::<Luma<u8>>().quiet_zone(true).build();
    let (width, height) = (image_buf.width(), image_buf.height());

    let mut png_bytes: Vec<u8> = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png_bytes)
        .write_image(&image_buf, width, height, image::ExtendedColorType::L8)
        .map_err(|e| SftpError::KeyGeneration(format!("PNG変換に失敗: {e}")))?;

    let b64 = base64::engine::general_purpose::STANDARD.encode(png_bytes);
    Ok(format!("data:image/png;base64,{b64}"))
}
