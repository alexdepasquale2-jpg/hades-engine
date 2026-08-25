use axum::{
  extract::{
    ws::{Message, WebSocket, WebSocketUpgrade},
    State,
  },
  response::{Html, IntoResponse},
  routing::get,
  Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tbc_engine::aum::{AumConfig, AumCore};
use tbc_engine::frame::FrameSnapshot;
use tbc_engine::intent::{Intent, Verb};
use tbc_engine::ruleset::Ruleset;
use tbc_engine::types::{FwauId, IuocId, Vec3};
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing::info;

struct AppState {
  aum: Mutex<AumCore>,
}

#[derive(Serialize)]
struct StatusResponse {
  tick: u64,
  dt_ms: u16,
  entities: usize,
  fwau_count: usize,
  ruleset: String,
}

#[derive(Serialize)]
struct LoginResponse {
  iuoc: u128,
  fwau: u128,
  quality_band: String,
  message: String,
}

#[derive(Deserialize)]
struct MoveRequest {
  fwau: u128,
  dx: f32,
  dy: f32,
}

#[derive(Deserialize)]
struct PsiRequest {
  fwau: u128,
}

#[derive(Serialize)]
struct PsiResponse {
  odds: Vec<PsiOdd>,
}

#[derive(Serialize)]
struct PsiOdd {
  label: String,
  p: f32,
}

#[derive(Deserialize)]
struct WsClientMessage {
  kind: String,
  fwau: Option<u128>,
  dx: Option<f32>,
  dy: Option<f32>,
}

#[tokio::main]
async fn main() {
  tracing_subscriber::fmt()
    .with_env_filter("info")
    .init();

  let genesis = *blake3::hash(b"TBC-GENESIS-M1").as_bytes();
  let aum = AumCore::boot(AumConfig {
    genesis_hash: genesis,
    ruleset: Ruleset::pmr_prime(),
  });

  let state = Arc::new(AppState {
    aum: Mutex::new(aum),
  });

  // Spawn demo world
  {
    let mut guard = state.aum.lock().await;
    guard.primary_frame().spawn_demo_world(50);
  }

  let app = Router::new()
    .route("/", get(index))
    .route("/api/status", get(status))
    .route("/api/login", get(login))
    .route("/api/move", axum::routing::post(move_player))
    .route("/api/psi", axum::routing::post(psi_query))
    .route("/ws", get(ws_handler))
    .nest_service("/static", ServeDir::new("web"))
    .layer(CorsLayer::permissive())
    .with_state(state.clone());

  let sim_state = state.clone();
  tokio::spawn(async move {
    sim_loop(sim_state).await;
  });

  let addr = SocketAddr::from(([0, 0, 0, 0], 6014));
  info!("TBC server listening on http://{}", addr);
  let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
  axum::serve(listener, app).await.unwrap();
}

async fn index() -> Html<String> {
  let path = format!("{}/../web/index.html", env!("CARGO_MANIFEST_DIR"));
  Html(std::fs::read_to_string(path).unwrap_or_else(|_| "<h1>TBC</h1>".into()))
}

async fn status(State(state): State<Arc<AppState>>) -> Json<StatusResponse> {
  let guard = state.aum.lock().await;
  let frame = guard.frames[0].build_snapshot();
  Json(StatusResponse {
    tick: frame.tick,
    dt_ms: guard.frames[0].spec.ruleset.dt_ms,
    entities: frame.entities.len(),
    fwau_count: frame.fwau_count,
    ruleset: guard.frames[0].spec.ruleset.id.clone(),
  })
}

async fn login(State(state): State<Arc<AppState>>) -> Json<LoginResponse> {
  let mut guard = state.aum.lock().await;
  let iuoc = guard.iuoc.create_soul();
  let angle = (guard.frames[0].clock.tick.0 as f32 * 0.1).sin();
  let pos = Vec3::new(angle * 20.0, angle * 15.0, 0.0);

  let fwau = guard.bind_player(iuoc, pos).unwrap();

  let band = guard
    .iuoc
    .get(iuoc)
    .map(|s| s.quality.band().label().to_string())
    .unwrap_or_else(|| "Settled".to_string());

  Json(LoginResponse {
    iuoc: iuoc.0,
    fwau: fwau.0,
    quality_band: band,
    message: "IUOC partitioned. FWAU bound. Something settled.".to_string(),
  })
}

async fn move_player(
  State(state): State<Arc<AppState>>,
  Json(req): Json<MoveRequest>,
) -> Json<FrameSnapshot> {
  let mut guard = state.aum.lock().await;
  let frame = guard.primary_frame();
  let fwau = FwauId(req.fwau);
  let tick = frame.now();
  let mut payload = Vec::with_capacity(8);
  payload.extend_from_slice(&req.dx.to_le_bytes());
  payload.extend_from_slice(&req.dy.to_le_bytes());

  let intent = Intent {
    fwau,
    tick,
    seq: tick.0 as u32,
    verb: Verb::Move,
    payload,
    consent: None,
    checksum: 0,
  };
  frame.submit_intent(intent);
  Json(frame.build_snapshot())
}

async fn psi_query(
  State(state): State<Arc<AppState>>,
  Json(req): Json<PsiRequest>,
) -> Json<PsiResponse> {
  let guard = state.aum.lock().await;
  let fwau = FwauId(req.fwau);
  let odds = guard
    .frames
    .first()
    .map(|f| f.psi_future_self(fwau))
    .unwrap_or_default()
    .into_iter()
    .map(|(label, p)| PsiOdd { label, p })
    .collect();
  Json(PsiResponse { odds })
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
  ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
  let (mut sender, mut receiver) = socket.split();
  let send_state = state.clone();

  let send_task = tokio::spawn(async move {
    loop {
      let snap = {
        let guard = send_state.aum.lock().await;
        guard.frames.first().map(|f| f.build_snapshot())
      };
      if let Some(snap) = snap {
        let json = serde_json::to_string(&snap).unwrap_or_default();
        if sender.send(Message::Text(json)).await.is_err() {
          break;
        }
      }
      tokio::time::sleep(Duration::from_millis(50)).await;
    }
  });

  let recv_state = state.clone();
  let recv_task = tokio::spawn(async move {
    while let Some(msg) = receiver.next().await {
      if let Ok(Message::Text(text)) = msg {
        if let Ok(client_msg) = serde_json::from_str::<WsClientMessage>(&text) {
          if client_msg.kind == "move" {
            let mut guard = recv_state.aum.lock().await;
            let frame = guard.primary_frame();
            if let (Some(fwau), Some(dx), Some(dy)) =
              (client_msg.fwau, client_msg.dx, client_msg.dy)
            {
              let tick = frame.now();
              let mut payload = Vec::with_capacity(8);
              payload.extend_from_slice(&dx.to_le_bytes());
              payload.extend_from_slice(&dy.to_le_bytes());
              frame.submit_intent(Intent {
                fwau: FwauId(fwau),
                tick,
                seq: tick.0 as u32,
                verb: Verb::Move,
                payload,
                consent: None,
                checksum: 0,
              });
            }
          }
        }
      }
    }
  });

  tokio::select! {
    _ = send_task => {},
    _ = recv_task => {},
  }
}

async fn sim_loop(state: Arc<AppState>) {
  let dt = Duration::from_millis(50);
  loop {
    let mut guard = state.aum.lock().await;
    let steps = guard.frames[0].drain_elapsed(dt);
    if steps > 0 {
      guard.run_frame_ticks(steps);
    }
    drop(guard);
    tokio::time::sleep(dt).await;
  }
}
