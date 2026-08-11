# Skill: Mobile profile + Reddit signup test

## Features used

1. Profile template **`android_chrome_pixel`** (`os: android`)
2. CDP mobile emulation on launch (viewport, touch, UA override, init script)
3. Page control via REST `/v1/sessions/{id}/page/*` or MCP `page_*` / `setup_mobile_profile`

## Create + launch

```bash
# UI: Create profile → template android_chrome_pixel → Start
# or API:
curl -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"name":"phone1","template":"android_chrome_pixel","os":"android"}' \
  http://127.0.0.1:3847/v1/profiles

curl -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"profile_id":"prf_...","headed":true,"start_url":"https://www.reddit.com/register/"}' \
  http://127.0.0.1:3847/v1/sessions
```

MCP: `setup_mobile_profile` → `launch_session` → `page_evaluate` / `page_type` / …

## Verify mobile signals

```js
({
  ua: navigator.userAgent,
  touch: navigator.maxTouchPoints,
  w: window.innerWidth,
  h: window.innerHeight
})
```

Expect: width ~390, touch ≥ 5, UA contains `Android` + `Mobile` when override sticks.

## Reddit blockers (observed)

- **“Prove your humanity”** captcha page before registration form
- Email verification after signup (needs real inbox)
- Sometimes phone verification

Open Anty does **not** ship captcha solvers. Complete captcha manually in the headed window, then continue automation (cookies persist on profile).

## Go to profile page after login

```
page_navigate → https://www.reddit.com/user/<username>/
page_content mode=text
```
