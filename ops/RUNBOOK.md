# TBC Engine Operations Runbook (M12)

Operations guide for production-style deployments: health probes, metrics, TLS, and multi-node PMR.

## Service map

| Service | Protocol | Default port | Purpose |
| --- | --- | --- | --- |
| `tbc-server` (cluster) | HTTP/WebSocket | 6014 | Debug UI + game API |
| `tbc-server` shard-00 | HTTP/WebSocket | 6020 | M11 distributed west shard |
| `tbc-server` shard-01 | HTTP/WebSocket | 6021 | M11 distributed east shard |
| `tbc-gateway` | QUIC | 4433 | Production client transport |
| `tbc-gateway` ops | HTTP | 9443 | Health + Prometheus metrics |
| NATS JetStream | TCP | 4222 | RWW replication (`TBC_RWW` stream) |

## Health and readiness

### HTTP debug server (`tbc-server`)

| Endpoint | Use | Success |
| --- | --- | --- |
| `GET /health` | Liveness + JSON ops snapshot | Always 200 when process is up |
| `GET /ready` | Readiness for load balancers | 200 when frames booted, archive OK, NATS connected (if configured) |
| `GET /metrics` | Prometheus scrape | `text/plain; version=0.0.4` |

Example:

```bash
curl -s http://127.0.0.1:6014/ready | jq .status,.ready,.checks
curl -s http://127.0.0.1:6014/metrics | head
```

### QUIC gateway ops HTTP

Same paths on `TBC_HEALTH_PORT` (default **9443**):

```bash
curl -s http://127.0.0.1:9443/health
curl -s http://127.0.0.1:9443/metrics
```

Kubernetes probes:

```yaml
livenessProbe:
  httpGet:
    path: /health
    port: 6014
readinessProbe:
  httpGet:
    path: /ready
    port: 6014
```

## Prometheus metrics

Key series (labels include `frame` and `ruleset`):

- `tbc_ready` — 1 when ready, 0 otherwise
- `tbc_sessions_active` — connected HTTP/QUIC sessions
- `tbc_tick`, `tbc_entities`, `tbc_fwau_count` — per-frame simulation
- `tbc_guardrail_*` — stalls, overruns, rate limits, budget
- `tbc_archive_souls`, `tbc_archive_packets` — SQLite archive size
- `tbc_rww_connected` — NATS JetStream link

## TLS / mTLS (QUIC gateway)

| Variable | Description |
| --- | --- |
| `TBC_TLS_CERT` | PEM server certificate path |
| `TBC_TLS_KEY` | PEM private key path |
| `TBC_TLS_CLIENT_CA` | Optional PEM CA for **mTLS** (require client certs) |
| `TBC_QUIC_PORT` | QUIC listen port (default 4433) |
| `TBC_HEALTH_PORT` | Ops HTTP port (default 9443) |

Without cert paths, the gateway generates a **dev self-signed** cert (not for production).

Generate dev certs:

```bash
./deploy/generate-dev-tls.sh deploy/tls
export TBC_TLS_CERT=deploy/tls/cert.pem
export TBC_TLS_KEY=deploy/tls/key.pem
cargo run -p tbc-gateway --release
```

## Environment reference

| Variable | Default | Description |
| --- | --- | --- |
| `TBC_ARCHIVE_PATH` | `data/tbc-archive.db` | SQLite soul archive |
| `TBC_NATS_URL` | — | NATS URL; omit for in-memory RWW |
| `TBC_NATS_STREAM` | `TBC_RWW` | JetStream stream name |
| `TBC_SHARD_ID` | — | `0` or `1` for standalone shard node |
| `TBC_PORT` | 6014/6020/6021 | HTTP server port override |
| `RUST_LOG` | `info` | Log filter (`tracing`) |

## Docker Compose

```bash
cd deploy
cp env.example .env
./generate-dev-tls.sh tls
docker compose up --build
```

- Cluster UI: http://localhost:6014
- Gateway ops: http://localhost:9443/health
- QUIC: `quic://localhost:4433`

Optional shard profile:

```bash
docker compose --profile shards up --build
```

## Common issues

### `/ready` returns 503

1. Check `checks` in JSON — often `rww` when `TBC_NATS_URL` is set but NATS is down.
2. Verify archive path is writable (`tbc_archive_souls` in metrics).
3. Guardrail `degraded` — beam budget overrun; reduce entities or inspect `tbc_guardrail_overruns`.

### Shard cross does not complete (M11)

1. Both shard nodes must share `TBC_ARCHIVE_PATH` (or volume) and `TBC_NATS_URL`.
2. Source publishes `rww.shard.cross`; confirm with `curl /api/rww` on cluster or NATS CLI.
3. Client must connect to target shard port after crossing (session is dropped on source).

### QUIC clients cannot connect

1. Cert must match hostname or use dev cert with SAN `localhost`.
2. For mTLS, client must present cert signed by `TBC_TLS_CLIENT_CA`.
3. Confirm gateway ops `/ready` is 200 before testing QUIC.

## Backup

Soul archive is a single SQLite file at `TBC_ARCHIVE_PATH`. Stop writers or use filesystem snapshot:

```bash
sqlite3 data/tbc-archive.db ".backup backup-$(date +%F).db"
```

## Logs

Structured logs via `tracing`. Increase verbosity:

```bash
RUST_LOG=debug cargo run -p tbc-server --release
```

Look for `Inbound shard cross`, `Outbound shard cross`, and `NATS RWW unavailable` warnings.
