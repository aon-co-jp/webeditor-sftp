import * as vscode from "vscode";
import { SidecarClient, resolveSidecarPath } from "./sidecarClient";
import { BlogPreviewPanel } from "./blogPreviewPanel";

let sidecar: SidecarClient | undefined;

function getSidecar(context: vscode.ExtensionContext): SidecarClient {
  if (sidecar) return sidecar;
  const configured = vscode.workspace.getConfiguration("webeditorSftp").get<string>("sidecarPath");
  const path = resolveSidecarPath(context.extensionPath, configured);
  sidecar = new SidecarClient(path);
  return sidecar;
}

interface KeyInfo {
  label: string;
  algorithm: string;
  public_key_openssh: string;
  fingerprint: string;
}

interface UploadResult {
  remote_path: string;
  bytes_written: number;
  host_key_trust: "trusted_first_time" | "known";
  exec_output?: { exit_status?: number; stdout: string; stderr: string };
}

export function activate(context: vscode.ExtensionContext): void {
  const output = vscode.window.createOutputChannel("webeditor-sftp");
  context.subscriptions.push(output);

  context.subscriptions.push(
    vscode.commands.registerCommand("webeditorSftp.openBlogPreview", async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) {
        void vscode.window.showWarningMessage("プレビューするファイルを開いてください。");
        return;
      }
      try {
        await BlogPreviewPanel.createOrShow(getSidecar(context), editor.document);
      } catch (e) {
        void vscode.window.showErrorMessage(`webeditor-sftp: ${describeError(e)}`);
      }
    }),

    vscode.commands.registerCommand("webeditorSftp.convertToHtml", async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) return;
      try {
        const text = editor.document.getText();
        const html = await getSidecar(context).request<string>("editor.blogToHtml", { text });
        const doc = await vscode.workspace.openTextDocument({ content: html, language: "html" });
        await vscode.window.showTextDocument(doc, { viewColumn: vscode.ViewColumn.Beside });
      } catch (e) {
        void vscode.window.showErrorMessage(`webeditor-sftp: ${describeError(e)}`);
      }
    }),

    vscode.commands.registerCommand("webeditorSftp.convertToBlog", async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) return;
      try {
        const html = editor.document.getText();
        const text = await getSidecar(context).request<string>("editor.htmlToBlog", { html });
        const doc = await vscode.workspace.openTextDocument({ content: text, language: "plaintext" });
        await vscode.window.showTextDocument(doc, { viewColumn: vscode.ViewColumn.Beside });
      } catch (e) {
        void vscode.window.showErrorMessage(`webeditor-sftp: ${describeError(e)}`);
      }
    }),

    vscode.commands.registerCommand("webeditorSftp.generateKeypair", async () => {
      const label = await vscode.window.showInputBox({
        prompt: "鍵ラベル(識別名)を入力してください",
        placeHolder: "例: audiocafe.tokyo-root",
        validateInput: (v) => (v.trim() ? undefined : "ラベルを入力してください"),
      });
      if (!label) return;

      try {
        const info = await getSidecar(context).request<KeyInfo>("sftp.generateKeypair", { label });
        output.appendLine(`鍵ペア生成完了: ${info.label} (${info.fingerprint})`);
        output.appendLine(`公開鍵: ${info.public_key_openssh}`);
        output.show(true);
        void vscode.window.showInformationMessage(
          `鍵ペア "${info.label}" を生成しOSセキュアストレージへ保存しました(フィンガープリント: ${info.fingerprint})。`,
        );
      } catch (e) {
        void vscode.window.showErrorMessage(`webeditor-sftp: ${describeError(e)}`);
      }
    }),

    vscode.commands.registerCommand("webeditorSftp.uploadCurrentFile", async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) {
        void vscode.window.showWarningMessage("アップロードするファイルを開いてください。");
        return;
      }

      const keyLabel = await vscode.window.showInputBox({ prompt: "使用する鍵ラベル" });
      if (!keyLabel) return;
      const host = await vscode.window.showInputBox({ prompt: "ホスト", placeHolder: "例: audiocafe.tokyo" });
      if (!host) return;
      const username = await vscode.window.showInputBox({ prompt: "ユーザー名", placeHolder: "例: root" });
      if (!username) return;
      const remotePath = await vscode.window.showInputBox({
        prompt: "アップロード先パス",
        placeHolder: "例: /var/www/audiocafe.tokyo/public/index.html",
      });
      if (!remotePath) return;
      const portStr = await vscode.window.showInputBox({ prompt: "ポート", value: "22" });
      const port = Number(portStr ?? "22") || 22;
      const execAfterUpload = await vscode.window.showInputBox({
        prompt: "アップロード後に実行するコマンド(任意、空欄可)",
        placeHolder: "例: cd /path && git pull && cargo build --release && systemctl restart ...",
      });

      try {
        const result = await getSidecar(context).request<UploadResult>("sftp.uploadText", {
          host,
          port,
          username,
          keyLabel,
          remotePath,
          content: editor.document.getText(),
          execAfterUpload: execAfterUpload || undefined,
        });

        if (result.host_key_trust === "trusted_first_time") {
          output.appendLine(
            "⚠️ このホストへの初回接続です。ホスト鍵を新規に信頼して記録しました(TOFU)。",
          );
        }
        output.appendLine(`アップロード完了: ${result.remote_path} (${result.bytes_written} bytes)`);
        if (result.exec_output) {
          output.appendLine(`コマンド実行結果 (exit=${result.exec_output.exit_status ?? "unknown"}):`);
          if (result.exec_output.stdout.trim()) output.appendLine(`  stdout: ${result.exec_output.stdout.trim()}`);
          if (result.exec_output.stderr.trim()) output.appendLine(`  stderr: ${result.exec_output.stderr.trim()}`);
        }
        output.show(true);
        void vscode.window.showInformationMessage(`アップロード完了: ${result.remote_path}`);
      } catch (e) {
        void vscode.window.showErrorMessage(`webeditor-sftp: ${describeError(e)}`);
      }
    }),
  );
}

export function deactivate(): void {
  sidecar?.dispose();
  sidecar = undefined;
}

function describeError(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}
