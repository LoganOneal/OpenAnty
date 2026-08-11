# Open Anty control panel (UI)

Dolphin{anty}-inspired **operator UI** embedded in `openantyd serve`.

## Open it

```bash
openantyd serve
openanty ui
# browser → http://127.0.0.1:3847/
```

The page injects your local API token (localhost-only bootstrap). No separate login for single-machine use.

## Layout (Dolphin-like)

| Area | Contents |
| --- | --- |
| Left sidebar | Browser Profiles, Proxies, Extensions, Automation, Synchronizer, Cookie Robot, Team, Settings |
| Profiles table | Name, status, tags, proxy, OS/browser, fingerprint hash, created, Start/Stop/Delete |
| Bulk bar | Multi-select → start / stop / delete / export cookies |
| Create modal | General · Proxy · Fingerprint · Cookies tabs |

Branding is **Open Anty** (purple accent, dark navy). Workflows and density intentionally follow commercial antidetect UIs.

## Source

- `ui/index.html`, `ui/assets/app.css`, `ui/assets/app.js`
- Embedded at compile time via `crates/openantyd/src/ui_static.rs`
