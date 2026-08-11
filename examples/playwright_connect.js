/**
 * Connect to a OpenAnty session via CDP.
 * Usage:
 *   OpenAnty session launch prf_... --headless
 *   set CDP_URL=ws://127.0.0.1:...
 *   node examples/playwright_connect.js
 */
const { chromium } = require('playwright');

async function main() {
  const cdp = process.env.CDP_URL;
  if (!cdp) {
    console.error('Set CDP_URL to the cdp_ws_url from launch_session');
    process.exit(1);
  }
  const browser = await chromium.connectOverCDP(cdp);
  const context = browser.contexts()[0];
  const page = context.pages()[0] || (await context.newPage());
  await page.goto(process.env.START_URL || 'https://example.com');
  console.log('title:', await page.title());
  await browser.close();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
