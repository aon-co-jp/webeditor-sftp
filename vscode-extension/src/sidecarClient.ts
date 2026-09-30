import * as cp from "child_process";
import * as readline from "readline";
import * as fs from "fs";
import * as path from "path";

/**
 * webeditor-sidecar(Rust製CLI)を子プロセスとして起動し、標準入出力の
 * JSON Linesプロトコルで通信するクライアント。
 *
 * プロトコル(cli/src/main.rsと対になる):
 *   → {"id": <number>, "method": "...", "params": {...}}
 *   ← {"id": <number>, "ok": true, "result": ...}
 *   ← {"id": <number>, "ok": false, "error": "..."}
 *
 * 実装ロジック本体はRust側(webeditor-core)に一本化してあり、この
 * クライアントは単なる薄い通信レイヤー(このファイルにロジックの
 * 複製を作らないこと)。
 */
export class SidecarClient {
  private process: cp.ChildProcessWithoutNullStreams | undefined;
  private rl: readline.Interface | undefined;
  private nextId = 1;
  private pending = new Map<number, { resolve: (v: unknown) => void; reject: (e: Error) => void }>();
  private stderrBuffer: string[] = [];

  constructor(private readonly sidecarPath: string) {}

  private ensureStarted(): void {
    if (this.process) return;

    if (!fs.existsSync(this.sidecarPath)) {
      throw new Error(
        `webeditor-sidecar実行ファイルが見つかりません: ${this.sidecarPath}\n` +
          `設定 "webeditorSftp.sidecarPath" でパスを指定するか、` +
          `開発時は "cargo build -p webeditor-sidecar" を実行してください。`,
      );
    }

    this.process = cp.spawn(this.sidecarPath, [], { stdio: ["pipe", "pipe", "pipe"] });

    this.rl = readline.createInterface({ input: this.process.stdout });
    this.rl.on("line", (line) => this.handleLine(line));

    this.process.stderr.on("data", (chunk: Buffer) => {
      this.stderrBuffer.push(chunk.toString());
      if (this.stderrBuffer.length > 200) this.stderrBuffer.shift();
    });

    this.process.on("exit", (code) => {
      const err = new Error(
        `webeditor-sidecarが終了しました(code=${code})。` +
          (this.stderrBuffer.length ? `\nstderr:\n${this.stderrBuffer.join("")}` : ""),
      );
      for (const [, waiter] of this.pending) waiter.reject(err);
      this.pending.clear();
      this.process = undefined;
      this.rl = undefined;
    });
  }

  private handleLine(line: string): void {
    if (!line.trim()) return;
    let msg: { id: number; ok: boolean; result?: unknown; error?: unknown };
    try {
      msg = JSON.parse(line);
    } catch {
      // サイドカー側のログ出力等、JSONでない行は無視する。
      return;
    }
    const waiter = this.pending.get(msg.id);
    if (!waiter) return;
    this.pending.delete(msg.id);
    if (msg.ok) {
      waiter.resolve(msg.result);
    } else {
      waiter.reject(new Error(typeof msg.error === "string" ? msg.error : JSON.stringify(msg.error)));
    }
  }

  /** サイドカーへ1リクエストを送り、応答(result)を待つ。 */
  request<T = unknown>(method: string, params: Record<string, unknown> = {}): Promise<T> {
    this.ensureStarted();
    const id = this.nextId++;
    const payload = JSON.stringify({ id, method, params }) + "\n";

    return new Promise<T>((resolve, reject) => {
      this.pending.set(id, { resolve: resolve as (v: unknown) => void, reject });
      this.process!.stdin.write(payload, (err) => {
        if (err) {
          this.pending.delete(id);
          reject(err);
        }
      });
    });
  }

  dispose(): void {
    this.rl?.close();
    this.process?.kill();
    this.process = undefined;
  }
}

/**
 * サイドカー実行ファイルの場所を解決する。
 * 1. 設定 `webeditorSftp.sidecarPath` が指定されていればそれを使う。
 * 2. 開発時: 拡張機能フォルダの兄弟である `cli/target/{release,debug}/`
 *    配下を探す(このリポジトリのモノレポ構成前提、配布パッケージング
 *    (VSIX同梱)は次回対応、PORTING.md参照)。
 */
export function resolveSidecarPath(extensionPath: string, configuredPath: string | undefined): string {
  if (configuredPath && configuredPath.trim().length > 0) {
    return configuredPath;
  }

  const exeName = process.platform === "win32" ? "webeditor-sidecar.exe" : "webeditor-sidecar";
  const repoRoot = path.resolve(extensionPath, ".."); // vscode-extension/ の1つ上 = リポジトリルート
  const candidates = [
    path.join(repoRoot, "cli", "target", "release", exeName),
    path.join(repoRoot, "cli", "target", "debug", exeName),
    path.join(repoRoot, "target", "release", exeName),
    path.join(repoRoot, "target", "debug", exeName),
    path.join(extensionPath, "bin", exeName), // 将来のVSIX同梱先
  ];
  return candidates.find((p) => fs.existsSync(p)) ?? candidates[0];
}
