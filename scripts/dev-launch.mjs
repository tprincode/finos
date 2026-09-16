#!/usr/bin/env node
// finos coding launch: preflight, then `npm run desktop`, then explain the
// exit code.
//
// `npm run desktop` -> `tauri dev` -> beforeDevCommand `npm run dev` (Vite).
// When the Tauri host stops, it terminates that Vite child and npm prints a
// bare lifecycle failure -- on Windows `code 4294967295`. That line is the
// shutdown, not the cause, and owners have read it as a build failure.

import { spawn } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { preflight } from "./dev-preflight.mjs";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const isWindows = process.platform === "win32";

const EXIT_NOTES = new Map([
  [
    4294967295,
    "-1 (0xFFFFFFFF) on Windows means the process was terminated by something else, not that it reported an error. For the Vite child this is the normal end of a session: the window closed, you pressed Ctrl+C, or File -> Restart ran.",
  ],
  [
    3221225786,
    "0xC000013A is Ctrl+C / console close. Nothing failed.",
  ],
  [
    3221225477,
    "0xC0000005 is an access violation: the host crashed. Look for the last Rust log line above.",
  ],
  [
    3221225794,
    "0xC0000142 means a DLL failed to initialise, usually a missing WebView2 runtime or Visual C++ redistributable.",
  ],
  [9009, "9009 is cmd.exe's \"command not found\". A required tool is not on PATH."],
  [101, "101 is a cargo build or Rust panic failure. The real error is the first `error[E...]` or `error:` line above."],
  [2, "2 is a tsc type-check failure from beforeBuildCommand. Fix the reported TypeScript errors."],
]);

function explain(code) {
  const lines = [
    "",
    `finos: the desktop session ended with exit code ${code}.`,
  ];
  const note = EXIT_NOTES.get(code);
  if (note) lines.push(`  ${note}`);
  lines.push(
    "",
    "  The first error above is the real one; npm only repeats the last child's code.",
    "  To capture the whole session:",
    isWindows
      ? "    npm run desktop > finos-dev.log 2>&1"
      : "    npm run desktop > finos-dev.log 2>&1",
    "  To see what npm itself did:",
    isWindows
      ? "    npm run desktop --loglevel verbose   (full log: %LOCALAPPDATA%\\npm-cache\\_logs)"
      : "    npm run desktop --loglevel verbose   (full log: ~/.npm/_logs)",
    "  To run the pieces separately:",
    "    npm run doctor                                 checks only",
    "    npm run dev:vite                               frontend dev server only (no window)",
    "    cargo build --manifest-path apps/desktop/src-tauri/Cargo.toml   Rust host only",
    "",
  );
  process.stdout.write(lines.join("\n"));
}

// tauri.conf.json's beforeDevCommand must stay `npm run dev:vite`. If it ever
// points back at a `dev` script that reaches this launcher, stop instead of
// spawning tauri forever.
if (process.env.FINOS_DEV_LAUNCH) {
  process.stderr.write(
    "\nfinos: the coding launch re-entered itself.\n" +
      "  apps/desktop/src-tauri/tauri.conf.json beforeDevCommand must be `npm run dev:vite`.\n\n",
  );
  process.exit(1);
}

const preflightCode = await preflight();
if (preflightCode !== 0) {
  process.exit(preflightCode);
}

// process.execPath + npm-cli.js avoids Windows' refusal to spawn npm.cmd
// without a shell, and guarantees the same npm that started this script.
const npmExecPath = process.env.npm_execpath;
const [command, args] = npmExecPath
  ? [process.execPath, [npmExecPath, "run", "desktop"]]
  : [isWindows ? "npm.cmd" : "npm", ["run", "desktop"]];

const child = spawn(command, args, {
  cwd: repoRoot,
  stdio: "inherit",
  shell: !npmExecPath && isWindows,
  env: { ...process.env, FINOS_PREFLIGHT_DONE: "1", FINOS_DEV_LAUNCH: "1" },
});

child.on("error", (err) => {
  process.stdout.write(`\nfinos: could not start npm (${err.message}).\n`);
  process.exit(1);
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.stdout.write(`\nfinos: the desktop session was stopped (${signal}).\n`);
    process.exit(0);
  }
  if (code === 0) process.exit(0);
  explain(code >>> 0);
  process.exit(code === null ? 1 : code);
});
