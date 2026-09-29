# 開発方針＆開発環境ルール(webeditor-sftp)

作業ドライブは`F:\`。本リポジトリのローカルcloneは`F:\webeditor-sftp`
(サブディレクトリ無し)。全リポジトリ共通ルール(参照資料一覧・AI駆動開発
ツールに関する所感・確認不要の自動継続/リミット解除後の自動再開・
白画面バグ等を見逃さない検証徹底、等)は
[`open-raid-z`](https://github.com/aon-co-jp/open-raid-z)の`CLAUDE.md`を
正本とし、詳細をここに複製せず参照すること。

## このリポジトリの役割

ブログ感覚のWEB文章編集エディターと、秘密鍵・公開鍵を安全に受け渡し
できるSFTPツールをハイブリッド統合したクロスプラットフォームアプリ。
役割の全体像は[`README.md`](README.md)を参照。

## アーキテクチャ概要

```
webeditor-sftp/
  src-tauri/        Rust側(Tauriバックエンド)
    src/
      editor/        文章解析・HTML/CSS/JS生成補助(タグ自動化)
      sftp/           鍵ペア生成・セキュアストレージ・SFTP接続
      main.rs
    Cargo.toml
    tauri.conf.json
  src/               フロントエンド(Web技術、CodeMirror 6ベース)
    editor/           ブログモード(タグ非表示)/コーディングモード切替
    sftp-ui/          鍵の生成・インポート・エクスポート・接続UI
    index.html
  PORTING.md          複数セッション横断の到達点・次回再開ポイント
```

### プラットフォーム対応方針

- Windows / Linux(WindowsにインストールされたLinux含む) / (将来)macOS:
  Tauriデスクトップビルドで対応。
- Android / タブレット: Tauri 2.0 Androidターゲット
  (`x86_64-linux-android`等、本機は既に`rustup target`導入済み)。
- (将来購入予定)iPhone / iPad: Tauri 2.0 iOSターゲット。Xcode環境が
  必要なため、macOS実機/CI入手まで実装は保留。

### 鍵の安全な受け渡し(最重要方針)

- 秘密鍵は生成後、平文でアプリ外(クリップボード履歴・一時ファイル等)
  に残さない設計とする。
- 保存先は各OSのセキュアストレージAPI経由。実装は`keyring`クレート
  (Windows: Credential Manager / macOS: Keychain / Linux: Secret
  Service)。`src-tauri/src/sftp/keystore.rs`参照。
  **⚠️重要**: `keyring`クレート3.x系はOS別バックエンドfeature
  (`windows-native`/`apple-native`/`linux-native-sync-persistent`)を
  `Cargo.toml`の`[target.'cfg(...)'.dependencies]`で明示的に有効化
  しないと、書き込みがエラーを返さず無音で失敗する(2026-09-29に
  実機検証で発覚・修正済み、詳細はPORTING.md参照)。新しいOSターゲット
  を追加する際は、対応するfeatureを必ず追加すること。Android向けの
  ネイティブバックエンドはkeyringクレートに存在しないため、Android
  実機対応時は別のセキュアストレージ手段を要検討。
- 鍵ペア生成はed25519(`russh-keys`、`src-tauri/src/sftp/keys.rs`)。
- SFTP接続は`russh`+`russh-sftp`(純Rust実装、libssh2/OpenSSLへの
  ネイティブ依存なし、`src-tauri/src/sftp/client.rs`)。
  ホスト鍵検証はTOFU方式(`src-tauri/src/sftp/known_hosts.rs`)。
  初回接続でフィンガープリントを記録し、以後不一致なら接続を拒否する。
  記録は`~/.ssh/known_hosts`と同様、秘密情報ではないため平文JSON
  (アプリ設定ディレクトリ配下)。
- デバイス間の鍵の受け渡し(例: Windows→Android)は、この端末間で
  Claude等の外部AIツールを経由させない(ユーザー要件)。**QRコード方式
  を採用**(`src-tauri/src/sftp/pairing.rs`)。公開鍵のみをQR化し、
  秘密鍵は画面越しの盗撮リスクがあるため対象外。生成・カメラでの
  読み取り(`jsqr`、`src/main.ts`)の双方向を実装済み。読み取った
  公開鍵は`sftp_append_authorized_key`コマンドでリモートの
  `~/.ssh/authorized_keys`へ追記できる。
- `audiocafe-tokyo-rust`のような`include_str!`コンパイル時埋め込み
  方式のサイト向けに、SFTPアップロード後の任意コマンド実行
  (`exec_after_upload`、`sftp_upload_text`)を実装。SSH execチャネル
  経由でリモートビルド・サービス再起動まで一括実行できる。

### エディターの2モード

1. **ブログモード**: HTML/CSSタグを意識せず文章を書くと、見出し・
   段落・強調等を自動でタグ化。括弧/記号の自動補完付き。
2. **コーディングモード**: 通常のVS Code的な生のHTML+CSS+
   JavaScript/TypeScript/React編集。UTF-8前提、シンタックス
   ハイライト・自動補完付き。

両モードは同じドキュメントに対する見た目の切り替えであり、
コーディングモードで書いたHTMLをブログモードで開いても崩れない
ことを設計上の必須要件とする。

## 対応バックエンド(サーバー側プログラミング言語)

エディターがシンタックスハイライト/自動補完で意識する対象:
Rust+Axum/Poem/RPoem、PHP+Laravel、Python+FastAPI。
