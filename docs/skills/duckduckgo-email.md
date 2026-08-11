# Skill: DuckDuckGo private `@duck.com` aliases (experimental)

## Official API?

**No.** DuckDuckGo does not publish a developer API for Email Protection.

Open Anty uses the **same unofficial endpoint** as Bitwarden / the DDG extension:

```http
POST https://quack.duckduckgo.com/api/email/addresses
Authorization: Bearer <token>
```

This can break without notice. Daily rate limits apply. Treat as **experimental**.

## One-time: get a Bearer token

1. Open [https://duckduckgo.com/email](https://duckduckgo.com/email) and finish Email Protection setup (link a real inbox).
2. Go to the **Autofill** tab (settings).
3. Open DevTools → **Network**.
4. Click **Generate Private Duck Address**.
5. Select the request named **addresses** (POST to `quack.duckduckgo.com`).
6. Copy `Authorization: Bearer …` value (token only, or whole header).

Helpers others use: Bitwarden docs, community token grabbers — prefer copying yourself.

## Open Anty CLI

```bash
# Save token (encrypted under data dir)
openanty mail duck-connect --token "YOUR_BEARER_TOKEN"
# or: set OPENANTY_DDG_TOKEN=...

openanty mail duck-status
openanty mail create-alias --provider duckduckgo
# → { "address": "random-words@duck.com", ... }
```

Also free without DDG: `openanty mail create-alias --provider gmail_plus` after `mail connect` to Gmail.

## MCP tools

| Tool | Purpose |
| --- | --- |
| `mail_duck_connect` | Save Bearer token |
| `mail_duck_status` | Configured? |
| `mail_duck_disconnect` | Wipe token |
| `mail_create_alias` | `provider: duckduckgo` or `gmail_plus` |

## Agent signup flow

```
1. mail_duck_connect (once)
2. mail_connect Gmail = DDG forward destination (for OTP)
3. mail_create_alias duckduckgo → address
4. launch mobile browser → signup with that @duck.com address
5. mail_wait_otp { from_contains: "..." } on Gmail
6. page_type OTP → continue
```

## REST

- `POST /v1/mail/duck/connect` `{ "token": "..." }`
- `GET /v1/mail/duck/status`
- `POST /v1/mail/create-alias` `{ "provider": "duckduckgo" }`

## Security

- Token is a **session secret** — encrypted at rest; never commit it.
- Re-copy from DevTools if generation starts returning 401.
- Not for high-volume abuse; DDG rate-limits and ToS apply.
