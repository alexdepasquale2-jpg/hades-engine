//! QUIC gateway — spec §10 transport + M12 production TLS/mTLS and ops HTTP.

mod tls;

use axum::{routing::get, Json, Router};
use quinn::{Endpoint, ServerConfig};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tbc_engine::aum::AumCore;
use tbc_engine::intent::{Intent, Verb};
use tbc_engine::persist::SoulArchive;
use tbc_engine::shard::ShardNodeConfig;
use tbc_engine::transport::{
  drain_reliable, decode_move_datagram, encode_reliable, WireMessage,
};
use tbc_engine::types::{FwauId, IuocId, Tick, Vec3};
use tokio::sync::Mutex;
use tracing::{info, warn};

#[derive(Clone)]
struct SessionInfo {
  iuoc: IuocId,
  fwau: FwauId,
  frame_idx: usize,
}

struct GatewayState {
  aum: Mutex<AumCore>,
  sessions: Mutex<HashMap<FwauId, SessionInfo>>,
}

#[tokio::main]
async fn main() {
  tracing_subscriber::fmt()
    .with_env_filter("info")
    .init();

  let genesis = *blake3::hash(b"TBC-GENESIS-QUIC").as_bytes();
  let archive_path =
    std::env::var("TBC_ARCHIVE_PATH").unwrap_or_else(|_| "data/tbc-archive.db".into());
  let archive = SoulArchive::open(&archive_path).expect("open soul archive");
  let node = ShardNodeConfig::from_env();
  let aum = AumCore::boot_node_with_archive(genesis, archive, node.clone()).expect("boot node");
  info!(
    "Gateway node mode={} shard={:?} RWW backend={} connected={}",
    aum.node.label(),
    aum.node.shard_id,
    aum.rww.status().backend,
    aum.rww.status().connected
  );

  let tls_paths = tls::TlsPaths::from_env().expect("load TLS paths");
  if tls_paths.is_some() {
    info!("TLS: using TBC_TLS_CERT / TBC_TLS_KEY");
    if std::env::var("TBC_TLS_CLIENT_CA").is_ok() {
      info!("TLS: mTLS client CA configured");
    }
  } else {
    info!("TLS: self-signed dev certificate (set TBC_TLS_CERT and TBC_TLS_KEY for production)");
  }

  let state = Arc::new(GatewayState {
    aum: Mutex::new(aum),
    sessions: Mutex::new(HashMap::new()),
  });

  {
    let mut guard = state.aum.lock().await;
    for (i, frame) in guard.frames.iter_mut().enumerate() {
      let count = if node.distributed {
        40
      } else if i < 2 {
        40
      } else {
        8
      };
      frame.spawn_demo_world(count);
    }
  }

  spawn_sim_loop(state.clone());

  let health_state = state.clone();
  let health_port = std::env::var("TBC_HEALTH_PORT")
    .ok()
    .and_then(|p| p.parse().ok())
    .unwrap_or(9443);
  tokio::spawn(async move {
    let app = Router::new()
      .route("/health", get(health_handler))
      .route("/ready", get(ready_handler))
      .route("/metrics", get(metrics_handler))
      .with_state(health_state);
    let addr = SocketAddr::from(([0, 0, 0, 0], health_port));
    info!("Gateway ops HTTP on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind health");
    axum::serve(listener, app).await.expect("health server");
  });

  let quic_port = std::env::var("TBC_QUIC_PORT")
    .ok()
    .and_then(|p| p.parse().ok())
    .unwrap_or(4433);
  let addr: SocketAddr = format!("0.0.0.0:{}", quic_port).parse().unwrap();
  let endpoint = make_endpoint(addr, tls_paths).expect("QUIC endpoint");
  info!("TBC QUIC gateway listening on quic://{}", addr);

  while let Some(conn) = endpoint.accept().await {
    let state = state.clone();
    tokio::spawn(async move {
      if let Ok(connection) = conn.await {
        handle_connection(connection, state).await;
      }
    });
  }
}

fn spawn_sim_loop(state: Arc<GatewayState>) {
  tokio::spawn(async move {
    loop {
      let mut guard = state.aum.lock().await;
      for i in 0..guard.frames.len() {
        let dt = Duration::from_millis(guard.frames[i].spec.ruleset.dt_ms as u64);
        let steps = guard.frames[i].drain_elapsed(dt);
        if steps > 0 {
          guard.run_frame_ticks(i, steps);
        }
      }
      let inbound = guard.process_inbound_shard_crosses();
      let handoffs = guard.process_shard_handoffs();
      drop(guard);

      if !inbound.is_empty() || !handoffs.is_empty() {
        let mut sessions = state.sessions.lock().await;
        for (fwau, iuoc) in inbound {
          sessions.insert(
            fwau,
            SessionInfo {
              iuoc,
              fwau,
              frame_idx: 0,
            },
          );
        }
        for (old_fwau, new_fwau, to_idx) in handoffs {
          if new_fwau.0 == 0 {
            sessions.remove(&old_fwau);
            continue;
          }
          if let Some(info) = sessions.remove(&old_fwau) {
            sessions.insert(
              new_fwau,
              SessionInfo {
                iuoc: info.iuoc,
                fwau: new_fwau,
                frame_idx: to_idx,
              },
            );
          }
        }
      }

      tokio::time::sleep(Duration::from_millis(50)).await;
    }
  });
}

async fn health_handler(
  axum::extract::State(state): axum::extract::State<Arc<GatewayState>>,
) -> Json<tbc_engine::ops::OpsSnapshot> {
  let sessions = state.sessions.lock().await.len();
  let guard = state.aum.lock().await;
  Json(guard.ops_snapshot(sessions))
}

async fn ready_handler(
  axum::extract::State(state): axum::extract::State<Arc<GatewayState>>,
) -> impl axum::response::IntoResponse {
  let sessions = state.sessions.lock().await.len();
  let guard = state.aum.lock().await;
  let snap = guard.ops_snapshot(sessions);
  let status = if snap.ready {
    axum::http::StatusCode::OK
  } else {
    axum::http::StatusCode::SERVICE_UNAVAILABLE
  };
  (status, Json(snap))
}

async fn metrics_handler(
  axum::extract::State(state): axum::extract::State<Arc<GatewayState>>,
) -> impl axum::response::IntoResponse {
  let sessions = state.sessions.lock().await.len();
  let guard = state.aum.lock().await;
  let body = guard.ops_snapshot(sessions).prometheus_lines();
  (
    [(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4")],
    body,
  )
}

fn make_endpoint(
  addr: SocketAddr,
  tls_paths: Option<tls::TlsPaths>,
) -> Result<Endpoint, Box<dyn std::error::Error>> {
  let rustls_config = tls::build_rustls_server_config(tls_paths)?;
  let quic_crypto = quinn::crypto::rustls::QuicServerConfig::try_from(rustls_config)?;
  let server_config = ServerConfig::with_crypto(Arc::new(quic_crypto));
  Endpoint::server(server_config, addr).map_err(|e| e.into())
}

async fn handle_connection(connection: quinn::Connection, state: Arc<GatewayState>) {
  let remote = connection.remote_address();
  info!("QUIC client connected from {}", remote);

  let state_bi = state.clone();
  let conn_bi = connection.clone();
  tokio::spawn(async move {
    while let Ok((mut send, mut recv)) = conn_bi.accept_bi().await {
      let state_c = state_bi.clone();
      tokio::spawn(async move {
        let mut buf = Vec::new();
        loop {
          match recv.read_chunk(8192, true).await {
            Ok(Some(chunk)) => {
              buf.extend_from_slice(&chunk.bytes);
              for msg in drain_reliable(&mut buf) {
                handle_reliable(&state_c, &mut send, msg).await;
              }
            }
            Ok(None) => break,
            Err(_) => break,
          }
        }
      });
    }
  });

  let state_d = state.clone();
  tokio::spawn(async move {
    loop {
      match connection.read_datagram().await {
        Ok(bytes) => {
          if let Some((fwau_raw, tick, dx, dy)) = decode_move_datagram(&bytes) {
            submit_move(&state_d, FwauId(fwau_raw), Tick(tick), dx, dy).await;
          }
        }
        Err(_) => break,
      }
    }
  });
}

async fn handle_reliable(
  state: &Arc<GatewayState>,
  send: &mut quinn::SendStream,
  msg: WireMessage,
) {
  match msg.kind.as_str() {
    "login" => {
      let mut guard = state.aum.lock().await;
      let iuoc = guard.iuoc.create_soul();
      let pos = spawn_pos_for_node(&guard.node);
      let fwau = guard.bind_player(0, iuoc, pos).unwrap();
      state.sessions.lock().await.insert(
        fwau,
        SessionInfo {
          iuoc,
          fwau,
          frame_idx: 0,
        },
      );
      let snap = guard.frames[0].build_snapshot();
      let reply = WireMessage::login_ok(fwau.0, iuoc.0, snap.to_json_value());
      if let Ok(bytes) = encode_reliable(&reply) {
        let _ = send.write_all(&bytes).await;
      }
    }
    "move" => {
      if let (Some(fwau), Some(tick), Some(payload)) =
        (msg.fwau_u128(), msg.tick, msg.payload.as_object())
      {
        let dx = payload.get("dx").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let dy = payload.get("dy").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        submit_move(state, FwauId(fwau), Tick(tick), dx, dy).await;
        push_snapshot(state, FwauId(fwau), send).await;
      }
    }
    "snapshot" => {
      if let Some(fwau) = msg.fwau_u128() {
        push_snapshot(state, FwauId(fwau), send).await;
      }
    }
    _ => warn!("unknown wire kind: {}", msg.kind),
  }
}

fn spawn_pos_for_node(node: &ShardNodeConfig) -> Vec3 {
  match node.shard_id {
    Some(1) => Vec3::new(80.0, 0.0, 0.0),
    _ => Vec3::new(-80.0, 0.0, 0.0),
  }
}

async fn push_snapshot(
  state: &Arc<GatewayState>,
  fwau: FwauId,
  send: &mut quinn::SendStream,
) {
  let snap = {
    let mut guard = state.aum.lock().await;
    let frame_idx = state
      .sessions
      .lock()
      .await
      .get(&fwau)
      .map(|s| s.frame_idx)
      .unwrap_or(0);
    if frame_idx < guard.frames.len() {
      Some(guard.frames[frame_idx].build_snapshot())
    } else {
      None
    }
  };
  if let Some(s) = snap {
    let reply = WireMessage::snapshot(s.to_json_value());
    if let Ok(bytes) = encode_reliable(&reply) {
      let _ = send.write_all(&bytes).await;
    }
  }
}

async fn submit_move(state: &Arc<GatewayState>, fwau: FwauId, tick: Tick, dx: f32, dy: f32) {
  let frame_idx = state
    .sessions
    .lock()
    .await
    .get(&fwau)
    .map(|s| s.frame_idx)
    .unwrap_or(0);
  let mut guard = state.aum.lock().await;
  if frame_idx >= guard.frames.len() {
    return;
  }
  let frame = &mut guard.frames[frame_idx];
  let mut payload = Vec::with_capacity(8);
  payload.extend_from_slice(&dx.to_le_bytes());
  payload.extend_from_slice(&dy.to_le_bytes());
  let intent = Intent {
    fwau,
    tick,
    seq: tick.0 as u32,
    verb: Verb::Move,
    payload,
    consent: None,
    checksum: 0,
  };
  frame.submit_intent(intent).ok();
}
