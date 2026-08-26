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

/** How often presence turns into a pulse. Must be well under the daemon's idle
 *  timeout, or the gap between pulses stops counting as time at the editor. */
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
}

let queue: Pulse[] = [];
let token: string | undefined;
let status: vscode.StatusBarItem;
let flushTimer: ReturnType<typeof setInterval> | undefined;
let beatTimer: ReturnType<typeof setInterval> | undefined;

export function activate(context: vscode.ExtensionContext): void {
  status = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 0);
  context.subscriptions.push(status);

  token = readToken();
  showState();

  // What this measures, and why it is measured this way.
  //
  // The question is whether you were at the editor, which is not the same as
  // whether you were typing in it. An earlier version asked the document API —
  // saves, tab switches, caret movements — and over thirty-seven hours of real
  // work it reported nothing at all, because a day spent directing an agent
  // touches none of those: the prompt goes into a panel that is not a document,
  // and the edits come back from something that is not you.
  //
  // Window focus is the one signal that is true of every way of working. While
  // this window has focus you are here, whoever is typing; when it does not,
  // nothing is claimed. That leaves one honest gap, a window left focused while
  // you walk away, which the daemon's idle timeout bounds but cannot see. It is
  // documented rather than papered over, and it is a far smaller error than
  // reporting zero.
  //
  // Agent time is a separate measure taken from a separate source, so an agent
  // working while you are away is counted there and not here. Nothing sums them.
  context.subscriptions.push(
    vscode.window.onDidChangeWindowState((state) => {
      if (state.focused) {
        beat();
        startBeating();
      } else {
        stopBeating();
        void flush();
      }
    }),
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

  // Say hello before any work happens. A window in the background produces no
  // pulses, which is indistinguishable from a plugin that never loaded, and the
  // difference is the first thing anyone wants to know.
  void announce();

  // A window that already has focus when the plugin loads is a window being
  // worked in, and waiting for it to be focused again would lose the session
  // that installed the plugin.
  if (vscode.window.state.focused) {
    beat();
    startBeating();
  }

  flushTimer = setInterval(() => void flush(), FLUSH_SECONDS * 1000);
  context.subscriptions.push({ dispose: stop });
}

export function deactivate(): void {
  stop();
}

function stop(): void {
  stopBeating();
  if (flushTimer) {
    clearInterval(flushTimer);
    flushTimer = undefined;
  }
  void flush();
}

function startBeating(): void {
  if (!beatTimer) {
    beatTimer = setInterval(beat, pulseSeconds() * 1000);
  }
}

function stopBeating(): void {
  if (beatTimer) {
    clearInterval(beatTimer);
    beatTimer = undefined;
  }
}

/// One instant of being at the editor, filed under whatever is open.
function beat(): void {
  if (!enabled() || !vscode.window.state.focused) {
    return;
  }
  queue.push({
    at: Math.floor(Date.now() / 1000),
    day: today(),
    ext: openExtension(),
  });
  if (queue.length > MAX_QUEUED) {
    queue = queue.slice(queue.length - MAX_QUEUED);
  }
  showState();
}

/// The kind of file in front of you, if it is a file at all.
///
/// Output panels, diff views and settings editors are not work on a project, and
/// a window showing one still counts as time — it is filed under no language
/// rather than being dropped, because you were there either way.
function openExtension(): string {
  const document = vscode.window.activeTextEditor?.document;
  if (!document || document.uri.scheme !== "file") {
    return "";
  }
  return extensionOf(document.uri.fsPath);
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

async function announce(): Promise<void> {
  if (!enabled()) {
    return;
  }
  if (!token) {
    token = readToken();
    if (!token) {
      showState();
      return;
    }
  }
  try {
    await post(JSON.stringify({ editor: EDITOR, version: version() }), "/v1/hello");
  } catch {
    // The daemon may not be up yet. There is nothing to keep and nothing lost:
    // the next start says hello again.
  }
}

function version(): string {
  return (
    vscode.extensions.getExtension("liuchong.github-personal-stats-vscode")?.packageJSON
      ?.version ?? ""
  );
}

function post(body: string, path = "/v1/pulses"): Promise<void> {
  return new Promise((resolve, reject) => {
    const target = new URL(path, daemonUrl());
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
        } else if (status === 401) {
          // The token is rotated when the daemon's state is rebuilt, and a
          // cached one then fails for ever. Forgetting it means the next
          // attempt reads the file again, and rejecting keeps the pulses for
          // that attempt rather than throwing away the morning.
          token = undefined;
          reject(new Error("daemon rejected the token"));
        } else if (status >= 400 && status < 500) {
          console.warn(`github-personal-stats: daemon refused ${path} (${status})`);
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
