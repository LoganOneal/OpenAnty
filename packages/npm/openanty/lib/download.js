"use strict";

const fs = require("fs");
const path = require("path");
const https = require("https");
const http = require("http");
const { platformKey, binaryNames, vendorDir } = require("./platform");

const REPO = process.env.OPENANTY_GITHUB_REPO || "LoganOneal/OpenAnty";
const DEFAULT_VERSION = process.env.OPENANTY_VERSION || require("../package.json").version;

function log(...args) {
  if (process.env.OPENANTY_QUIET === "1") return;
  console.error("[openanty]", ...args);
}

function request(url, redirects = 0) {
  return new Promise((resolve, reject) => {
    const lib = url.startsWith("https") ? https : http;
    const req = lib.get(
      url,
      {
        headers: {
          "User-Agent": "openanty-npm-install",
          Accept: "application/octet-stream",
        },
      },
      (res) => {
        if (
          res.statusCode >= 300 &&
          res.statusCode < 400 &&
          res.headers.location &&
          redirects < 8
        ) {
          res.resume();
          return resolve(request(res.headers.location, redirects + 1));
        }
        if (res.statusCode !== 200) {
          res.resume();
          return reject(
            new Error(`HTTP ${res.statusCode} for ${url}`)
          );
        }
        const chunks = [];
        res.on("data", (c) => chunks.push(c));
        res.on("end", () => resolve(Buffer.concat(chunks)));
        res.on("error", reject);
      }
    );
    req.on("error", reject);
  });
}

async function fetchJson(url) {
  const buf = await request(url);
  return JSON.parse(buf.toString("utf8"));
}

async function resolveRelease(version) {
  // Prefer exact tag vX.Y.Z
  const tag = version.startsWith("v") ? version : `v${version}`;
  try {
    return await fetchJson(
      `https://api.github.com/repos/${REPO}/releases/tags/${tag}`
    );
  } catch (_) {
    log(`tag ${tag} not found, trying latest release`);
    return await fetchJson(
      `https://api.github.com/repos/${REPO}/releases/latest`
    );
  }
}

function assetUrl(release, name) {
  const assets = release.assets || [];
  const hit = assets.find((a) => a.name === name);
  if (!hit) {
    const names = assets.map((a) => a.name).join(", ");
    throw new Error(
      `Release asset ${name} not found. Available: ${names || "(none)"}`
    );
  }
  return hit.browser_download_url || hit.url;
}

async function downloadBinary(url, dest) {
  const buf = await request(url);
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  fs.writeFileSync(dest, buf);
  if (process.platform !== "win32") {
    fs.chmodSync(dest, 0o755);
  }
}

/**
 * Ensure CLI + daemon binaries exist under package vendor/.
 * @param {string} packageRoot
 * @param {{ force?: boolean, version?: string }} opts
 */
async function ensureBinaries(packageRoot, opts = {}) {
  const version = opts.version || DEFAULT_VERSION;
  const key = platformKey();
  const names = binaryNames(key);
  const dir = vendorDir(packageRoot);
  const cliPath = path.join(dir, names.localCli);
  const daemonPath = path.join(dir, names.localDaemon);

  if (
    !opts.force &&
    fs.existsSync(cliPath) &&
    fs.existsSync(daemonPath)
  ) {
    return { cliPath, daemonPath, dir, key, cached: true };
  }

  log(`downloading Open Anty binaries for ${key} (v${version})…`);
  const release = await resolveRelease(version);
  const cliUrl = assetUrl(release, names.cli);
  const daemonUrl = assetUrl(release, names.daemon);
  await downloadBinary(cliUrl, cliPath);
  await downloadBinary(daemonUrl, daemonPath);
  log(`installed to ${dir}`);
  return { cliPath, daemonPath, dir, key, cached: false };
}

function findBinary(packageRoot, which) {
  const key = platformKey();
  const names = binaryNames(key);
  const dir = vendorDir(packageRoot);
  const file =
    which === "daemon" ? names.localDaemon : names.localCli;
  const p = path.join(dir, file);
  if (!fs.existsSync(p)) {
    throw new Error(
      `Binary missing at ${p}. Re-run: npm install openanty  (or set OPENANTY_BIN_DIR)`
    );
  }
  return p;
}

module.exports = {
  ensureBinaries,
  findBinary,
  REPO,
  DEFAULT_VERSION,
};
