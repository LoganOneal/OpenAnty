"use strict";

/**
 * Download platform binaries from GitHub Releases after npm install.
 * Skip when OPENANTY_SKIP_DOWNLOAD=1 or CI without network is intentional.
 */

const path = require("path");
const { ensureBinaries } = require("../lib/download");

async function main() {
  if (process.env.OPENANTY_SKIP_DOWNLOAD === "1") {
    console.error("[openanty] OPENANTY_SKIP_DOWNLOAD=1 — skipping binary download");
    return;
  }
  // Optional: allow using pre-placed binaries (dev)
  if (process.env.OPENANTY_BIN_DIR) {
    console.error(
      "[openanty] OPENANTY_BIN_DIR set — skipping download, use env at runtime"
    );
    return;
  }
  const packageRoot = path.join(__dirname, "..");
  try {
    await ensureBinaries(packageRoot);
  } catch (e) {
    console.error("[openanty] postinstall download failed:", e.message);
    console.error(
      "[openanty] You can still download from https://github.com/LoganOneal/OpenAnty/releases"
    );
    console.error(
      "[openanty] or set OPENANTY_BIN_DIR to a folder containing openanty/openantyd binaries."
    );
    // Soft-fail: package still installs; first run will retry
    process.exitCode = 0;
  }
}

main();
