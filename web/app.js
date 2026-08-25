const SCALE = 3;
const ORIGIN_X = 450;
const ORIGIN_Y = 300;

let session = { iuoc: null, fwau: null, band: "Settled", frame: "pmr.v1" };
let entities = [];
let keys = {};
let ws = null;
let moveVec = { dx: 0, dy: 0 };
let clientTick = 0;
let isNpmr = false;

const canvas = document.getElementById("world");
const ctx = canvas.getContext("2d");

document.getElementById("btn-login").addEventListener("click", login);
document.getElementById("btn-psi").addEventListener("click", () => queryPsi("FutureSelf"));
document.getElementById("btn-past").addEventListener("click", () => queryPsi("PastOwn"));
document.getElementById("btn-npmr").addEventListener("click", () => handoff("npmr.academy.v1"));
document.getElementById("btn-pmr").addEventListener("click", () => handoff("pmr.v1"));

window.addEventListener("keydown", (e) => {
  keys[e.code] = true;
  if (e.code === "Space") {
    e.preventDefault();
    queryPsi("FutureSelf");
  }
  if (e.code === "KeyB" && isNpmr) blinkTowardCursor();
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
  isNpmr = data.frame.includes("npmr");
  document.getElementById("session-info").textContent =
    `IUOC ${data.iuoc.toString().slice(0, 8)}…`;
  document.getElementById("frame-name").textContent = data.frame;
  setBand(data.quality_band);
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
  if (scope === "PastOwn") {
    const list = document.getElementById("psi-recall");
    list.innerHTML = "";
    data.recall.forEach((r) => {
      const li = document.createElement("li");
      li.textContent = r;
      list.appendChild(li);
    });
  } else {
    const list = document.getElementById("psi-odds");
    list.innerHTML = "";
    data.odds.forEach((o) => {
      const li = document.createElement("li");
      li.textContent = `${(o.p * 100).toFixed(1)}% — ${o.label}`;
      list.appendChild(li);
    });
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
  document.getElementById("tick").textContent = snap.tick;
  document.getElementById("entity-count").textContent = entities.length;
  isNpmr = (snap.ruleset_id || "").includes("npmr");
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
  const label = isNpmr ? "NPMR-Academy · blink enabled" : "PMR-Prime · server-authoritative";
  ctx.fillText(label, 12, canvas.height - 10);
}

setInterval(pollStatus, 1000);
setInterval(updateMoveVec, 50);
draw();
