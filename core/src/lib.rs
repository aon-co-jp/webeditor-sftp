//! webeditor-sftp のコアロジック(Tauriに依存しない)。
//!
//! - `editor`: ブログモード⇔コーディングモードの文章/HTML相互変換。
//! - `sftp`: 鍵ペア生成・OSセキュアストレージ・SFTP接続/アップロード。
//!
//! このクレートは `src-tauri`(デスクトップアプリ本体)と
//! `cli`(VSCode拡張向けサイドカーCLI)の両方から参照される、
//! 唯一の実装場所(single source of truth)。呼び出し側固有の処理
//! (Tauriコマンド属性、CLIのstdio JSONプロトコル)はこのクレートには
//! 置かない。

pub mod editor;
pub mod sftp;
