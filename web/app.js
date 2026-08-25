const SCALE = 3;
const ORIGIN_X = 450;
const ORIGIN_Y = 300;
const SHARD_SEAM_X = 0;

let session = { iuoc: null, fwau: null, band: "Settled", frame: "pmr.v1" };
let entities = [];
let keys = {};
let ws = null;
let moveVec = { dx: 0, dy: 0 };
let clientTick = 0;
let isNpmr = false;
let shardId = null;
let nodeDistributed = false;
let worldProps = [];

function updateOps(ops) {
  if (!ops) return;
  document.getElementById("ops-ready").textContent = ops.ready ? "yes" : "no";
  document.getElementById("ops-status").textContent = ops.status || "—";
  document.getElementById("ops-version").textContent = ops.version || "—";
}

function updateNodeStatus(node) {
  if (!node) return;
  nodeDistributed = node.distributed;
  document.getElementById("node-mode").textContent = node.mode || "cluster";
  document.getElementById("node-shard").textContent =
    node.shard_id != null ? `shard-${node.shard_id}` : "all";
  document.querySelectorAll(".npmr-only").forEach((el) => {
    el.style.display = nodeDistributed ? "none" : "";
  });
}

const canvas = document.getElementById("world");
const ctx = canvas.getContext("2d");

document.getElementById("btn-login").addEventListener("click", login);
document.getElementById("btn-resume").addEventListener("click", resumeSoul);
document.getElementById("btn-psi").addEventListener("click", () => queryPsi("FutureSelf"));
document.getElementById("btn-past").addEventListener("click", () => queryPsi("PastOwn"));
document.getElementById("btn-psi-island").addEventListener("click", () => queryPsi("FutureIsland"));
document.getElementById("btn-psi-shared").addEventListener("click", () => queryPsi("PastShared"));
document.getElementById("btn-psi-rww").addEventListener("click", () => queryPsi("RwwQuery"));
document.getElementById("btn-speak").addEventListener("click", speak);
document.getElementById("btn-consent").addEventListener("click", grantConsent);
document.getElementById("btn-npmr").addEventListener("click", () => handoff("npmr.academy.v1"));
document.getElementById("btn-npmr-dream").addEventListener("click", () => handoff("npmr.dream.v1"));
document.getElementById("btn-pmr").addEventListener("click", () => handoff("pmr.v1"));
document.getElementById("btn-unbind").addEventListener("click", unbindDeath);
document.getElementById("btn-attack").addEventListener("click", () => attackNearest());
document.getElementById("btn-interact").addEventListener("click", () => interactNearest());

window.addEventListener("keydown", (e) => {
  keys[e.code] = true;
  if (e.code === "Space") {
    e.preventDefault();
    queryPsi("FutureSelf");
  }
  if (e.code === "KeyB" && isNpmr) blinkTowardCursor();
  if (e.code === "KeyF") {
    e.preventDefault();
    attackNearest();
  }
  if (e.code === "KeyE") {
    e.preventDefault();
    interactNearest();
  }
  if (e.code === "KeyT") {
    e.preventDefault();
    speak();
  }
  updateMoveVec();
});

window.addEventListener("keyup", (e) => {
  keys[e.code] = false;
  updateMoveVec();
});

function updateMoveVec() {
  let dx = 0, dy = 0;
  if (keys["KeyW"] || keys["ArrowUp"]) dy -= 1;
  if (keys["KeyS"] || keys["ArrowDown"]) dy += 1;
  if (keys["KeyA"] || keys["ArrowLeft"]) dx -= 1;
  if (keys["KeyD"] || keys["ArrowRight"]) dx += 1;
  const len = Math.hypot(dx, dy);
  if (len > 0) { dx /= len; dy /= len; }
  moveVec = { dx, dy };

  if (session.fwau && ws && ws.readyState === WebSocket.OPEN) {
    clientTick += 1;
    document.getElementById("client-tick").textContent = clientTick;
    ws.send(JSON.stringify({
      kind: "move",
      fwau: session.fwau,
      dx: moveVec.dx,
      dy: moveVec.dy,
      tick: clientTick,
    }));
  }
}

async function login() {
  const res = await fetch("/api/login");
  const data = await res.json();
  session = { iuoc: data.iuoc, fwau: data.fwau, band: data.quality_band, frame: data.frame };
  localStorage.setItem("tbc_iuoc", String(data.iuoc));
  showResumeButton();
  isNpmr = data.frame.includes("npmr");
  document.getElementById("session-info").textContent =
    `IUOC ${data.iuoc.toString().slice(0, 8)}…`;
  document.getElementById("frame-name").textContent = data.frame;
  setBand(data.quality_band);
  clearOffers();
  connectWs();
}

function showResumeButton() {
  const stored = localStorage.getItem("tbc_iuoc");
  const btn = document.getElementById("btn-resume");
  if (stored && !session.fwau) {
    btn.classList.remove("hidden");
  } else if (stored) {
    btn.classList.remove("hidden");
  } else {
    btn.classList.add("hidden");
  }
}

async function resumeSoul() {
  const iuoc = localStorage.getItem("tbc_iuoc");
  if (!iuoc) return;
  const res = await fetch(`/api/resume?iuoc=${iuoc}`);
  const data = await res.json();
  if (!data.fwau) {
    flash("reject-flash", data.message);
    return;
  }
  session = { iuoc: data.iuoc, fwau: data.fwau, band: data.quality_band, frame: data.frame };
  isNpmr = data.frame.includes("npmr");
  document.getElementById("session-info").textContent =
    `IUOC ${data.iuoc.toString().slice(0, 8)}… (resumed)`;
  document.getElementById("frame-name").textContent = data.frame;
  setBand(data.quality_band);
  flash("correction-flash", data.message);
  clearOffers();
  connectWs();
}

function setBand(band) {
  const el = document.getElementById("band-display");
  el.textContent = band;
  el.className = "band " + band.toLowerCase();
}

async function handoff(toFrame) {
  if (!session.fwau) return;
  const res = await fetch("/api/handoff", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ fwau: session.fwau, to_frame: toFrame }),
  });
  const data = await res.json();
  session.fwau = data.fwau;
  session.frame = data.frame;
  isNpmr = data.frame.includes("npmr");
  document.getElementById("frame-name").textContent = data.frame;
  setBand(data.quality_band);
}

async function blinkTowardCursor() {
  if (!session.fwau) return;
  const player = entities.find((e) => e.is_player);
  if (!player) return;
  const tx = player.x + moveVec.dx * 30;
  const ty = player.y + moveVec.dy * 30;
  await fetch("/api/blink", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ fwau: session.fwau, x: tx, y: ty }),
  });
}

async function queryPsi(scope) {
  if (!session.fwau) return;
  const res = await fetch("/api/psi", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ fwau: session.fwau, scope }),
  });
  const data = await res.json();
  document.getElementById("psi-budget").textContent =
    data.psi_budget_remaining != null ? data.psi_budget_remaining.toFixed(3) : "—";
  document.getElementById("psi-message").textContent =
    data.message || (data.allowed ? "" : "Psi blocked");

  const recallList = document.getElementById("psi-recall");
  const oddsList = document.getElementById("psi-odds");
  recallList.innerHTML = "";
  oddsList.innerHTML = "";

  if (data.recall && data.recall.length) {
    data.recall.forEach((r) => {
      const li = document.createElement("li");
      li.textContent = r;
      recallList.appendChild(li);
    });
  }
  if (data.odds && data.odds.length) {
    data.odds.forEach((o) => {
      const li = document.createElement("li");
      const label = o.label || o[0];
      const p = o.p != null ? o.p : o[1];
      li.textContent = `${(p * 100).toFixed(1)}% — ${label}`;
      oddsList.appendChild(li);
    });
  }
}

async function speak() {
  if (!session.fwau) return;
  const input = document.getElementById("speak-text");
  const text = (input.value || "").trim();
  if (!text) return;
  const res = await fetch("/api/speak", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ fwau: session.fwau, text }),
  });
  const data = await res.json();
  document.getElementById("speak-result").textContent = data.message || "";
  if (data.heard) {
    flash("correction-flash", data.message);
    input.value = "";
  } else {
    flash("reject-flash", data.message);
  }
}

async function grantConsent() {
  if (!session.fwau || !session.iuoc) return;
  const res = await fetch("/api/consent", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      fwau: session.fwau,
      target_iuoc: session.iuoc,
      scope: "assist",
      ttl_ticks: 50000,
    }),
  });
  const data = await res.json();
  const msg = data.granted
    ? `Consent granted (${data.pact})`
    : data.pact || "Consent failed";
  document.getElementById("speak-result").textContent = msg;
  flash(data.granted ? "correction-flash" : "reject-flash", msg);
}

function clearOffers() {
  document.getElementById("offer-list").innerHTML = "";
}

function showOffers(offers) {
  const list = document.getElementById("offer-list");
  list.innerHTML = "";
  if (!offers || offers.length === 0) return;
  offers.forEach((o) => {
    const li = document.createElement("li");
    const btn = document.createElement("button");
    btn.className = "offer-btn";
    btn.textContent = `${o.title} (shard ${o.start_shard})`;
    btn.title = o.situation;
    btn.addEventListener("click", () => acceptOffer(o.template_id));
    li.appendChild(btn);
    const meta = document.createElement("span");
    meta.className = "offer-meta";
    meta.textContent = `${o.faction} · ${o.odds_label}`;
    li.appendChild(meta);
    list.appendChild(li);
  });
}

async function unbindDeath() {
  if (!session.fwau) return;
  const res = await fetch("/api/unbind", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ fwau: session.fwau }),
  });
  const data = await res.json();
  session.fwau = null;
  flash("reject-flash", data.message || "Unbound");
  showOffers(data.offers);
}

async function attackNearest() {
  if (!session.fwau) return;
  const res = await fetch("/api/attack", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ fwau: session.fwau }),
  });
  const data = await res.json();
  if (data.hit) {
    flash("correction-flash", data.message);
    if (!data.killed) {
      document.getElementById("player-stamina").textContent = Math.round(data.player_stamina);
    }
  } else {
    flash("reject-flash", data.message);
  }
  if (data.killed && data.target_entity != null) {
  }
  const snapRes = await fetch("/api/move", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ fwau: session.fwau, dx: 0, dy: 0 }),
  });
  if (snapRes.ok) onSnapshot(await snapRes.json());
}

async function interactNearest() {
  if (!session.fwau) return;
  const res = await fetch("/api/interact", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ fwau: session.fwau }),
  });
  const data = await res.json();
  if (data.hit) {
    flash("correction-flash", data.message);
    updateInventory(data.inventory);
  } else {
    flash("reject-flash", data.message);
  }
}

function updateInventory(items) {
  const list = document.getElementById("inventory-list");
  list.innerHTML = "";
  if (!items || items.length === 0) return;
  items.forEach((item) => {
    const li = document.createElement("li");
    li.textContent = item;
    list.appendChild(li);
  });
}

function updatePlayerStats(snap) {
  const player = (snap.entities || []).find((e) => e.is_player);
  if (!player) {
    document.getElementById("player-hp").textContent = "—";
    document.getElementById("player-stamina").textContent = "—";
    updateInventory([]);
    return;
  }
  document.getElementById("player-hp").textContent =
    player.hp != null ? Math.round(player.hp) : "—";
  document.getElementById("player-stamina").textContent =
    player.stamina != null ? Math.round(player.stamina) : "—";
  updateInventory(player.inventory || []);
}

async function acceptOffer(templateId) {
  if (!session.iuoc) return;
  const res = await fetch("/api/reincarnate", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ iuoc: session.iuoc, template_id: templateId }),
  });
  const data = await res.json();
  if (data.fwau) {
    session.fwau = data.fwau;
    session.frame = data.frame;
    isNpmr = data.frame.includes("npmr");
    document.getElementById("frame-name").textContent = data.frame;
    setBand(data.quality_band);
    clearOffers();
    flash("correction-flash", data.message);
    if (!ws || ws.readyState !== WebSocket.OPEN) connectWs();
  } else {
    flash("reject-flash", data.message);
  }
}

function connectWs() {
  const proto = location.protocol === "https:" ? "wss:" : "ws:";
  ws = new WebSocket(`${proto}//${location.host}/ws`);
  ws.onmessage = (ev) => {
    try {
      const snap = JSON.parse(ev.data);
      onSnapshot(snap);
    } catch (_) {}
  };
  ws.onclose = () => setTimeout(connectWs, 2000);
}

function flash(elId, text) {
  const el = document.getElementById(elId);
  if (text) el.textContent = text;
  el.classList.remove("hidden");
  setTimeout(() => el.classList.add("hidden"), 800);
}

function updateRww(status, events) {
  if (status) {
    document.getElementById("rww-backend").textContent = status.backend;
    document.getElementById("rww-connected").textContent = status.connected ? "yes" : "no";
  }
  if (!events) return;
  const list = document.getElementById("rww-events");
  list.innerHTML = "";
  events.forEach((m) => {
    const li = document.createElement("li");
    let text = `${m.subject} tick=${m.at_tick}`;
    try {
      const bytes = Array.isArray(m.payload) ? m.payload : [];
      const body = JSON.parse(new TextDecoder().decode(new Uint8Array(bytes)));
      if (body.event === "Handoff") {
        text = `Handoff ${body.from} → ${body.to}`;
      } else if (body.event) {
        text = `${body.event} · ${body.frame || ""}`;
      }
    } catch (_) {}
    li.textContent = text;
    list.appendChild(li);
  });
}

async function pollRww() {
  try {
    const res = await fetch("/api/rww?subject=rww.handoff&limit=5");
    const data = await res.json();
    updateRww(data.status, data.messages);
  } catch (_) {}
}

function updateProfiler(prof) {
  if (!prof) return;
  document.getElementById("island-count").textContent = prof.island_count;
  document.getElementById("island-steps").textContent = prof.total_steps_last_tick;
  document.getElementById("island-max").textContent = prof.max_steps_per_island;
  const budgetEl = document.getElementById("island-budget");
  budgetEl.textContent = prof.within_budget ? "OK" : "OVER";
  budgetEl.className = prof.within_budget ? "budget-ok" : "budget-over";
}

function updateGuardrails(g) {
  if (!g) return;
  document.getElementById("guard-stalls").textContent = g.stall_count;
  document.getElementById("guard-overruns").textContent = g.budget_overruns;
  document.getElementById("guard-rate").textContent = g.rate_limited;
  document.getElementById("guard-throttled").textContent = g.throttled_ticks;
}

function onSnapshot(snap) {
  if (snap.corrections && snap.corrections.length > 0) {
    flash("correction-flash", "Rewind correction");
    const c = snap.corrections[0];
    if (c.poses) {
      entities = entities.map((e) => {
        const p = c.poses.find((pp) => pp.entity.index === e.entity.index);
        return p ? { ...e, x: p.x, y: p.y, z: p.z } : e;
      });
    }
  }
  if (snap.rejects && snap.rejects.length > 0) {
    flash("reject-flash", snap.rejects[0].reason);
  }

  entities = snap.entities || [];
  worldProps = snap.props || [];
  if (snap.recent_speaks && snap.recent_speaks.length) {
    const last = snap.recent_speaks[snap.recent_speaks.length - 1];
    if (last.text) {
      document.getElementById("speak-result").textContent = `Heard nearby: “${last.text}”`;
    }
  }
  document.getElementById("tick").textContent = snap.tick;
  document.getElementById("entity-count").textContent = entities.length;
  isNpmr = (snap.ruleset_id || "").includes("npmr");
  shardId = snap.shard_id;
  document.getElementById("shard-id").textContent =
    shardId != null ? `shard-${shardId}` : (isNpmr ? "NPMR" : "—");
  updateProfiler(snap.island_profiler);
  updateGuardrails(snap.guardrails);
  updatePlayerStats(snap);
  draw();
}

async function pollStatus() {
  try {
    const res = await fetch("/api/status");
    const data = await res.json();
    document.getElementById("dt").textContent = `${data.dt_ms} ms`;
    if (!session.fwau) {
      document.getElementById("tick").textContent = data.tick;
      document.getElementById("entity-count").textContent = data.entities;
    }
    updateProfiler(data.island_profiler);
    updateGuardrails(data.guardrails);
    if (data.archive) {
      document.getElementById("archive-souls").textContent = data.archive.souls;
      document.getElementById("archive-packets").textContent = data.archive.packets;
    }
    updateRww(data.rww, null);
    updateNodeStatus(data.node);
    const healthRes = await fetch("/health");
    if (healthRes.ok) {
      updateOps(await healthRes.json());
    }
  } catch (_) {}
}

function draw() {
  ctx.fillStyle = isNpmr ? "#0a0814" : "#060a10";
  ctx.fillRect(0, 0, canvas.width, canvas.height);

  const cellPx = 32 * SCALE;
  ctx.strokeStyle = isNpmr ? "#1a1530" : "#141c28";
  ctx.lineWidth = 1;
  for (let x = ORIGIN_X % cellPx; x < canvas.width; x += cellPx) {
    ctx.beginPath();
    ctx.moveTo(x, 0);
    ctx.lineTo(x, canvas.height);
    ctx.stroke();
  }
  for (let y = ORIGIN_Y % cellPx; y < canvas.height; y += cellPx) {
    ctx.beginPath();
    ctx.moveTo(0, y);
    ctx.lineTo(canvas.width, y);
    ctx.stroke();
  }

  // PMR shard seam at world x=0
  if (!isNpmr) {
    const seamPx = ORIGIN_X + SHARD_SEAM_X * SCALE;
    ctx.strokeStyle = "rgba(250, 204, 21, 0.55)";
    ctx.lineWidth = 2;
    ctx.setLineDash([6, 6]);
    ctx.beginPath();
    ctx.moveTo(seamPx, 0);
    ctx.lineTo(seamPx, canvas.height);
    ctx.stroke();
    ctx.setLineDash([]);
    ctx.fillStyle = "rgba(250, 204, 21, 0.7)";
    ctx.font = "10px sans-serif";
    ctx.fillText("shard seam", seamPx + 4, 16);
  }

  const player = entities.find((e) => e.is_player);
  if (player) {
    ctx.strokeStyle = isNpmr ? "rgba(180, 120, 255, 0.3)" : "rgba(61, 158, 255, 0.25)";
    ctx.beginPath();
    ctx.arc(
      ORIGIN_X + player.x * SCALE,
      ORIGIN_Y - player.y * SCALE,
      128 * SCALE,
      0,
      Math.PI * 2
    );
    ctx.stroke();
  }

  worldProps.forEach((p) => {
    const px = ORIGIN_X + p.x * SCALE;
    const py = ORIGIN_Y - p.y * SCALE;
    ctx.fillStyle = "rgba(236, 72, 153, 0.75)";
    ctx.beginPath();
    ctx.moveTo(px, py - 5);
    ctx.lineTo(px + 5, py);
    ctx.lineTo(px, py + 5);
    ctx.lineTo(px - 5, py);
    ctx.closePath();
    ctx.fill();
  });

  entities.forEach((e) => {
    if (!e.awake && !e.is_player) return;
    const px = ORIGIN_X + e.x * SCALE;
    const py = ORIGIN_Y - e.y * SCALE;
    const r = e.is_player ? 7 : 4;

    ctx.beginPath();
    ctx.arc(px, py, r, 0, Math.PI * 2);
    if (e.is_player) {
      ctx.fillStyle = isNpmr ? "#c084fc" : "#4ade80";
    } else {
      ctx.fillStyle = isNpmr ? "#a78bfa" : "#f59e0b";
    }
    ctx.fill();
  });

  ctx.fillStyle = "#7d8da1";
  ctx.font = "11px sans-serif";
  const shardLabel = shardId != null ? `shard-${shardId}` : "NPMR";
  const label = isNpmr
    ? "NPMR-Academy · blink enabled"
    : `PMR ${shardLabel} · walk → to cross seam`;
  ctx.fillText(label, 12, canvas.height - 10);
}

setInterval(pollStatus, 1000);
setInterval(pollRww, 2000);
setInterval(updateMoveVec, 50);
showResumeButton();
draw();
