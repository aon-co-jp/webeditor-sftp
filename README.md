# webeditor-sftp

ブログ感覚でホームページ/WEBサイトの文章を読み書きできるエディターと、
秘密鍵・公開鍵を安全に受け渡しできるSFTPツールをハイブリッド統合した
クロスプラットフォームアプリ。

- Android / タブレット / (将来) iPhone・iPad / Windows / (将来) macOS /
  Windowsにインストール済みのLinux、をハイブリッド対応。
- VPSの秘密鍵・公開鍵をアプリ内で安全に受け渡し(Claude等の外部AIツール
  を介さずに)、そのままSFTPで主要VPSレンタルサーバーへアップロードする
  ところまでを一貫してカバーする。
- エディター本体は「HTMLタグを意識しない」文章ベースの編集(ブログ執筆
  モード)と、通常のVisual Studio Code的な生のHTML+CSS+JavaScript/
  TypeScript/React編集(コーディングモード)の両方を切り替えて使える。
  括弧・記号の自動補完付き。UTF-8前提。
- 対応バックエンド言語/フレームワーク(想定): Rust+Axum/Poem/RPoem、
  PHP+Laravel、Python+FastAPI。エディター側はどの言語のプロジェクトを
  開いても崩れないシンタックスハイライト・自動補完を目指す。
- 最初の実地検証対象: [`audiocafe.tokyo`](https://audiocafe.tokyo/)
  (Rust+Poemベース、`aon-co-jp/audiocafe-tokyo-rust`)のページ編集・
  アップロード。

## 技術方針

詳細は[`CLAUDE.md`](CLAUDE.md)、設計判断の経緯・次回再開ポイントは
[`PORTING.md`](PORTING.md)を参照。

- クロスプラットフォーム基盤: [Tauri](https://tauri.app/) (Rust +
  Webビュー)。open-raid-z系エコシステムのRust方針と親和性が高く、
  デスクトップは軽量。モバイルはTauri 2.0のAndroid/iOS対応を利用。
- エディターUI: Web技術(TypeScript)。CodeMirror 6を中核エンジンと
  して採用予定(ブログモード/コーディングモードの2レイヤーUIをその上に
  構築)。
- SFTPツール: Rust側(Tauri backend)で鍵ペア生成・保管・SFTP接続を
  実装。秘密鍵は端末のセキュアストレージ(Keychain/Credential Manager/
  Android Keystore等)に保存し、平文でディスクに残さない。

## 現状

- ブログモード⇔コーディングモードの相互変換(見出し/箇条書き/段落/
  強調)、CodeMirror 6統合、ed25519鍵ペア生成+OSセキュアストレージ
  保存、SFTPアップロード(russh/russh-sftp)、QRコードによる公開鍵
  ペアリング(生成側のみ)まで実装済み。
- 未実装: ホスト鍵検証(TOFU、**本番接続前に必須**)、QRコード読み取り
  (受信側)、Android/iOS/macOS実機検証。
- 詳細・既知の制約は[`PORTING.md`](PORTING.md)を参照。

## 開発

```bash
npm install
npm run tauri dev
```
