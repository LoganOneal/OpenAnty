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
