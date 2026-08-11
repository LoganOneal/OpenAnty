# End-to-end test for Open Anty (CLI + MCP + REST)
# Usage: .\scripts\e2e_release_test.ps1 [-BinDir path]
param(
  [string]$BinDir = "",
  [string]$DataDir = "$env:TEMP\openanty-e2e-data"
)

$ErrorActionPreference = "Stop"
if (-not $BinDir) {
  $cand = @(
    ".\target\release",
    ".\target\debug",
    "$env:USERPROFILE\OpenAnty\OpenAnty-windows-x64\bin"
  )
  foreach ($c in $cand) {
    if (Test-Path (Join-Path $c "openanty.exe")) { $BinDir = (Resolve-Path $c).Path; break }
  }
}
if (-not $BinDir) { throw "openanty.exe not found; build with cargo build --release" }

$oa = Join-Path $BinDir "openanty.exe"
$od = Join-Path $BinDir "openantyd.exe"
$env:OPENANTY_DATA_DIR = $DataDir
$env:OPENANTY_SKIP_BROWSER_VERSION = "0"
Remove-Item -Recurse -Force $DataDir -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $DataDir | Out-Null

function Invoke-OA([string[]]$ArgsList, [int]$TimeoutSec = 60) {
  # File redirects avoid pipe deadlocks when the CLI spawns Chrome.
  $outf = Join-Path $DataDir ("out-" + [guid]::NewGuid().ToString("n") + ".txt")
  $errf = Join-Path $DataDir ("err-" + [guid]::NewGuid().ToString("n") + ".txt")
  $env:OPENANTY_DATA_DIR = $DataDir
  $p = Start-Process -FilePath $oa -ArgumentList $ArgsList -PassThru -WindowStyle Hidden `
    -RedirectStandardOutput $outf -RedirectStandardError $errf
  if (-not $p.WaitForExit($TimeoutSec * 1000)) {
    try { Stop-Process -Id $p.Id -Force } catch {}
    return @{ code = -1; out = ""; err = "TIMEOUT: $($ArgsList -join ' ')" }
  }
  # Ensure process handle is fully reaped so ExitCode is populated
  try { $p.WaitForExit(2000) | Out-Null } catch {}
  $code = 0
  try { if ($null -ne $p.ExitCode) { $code = [int]$p.ExitCode } } catch { $code = 0 }
  $out = if (Test-Path $outf) { Get-Content $outf -Raw -ErrorAction SilentlyContinue } else { "" }
  $err = if (Test-Path $errf) { Get-Content $errf -Raw -ErrorAction SilentlyContinue } else { "" }
  Remove-Item $outf, $errf -ErrorAction SilentlyContinue
  return @{ code = $code; out = $(if ($null -eq $out) { "" } else { $out }); err = $(if ($null -eq $err) { "" } else { $err }) }
}

$failed = @()
function Assert-Ok($name, $r, [scriptblock]$extra = $null) {
  if ($r.code -ne 0) {
    $script:failed += $name
    Write-Host "FAIL $name code=$($r.code) err=$($r.err) out=$($r.out)" -ForegroundColor Red
    return
  }
  if ($extra) {
    try { & $extra $r | Out-Null } catch {
      $script:failed += $name
      Write-Host "FAIL $name $_" -ForegroundColor Red
      return
    }
  }
  Write-Host "PASS $name" -ForegroundColor Green
}

Write-Host "BinDir=$BinDir DataDir=$DataDir"

$r = Invoke-OA @("init") 30
Assert-Ok "init" $r

$r = Invoke-OA @("doctor","--json") 30
Assert-Ok "doctor" $r { param($x) ($x.out | ConvertFrom-Json).ok -eq $true }

$r = Invoke-OA @("mcp-config") 10
Assert-Ok "mcp-config" $r { param($x) $x.out -match "openantyd" }

$r = Invoke-OA @("profile","create","e2e","--template","win11_chrome_mid") 30
Assert-Ok "profile create" $r
$prof = $r.out | ConvertFrom-Json
$id = $prof.id
Write-Host "  profile=$id"

$r = Invoke-OA @("profile","list") 15
Assert-Ok "profile list" $r

$cf = Join-Path $DataDir "cookies.json"
'[{"name":"sid","value":"abc123","domain":".example.com","path":"/","httpOnly":true,"secure":true,"sameSite":"Lax"}]' | Set-Content $cf -Encoding ascii
$r = Invoke-OA @("profile","import-cookies",$id,"--file",$cf) 15
Assert-Ok "import cookies" $r

$r = Invoke-OA @("profile","export-cookies",$id) 15
Assert-Ok "export cookies" $r { param($x) $x.out -match "abc123" }

$r = Invoke-OA @("session","launch",$id,"--headless","--start-url","about:blank") 90
Assert-Ok "session launch" $r
$ses = $null
try { $ses = $r.out | ConvertFrom-Json } catch {}
if (-not $ses.cdp_ws_url) {
  $failed += "session launch cdp"
  Write-Host "FAIL no cdp_ws_url" -ForegroundColor Red
} else {
  Write-Host "  cdp=$($ses.cdp_ws_url)"
  try {
    $v = Invoke-WebRequest "http://127.0.0.1:$($ses.debug_port)/json/version" -UseBasicParsing -TimeoutSec 5
    Write-Host "PASS cdp http $($v.StatusCode)" -ForegroundColor Green
  } catch {
    $failed += "cdp http"
    Write-Host "FAIL cdp http $_" -ForegroundColor Red
  }
}

$r = Invoke-OA @("session","list") 15
Assert-Ok "session list" $r

if ($ses.id) {
  $r = Invoke-OA @("session","cdp",$ses.id) 15
  Assert-Ok "session cdp" $r { param($x) $x.out -match "ws://" }

  $r = Invoke-OA @("session","stop",$ses.id) 30
  Assert-Ok "session stop" $r
}

$r = Invoke-OA @("status") 15
Assert-Ok "status" $r

$r = Invoke-OA @("profile","delete",$id) 15
Assert-Ok "profile delete" $r

# --- REST API ---
Write-Host "--- REST ---"
$token = (Get-Content (Join-Path $DataDir "api.token") -Raw).Trim()
$daemonOut = Join-Path $DataDir "daemon-stdout.log"
$daemonErr = Join-Path $DataDir "daemon-stderr.log"
$dp = Start-Process -FilePath $od -ArgumentList @("serve") -PassThru -WindowStyle Hidden `
  -RedirectStandardOutput $daemonOut -RedirectStandardError $daemonErr
Start-Sleep 2
try {
  $headers = @{ Authorization = "Bearer $token" }
  $st = Invoke-RestMethod -Uri "http://127.0.0.1:3847/v1/system/status" -Headers $headers
  if ($st.ok) { Write-Host "PASS rest status" -ForegroundColor Green } else { $failed += "rest status"; Write-Host "FAIL rest status" -ForegroundColor Red }
  $body = @{ name = "rest-prof"; template = "win11_chrome_mid" } | ConvertTo-Json
  $pr = Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:3847/v1/profiles" -Headers $headers -ContentType "application/json" -Body $body
  if ($pr.ok -and $pr.profile.id) { Write-Host "PASS rest create profile $($pr.profile.id)" -ForegroundColor Green } else { $failed += "rest create"; Write-Host "FAIL rest create $($pr | ConvertTo-Json -Compress)" -ForegroundColor Red }
  $plist = Invoke-RestMethod -Uri "http://127.0.0.1:3847/v1/profiles" -Headers $headers
  if ($plist.ok) { Write-Host "PASS rest list profiles" -ForegroundColor Green } else { $failed += "rest list" }
} catch {
  $failed += "rest"
  Write-Host "FAIL rest $_" -ForegroundColor Red
  if (Test-Path $daemonErr) { Get-Content $daemonErr | Select-Object -Last 20 }
} finally {
  if ($dp -and -not $dp.HasExited) { Stop-Process -Id $dp.Id -Force -ErrorAction SilentlyContinue }
}

# --- MCP ---
Write-Host "--- MCP ---"
$mcpPy = @'
import json, subprocess, sys, os, struct, time

bin_dir = sys.argv[1]
data_dir = sys.argv[2]
exe = os.path.join(bin_dir, "openantyd.exe" if os.name == "nt" else "openantyd")
env = os.environ.copy()
env["OPENANTY_DATA_DIR"] = data_dir
env["OPENANTY_SKIP_BROWSER_VERSION"] = "1"
proc = subprocess.Popen([exe, "mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env)

def send(msg):
    body = json.dumps(msg).encode()
    header = f"Content-Length: {len(body)}\r\n\r\n".encode()
    proc.stdin.write(header + body)
    proc.stdin.flush()

def read_msg(timeout=10):
    # Read Content-Length framed response
    import select
    deadline = time.time() + timeout
    buf = b""
    while time.time() < deadline:
        # non-blocking-ish read
        chunk = proc.stdout.read(1)
        if not chunk:
            time.sleep(0.01)
            continue
        buf += chunk
        if b"\r\n\r\n" in buf:
            header, rest = buf.split(b"\r\n\r\n", 1)
            length = None
            for line in header.decode().split("\r\n"):
                if line.lower().startswith("content-length:"):
                    length = int(line.split(":")[1].strip())
            if length is None:
                raise RuntimeError("no content-length")
            while len(rest) < length:
                rest += proc.stdout.read(length - len(rest))
            return json.loads(rest[:length].decode())
    raise TimeoutError("mcp read timeout")

try:
    send({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
        "protocolVersion":"2024-11-05",
        "capabilities":{},
        "clientInfo":{"name":"e2e","version":"0.1"}
    }})
    init = read_msg()
    assert "result" in init, init
    print("PASS mcp initialize", flush=True)

    send({"jsonrpc":"2.0","method":"notifications/initialized"})
    send({"jsonrpc":"2.0","id":2,"method":"tools/list"})
    tools = read_msg()
    names = [t["name"] for t in tools["result"]["tools"]]
    assert "create_profile" in names and "launch_session" in names, names
    print("PASS mcp tools/list", len(names), "tools", flush=True)

    send({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{
        "name":"create_profile",
        "arguments":{"name":"mcp-e2e","template":"win11_chrome_mid"}
    }})
    created = read_msg(timeout=30)
    text = created["result"]["content"][0]["text"]
    payload = json.loads(text)
    assert payload.get("ok") is True, payload
    pid = payload["profile"]["id"]
    print("PASS mcp create_profile", pid, flush=True)

    send({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{
        "name":"doctor",
        "arguments":{}
    }})
    doc = read_msg(timeout=30)
    print("PASS mcp doctor", flush=True)

    send({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{
        "name":"list_profiles",
        "arguments":{"limit":10}
    }})
    listed = read_msg(timeout=20)
    print("PASS mcp list_profiles", flush=True)

    send({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{
        "name":"delete_profile",
        "arguments":{"profile_id": pid, "confirm": True}
    }})
    deleted = read_msg(timeout=20)
    print("PASS mcp delete_profile", flush=True)
    print("MCP_OK", flush=True)
finally:
    proc.kill()
'@
$pyFile = Join-Path $DataDir "mcp_e2e.py"
Set-Content -Path $pyFile -Value $mcpPy -Encoding utf8
try {
  python $pyFile $BinDir $DataDir
  if ($LASTEXITCODE -ne 0) { $failed += "mcp" }
} catch {
  $failed += "mcp"
  Write-Host "FAIL mcp $_" -ForegroundColor Red
}

Write-Host ""
if ($failed.Count -eq 0) {
  Write-Host "ALL E2E TESTS PASSED" -ForegroundColor Green
  exit 0
} else {
  Write-Host "FAILED: $($failed -join ', ')" -ForegroundColor Red
  exit 1
}
