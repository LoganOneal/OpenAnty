# Skill: Hands-off agent email (AgentMail)

## Goal

User (or agent) runs **one command** or clicks **+ Agent email** and gets a real inbox address for signups. OTPs are read automatically with `mail_wait_otp`.

## One-time setup

### A. Console API key (simplest for humans)

1. Create key at [console.agentmail.to](https://console.agentmail.to)  
2. Connect:

```bash
openanty mail agentmail-connect --api-key "am_..."
# or: AGENTMAIL_API_KEY=am_...
```

GUI: **Agent Email** → paste key → **Save API key**.

### B. Agent self sign-up (no console)

```bash
openanty mail agent-signup you@gmail.com my-openanty-bot
# OTP emailed to you@gmail.com
openanty mail agent-verify 123456
```

## Create email (hands-off)

```bash
openanty mail create-inbox
# → { "email": "random@agentmail.to", "inbox_id": "...", "ok": true }
```

GUI: top bar **+ Agent email** or Agent Email page → **Create agent email**.

MCP: `mail_create_inbox` (provider default `agentmail`).

## Signup flow

```
mail_create_inbox
→ launch_session / page tools with returned email
→ mail_wait_otp { "from_contains": "reddit", "timeout_seconds": 180 }
→ enter OTP in page
```

`mail_wait_otp` uses the **last created AgentMail inbox** when AgentMail is configured.

## Other providers (same button)

| Provider | When |
| --- | --- |
| `agentmail` | Default — full inbox |
| `gmail_plus` | After `mail connect` Gmail — high trust |
| `duckduckgo` | After `mail duck-connect` — experimental |

```bash
openanty mail create-inbox --provider gmail_plus
openanty mail create-inbox --provider duckduckgo
```

## REST

- `POST /v1/mail/agentmail/connect` `{ "api_key": "am_..." }`
- `POST /v1/mail/create-inbox` `{ "provider": "agentmail" }`
- `GET /v1/mail/overview`
- `POST /v1/mail/wait-otp` …

## Trust note

`@agentmail.to` is real mail but not Gmail reputation. For max trust use Gmail+ after connecting Gmail.
