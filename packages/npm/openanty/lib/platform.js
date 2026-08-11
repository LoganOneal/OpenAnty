"use strict";

/**
 * Map Node process.platform / arch → GitHub release asset names.
 * Assets: openanty-windows-x64.exe, openantyd-linux-x64, openanty-macos-arm64, ...
 */
function platformKey() {
  const p = process.platform;
  const a = process.arch;
  if (p === "win32" && a === "x64") return "windows-x64";
  if (p === "win32" && a === "arm64") return "windows-x64"; // fallback until arm64 builds
  if (p === "linux" && (a === "x64" || a === "x86_64")) return "linux-x64";
  if (p === "darwin" && a === "arm64") return "macos-arm64";
  if (p === "darwin" && a === "x64") return "macos-x64";
  throw new Error(
    `Unsupported platform ${p}/${a}. Download binaries from https://github.com/LoganOneal/OpenAnty/releases`
  );
}

function binaryNames(key) {
  const ext = key.startsWith("windows") ? ".exe" : "";
  return {
    cli: `openanty-${key}${ext}`,
    daemon: `openantyd-${key}${ext}`,
    localCli: `openanty${ext}`,
    localDaemon: `openantyd${ext}`,
  };
}

function vendorDir(packageRoot) {
  const path = require("path");
  return path.join(packageRoot, "vendor", platformKey());
}

module.exports = { platformKey, binaryNames, vendorDir };
