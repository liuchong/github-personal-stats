// Drives the compiled extension against a running daemon with a stand-in for the
// editor API, so the reporting path can be checked without installing anything.
//
// Usage: node harness.cjs <state-dir> <daemon-url>

const Module = require("node:module");
const path = require("node:path");

const stateDir = process.argv[2];
const daemonUrl = process.argv[3];

const handlers = { change: [], save: [], active: [], selection: [] };
const commands = {};
let statusText = "";

const settings = {
  enabled: true,
  daemonUrl,
  statePath: stateDir,
  pulseSeconds: 5,
};

const vscode = {
  StatusBarAlignment: { Right: 2 },
  window: {
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
    onDidChangeActiveTextEditor: (fn) => (handlers.active.push(fn), { dispose() {} }),
    onDidChangeTextEditorSelection: (fn) => (handlers.selection.push(fn), { dispose() {} }),
    showInformationMessage: () => {},
  },
  workspace: {
    onDidChangeTextDocument: (fn) => (handlers.change.push(fn), { dispose() {} }),
    onDidSaveTextDocument: (fn) => (handlers.save.push(fn), { dispose() {} }),
    getConfiguration: () => ({ get: (name) => settings[name] }),
  },
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

function documentAt(filePath) {
  return { document: { uri: { scheme: "file", fsPath: filePath } } };
}

function check(claim, held) {
  if (!held) {
    throw new Error(`failed: ${claim}`);
  }
  console.log(`ok: ${claim}`);
}

async function main() {
  extension.activate({ subscriptions: [] });

  // A document change says text moved, not that a person did it: an agent
  // editing a file raises it just as typing does. Subscribing to it would put
  // agent work into the measure of the author being at the editor, which is the
  // one thing that measure exists to keep out.
  check("a document change is not treated as someone working", handlers.change.length === 0);
  check("caret movement is watched instead", handlers.selection.length === 1);
  check("switching file is watched", handlers.active.length === 1);
  check("saving is watched", handlers.save.length === 1);

  const files = ["/private/secret-project/src/main.rs", "/private/secret-project/notes.md"];
  for (let index = 0; index < 6; index += 1) {
    handlers.selection.forEach((fn) => fn(documentAt(files[index % 2])));
    // Non-file documents must never be reported.
    handlers.selection.forEach((fn) =>
      fn({ document: { uri: { scheme: "output", fsPath: "extension-output" } } }),
    );
    await new Promise((resolve) => setTimeout(resolve, 5100));
  }

  await commands["githubPersonalStats.sendNow"]();
  console.log(`status line: ${statusText}`);
  extension.deactivate();
  await new Promise((resolve) => setTimeout(resolve, 500));
}

main().then(
  () => process.exit(0),
  (error) => {
    console.error(error);
    process.exit(1);
  },
);
