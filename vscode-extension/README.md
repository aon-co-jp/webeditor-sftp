# webeditor-sftp (VSCode拡張機能・雛形)

`webeditor-sftp`のRustコア(`../core`)を、Node.js製サイドカーCLI
(`../cli`、`webeditor-sidecar`)経由でVSCodeから呼び出す拡張機能。

## 現状(プロトタイプ)

- コマンドパレットから以下を実行可能:
  - `webeditor-sftp: ブログモードでプレビュー` — 開いているHTMLを
    タグ非表示の文章としてWebviewに表示(読み取り専用)。
  - `webeditor-sftp: ブログ文章 → HTML変換して新規ファイルに開く`
  - `webeditor-sftp: HTML → ブログ文章に変換して新規ファイルに開く`
  - `webeditor-sftp: SFTP鍵ペアを生成(OSセキュアストレージに保存)`
  - `webeditor-sftp: 現在のファイルをSFTPでアップロード`
- ロジック本体は一切持たない。`webeditor-sidecar`プロセスを起動し、
  stdin/stdoutのJSON Linesプロトコルで通信するだけ(`src/sidecarClient.ts`)。

## 開発・動作確認

```bash
# 1. サイドカーCLIをビルド(リポジトリルートで)
cargo build -p webeditor-sidecar

# 2. 拡張機能をコンパイル
cd vscode-extension
npm install
npm run compile

# 3. VSCodeでこのフォルダを開き F5(拡張機能開発ホストを起動)
```

サイドカー実行ファイルは既定で `../cli/target/{release,debug}/` または
リポジトリ直下 `../target/{release,debug}/`(ワークスペースビルド時の
出力先)を自動探索する。見つからない/別の場所にある場合は設定
`webeditorSftp.sidecarPath` で絶対パスを指定する。

## 未実装・既知の制約

- VSIXパッケージ化(`vsce package`)、サイドカーバイナリの同梱は未対応。
  現状はこのリポジトリをcloneした開発環境でのみ動作する。
- コーディングモード(VSCode組み込みMonacoエディターとの統合)は
  未実装。現状は変換結果を新規タブで開くだけ。
- ブログモードは読み取り専用プレビューのみ。双方向編集(Webview内で
  編集した内容を元のHTMLファイルへ書き戻す)は未実装。
- QRコードによる鍵ペアリング機能は未移植(デスクトップアプリ版のみ)。

詳細な設計方針・次回再開ポイントは `../CLAUDE.md` / `../PORTING.md` を参照。
