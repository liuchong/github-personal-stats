// Drives the compiled extension against a running daemon with a stand-in for the
// editor API, so the reporting path can be checked without installing anything.
//
// Usage: node harness.cjs <state-dir> <daemon-url>

const Module = require("node:module");
const path = require("node:path");

const stateDir = process.argv[2];
const daemonUrl = process.argv[3];

const handlers = { window: [], change: [], save: [], active: [], selection: [] };
const commands = {};
let statusText = "";
let focused = false;
let open = { uri: { scheme: "file", fsPath: "/private/secret-project/src/main.rs" } };

const settings = {
  enabled: true,
  daemonUrl,
  statePath: stateDir,
  pulseSeconds: 5,
};

const vscode = {
  StatusBarAlignment: { Right: 2 },
  window: {
    get state() {
      return { focused };
    },
    get activeTextEditor() {
      return open ? { document: open } : undefined;
    },
    createStatusBarItem: () => ({
      show() {},
      hide() {},
      dispose() {},
      set text(value) {
        statusText = value;
      },
      get text() {
        return statusText;
      },
    }),
    onDidChangeWindowState: (fn) => (handlers.window.push(fn), { dispose() {} }),
    onDidChangeActiveTextEditor: (fn) => (handlers.active.push(fn), { dispose() {} }),
    onDidChangeTextEditorSelection: (fn) => (handlers.selection.push(fn), { dispose() {} }),
    showInformationMessage: () => {},
  },
  workspace: {
    onDidChangeTextDocument: (fn) => (handlers.change.push(fn), { dispose() {} }),
    onDidSaveTextDocument: (fn) => (handlers.save.push(fn), { dispose() {} }),
    getConfiguration: () => ({ get: (name) => settings[name] }),
  },
  extensions: { getExtension: () => undefined },
  commands: {
    registerCommand: (name, fn) => ((commands[name] = fn), { dispose() {} }),
  },
};

const load = Module._load;
Module._load = function (request, parent, isMain) {
  if (request === "vscode") {
    return vscode;
  }
  return load.call(this, request, parent, isMain);
};

const extension = require(path.join(__dirname, "..", "out", "extension.js"));

function focus(now) {
  focused = now;
  handlers.window.forEach((fn) => fn({ focused: now }));
}

function check(claim, held) {
  if (!held) {
    throw new Error(`failed: ${claim}`);
  }
  console.log(`ok: ${claim}`);
}

const waiting = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

async function main() {
  extension.activate({ subscriptions: [] });

  // Focus is the whole signal. The document API is deliberately not subscribed
  // to: a day spent directing an agent raises none of its events, and an agent
  // editing a file raises them whether or not anyone is watching.
  check("window focus is watched", handlers.window.length === 1);
  check("a document change is not treated as someone working", handlers.change.length === 0);
  check("caret movement is not the signal either", handlers.selection.length === 0);

  // A window in the background is not somewhere anybody is working.
  await waiting(11_000);
  check("an unfocused window reports nothing", statusText === "$(pulse) stats");

  focus(true);
  await waiting(11_000);
  check("a focused window reports without being typed in", /stats \d/.test(statusText));

  // Whatever is open when a pulse is taken says what kind of work it was, and a
  // window showing something that is not a file still counts as time.
  open = { uri: { scheme: "output", fsPath: "extension-output" } };
  await waiting(6_000);
  open = { uri: { scheme: "file", fsPath: "/private/secret-project/notes.md" } };
  await waiting(6_000);

  focus(false);
  await commands["githubPersonalStats.sendNow"]();
  console.log(`status line: ${statusText}`);
  extension.deactivate();
  await waiting(500);
}

main().then(
  () => process.exit(0),
  (error) => {
    console.error(error);
    process.exit(1);
  },
);
