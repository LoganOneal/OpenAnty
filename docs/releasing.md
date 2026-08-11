# Creating a GitHub Release (downloadable executables)

Open Anty **does not** publish binaries on every commit.  
Releases are intentional: you create a version tag → GitHub Actions builds → assets attach to a GitHub Release page.

## What users download

After a successful release, the release page has archives like:

| Asset | Platform |
| --- | --- |
| **`openanty-windows-x64.exe`** | Windows CLI (direct download) |
| **`openantyd-windows-x64.exe`** | Windows daemon / MCP (direct download) |
| `openanty-linux-x64` / `openantyd-linux-x64` | Linux bare binaries |
| `openanty-macos-arm64` / `openantyd-macos-arm64` | Apple Silicon bare binaries |
| `openanty-macos-x64` / `openantyd-macos-x64` | Intel Mac bare binaries |
| `OpenAnty-windows-x64.zip` | Full portable folder (recommended) |
| `OpenAnty-linux-x64.tar.gz` | Full portable folder |
| `OpenAnty-macos-*.tar.gz` | Full portable folders |
| `SHA256SUMS.txt` | Checksums |

**Quick start (Windows):** download `openanty-windows-x64.exe` + `openantyd-windows-x64.exe`, rename to `openanty.exe` / `openantyd.exe` if you like, put them on PATH.

Full archives contain:

```
bin/openanty(.exe)
bin/openantyd(.exe)
README.md
LICENSE
RESPONSIBLE_USE.md
QUICKSTART.md
INSTALL.ps1 or install.sh
```

## How to cut a release

### Option A — Tag from git (recommended)

```bash
# 1. Ensure main/master is green and up to date
git checkout master
git pull origin master

# 2. Optional: bump version in Cargo.toml workspace.package.version
#    then commit

# 3. Create an annotated tag
git tag -a v0.1.0 -m "Open Anty v0.1.0"

# 4. Push ONLY the tag (this starts the Release workflow)
git push origin v0.1.0
```

Then open:  
https://github.com/LoganOneal/OpenAnty/releases

Wait for the **Release** workflow to finish (Actions tab). Binaries appear on the release when the job completes (~10–20 minutes).

### Option B — GitHub UI

1. Go to **Releases → Draft a new release**
2. **Choose a tag** → create new tag `v0.1.0` on `master`
3. Title: `Open Anty v0.1.0`
4. Click **Publish release**

If the tag is created/pushed as part of this, the same `release.yml` workflow runs and uploads assets.

### Option C — Re-run a build for an existing tag

**Actions → Release → Run workflow**  
Optionally pass the tag name (e.g. `v0.1.0`).

## Version tag rules

- Tags **must** match `v*` (e.g. `v0.1.0`, `v0.2.0-beta.1`)
- Tags with a hyphen (e.g. `v0.2.0-rc.1`) are marked **pre-release** automatically
- Normal CI (`ci.yml`) still runs on every PR/push but **never** creates a release

## Local portable build (no GitHub)

Windows:

```powershell
.\packaging\windows\build-portable.ps1
# → packaging/out/OpenAnty-windows-x64/
```

## Troubleshooting

| Issue | Fix |
| --- | --- |
| No assets on release | Check **Actions → Release** for failures |
| Workflow didn’t start | Confirm tag starts with `v` and was pushed: `git push origin v0.1.0` |
| Permission error on upload | Repo must allow Actions to write; workflow has `contents: write` |
| Want only Windows | Edit matrix in `.github/workflows/release.yml` |

## First release checklist

- [ ] `cargo test --workspace` passes locally  
- [ ] README / CHANGELOG updated  
- [ ] Tag `v0.1.0` pushed  
- [ ] Release workflow green  
- [ ] Download Windows zip and run `INSTALL.ps1` or `bin\openanty.exe init`  
