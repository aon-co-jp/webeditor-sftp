import * as vscode from "vscode";
import { SidecarClient } from "./sidecarClient";

/**
 * 現在編集中のHTMLを「ブログモード」(タグを意識しない文章表示)で
 * プレビューするWebviewパネル。変換ロジック自体はRust側
 * (webeditor-core::editor)にあり、ここではサイドカーへ投げて結果を
 * 表示するだけ。
 */
export class BlogPreviewPanel {
  private static current: BlogPreviewPanel | undefined;
  private readonly panel: vscode.WebviewPanel;
  private disposables: vscode.Disposable[] = [];

  private constructor(
    panel: vscode.WebviewPanel,
    private readonly sidecar: SidecarClient,
    private readonly document: vscode.TextDocument,
  ) {
    this.panel = panel;
    this.panel.onDidDispose(() => this.dispose(), null, this.disposables);
    void this.refresh();
  }

  static async createOrShow(sidecar: SidecarClient, document: vscode.TextDocument): Promise<void> {
    const column = vscode.window.activeTextEditor?.viewColumn;

    if (BlogPreviewPanel.current) {
      BlogPreviewPanel.current.panel.reveal(column);
      BlogPreviewPanel.current.document === document && (await BlogPreviewPanel.current.refresh());
      return;
    }

    const panel = vscode.window.createWebviewPanel(
      "webeditorSftp.blogPreview",
      `ブログモード: ${document.fileName.split(/[\\/]/).pop()}`,
      column ?? vscode.ViewColumn.Beside,
      { enableScripts: false },
    );
    BlogPreviewPanel.current = new BlogPreviewPanel(panel, sidecar, document);
  }

  private async refresh(): Promise<void> {
    const html = this.document.getText();
    try {
      const blogText = await this.sidecar.request<string>("editor.htmlToBlog", { html });
      this.panel.webview.html = this.renderHtml(blogText);
    } catch (e) {
      this.panel.webview.html = this.renderError(e instanceof Error ? e.message : String(e));
    }
  }

  private renderHtml(blogText: string): string {
    const escaped = escapeHtml(blogText);
    return `<!doctype html>
<html><head><meta charset="utf-8">
<style>
  body { font-family: system-ui, sans-serif; padding: 16px; white-space: pre-wrap; line-height: 1.6; }
  .hint { color: var(--vscode-descriptionForeground); font-size: 12px; margin-bottom: 12px; }
</style></head>
<body>
  <div class="hint">HTMLタグを意識しない「ブログモード」表示(読み取り専用プレビュー)。編集は元のHTMLファイル側で行ってください。</div>
  <div>${escaped}</div>
</body></html>`;
  }

  private renderError(message: string): string {
    return `<!doctype html><html><body style="font-family:sans-serif;padding:16px;color:var(--vscode-errorForeground);">
      <b>変換エラー</b><pre>${escapeHtml(message)}</pre></body></html>`;
  }

  private dispose(): void {
    BlogPreviewPanel.current = undefined;
    this.disposables.forEach((d) => d.dispose());
  }
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/\n/g, "<br>");
}
