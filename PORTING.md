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
  **重要な制約が判明**: `audiocafe-tokyo-rust`のページ内容の一部
  (`assets/search_series.json`等)は`include_str!`でコンパイル時に
  埋め込まれる方式であり、SFTPでファイルを上書きするだけでは本番に
  反映されない(VPS側で`git pull`→`cargo build --release`→
  `systemctl restart`まで必要)。本アプリのSFTPアップロード機能が
  素直に効くのは静的HTML/CSS/JS資産や、旧PHPモノリス
  (`audiocafe-tokyo-php`)側のような直接配信されるファイル群であり、
  `audiocafe-tokyo-rust`のようなコンパイル時埋め込み方式のRust実装には
  そのままでは向かない。将来的にはSFTPアップロード後に
  リモートビルド・再起動コマンドを叩くオプション(SSH経由でのコマンド
  実行)を追加するか、対象を静的資産ファイルに限定するかの判断が必要
  (未確定、次回検討)。

- **2026-09-29 4項目まとめて実装(ユーザー指示「実装して」)**:
  前回列挙した未実装4項目を実装した。

  1. **CodeMirror 6統合**: `codemirror`(basicSetup込み)+
     `@codemirror/lang-html`を導入。[`src/main.ts`](src/main.ts)で
     コーディングモードのエディターとして稼働。
  2. **ブログモード⇔コーディングモードの相互変換ロジック**:
     Rust側(`src-tauri/src/editor/mod.rs`)に実装し、
     `editor_blog_to_html`/`editor_html_to_blog`のTauriコマンドとして
     フロントから呼び出す設計にした(変換ロジックをRust側に置くのは、
     将来的にサーバーサイド共有ライブラリとしても再利用できるように
     するため)。対応範囲: 見出し(`#`〜`###`)・箇条書き(`- `)・段落・
     強調(`**太字**`/`*斜体*`)。単体テスト2件(`cargo test`)で
     ラウンドトリップ(このコンバーターが生成したHTMLを開き直しても
     崩れないこと)を確認済み。**ブログモードで書かれた任意の自由形式
     HTML(手書きの複雑なネスト構造等)の完全な逆変換には未対応**
     (プロトタイプの既知の範囲外、CLAUDE.mdにも明記)。
  3. **SFTP鍵の生成・セキュアストレージ保存・SFTP接続**:
     `src-tauri/src/sftp/`配下に実装。
     - `keys.rs`: `russh-keys`でed25519鍵ペアを生成。
     - `keystore.rs`: `keyring`クレートでOSセキュアストレージ
       (Windows Credential Manager等)に秘密鍵を保存。公開鍵と
       フィンガープリントのみをフロントへ返す設計を徹底。
     - `client.rs`: `russh`+`russh-sftp`(共に純Rust実装、
       libssh2/OpenSSLへのネイティブ依存なし、将来のAndroid/iOS
       クロスコンパイルを見据えて選定)で公開鍵認証SSH接続→SFTP
       アップロードを実装。
     - **既知の重大な制約(未解決)**: ホスト鍵検証が`accept-all`の
       まま(`client.rs`内に`TODO`コメントで明記)。中間者攻撃を
       防げないため、**本番の実サーバーへの接続前に必ずTOFU
       (既知ホスト鍵の記録・照合)を実装すること**。次回最優先。
  4. **端末間鍵受け渡し方式**: QRコード方式を採用(同一LAN内
     ペアリングは不採用、実装コストとユーザーの環境非依存性を優先)。
     `pairing.rs`で`qrcode`+`image`クレートを使い、**公開鍵のみ**を
     JSON化してQRコード(PNG data URI)化する片方向フローを実装。
     秘密鍵は画面越しに盗撮されうるためQR化の対象から意図的に除外。
     **QRコードのカメラ読み取り機能(受信側)は未実装** — 生成・表示
     のみのプロトタイプ。次回、モバイル側でのカメラアクセス(Tauri
     mobile plugin調査が必要)から着手。

  **検証内容**: `cargo check`/`cargo test --lib editor::`(2件pass)/
  `npx tsc --noEmit`/`npm run build`(vite本番ビルド成功)/
  vite dev serverをブラウザプレビューで開きタブ切替・CodeMirror
  マウント・SFTPパネルのログ出力(Tauri外のためinvoke失敗を正しく
  ログ表示することを確認、クラッシュしないことを確認)まで実施。
  **`tauri dev`でのネイティブGUIウィンドウの実機起動・実際の鍵生成/
  SFTP接続の実サーバーに対する動作確認は未実施**(このセッション環境
  ではGUIウィンドウの目視確認手段が確立できなかったため)。

  **次回再開ポイント(優先順)**:
  1. **最優先**: `client.rs`のホスト鍵検証をTOFU方式に置き換える
     (現状は接続先を無条件に信頼しており、実サーバーに接続する前に
     必ず対応すること)。
  2. `tauri dev`のネイティブウィンドウ実機起動・実際のボタン操作での
     鍵生成→OSセキュアストレージ格納確認→(検証用VPS等への)実SFTP
     アップロードのE2E確認。
  3. QRコードのカメラ読み取り(受信側)の実装。
  4. `audiocafe.tokyo`向けには「静的資産ファイルのみアップロード」
     モードの検討、または「アップロード後にSSH経由でリモートビルド
     コマンドを実行するオプション」の追加。
  5. ブログモード⇔コーディングモード変換の対応範囲拡大(画像・
     リンク・テーブル等)。
