import "./styles.css";
import { invoke } from "@tauri-apps/api/core";
import { EditorView, basicSetup } from "codemirror";
import { EditorState } from "@codemirror/state";
import { html } from "@codemirror/lang-html";

// ==========================================================================
// モード切替(ブログ/コーディング/SFTP鍵管理)
//
// 単一の「HTML」を内部の正データとして保持し、
// - ブログモードのテキストエリアはそのHTMLの「文章表現」
// - コーディングモードのCodeMirrorはそのHTML「そのもの」
// を表示する。タブ切替のたびに双方向変換する(CLAUDE.mdの必須要件:
// コーディングモードで書いたHTMLをブログモードで開いても崩れないこと)。
// ==========================================================================

type Mode = "blog" | "code" | "sftp";

const tabs = document.querySelectorAll<HTMLButtonElement>(".tab");
const panels: Record<Mode, HTMLElement> = {
  blog: document.querySelector<HTMLElement>("#panel-blog")!,
  code: document.querySelector<HTMLElement>("#panel-code")!,
  sftp: document.querySelector<HTMLElement>("#panel-sftp")!,
};

const blogEditor = document.querySelector<HTMLTextAreaElement>("#blog-editor")!;
const codeHost = document.querySelector<HTMLDivElement>("#code-editor-host")!;

let currentHtml = "";
let currentMode: Mode = "blog";

const codeView = new EditorView({
  state: EditorState.create({ doc: "", extensions: [basicSetup, html()] }),
  parent: codeHost,
});

function getCodeContent(): string {
  return codeView.state.doc.toString();
}

function setCodeContent(text: string) {
  codeView.dispatch({
    changes: { from: 0, to: codeView.state.doc.length, insert: text },
  });
}

async function syncFromBlogToCode() {
  try {
    currentHtml = await invoke<string>("editor_blog_to_html", { text: blogEditor.value });
  } catch (e) {
    appendSftpLog(`[エディター変換エラー] ${String(e)}`);
    return;
  }
  setCodeContent(currentHtml);
}

async function syncFromCodeToBlog() {
  currentHtml = getCodeContent();
  try {
    blogEditor.value = await invoke<string>("editor_html_to_blog", { html: currentHtml });
  } catch (e) {
    appendSftpLog(`[エディター変換エラー] ${String(e)}`);
  }
}

async function switchTo(mode: Mode) {
  // 直前のモードの内容を正データ(currentHtml)へ反映してから切り替える。
  if (currentMode === "blog" && mode !== "blog") {
    await syncFromBlogToCode();
  } else if (currentMode === "code" && mode !== "code") {
    await syncFromCodeToBlog();
  }

  currentMode = mode;
  tabs.forEach((t) => t.classList.toggle("active", t.dataset.mode === mode));
  (Object.keys(panels) as Mode[]).forEach((key) => panels[key].classList.toggle("active", key === mode));
}

tabs.forEach((tab) => {
  tab.addEventListener("click", () => {
    void switchTo(tab.dataset.mode as Mode);
  });
});

// ==========================================================================
// SFTP鍵管理
// ==========================================================================

const log = document.querySelector<HTMLPreElement>("#sftp-log")!;
function appendSftpLog(line: string) {
  log.textContent += `${line}\n`;
  log.scrollTop = log.scrollHeight;
}

interface KeyInfo {
  label: string;
  algorithm: string;
  public_key_openssh: string;
  fingerprint: string;
}

// 公開鍵情報のみをローカルに保持(秘密鍵はRust側のOSセキュアストレージ)。
const KEY_LIST_STORAGE = "webeditor-sftp.keys";
function loadKeyList(): KeyInfo[] {
  try {
    return JSON.parse(localStorage.getItem(KEY_LIST_STORAGE) ?? "[]");
  } catch {
    return [];
  }
}
function saveKeyList(keys: KeyInfo[]) {
  try {
    localStorage.setItem(KEY_LIST_STORAGE, JSON.stringify(keys));
  } catch {
    // 保存に失敗してもアプリの継続を優先(プライベートウィンドウ等)。
  }
}

const keyListEl = document.querySelector<HTMLDivElement>("#key-list")!;
function renderKeyList() {
  const keys = loadKeyList();
  keyListEl.innerHTML = keys
    .map(
      (k) => `
      <div class="key-item">
        <div class="key-item-label">${escapeHtml(k.label)}</div>
        <div class="key-item-fp">${escapeHtml(k.fingerprint)}</div>
      </div>`,
    )
    .join("");
}
function escapeHtml(s: string) {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}
renderKeyList();

document.querySelector<HTMLButtonElement>("#btn-generate-key")?.addEventListener("click", async () => {
  const label = document.querySelector<HTMLInputElement>("#key-label")?.value.trim() ?? "";
  if (!label) {
    appendSftpLog("鍵ラベルを入力してください。");
    return;
  }
  try {
    const info = await invoke<KeyInfo>("sftp_generate_keypair", { label });
    const keys = loadKeyList().filter((k) => k.label !== info.label);
    keys.push(info);
    saveKeyList(keys);
    renderKeyList();
    appendSftpLog(`鍵ペア生成完了: ${info.label} (${info.fingerprint})`);

    const qr = await invoke<string>("sftp_generate_pairing_qr", {
      payload: {
        label: info.label,
        public_key_openssh: info.public_key_openssh,
        fingerprint: info.fingerprint,
      },
    });
    document.querySelector<HTMLImageElement>("#pairing-qr")!.src = qr;
  } catch (e) {
    appendSftpLog(`鍵生成エラー: ${String(e)}`);
  }
});

document.querySelector<HTMLButtonElement>("#btn-upload")?.addEventListener("click", async () => {
  const keyLabel = document.querySelector<HTMLInputElement>("#upload-key-label")?.value.trim() ?? "";
  const host = document.querySelector<HTMLInputElement>("#sftp-host")?.value.trim() ?? "";
  const port = Number(document.querySelector<HTMLInputElement>("#sftp-port")?.value.trim() || "22");
  const username = document.querySelector<HTMLInputElement>("#sftp-user")?.value.trim() ?? "";
  const remotePath = document.querySelector<HTMLInputElement>("#sftp-path")?.value.trim() ?? "";

  if (!keyLabel || !host || !username || !remotePath) {
    appendSftpLog("鍵ラベル・ホスト・ユーザー名・アップロード先パスは必須です。");
    return;
  }

  // アップロード対象は現在のコーディングモードの内容(=正データのHTML)。
  const content = currentMode === "code" ? getCodeContent() : currentHtml;

  appendSftpLog(`アップロード開始: ${username}@${host}:${port} -> ${remotePath}`);
  try {
    const result = await invoke<{ remote_path: string; bytes_written: number }>("sftp_upload_text", {
      host,
      port,
      username,
      keyLabel,
      remotePath,
      content,
    });
    appendSftpLog(`アップロード完了: ${result.remote_path} (${result.bytes_written} bytes)`);
  } catch (e) {
    appendSftpLog(`アップロードエラー: ${String(e)}`);
  }
});
