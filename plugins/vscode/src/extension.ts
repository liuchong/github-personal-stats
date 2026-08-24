// Reports pulses to the local daemon while you work.
//
// What leaves this process is a timestamp, a local date, a file extension and
// whether the file was being changed. No path, no project name, no repository, no
// file content. The extension is the only thing that sees a path, and it keeps it.

import * as fs from "node:fs";
import * as http from "node:http";
import * as os from "node:os";
import * as path from "node:path";
import * as vscode from "vscode";

const EDITOR = "vscode";
const STATE_DIR = "github-personal-stats";
const TOKEN_FILE = "token";
const DEFAULT_URL = "http://127.0.0.1:7391";

/** How often work in progress turns into a pulse. Must be well under the
 *  daemon's idle timeout, or time between pulses stops counting as work. */
const DEFAULT_PULSE_SECONDS = 30;

/** How often queued pulses are sent. Sending is batched so typing does not
 *  become a request per keystroke. */
const FLUSH_SECONDS = 60;

/** The queue is bounded so a daemon that stays down cannot grow it without
 *  limit. The oldest pulses are dropped first: recent work matters more. */
const MAX_QUEUED = 2000;

const SAFE_EXTENSION = /^[a-z0-9-]{1,24}$/;

interface Pulse {
  at: number;
  day: string;
  ext: string;
  write: boolean;
}

let queue: Pulse[] = [];
let lastPulseAt = 0;
let token: string | undefined;
let status: vscode.StatusBarItem;
let flushTimer: ReturnType<typeof setInterval> | undefined;

export function activate(context: vscode.ExtensionContext): void {
  status = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 0);
  context.subscriptions.push(status);

  token = readToken();
  showState();

  const seen = (write: boolean) => (subject: unknown) => {
    if (!enabled()) {
      return;
    }
    const document = documentOf(subject);
    if (document) {
      record(document, write);
    }
  };

  context.subscriptions.push(
    vscode.workspace.onDidChangeTextDocument(seen(true)),
    vscode.workspace.onDidSaveTextDocument(seen(true)),
    vscode.window.onDidChangeActiveTextEditor(seen(false)),
    vscode.window.onDidChangeTextEditorSelection(seen(false)),
    vscode.commands.registerCommand("githubPersonalStats.sendNow", async () => {
      token = readToken();
      await flush();
      showState();
      vscode.window.showInformationMessage(
        token
          ? `Reporting to ${daemonUrl()}. ${queue.length} pulses waiting.`
          : `No token at ${tokenPath()}. Start the daemon first.`,
      );
    }),
  );

  flushTimer = setInterval(() => void flush(), FLUSH_SECONDS * 1000);
  context.subscriptions.push({
    dispose: () => {
      if (flushTimer) {
        clearInterval(flushTimer);
      }
      void flush();
    },
  });
}

export function deactivate(): void {
  if (flushTimer) {
    clearInterval(flushTimer);
  }
  void flush();
}

function documentOf(subject: unknown): vscode.TextDocument | undefined {
  if (!subject || typeof subject !== "object") {
    return undefined;
  }
  const candidate = subject as {
    document?: vscode.TextDocument;
    textEditor?: { document?: vscode.TextDocument };
    uri?: vscode.Uri;
  };
  if (candidate.textEditor?.document) {
    return candidate.textEditor.document;
  }
  if (candidate.document) {
    return candidate.document;
  }
  return candidate.uri ? (subject as vscode.TextDocument) : undefined;
}

function record(document: vscode.TextDocument, write: boolean): void {
  // Only real files on disk. Output panels, diff views and settings editors are
  // not work on a project and would inflate the record.
  if (document.uri.scheme !== "file") {
    return;
  }

  const now = Math.floor(Date.now() / 1000);
  if (now - lastPulseAt < pulseSeconds()) {
    return;
  }
  lastPulseAt = now;

  queue.push({ at: now, day: today(), ext: extensionOf(document.uri.fsPath), write });
  if (queue.length > MAX_QUEUED) {
    queue = queue.slice(queue.length - MAX_QUEUED);
  }
  showState();
}

/** The local date, as this machine sees it. The daemon records the day the
 *  editor reported rather than deriving one, so a journal read later, or under a
 *  changed timezone, still lands in the day the work happened. */
function today(): string {
  const now = new Date();
  const month = `${now.getMonth() + 1}`.padStart(2, "0");
  const day = `${now.getDate()}`.padStart(2, "0");
  return `${now.getFullYear()}-${month}-${day}`;
}

function extensionOf(filePath: string): string {
  const found = path.extname(filePath).replace(/^\./, "").toLowerCase();
  if (found && SAFE_EXTENSION.test(found)) {
    return found;
  }
  // A name with no extension, or one strange enough to be worth not sending,
  // still counts as time: the daemon files it under an unknown kind.
  return "";
}

async function flush(): Promise<void> {
  if (queue.length === 0 || !enabled()) {
    return;
  }
  if (!token) {
    token = readToken();
    if (!token) {
      showState();
      return;
    }
  }

  const sending = queue;
  queue = [];

  try {
    await post(JSON.stringify({ editor: EDITOR, pulses: sending }));
  } catch {
    // The daemon may simply not be running. Keep the pulses and try again on the
    // next tick, so a restart does not cost the morning's work.
    queue = sending.concat(queue).slice(-MAX_QUEUED);
  }
  showState();
}

function post(body: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const target = new URL("/v1/pulses", daemonUrl());
    const request = http.request(
      {
        hostname: target.hostname,
        port: target.port,
        path: target.pathname,
        method: "POST",
        timeout: 5000,
        headers: {
          "content-type": "application/json",
          "content-length": Buffer.byteLength(body),
          authorization: `Bearer ${token}`,
        },
      },
      (response) => {
        response.resume();
        const status = response.statusCode ?? 0;
        // A refusal is the daemon's final answer, so the pulses are dropped
        // rather than retried for ever. Anything else is worth trying again.
        if (status >= 200 && status < 300) {
          resolve();
        } else if (status >= 400 && status < 500) {
          console.warn(`github-personal-stats: daemon refused pulses (${status})`);
          resolve();
        } else {
          reject(new Error(`daemon answered ${status}`));
        }
      },
    );
    request.on("timeout", () => request.destroy(new Error("daemon did not answer")));
    request.on("error", reject);
    request.write(body);
    request.end();
  });
}

function readToken(): string | undefined {
  try {
    const found = fs.readFileSync(tokenPath(), "utf8").trim();
    return found.length === 64 ? found : undefined;
  } catch {
    return undefined;
  }
}

function tokenPath(): string {
  const configured = settings().get<string>("statePath");
  if (configured) {
    return path.join(configured, TOKEN_FILE);
  }
  const base =
    process.env.XDG_STATE_HOME ?? path.join(os.homedir(), ".local", "state");
  return path.join(base, STATE_DIR, TOKEN_FILE);
}

function daemonUrl(): string {
  return settings().get<string>("daemonUrl") || DEFAULT_URL;
}

function pulseSeconds(): number {
  const configured = settings().get<number>("pulseSeconds") ?? DEFAULT_PULSE_SECONDS;
  return Math.min(Math.max(configured, 5), 240);
}

function enabled(): boolean {
  return settings().get<boolean>("enabled") ?? true;
}

function settings(): vscode.WorkspaceConfiguration {
  return vscode.workspace.getConfiguration("githubPersonalStats");
}

function showState(): void {
  if (!enabled()) {
    status.hide();
    return;
  }
  if (!token) {
    status.text = "$(circle-slash) stats";
    status.tooltip = `No daemon token at ${tokenPath()}. Start github-personal-stats-daemon.`;
  } else if (queue.length > 0) {
    status.text = `$(pulse) stats ${queue.length}`;
    status.tooltip = `${queue.length} pulses waiting to reach ${daemonUrl()}.`;
  } else {
    status.text = "$(pulse) stats";
    status.tooltip = `Reporting to ${daemonUrl()}.`;
  }
  status.command = "githubPersonalStats.sendNow";
  status.show();
}
