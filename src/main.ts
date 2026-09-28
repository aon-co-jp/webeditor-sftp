import "./styles.css";
import { invoke } from "@tauri-apps/api/core";

// --- モード切替(ブログ/コーディング/SFTP鍵管理) ---
const tabs = document.querySelectorAll<HTMLButtonElement>(".tab");
const panels = {
  blog: document.querySelector<HTMLElement>("#panel-blog")!,
  code: document.querySelector<HTMLElement>("#panel-code")!,
  sftp: document.querySelector<HTMLElement>("#panel-sftp")!,
};

tabs.forEach((tab) => {
  tab.addEventListener("click", () => {
    const mode = tab.dataset.mode as keyof typeof panels;
    tabs.forEach((t) => t.classList.toggle("active", t === tab));
    (Object.keys(panels) as (keyof typeof panels)[]).forEach((key) =>
      panels[key].classList.toggle("active", key === mode),
    );
  });
});

// --- SFTP鍵管理(プロトタイプ: バックエンドの greet コマンドで疎通確認のみ) ---
const log = document.querySelector<HTMLPreElement>("#sftp-log")!;

function appendLog(line: string) {
  log.textContent += `${line}\n`;
}

document.querySelector<HTMLButtonElement>("#btn-generate-key")?.addEventListener("click", async () => {
  appendLog("[未実装] 鍵ペア生成はRust側 src-tauri/src/sftp/ に実装予定です。");
  try {
    // Tauri backendとの疎通確認用(実際の鍵生成コマンドに置き換え予定)
    const res = await invoke<string>("greet", { name: "webeditor-sftp" });
    appendLog(`backend疎通確認: ${res}`);
  } catch (e) {
    appendLog(`backend呼び出しエラー: ${String(e)}`);
  }
});

document.querySelector<HTMLButtonElement>("#btn-upload")?.addEventListener("click", () => {
  const host = (document.querySelector<HTMLInputElement>("#sftp-host")?.value ?? "").trim();
  const user = (document.querySelector<HTMLInputElement>("#sftp-user")?.value ?? "").trim();
  const path = (document.querySelector<HTMLInputElement>("#sftp-path")?.value ?? "").trim();
  appendLog(`[未実装] SFTPアップロード: ${user}@${host}:${path || "(未入力)"}`);
});
