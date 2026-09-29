import "./styles.css";
import { invoke } from "@tauri-apps/api/core";
import { EditorView, basicSetup } from "codemirror";
import { EditorState } from "@codemirror/state";
import { html } from "@codemirror/lang-html";
import jsQR from "jsqr";

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

interface ExecOutputDto {
  exit_status?: number;
  stdout: string;
  stderr: string;
}
interface UploadResultDto {
  remote_path: string;
  bytes_written: number;
  host_key_trust: "trusted_first_time" | "known";
  exec_output?: ExecOutputDto;
}

function logUploadResult(result: UploadResultDto) {
  if (result.host_key_trust === "trusted_first_time") {
    appendSftpLog(
      "⚠️ このホストへの初回接続です。ホスト鍵を新規に信頼して記録しました(TOFU)。" +
        "サーバーのフィンガープリントを別経路(VPSコンソール等)で確認することを推奨します。",
    );
  }
  appendSftpLog(`アップロード完了: ${result.remote_path} (${result.bytes_written} bytes)`);
  if (result.exec_output) {
    const { exit_status, stdout, stderr } = result.exec_output;
    appendSftpLog(`コマンド実行結果 (exit=${exit_status ?? "unknown"}):`);
    if (stdout.trim()) appendSftpLog(`  stdout: ${stdout.trim()}`);
    if (stderr.trim()) appendSftpLog(`  stderr: ${stderr.trim()}`);
  }
}

function readUploadForm() {
  const keyLabel = document.querySelector<HTMLInputElement>("#upload-key-label")?.value.trim() ?? "";
  const host = document.querySelector<HTMLInputElement>("#sftp-host")?.value.trim() ?? "";
  const port = Number(document.querySelector<HTMLInputElement>("#sftp-port")?.value.trim() || "22");
  const username = document.querySelector<HTMLInputElement>("#sftp-user")?.value.trim() ?? "";
  return { keyLabel, host, port, username };
}

document.querySelector<HTMLButtonElement>("#btn-upload")?.addEventListener("click", async () => {
  const { keyLabel, host, port, username } = readUploadForm();
  const remotePath = document.querySelector<HTMLInputElement>("#sftp-path")?.value.trim() ?? "";
  const execAfterUpload = document.querySelector<HTMLInputElement>("#sftp-exec")?.value.trim() || undefined;

  if (!keyLabel || !host || !username || !remotePath) {
    appendSftpLog("鍵ラベル・ホスト・ユーザー名・アップロード先パスは必須です。");
    return;
  }

  // アップロード対象は現在のコーディングモードの内容(=正データのHTML)。
  const content = currentMode === "code" ? getCodeContent() : currentHtml;

  appendSftpLog(`アップロード開始: ${username}@${host}:${port} -> ${remotePath}`);
  try {
    const result = await invoke<UploadResultDto>("sftp_upload_text", {
      host,
      port,
      username,
      keyLabel,
      remotePath,
      content,
      execAfterUpload,
    });
    logUploadResult(result);
  } catch (e) {
    appendSftpLog(`アップロードエラー: ${String(e)}`);
  }
});

document.querySelector<HTMLButtonElement>("#btn-forget-host")?.addEventListener("click", async () => {
  const { host, port } = readUploadForm();
  if (!host) {
    appendSftpLog("ホストを入力してください。");
    return;
  }
  try {
    await invoke("sftp_forget_host", { host, port });
    appendSftpLog(`${host}:${port} の記録済みホスト鍵をリセットしました。次回接続時に再度TOFUで記録します。`);
  } catch (e) {
    appendSftpLog(`リセットエラー: ${String(e)}`);
  }
});

// ==========================================================================
// QRコード読み取り(端末間ペアリングの受信側)
// ==========================================================================

const scanVideo = document.querySelector<HTMLVideoElement>("#scan-video")!;
const scanCanvas = document.querySelector<HTMLCanvasElement>("#scan-canvas")!;
const scanResultEl = document.querySelector<HTMLDivElement>("#scan-result")!;
const btnStartScan = document.querySelector<HTMLButtonElement>("#btn-start-scan")!;
const btnStopScan = document.querySelector<HTMLButtonElement>("#btn-stop-scan")!;
const btnInstallScannedKey = document.querySelector<HTMLButtonElement>("#btn-install-scanned-key")!;

let scanStream: MediaStream | null = null;
let scanLoopHandle: number | null = null;
let scannedPayload: { label: string; public_key_openssh: string; fingerprint: string } | null = null;

async function startScan() {
  try {
    scanStream = await navigator.mediaDevices.getUserMedia({ video: { facingMode: "environment" } });
  } catch (e) {
    appendSftpLog(`カメラを開けませんでした: ${String(e)}`);
    return;
  }
  scanVideo.srcObject = scanStream;
  scanVideo.style.display = "block";
  await scanVideo.play();

  btnStartScan.style.display = "none";
  btnStopScan.style.display = "inline-block";

  const ctx = scanCanvas.getContext("2d", { willReadFrequently: true })!;
  const tick = () => {
    if (!scanStream) return;
    if (scanVideo.readyState === scanVideo.HAVE_ENOUGH_DATA) {
      scanCanvas.width = scanVideo.videoWidth;
      scanCanvas.height = scanVideo.videoHeight;
      ctx.drawImage(scanVideo, 0, 0, scanCanvas.width, scanCanvas.height);
      const imageData = ctx.getImageData(0, 0, scanCanvas.width, scanCanvas.height);
      const code = jsQR(imageData.data, imageData.width, imageData.height, {
        inversionAttempts: "dontInvert",
      });
      if (code) {
        onScanSuccess(code.data);
        return;
      }
    }
    scanLoopHandle = requestAnimationFrame(tick);
  };
  scanLoopHandle = requestAnimationFrame(tick);
}

function stopScan() {
  if (scanLoopHandle !== null) cancelAnimationFrame(scanLoopHandle);
  scanLoopHandle = null;
  scanStream?.getTracks().forEach((t) => t.stop());
  scanStream = null;
  scanVideo.style.display = "none";
  btnStartScan.style.display = "inline-block";
  btnStopScan.style.display = "none";
}

function onScanSuccess(raw: string) {
  stopScan();
  try {
    const payload = JSON.parse(raw) as { label: string; public_key_openssh: string; fingerprint: string };
    if (!payload.public_key_openssh) throw new Error("公開鍵情報が含まれていません");
    scannedPayload = payload;
    scanResultEl.style.display = "block";
    scanResultEl.innerHTML = `
      <div class="key-item-label">${escapeHtml(payload.label)}</div>
      <div class="key-item-fp">${escapeHtml(payload.fingerprint)}</div>
      <div class="key-item-fp">${escapeHtml(payload.public_key_openssh)}</div>`;
    btnInstallScannedKey.style.display = "inline-block";
    appendSftpLog(`QR読み取り成功: ${payload.label} (${payload.fingerprint})`);
  } catch (e) {
    appendSftpLog(`QR内容の解析に失敗しました: ${String(e)}`);
  }
}

btnStartScan.addEventListener("click", () => void startScan());
btnStopScan.addEventListener("click", stopScan);

btnInstallScannedKey.addEventListener("click", async () => {
  if (!scannedPayload) return;
  const { keyLabel, host, port, username } = readUploadForm();
  if (!keyLabel || !host || !username) {
    appendSftpLog("3のセクションに鍵ラベル・ホスト・ユーザー名を入力してから実行してください。");
    return;
  }
  appendSftpLog(`authorized_keys追記開始: ${username}@${host}:${port} <- ${scannedPayload.label}`);
  try {
    const result = await invoke<UploadResultDto>("sftp_append_authorized_key", {
      host,
      port,
      username,
      keyLabel,
      publicKeyLine: scannedPayload.public_key_openssh,
    });
    logUploadResult(result);
  } catch (e) {
    appendSftpLog(`authorized_keys追記エラー: ${String(e)}`);
  }
});
