/* Open Anty control panel — Dolphin{anty}-inspired operator UI */
(function () {
  const cfg = window.__OPENANTY__ || {};
  const state = {
    token: cfg.token && cfg.token !== "{{TOKEN}}" ? cfg.token : localStorage.getItem("oa_token") || "",
    profiles: [],
    sessions: [],
    selected: new Set(),
    view: "profiles",
  };

  const $ = (sel, el = document) => el.querySelector(sel);
  const $$ = (sel, el = document) => [...el.querySelectorAll(sel)];

  function toast(msg, err) {
    const t = $("#toast");
    t.hidden = false;
    t.textContent = msg;
    t.classList.toggle("err", !!err);
    clearTimeout(toast._tm);
    toast._tm = setTimeout(() => (t.hidden = true), 3200);
  }

  async function api(path, opts = {}) {
    const headers = Object.assign(
      { "Content-Type": "application/json", Accept: "application/json" },
      opts.headers || {}
    );
    if (state.token) headers.Authorization = `Bearer ${state.token}`;
    const res = await fetch(path, { ...opts, headers });
    const text = await res.text();
    let data;
    try {
      data = text ? JSON.parse(text) : {};
    } catch {
      data = { raw: text };
    }
    if (!res.ok) {
      const msg = data?.error?.message || data?.message || res.statusText || "request failed";
      throw new Error(msg);
    }
    return data;
  }

  function sessionForProfile(profileId) {
    return state.sessions.find(
      (s) => s.profile_id === profileId && (s.status === "running" || s.status === "Running")
    );
  }

  function formatDate(iso) {
    if (!iso) return "—";
    try {
      return new Date(iso).toLocaleString();
    } catch {
      return iso;
    }
  }

  async function refresh() {
    const [plist, slist, doctor] = await Promise.all([
      api("/v1/profiles?limit=200"),
      api("/v1/sessions"),
      api("/v1/system/doctor"),
    ]);
    state.profiles = plist.items || [];
    state.sessions = slist.items || [];
    renderProfiles();
    fillProfileSelects();
    fillSessionSelects();
    renderDoctor(doctor);
    $("#app-version").textContent = "v" + (cfg.version || doctor.version || "—");
  }

  function renderDoctor(doctor) {
    const pill = $("#doctor-pill");
    if (doctor.ok) {
      pill.textContent = "doctor OK";
      pill.className = "pill ok";
    } else {
      pill.textContent = "doctor issues";
      pill.className = "pill bad";
    }
    $("#settings-doctor").textContent = JSON.stringify(doctor, null, 2);
    $("#api-token").textContent = state.token || "(no token)";
  }

  function renderProfiles() {
    const q = ($("#search").value || "").toLowerCase();
    const st = $("#filter-status").value;
    const tag = $("#filter-tag").value;
    const tags = new Set();
    state.profiles.forEach((p) => (p.tags || []).forEach((t) => tags.add(t)));
    const tagSel = $("#filter-tag");
    const cur = tagSel.value;
    tagSel.innerHTML = `<option value="">All tags</option>` + [...tags].map((t) => `<option value="${esc(t)}">${esc(t)}</option>`).join("");
    tagSel.value = cur;

    let rows = state.profiles.filter((p) => {
      const hay = `${p.name} ${(p.tags || []).join(" ")} ${p.notes || ""} ${p.id}`.toLowerCase();
      if (q && !hay.includes(q)) return false;
      const ses = sessionForProfile(p.id);
      if (st === "running" && !ses) return false;
      if (st === "idle" && ses) return false;
      if (tag && !(p.tags || []).includes(tag)) return false;
      return true;
    });

    const body = $("#profiles-body");
    if (!rows.length) {
      body.innerHTML = `<tr><td colspan="9" class="empty">No profiles yet. Click <b>+ Create profile</b> to start — same flow as Dolphin Browser Profiles.</td></tr>`;
      return;
    }
    body.innerHTML = rows
      .map((p) => {
        const ses = sessionForProfile(p.id);
        const running = !!ses;
        const checked = state.selected.has(p.id) ? "checked" : "";
        const tagsHtml = (p.tags || []).map((t) => `<span class="tag">${esc(t)}</span>`).join(" ") || "—";
        const proxy = p.proxy_configured ? "configured" : "—";
        const os = p.fingerprint_summary?.os || "—";
        const browser = p.fingerprint_summary?.browser || "";
        return `<tr data-id="${esc(p.id)}">
          <td><input type="checkbox" class="row-check" data-id="${esc(p.id)}" ${checked} /></td>
          <td><div class="name-cell"><strong>${esc(p.name)}</strong><small>${esc(p.id)}</small></div></td>
          <td><span class="status ${running ? "running" : "idle"}">${running ? "Running" : "Idle"}</span></td>
          <td>${tagsHtml}</td>
          <td>${esc(proxy)}</td>
          <td>${esc(os)}<br/><span class="muted sm">${esc(browser)}</span></td>
          <td><code class="muted sm">${esc((p.fingerprint_hash || "").slice(0, 12))}…</code></td>
          <td class="muted sm">${esc(formatDate(p.created_at))}</td>
          <td class="col-actions"><div class="actions">
            ${
              running
                ? `<button class="btn sm" data-act="stop" data-sid="${esc(ses.id)}">Stop</button>`
                : `<button class="btn sm primary" data-act="start" data-id="${esc(p.id)}">Start</button>`
            }
            <button class="btn sm" data-act="delete" data-id="${esc(p.id)}">Delete</button>
          </div></td>
        </tr>`;
      })
      .join("");
  }

  function esc(s) {
    return String(s ?? "")
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  function fillProfileSelects() {
    const opts = state.profiles.map((p) => `<option value="${esc(p.id)}">${esc(p.name)}</option>`).join("");
    ["#scen-profile", "#robot-profile"].forEach((sel) => {
      const el = $(sel);
      if (el) el.innerHTML = opts || `<option value="">No profiles</option>`;
    });
  }

  function fillSessionSelects() {
    const opts = state.sessions
      .filter((s) => s.status === "running" || s.cdp_ws_url)
      .map((s) => `<option value="${esc(s.id)}">${esc(s.id)} · ${esc(s.profile_id)}</option>`)
      .join("");
    $("#sync-master").innerHTML = opts || `<option value="">No running sessions</option>`;
    $("#sync-followers").innerHTML = opts;
  }

  function setView(name) {
    state.view = name;
    $$(".nav-item").forEach((b) => b.classList.toggle("active", b.dataset.view === name));
    $$(".view").forEach((v) => v.classList.toggle("active", v.id === "view-" + name));
    const titles = {
      profiles: ["Browser Profiles", "Manage isolated fingerprints, proxies, and sessions"],
      proxies: ["Proxies", "Proxy library for multi-account workflows"],
      extensions: ["Extensions", "Load unpacked Chromium extensions per profile launch"],
      scenarios: ["Automation", "Scenario builder — farm, collect, warm-up"],
      sync: ["Synchronizer", "Repeat actions across multiple profiles"],
      cookies: ["Cookie Robot", "Warm profiles and harvest cookies"],
      team: ["Team", "Local roles and permissions"],
      settings: ["Settings", "Doctor, health, bulk tools, API"],
    };
    const [t, s] = titles[name] || ["Open Anty", ""];
    $("#view-title").textContent = t;
    $("#view-sub").textContent = s;
    if (name === "proxies") loadProxies();
    if (name === "extensions") loadExt();
    if (name === "scenarios") loadScenarios();
    if (name === "team") loadTeam();
  }

  async function loadProxies() {
    try {
      const data = await api("/v1/proxy-pool");
      const items = data.items || [];
      $("#proxies-body").innerHTML =
        items
          .map(
            (p) => `<tr>
          <td>${esc(p.name)}</td><td><code>${esc(p.server)}</code></td>
          <td>${esc(p.last_status || "—")}</td>
          <td><button class="btn sm danger" data-del-proxy="${esc(p.id)}">Remove</button></td>
        </tr>`
          )
          .join("") || `<tr><td colspan="4" class="empty">No proxies in pool</td></tr>`;
    } catch (e) {
      $("#proxies-body").innerHTML = `<tr><td colspan="4" class="empty">${esc(e.message)}</td></tr>`;
    }
  }

  async function loadExt() {
    try {
      const data = await api("/v1/extensions");
      const items = data.items || [];
      $("#ext-body").innerHTML =
        items
          .map(
            (e) => `<tr>
          <td>${esc(e.name)}</td><td><code>${esc(e.path)}</code></td>
          <td>${e.enabled ? "yes" : "no"}</td>
          <td><button class="btn sm danger" data-del-ext="${esc(e.id)}">Remove</button></td>
        </tr>`
          )
          .join("") || `<tr><td colspan="4" class="empty">No extensions registered</td></tr>`;
    } catch (e) {
      $("#ext-body").innerHTML = `<tr><td colspan="4" class="empty">${esc(e.message)}</td></tr>`;
    }
  }

  async function loadScenarios() {
    try {
      const data = await api("/v1/scenarios");
      const items = data.items || [];
      $("#scen-body").innerHTML =
        items
          .map(
            (s) => `<tr>
          <td>${esc(s.name)}</td><td>${(s.steps || []).length}</td>
          <td><button class="btn sm" data-load-scen="${esc(s.id)}">Load</button>
          <button class="btn sm danger" data-del-scen="${esc(s.id)}">Delete</button></td>
        </tr>`
          )
          .join("") || `<tr><td colspan="3" class="empty">No saved scenarios</td></tr>`;
      window.__scenarios = items;
    } catch (e) {
      $("#scen-body").innerHTML = `<tr><td colspan="3" class="empty">${esc(e.message)}</td></tr>`;
    }
  }

  async function loadTeam() {
    try {
      const data = await api("/v1/users");
      const items = data.items || [];
      $("#team-body").innerHTML =
        items
          .map(
            (u) => `<tr>
          <td>${esc(u.username)}</td><td><span class="tag">${esc(u.role)}</span></td>
          <td class="muted sm">${esc(formatDate(u.created_at))}</td>
          <td><button class="btn sm danger" data-del-user="${esc(u.id)}">Remove</button></td>
        </tr>`
          )
          .join("") || `<tr><td colspan="4" class="empty">Only local admin token — add users for RBAC</td></tr>`;
    } catch (e) {
      $("#team-body").innerHTML = `<tr><td colspan="4" class="empty">${esc(e.message)}</td></tr>`;
    }
  }

  function openModal() {
    $("#modal").hidden = false;
    $("#m-name").focus();
  }
  function closeModal() {
    $("#modal").hidden = true;
  }

  async function createProfile() {
    const name = $("#m-name").value.trim();
    if (!name) return toast("Name required", true);
    const tags = $("#m-tags").value
      .split(",")
      .map((t) => t.trim())
      .filter(Boolean);
    const body = {
      name,
      tags,
      notes: $("#m-notes").value || null,
      os: $("#m-os").value || null,
      template: $("#m-template").value || null,
    };
    const created = await api("/v1/profiles", { method: "POST", body: JSON.stringify(body) });
    const profile = created.profile || created;
    const id = profile.id;
    const proxy = $("#m-proxy").value.trim();
    if (proxy) {
      await api(`/v1/profiles/${id}/proxy`, {
        method: "PUT",
        body: JSON.stringify({
          proxy: parseProxy(proxy),
          align_geo: true,
          check: true,
        }),
      });
    }
    const cookiesRaw = $("#m-cookies").value.trim();
    if (cookiesRaw) {
      const cookies = JSON.parse(cookiesRaw);
      await api(`/v1/profiles/${id}/cookies/import`, {
        method: "POST",
        body: JSON.stringify({ cookies, merge: true }),
      });
    }
    closeModal();
    toast("Profile created");
    await refresh();
  }

  function parseProxy(raw) {
    let s = raw.trim();
    if (!s.includes("://")) s = "http://" + s;
    try {
      const u = new URL(s);
      const server = `${u.protocol}//${u.hostname}${u.port ? ":" + u.port : ""}`;
      return {
        server,
        username: u.username || null,
        password: u.password || null,
        check_timeout_ms: 8000,
      };
    } catch {
      return { server: s, check_timeout_ms: 8000 };
    }
  }

  async function startProfile(id) {
    toast("Launching…");
    await api("/v1/sessions", {
      method: "POST",
      body: JSON.stringify({
        profile_id: id,
        headed: true,
        start_url: "https://example.com",
        ttl_seconds: 7200,
      }),
    });
    toast("Profile started");
    await refresh();
  }

  async function stopSession(sid) {
    await api(`/v1/sessions/${sid}/stop`, { method: "POST", body: "{}" });
    toast("Session stopped");
    await refresh();
  }

  async function deleteProfile(id) {
    if (!confirm("Delete this profile?")) return;
    await api(`/v1/profiles/${id}`, { method: "DELETE" });
    state.selected.delete(id);
    toast("Deleted");
    await refresh();
  }

  // —— Events ——
  $$(".nav-item").forEach((b) => b.addEventListener("click", () => setView(b.dataset.view)));
  $("#btn-refresh").addEventListener("click", () => refresh().catch((e) => toast(e.message, true)));
  $("#btn-create").addEventListener("click", openModal);
  $("#modal-close").addEventListener("click", closeModal);
  $("#modal-cancel").addEventListener("click", closeModal);
  $("#modal-save").addEventListener("click", () => createProfile().catch((e) => toast(e.message, true)));
  $("#search").addEventListener("input", renderProfiles);
  $("#filter-status").addEventListener("change", renderProfiles);
  $("#filter-tag").addEventListener("change", renderProfiles);

  $$(".tab").forEach((t) =>
    t.addEventListener("click", () => {
      $$(".tab").forEach((x) => x.classList.remove("active"));
      $$(".tab-pane").forEach((x) => x.classList.remove("active"));
      t.classList.add("active");
      $(`.tab-pane[data-pane="${t.dataset.tab}"]`).classList.add("active");
    })
  );

  $("#profiles-body").addEventListener("click", (ev) => {
    const btn = ev.target.closest("button[data-act]");
    if (!btn) return;
    const act = btn.dataset.act;
    if (act === "start") startProfile(btn.dataset.id).catch((e) => toast(e.message, true));
    if (act === "stop") stopSession(btn.dataset.sid).catch((e) => toast(e.message, true));
    if (act === "delete") deleteProfile(btn.dataset.id).catch((e) => toast(e.message, true));
  });

  $("#profiles-body").addEventListener("change", (ev) => {
    if (ev.target.classList.contains("row-check")) {
      const id = ev.target.dataset.id;
      if (ev.target.checked) state.selected.add(id);
      else state.selected.delete(id);
      updateBulk();
    }
  });
  $("#check-all").addEventListener("change", (ev) => {
    state.selected.clear();
    if (ev.target.checked) state.profiles.forEach((p) => state.selected.add(p.id));
    renderProfiles();
    updateBulk();
  });

  function updateBulk() {
    const bar = $("#bulk-bar");
    const n = state.selected.size;
    bar.hidden = n === 0;
    $("#bulk-count").textContent = `${n} selected`;
  }

  $("#bulk-bar").addEventListener("click", async (ev) => {
    const btn = ev.target.closest("[data-bulk]");
    if (!btn) return;
    const act = btn.dataset.bulk;
    const ids = [...state.selected];
    try {
      if (act === "start") {
        for (const id of ids) await startProfile(id);
      } else if (act === "stop") {
        for (const id of ids) {
          const ses = sessionForProfile(id);
          if (ses) await stopSession(ses.id);
        }
      } else if (act === "delete") {
        if (!confirm(`Delete ${ids.length} profiles?`)) return;
        for (const id of ids) await api(`/v1/profiles/${id}`, { method: "DELETE" });
        state.selected.clear();
        await refresh();
      } else if (act === "export-cookies") {
        const all = [];
        for (const id of ids) {
          const exp = await api(`/v1/profiles/${id}/cookies/export`);
          all.push({ profile_id: id, cookies: exp.cookies || [] });
        }
        const blob = new Blob([JSON.stringify(all, null, 2)], { type: "application/json" });
        const a = document.createElement("a");
        a.href = URL.createObjectURL(blob);
        a.download = "openanty-cookies.json";
        a.click();
      }
      toast("Bulk action done");
    } catch (e) {
      toast(e.message, true);
    }
  });

  $("#btn-add-proxy").addEventListener("click", async () => {
    try {
      await api("/v1/proxy-pool", {
        method: "POST",
        body: JSON.stringify({
          name: $("#proxy-name").value || "proxy",
          server: $("#proxy-server").value,
        }),
      });
      $("#proxy-server").value = "";
      loadProxies();
      toast("Proxy added");
    } catch (e) {
      toast(e.message, true);
    }
  });
  $("#proxies-body").addEventListener("click", async (ev) => {
    const id = ev.target.dataset.delProxy;
    if (!id) return;
    await api(`/v1/proxy-pool/${id}`, { method: "DELETE" });
    loadProxies();
  });

  $("#btn-add-ext").addEventListener("click", async () => {
    try {
      await api("/v1/extensions", {
        method: "POST",
        body: JSON.stringify({ name: $("#ext-name").value, path: $("#ext-path").value, enabled: true }),
      });
      loadExt();
      toast("Extension registered");
    } catch (e) {
      toast(e.message, true);
    }
  });
  $("#ext-body").addEventListener("click", async (ev) => {
    const id = ev.target.dataset.delExt;
    if (!id) return;
    await api(`/v1/extensions/${id}`, { method: "DELETE" });
    loadExt();
  });

  $("#btn-save-scen").addEventListener("click", async () => {
    try {
      let body = JSON.parse($("#scen-json").value);
      if ($("#scen-name").value) body.name = $("#scen-name").value;
      await api("/v1/scenarios", { method: "POST", body: JSON.stringify(body) });
      loadScenarios();
      toast("Scenario saved");
    } catch (e) {
      toast(e.message, true);
    }
  });
  $("#btn-run-scen").addEventListener("click", async () => {
    const log = $("#scen-log");
    try {
      let scen = JSON.parse($("#scen-json").value);
      const profile_id = $("#scen-profile").value;
      log.textContent = "Running…\n";
      const res = await api("/v1/scenarios/run", {
        method: "POST",
        body: JSON.stringify({ profile_id, scenario: scen, headed: true }),
      });
      log.textContent = JSON.stringify(res, null, 2);
      toast("Scenario finished");
      await refresh();
    } catch (e) {
      log.textContent = e.message;
      toast(e.message, true);
    }
  });
  $("#scen-body").addEventListener("click", async (ev) => {
    if (ev.target.dataset.loadScen) {
      const s = (window.__scenarios || []).find((x) => x.id === ev.target.dataset.loadScen);
      if (s) {
        $("#scen-json").value = JSON.stringify({ name: s.name, steps: s.steps }, null, 2);
        $("#scen-name").value = s.name;
      }
    }
    if (ev.target.dataset.delScen) {
      await api(`/v1/scenarios/${ev.target.dataset.delScen}`, { method: "DELETE" });
      loadScenarios();
    }
  });

  $("#btn-sync-nav").addEventListener("click", async () => {
    const log = $("#sync-log");
    try {
      const master = $("#sync-master").value;
      const followers = [...$("#sync-followers").selectedOptions].map((o) => o.value).filter((id) => id !== master);
      const url = $("#sync-url").value;
      const res = await api("/v1/synchronizer/navigate", {
        method: "POST",
        body: JSON.stringify({ master_session_id: master, follower_session_ids: followers, url }),
      });
      log.textContent = JSON.stringify(res, null, 2);
      toast("Sync navigate done");
    } catch (e) {
      log.textContent = e.message;
      toast(e.message, true);
    }
  });

  $("#btn-robot-run").addEventListener("click", async () => {
    const log = $("#robot-log");
    try {
      const urls = $("#robot-urls").value
        .split("\n")
        .map((u) => u.trim())
        .filter(Boolean);
      const res = await api("/v1/cookie-robot/run", {
        method: "POST",
        body: JSON.stringify({
          profile_id: $("#robot-profile").value,
          urls,
          headed: false,
          export_after: true,
        }),
      });
      log.textContent = JSON.stringify(res, null, 2);
      toast("Cookie robot done");
    } catch (e) {
      log.textContent = e.message;
      toast(e.message, true);
    }
  });

  $("#btn-add-user").addEventListener("click", async () => {
    try {
      await api("/v1/users", {
        method: "POST",
        body: JSON.stringify({ username: $("#user-name").value, role: $("#user-role").value }),
      });
      loadTeam();
      toast("User added");
    } catch (e) {
      toast(e.message, true);
    }
  });
  $("#team-body").addEventListener("click", async (ev) => {
    const id = ev.target.dataset.delUser;
    if (!id) return;
    await api(`/v1/users/${id}`, { method: "DELETE" });
    loadTeam();
  });

  $("#btn-fp-health").addEventListener("click", async () => {
    try {
      const res = await api("/v1/fingerprint/health", { method: "POST", body: "{}" });
      $("#settings-log").textContent = JSON.stringify(res, null, 2);
    } catch (e) {
      toast(e.message, true);
    }
  });
  $("#btn-bulk-create").addEventListener("click", async () => {
    try {
      const res = await api("/v1/profiles/bulk", {
        method: "POST",
        body: JSON.stringify({ count: 5, name_prefix: "bulk", tags: ["bulk"] }),
      });
      $("#settings-log").textContent = JSON.stringify(res, null, 2);
      await refresh();
      toast("Bulk created");
    } catch (e) {
      toast(e.message, true);
    }
  });

  // bootstrap
  if (state.token && state.token !== "{{TOKEN}}") {
    localStorage.setItem("oa_token", state.token);
  }
  refresh().catch(async (e) => {
    // try token from bootstrap endpoint (localhost convenience)
    try {
      const boot = await fetch("/v1/ui/bootstrap").then((r) => r.json());
      if (boot.token) {
        state.token = boot.token;
        localStorage.setItem("oa_token", boot.token);
        await refresh();
        return;
      }
    } catch (_) {}
    toast("API error: " + e.message + " — is openantyd serve running?", true);
    $("#profiles-body").innerHTML = `<tr><td colspan="9" class="empty">${esc(e.message)}</td></tr>`;
  });

  setInterval(() => {
    if (state.view === "profiles") refresh().catch(() => {});
  }, 8000);
})();
