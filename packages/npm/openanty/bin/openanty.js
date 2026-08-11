#!/usr/bin/env node
"use strict";

/**
 * openanty CLI wrapper.
 * Special-case: `openanty mcp` → openantyd mcp (agent install path).
 */

const { spawn } = require("child_process");
const path = require("path");
const fs = require("fs");
const { ensureBinaries, findBinary } = require("../lib/download");

const packageRoot = path.join(__dirname, "..");

function resolveBin(which) {
  const envKey = which === "daemon" ? "OPENANTY_DAEMON" : "OPENANTY_CLI";
  if (process.env[envKey] && fs.existsSync(process.env[envKey])) {
    return process.env[envKey];
  }
  if (process.env.OPENANTY_BIN_DIR) {
    const ext = process.platform === "win32" ? ".exe" : "";
    const name = which === "daemon" ? `openantyd${ext}` : `openanty${ext}`;
    const p = path.join(process.env.OPENANTY_BIN_DIR, name);
    if (fs.existsSync(p)) return p;
  }
  return null;
}

async function getBinary(which) {
  let bin = resolveBin(which);
  if (bin) return bin;
  try {
    return findBinary(packageRoot, which);
  } catch (_) {
    await ensureBinaries(packageRoot);
    return findBinary(packageRoot, which);
  }
}

function run(bin, args) {
  const child = spawn(bin, args, {
    stdio: "inherit",
    env: process.env,
    windowsHide: true,
  });
  child.on("exit", (code, signal) => {
    if (signal) {
      try {
        process.kill(process.pid, signal);
      } catch (_) {
        process.exit(1);
      }
      return;
    }
    process.exit(code == null ? 1 : code);
  });
}

async function main() {
  const args = process.argv.slice(2);
  // Agent path: npx openanty mcp  →  openantyd mcp
  const first = (args[0] || "").toLowerCase();
  if (first === "mcp") {
    const daemon = await getBinary("daemon");
    return run(daemon, args);
  }
  // `openanty mcp-config --npx` still goes to CLI
  const cli = await getBinary("cli");
  return run(cli, args);
}

main().catch((e) => {
  console.error("[openanty]", e.message || e);
  process.exit(1);
});
