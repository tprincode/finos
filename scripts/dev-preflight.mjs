#!/usr/bin/env node
// finos dev/build preflight.
//
// `tauri dev` reports a failed `beforeDevCommand` as a bare npm lifecycle
// error (on Windows usually `code 4294967295`, i.e. -1), which says nothing
// about the cause. This runs the cheap checks first and names the blocker.
//
//   node scripts/dev-preflight.mjs            checks for the coding launch
//   node scripts/dev-preflight.mjs --build    checks for npm run desktop:build
//
// Set FINOS_PREFLIGHT_SKIP=1 to bypass every check.

import { spawnSync } from "node:child_process";
import { createServer } from "node:net";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const isWindows = process.platform === "win32";
const buildMode = process.argv.includes("--build");
const DEV_PORT = 1420;
const HMR_PORT = 1421;

const results = [];
const ok = (label, detail) => results.push({ level: "ok", label, detail });
const warn = (label, detail, fix) =>
  results.push({ level: "warn", label, detail, fix });
const fail = (label, detail, fix) =>
  results.push({ level: "FAIL", label, detail, fix });

/** Never let a probe hang or throw the preflight itself. */
function run(cmd, args, timeout = 8000) {
  try {
    const r = spawnSync(cmd, args, {
      encoding: "utf8",
      timeout,
      windowsHide: true,
      shell: false,
    });
    if (r.error) return null;
    return { status: r.status, stdout: r.stdout || "", stderr: r.stderr || "" };
  } catch {
    return null;
  }
}

function readJson(path) {
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch {
    return null;
  }
}

function parseVersion(text) {
  const m = /(\d+)\.(\d+)\.(\d+)/.exec(text || "");
  return m ? [Number(m[1]), Number(m[2]), Number(m[3])] : null;
}

function cmpVersion(a, b) {
  for (let i = 0; i < 3; i += 1) {
    if (a[i] !== b[i]) return a[i] - b[i];
  }
  return 0;
}

/** Handles the `^x.y.z || >=x.y.z` shapes npm engines fields actually use. */
function satisfiesEngine(version, range) {
  const clauses = range.split("||").map((c) => c.trim());
  let understood = false;
  for (const clause of clauses) {
    const want = parseVersion(clause);
    if (!want) continue;
    understood = true;
    if (clause.startsWith("^")) {
      if (version[0] === want[0] && cmpVersion(version, want) >= 0) return true;
    } else if (clause.startsWith(">=")) {
      if (cmpVersion(version, want) >= 0) return true;
    } else if (cmpVersion(version, want) === 0) {
      return true;
    }
  }
  return understood ? false : true;
}

function portHolder(port) {
  if (isWindows) {
    const out = run("netstat", ["-ano", "-p", "tcp"]);
    if (!out) return null;
    for (const line of out.stdout.split(/\r?\n/)) {
      if (!/LISTENING/i.test(line)) continue;
      const cols = line.trim().split(/\s+/);
      const local = cols[1] || "";
      if (!new RegExp(`:${port}$`).test(local)) continue;
      const pid = cols[cols.length - 1];
      const task = run("tasklist", ["/FI", `PID eq ${pid}`, "/NH", "/FO", "CSV"]);
      const name = task ? /^"([^"]+)"/.exec(task.stdout.trim())?.[1] : null;
      return { pid, name: name || "unknown" };
    }
    return null;
  }
  const lsof = run("lsof", ["-nP", `-iTCP:${port}`, "-sTCP:LISTEN", "-t"]);
  const pid = lsof?.stdout.trim().split(/\s+/)[0];
  return pid ? { pid, name: "unknown" } : null;
}

function bindOnce(port, host) {
  return new Promise((done) => {
    const server = createServer();
    server.once("error", (err) => done(err.code || "EADDRINUSE"));
    server.once("listening", () => server.close(() => done(null)));
    server.listen(port, host);
  });
}

/**
 * File -> Restart tears the old stack down while the new one starts, so a
 * momentarily held port is normal. Only a port still held after the grace
 * window is a real blocker.
 */
async function portFree(port, graceMs = 6000) {
  const deadline = Date.now() + graceMs;
  for (;;) {
    const v4 = await bindOnce(port, "127.0.0.1");
    const v6 = v4 ? null : await bindOnce(port, "::1");
    const busy = v4 === "EADDRINUSE" || v6 === "EADDRINUSE";
    if (!busy) return true;
    if (Date.now() >= deadline) return false;
    await new Promise((r) => setTimeout(r, 500));
  }
}

function checkRepo() {
  const pkg = readJson(join(repoRoot, "package.json"));
  if (!pkg || pkg.name !== "finos") {
    fail(
      "repo root",
      `${repoRoot} is not the finos repo root`,
      "Run npm commands from the repo root (the folder holding package.json and apps/).",
    );
    return false;
  }
  ok("repo root", repoRoot);
  if (/[\\/]onedrive[\\/]/i.test(repoRoot) || /[\\/]dropbox[\\/]/i.test(repoRoot)) {
    warn(
      "repo location",
      "the checkout is inside a cloud-sync folder",
      "Sync holds file handles open and intermittently breaks cargo target/ and the SQLite file. Move the checkout outside OneDrive/Dropbox.",
    );
  }
  return true;
}

function checkNode() {
  const version = parseVersion(process.version);
  const viteEngines = readJson(
    join(repoRoot, "node_modules", "vite", "package.json"),
  )?.engines?.node;
  const range = viteEngines || "^20.19.0 || >=22.12.0";
  if (version && !satisfiesEngine(version, range)) {
    fail(
      "node",
      `${process.version} does not satisfy vite's requirement ${range}`,
      "Install a supported Node LTS (https://nodejs.org) and reopen the terminal so PATH picks it up.",
    );
    return;
  }
  ok("node", process.version);
}

function checkDependencies() {
  if (!existsSync(join(repoRoot, "node_modules"))) {
    fail(
      "dependencies",
      "node_modules is missing",
      "Run: npm install",
    );
    return;
  }
  const cliPkg = readJson(
    join(repoRoot, "node_modules", "@tauri-apps", "cli", "package.json"),
  );
  const tauriBin = isWindows
    ? join(repoRoot, "node_modules", ".bin", "tauri.cmd")
    : join(repoRoot, "node_modules", ".bin", "tauri");
  if (!cliPkg || !existsSync(tauriBin)) {
    fail(
      "dependencies",
      "the tauri CLI is not linked into node_modules/.bin",
      "Run: npm install  (if it still fails, delete node_modules and package-lock.json is NOT needed — just rerun npm ci)",
    );
    return;
  }
  ok("dependencies", `@tauri-apps/cli ${cliPkg.version}`);
}

function checkRust() {
  const rustc = run("rustc", ["--version"]);
  if (!rustc || rustc.status !== 0) {
    fail(
      "rust",
      "rustc is not on PATH",
      isWindows
        ? 'Install Rust from https://rustup.rs, then reopen the terminal. If it is already installed, add %USERPROFILE%\\.cargo\\bin to PATH (apps\\desktop\\start-finos-dev.bat does this for you).'
        : "Install Rust from https://rustup.rs, then reopen the terminal.",
    );
    return;
  }
  const version = parseVersion(rustc.stdout);
  // Tauri 2 does not build on toolchains older than this.
  if (version && cmpVersion(version, [1, 77, 2]) < 0) {
    fail(
      "rust",
      `${rustc.stdout.trim()} is older than the 1.77.2 Tauri 2 needs`,
      "Run: rustup update stable",
    );
    return;
  }
  ok("rust", rustc.stdout.trim());
}

function checkWindowsToolchain() {
  if (!isWindows) return;

  const webview2Keys = [
    "HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
    "HKLM\\SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
    "HKCU\\SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
  ];
  const webview2 = webview2Keys.some((key) => {
    const r = run("reg", ["query", key, "/v", "pv"]);
    return r != null && r.status === 0 && /pv\s+REG_SZ\s+\S/i.test(r.stdout);
  });
  if (webview2) {
    ok("WebView2", "runtime present");
  } else {
    warn(
      "WebView2",
      "the Edge WebView2 runtime was not detected",
      "The Tauri window stays blank or the host exits without a message. Install the Evergreen runtime: https://developer.microsoft.com/microsoft-edge/webview2/",
    );
  }

  const vswhere = join(
    process.env["ProgramFiles(x86)"] || "C:\\Program Files (x86)",
    "Microsoft Visual Studio",
    "Installer",
    "vswhere.exe",
  );
  if (!existsSync(vswhere)) {
    warn(
      "MSVC build tools",
      "vswhere.exe is missing, so Visual Studio C++ tools could not be confirmed",
      "cargo needs link.exe. Install Build Tools for Visual Studio with the \"Desktop development with C++\" workload.",
    );
    return;
  }
  const found = run(vswhere, [
    "-latest",
    "-products",
    "*",
    "-requires",
    "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
    "-property",
    "installationPath",
  ]);
  if (found && found.status === 0 && found.stdout.trim()) {
    ok("MSVC build tools", found.stdout.trim().split(/\r?\n/)[0]);
  } else {
    warn(
      "MSVC build tools",
      "no Visual Studio install with the C++ x64 tools was found",
      'cargo will fail with "linker `link.exe` not found". Install Build Tools for Visual Studio with the "Desktop development with C++" workload.',
    );
  }
}

function checkRunningDesktop() {
  if (!isWindows) return;
  const task = run("tasklist", [
    "/FI",
    "IMAGENAME eq finos-desktop.exe",
    "/NH",
    "/FO",
    "CSV",
  ]);
  if (!task || !/finos-desktop\.exe/i.test(task.stdout)) {
    ok("running copy", "no finos-desktop.exe is running");
    return;
  }
  fail(
    "running copy",
    "finos-desktop.exe is already running",
    buildMode
      ? "The bundler cannot overwrite a running exe. Quit finos (installed window and any dev console), then rerun."
      : "The live SQLite file cannot be opened twice. Quit the other finos window, then rerun. Force: taskkill /IM finos-desktop.exe /F",
  );
}

async function checkPorts() {
  if (buildMode) return;
  if (await portFree(DEV_PORT)) {
    ok(`port ${DEV_PORT}`, "free");
  } else {
    const holder = portHolder(DEV_PORT);
    fail(
      `port ${DEV_PORT}`,
      holder
        ? `held by ${holder.name} (PID ${holder.pid})`
        : "still in use after waiting 6s",
      [
        `vite.config.ts pins ${DEV_PORT} with strictPort, so Vite exits instead of picking another port.`,
        holder && isWindows
          ? `Close that window, or: taskkill /PID ${holder.pid} /T /F`
          : holder
            ? `Close that process, or: kill ${holder.pid}`
            : isWindows
              ? `Find it with: netstat -ano | findstr :${DEV_PORT}`
              : `Find it with: lsof -nP -iTCP:${DEV_PORT} -sTCP:LISTEN`,
      ].join(" "),
    );
  }
  if (!(await portFree(HMR_PORT, 0))) {
    const holder = portHolder(HMR_PORT);
    warn(
      `port ${HMR_PORT}`,
      holder ? `held by ${holder.name} (PID ${holder.pid})` : "in use",
      "Only used for HMR when TAURI_DEV_HOST is set. Harmless otherwise.",
    );
  }
}

function checkUpdaterSigningKey() {
  if (!buildMode) return;
  const conf = readJson(
    join(repoRoot, "apps", "desktop", "src-tauri", "tauri.conf.json"),
  );
  if (!conf) return;
  const wantsArtifacts = conf.bundle?.createUpdaterArtifacts === true;
  const hasPubkey = Boolean(conf.plugins?.updater?.pubkey);
  if (!wantsArtifacts || !hasPubkey) return;
  if (process.env.TAURI_SIGNING_PRIVATE_KEY) {
    ok("updater signing key", "TAURI_SIGNING_PRIVATE_KEY is set");
    return;
  }
  fail(
    "updater signing key",
    "TAURI_SIGNING_PRIVATE_KEY is not set, but tauri.conf.json has createUpdaterArtifacts and an updater pubkey",
    "The bundler refuses to sign the update artifact and aborts after the full release compile. Set TAURI_SIGNING_PRIVATE_KEY (and TAURI_SIGNING_PRIVATE_KEY_PASSWORD) for this shell, or bypass this check with FINOS_PREFLIGHT_SKIP=1 to see the bundler's own error.",
  );
}

function checkSigningCertificate() {
  if (!buildMode || !isWindows) return;
  const conf = readJson(
    join(repoRoot, "apps", "desktop", "src-tauri", "tauri.conf.json"),
  );
  const thumbprint = conf?.bundle?.windows?.certificateThumbprint;
  if (!thumbprint) return;
  const store = run("certutil", ["-user", "-store", "My", thumbprint]);
  const machine = store?.status === 0 ? null : run("certutil", ["-store", "My", thumbprint]);
  if (store?.status === 0 || machine?.status === 0) {
    ok("signing certificate", `${thumbprint} found`);
    return;
  }
  warn(
    "signing certificate",
    `certificate ${thumbprint} was not found in the My store`,
    "tauri.conf.json asks signtool for that thumbprint; bundling fails at the very end if it is missing. Import the certificate, or clear bundle.windows.certificateThumbprint for an unsigned local build.",
  );
}

function checkProfileA() {
  const localAppData = process.env.LOCALAPPDATA;
  if (!localAppData) return;
  const candidates = [
    join(localAppData, "com.finos.desktop", "local.sqlite"),
    join(localAppData, "finos", "local.sqlite"),
  ];
  const found = candidates.find((p) => existsSync(p));
  if (found) {
    ok("Profile A data", found);
    return;
  }
  warn(
    "Profile A data",
    `no local.sqlite under ${join(localAppData, "com.finos.desktop")}`,
    "The app opens with empty screens until the one-time seed runs: npm run data-seed",
  );
}

function report() {
  const target = buildMode ? "desktop:build" : "dev";
  const width = Math.max(...results.map((r) => r.label.length));
  process.stdout.write(
    `\nfinos preflight (${buildMode ? "desktop:build" : "coding launch"})\n`,
  );
  for (const r of results) {
    const tag = r.level.padEnd(4);
    process.stdout.write(`  ${tag}  ${r.label.padEnd(width)}  ${r.detail}\n`);
    if (r.fix) process.stdout.write(`        ${" ".repeat(width)}  -> ${r.fix}\n`);
  }
  const failures = results.filter((r) => r.level === "FAIL");
  if (failures.length === 0) {
    process.stdout.write("\npreflight ok\n\n");
    return 0;
  }
  process.stdout.write(
    [
      "",
      `preflight found ${failures.length} blocker${failures.length === 1 ? "" : "s"}. Fix the -> lines above and rerun.`,
      "",
      "To bypass these checks and see the raw tool output instead:",
      isWindows
        ? `  set FINOS_PREFLIGHT_SKIP=1 && npm run ${target}`
        : `  FINOS_PREFLIGHT_SKIP=1 npm run ${target}`,
      "",
    ].join("\n"),
  );
  return 1;
}

export async function preflight() {
  // scripts/dev-launch.mjs already ran the same checks in this process tree.
  if (process.env.FINOS_PREFLIGHT_DONE) return 0;
  if (process.env.FINOS_PREFLIGHT_SKIP) {
    process.stdout.write("finos preflight: skipped (FINOS_PREFLIGHT_SKIP)\n");
    return 0;
  }
  if (!checkRepo()) return report();
  checkNode();
  checkDependencies();
  checkRust();
  checkWindowsToolchain();
  checkRunningDesktop();
  await checkPorts();
  checkUpdaterSigningKey();
  checkSigningCertificate();
  checkProfileA();
  return report();
}

const invokedDirectly =
  process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (invokedDirectly) {
  process.exitCode = await preflight();
}
