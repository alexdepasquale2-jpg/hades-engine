const SCALE = 3;
const ORIGIN_X = 450;
const ORIGIN_Y = 300;

let session = { iuoc: null, fwau: null, band: "Settled" };
let entities = [];
let keys = {};
let ws = null;
let moveVec = { dx: 0, dy: 0 };

const canvas = document.getElementById("world");
const ctx = canvas.getContext("2d");

document.getElementById("btn-login").addEventListener("click", login);
document.getElementById("btn-psi").addEventListener("click", queryPsi);

window.addEventListener("keydown", (e) => {
  keys[e.code] = true;
  if (e.code === "Space") {
    e.preventDefault();
    queryPsi();
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
    ws.send(JSON.stringify({
      kind: "move",
      fwau: session.fwau,
      dx: moveVec.dx,
      dy: moveVec.dy,
    }));
  }
}

async function login() {
  const res = await fetch("/api/login");
  const data = await res.json();
  session = { iuoc: data.iuoc, fwau: data.fwau, band: data.quality_band };
  document.getElementById("session-info").textContent =
    `IUOC ${data.iuoc.toString().slice(0, 8)}… · FWAU bound`;
  setBand(data.quality_band);
  connectWs();
}

function setBand(band) {
  const el = document.getElementById("band-display");
  el.textContent = band;
  el.className = "band " + band.toLowerCase();
}

async function queryPsi() {
  if (!session.fwau) return;
  const res = await fetch("/api/psi", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ fwau: session.fwau }),
  });
  const data = await res.json();
  const list = document.getElementById("psi-odds");
  list.innerHTML = "";
  if (!data.odds.length) {
    list.innerHTML = "<li>No probable futures in beam</li>";
    return;
  }
  data.odds.forEach((o) => {
    const li = document.createElement("li");
    li.textContent = `${(o.p * 100).toFixed(1)}% — ${o.label}`;
    list.appendChild(li);
  });
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

function onSnapshot(snap) {
  entities = snap.entities || [];
  document.getElementById("tick").textContent = snap.tick;
  document.getElementById("entity-count").textContent = entities.length;
  document.getElementById("ai-count").textContent = snap.ai_count ?? 0;
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
  ctx.fillStyle = "#060a10";
  ctx.fillRect(0, 0, canvas.width, canvas.height);

  // Grid — 32m cells (level 0)
  ctx.strokeStyle = "#141c28";
  ctx.lineWidth = 1;
  const cellPx = 32 * SCALE;
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

  // Interest radius (128m)
  const player = entities.find((e) => e.is_player);
  if (player) {
    ctx.strokeStyle = "rgba(61, 158, 255, 0.25)";
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
      ctx.fillStyle = "#4ade80";
    } else if (e.awake) {
      ctx.fillStyle = "#f59e0b";
    } else {
      ctx.fillStyle = "#475569";
    }
    ctx.fill();

    if (e.is_player) {
      ctx.strokeStyle = "#4ade80";
      ctx.lineWidth = 2;
      ctx.stroke();
    }
  });

  ctx.fillStyle = "#7d8da1";
  ctx.font = "11px sans-serif";
  ctx.fillText("PMR-Prime · observation allocates compute", 12, canvas.height - 10);
}

setInterval(pollStatus, 1000);
setInterval(updateMoveVec, 50);
draw();
