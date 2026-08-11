# Skill: BYO Gmail OTP for signups (Reddit, etc.)

## Why Gmail

Sites like Reddit trust **Gmail** far more than temp mail. Open Anty does **not** create Gmail accounts. You connect **your** Gmail (or Workspace) via IMAP App Password; the agent polls for codes.

## One-time Gmail setup (human)

1. Enable **2-Step Verification** on the Google Account.  
2. Create an **App Password**: [myaccount.google.com/apppasswords](https://myaccount.google.com/apppasswords)  
   - App: Mail · Device: Windows / Other  
   - Copy the **16-character** password (spaces optional).  
3. Use that password with Open Anty — **not** your normal Gmail password.

## Connect (CLI)

```bash
# Prefer env so password is not in shell history
set OPENANTY_MAIL_PASSWORD=xxxx xxxx xxxx xxxx   # Windows
# export OPENANTY_MAIL_PASSWORD='...'            # Unix

openanty mail connect you@gmail.com --provider gmail
openanty mail status
openanty mail list --limit 5
```

Or env-only (no save step):

```bash
OPENANTY_MAIL_USER=you@gmail.com
OPENANTY_MAIL_PASSWORD=your_app_password
# optional: OPENANTY_MAIL_HOST=imap.gmail.com OPENANTY_MAIL_PORT=993
```

Credentials are stored encrypted at `%APPDATA%/OpenAnty/mail.credentials.bin` (or `$OPENANTY_DATA_DIR`).

## MCP agent tools

| Tool | Use |
| --- | --- |
| `mail_connect` | Save Gmail IMAP + app password (`test: true` recommended) |
| `mail_status` | Is mail configured? |
| `mail_list` | Recent messages + detected OTPs |
| `mail_wait_otp` | Poll until code (filter `from_contains: "reddit.com"`) |
| `mail_handoff` | Ask human to paste code if mail not configured |
| `mail_disconnect` | Wipe saved credentials |

## Agent flow (Reddit example)

```
1. setup_mobile_profile / create_profile android_chrome_pixel
2. launch_session headed → reddit.com/register
3. page tools: enter **your Gmail address** (the one connected)
4. mail_wait_otp { from_contains: "reddit", timeout_seconds: 180 }
5. page tools: type otp into verification field → Continue
6. complete username/password steps
7. page_navigate → https://www.reddit.com/user/<username>/
```

REST equivalents:

- `POST /v1/mail/connect`
- `GET /v1/mail/status`
- `GET /v1/mail/list?limit=10`
- `POST /v1/mail/wait-otp` body: `{ "from_contains": "reddit.com", "timeout_seconds": 120 }`

## Security

- Localhost-only API token still required  
- Password never returned by `mail_status`  
- Do not commit app passwords  
- Prefer least-privilege App Password; revoke if leaked  

## Not included (yet)

- Full Gmail OAuth browser flow (App Password is v1)  
- AgentMail / third-party agent inbox providers  
- Creating Gmail accounts automatically  
