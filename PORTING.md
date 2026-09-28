# PORTING / セッション引き継ぎ

## HANDOFF

- **2026-09-28 リポジトリ新設**: ユーザーから「ブログ用エディター+
  秘密鍵/公開鍵を安全に受け渡しできるSFTPアプリをハイブリッドに一緒に
  開発してほしい」との構想提案。GitHubリポジトリ名に`&`が使えないため
  `editor&sftp-tool`→`webeditor-sftp`に変更(ユーザー選択)。
  `aon-co-jp/webeditor-sftp`として新規作成、`F:\webeditor-sftp`に
  clone。技術基盤はTauri(Rust+Web)採用(ユーザー選択、
  Flutter/Electron+Capacitorとの比較の上で決定)。
  今回のセッションスコープは「リポジトリ雛形+設計ドキュメント+
  最小プロトタイプまで」(ユーザー選択、実装の本格着手は次回以降)。
  本`F:\runo\README.md`のエコシステム索引に`webeditor-sftp`行を追加。

  **次回再開ポイント**:
  1. 最小プロトタイプ(`src-tauri/`+`src/`雛形、`cargo tauri dev`で
     起動確認まで)の実機検証(このセッションではビルド確認のみ、
     実行画面のスクリーンショット確認は未実施の場合はそこから)。
  2. エディター中核エンジンをCodeMirror 6で本実装
     (ブログモード⇔コーディングモードの相互変換ロジックが最難関、
     設計のみでコードは未着手)。
  3. SFTP鍵の生成・セキュアストレージ保存・SFTP接続の実装
     (Rust側、`ssh2`クレートまたは`russh`クレートを比較検討予定、
     未確定)。
  4. 端末間鍵受け渡し方式の確定(QRコード方式 or 同一LAN内ペアリング
     方式、[`CLAUDE.md`](CLAUDE.md)の「鍵の安全な受け渡し」節参照、
     未確定のため次回最優先で調査)。
  5. Android実機ビルド確認(`rustup target`は導入済み、Android SDK/NDK
     の有無は未確認)。iOS/macOS対応は実機・Xcode環境入手まで保留。

- **2026-09-28 最初の実対象サイト確定**: ユーザーより
  [`audiocafe.tokyo`](https://audiocafe.tokyo/)(Rust+RPoem/Poemベースの
  既存サイト)のページをブログ感覚で編集しSFTPでアップロードしたい、
  との具体的な最初の利用シーンが示された。これを本アプリの最初の
  実地検証対象とする。次回以降、`audiocafe.tokyo`のVPS上のページ構成
  (テンプレートエンジン・静的HTML出力の有無・配置パス)を調査した上で、
  ブログモード⇔コーディングモードの相互変換ロジックの設計を
  `audiocafe.tokyo`の実ページ構造に合わせて具体化する。
  調査メモ: `aon-co-jp/audiocafe-tokyo-rust`(Poem採用、PHPモノリスから
  移行中、`src/`+`assets/`構成)が現行のRust実装リポジトリ。
  旧`aon-co-jp/audiocafe-tokyo-php`も存在(移行元)。
