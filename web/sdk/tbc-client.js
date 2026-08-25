/**
 * TBC typed HTTP/WebSocket client (M17).
 * Use against tbc-server debug API on port 6014.
 */
export class TbcClient {
  constructor(baseUrl = "") {
    this.base = baseUrl.replace(/\/$/, "");
    this.ws = null;
  }

  async get(path) {
    const res = await fetch(`${this.base}${path}`);
    if (!res.ok) throw new Error(`GET ${path} → ${res.status}`);
    return res.json();
  }

  async post(path, body) {
    const res = await fetch(`${this.base}${path}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    if (!res.ok) throw new Error(`POST ${path} → ${res.status}`);
    return res.json();
  }

  async status() {
    return this.get("/api/status");
  }

  async login() {
    return this.get("/api/login");
  }

  async resume(iuoc) {
    return this.get(`/api/resume?iuoc=${iuoc}`);
  }

  async move(fwau, dx, dy, tick) {
    return this.post("/api/move", { fwau, dx, dy, tick });
  }

  async blink(fwau, x, y) {
    return this.post("/api/blink", { fwau, x, y });
  }

  async handoff(fwau, toFrame) {
    return this.post("/api/handoff", { fwau, to_frame: toFrame });
  }

  async attack(fwau, targetEntity = null) {
    return this.post("/api/attack", { fwau, target_entity: targetEntity });
  }

  async interact(fwau) {
    return this.post("/api/interact", { fwau });
  }

  async assist(fwau, targetEntity = null) {
    return this.post("/api/assist", { fwau, target_entity: targetEntity });
  }

  async speak(fwau, text) {
    return this.post("/api/speak", { fwau, text });
  }

  async psi(fwau, scope = "FutureSelf") {
    return this.post("/api/psi", { fwau, scope });
  }

  async grantConsent(fwau, helperIuoc, scope = "assist", ttlTicks = 50000) {
    return this.post("/api/consent", {
      fwau,
      helper_iuoc: helperIuoc,
      scope,
      ttl_ticks: ttlTicks,
    });
  }

  connectWebSocket(fwau, onSnapshot) {
    const proto = location.protocol === "https:" ? "wss:" : "ws:";
    this.ws = new WebSocket(`${proto}//${location.host}/ws`);
    this.ws.onmessage = (ev) => {
      try {
        const snap = JSON.parse(ev.data);
        onSnapshot(snap);
      } catch (_) {}
    };
    this.ws.onclose = () => {
      setTimeout(() => this.connectWebSocket(fwau, onSnapshot), 2000);
    };
    this.sendMove = (dx, dy, tick) => {
      if (this.ws?.readyState === WebSocket.OPEN) {
        this.ws.send(
          JSON.stringify({ kind: "move", fwau, dx, dy, tick })
        );
      }
    };
    return this.ws;
  }
}

// UMD for script tag without modules
if (typeof window !== "undefined") {
  window.TbcClient = TbcClient;
}
