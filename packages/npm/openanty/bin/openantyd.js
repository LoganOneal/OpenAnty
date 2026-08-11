#!/usr/bin/env node
"use strict";

const { spawn } = require("child_process");
const path = require("path");
const fs = require("fs");
const { ensureBinaries, findBinary } = require("../lib/download");

const packageRoot = path.join(__dirname, "..");

function resolveFromEnv() {
  if (process.env.OPENANTY_BIN_DIR) {
    const ext = process.platform === "win32" ? ".exe" : "";
    const p = path.join(process.env.OPENANTY_BIN_DIR, `openantyd${ext}`);
    if (fs.existsSync(p)) return p;
  }
  if (process.env.OPENANTY_DAEMON) {
    if (fs.existsSync(process.env.OPENANTY_DAEMON))
      return process.env.OPENANTY_DAEMON;
  }
  return null;
}

async function main() {
  // Convenience: `npx openanty mcp` is preferred, but `npx openantyd mcp` also works.
  let bin = resolveFromEnv();
  if (!bin) {
    try {
      bin = findBinary(packageRoot, "daemon");
    } catch (_) {
      await ensureBinaries(packageRoot);
      bin = findBinary(packageRoot, "daemon");
    }
  }
  const child = spawn(bin, process.argv.slice(2), {
    stdio: "inherit",
    env: process.env,
    windowsHide: true,
  });
  child.on("exit", (code, signal) => {
    if (signal) process.kill(process.pid, signal);
    process.exit(code == null ? 1 : code);
  });
}

main().catch((e) => {
  console.error("[openantyd]", e.message || e);
  process.exit(1);
});
