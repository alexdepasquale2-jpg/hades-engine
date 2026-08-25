use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tbc_engine::aum::AumCore;
use tbc_engine::frame::FrameSnapshot;
use tbc_engine::intent::{Intent, Verb};
use tbc_engine::persist::SoulArchive;
use tbc_engine::planner::ReincarnationOffer;
use tbc_engine::psi::PsiResponse as EnginePsiResponse;
use tbc_engine::shard::ShardNodeConfig;
use tbc_engine::social::SpeakResult;
use tbc_engine::types::{FwauId, IuocId, Tick, Vec3};
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing::info;

#[derive(Clone)]
struct SessionInfo {
    iuoc: IuocId,
    fwau: FwauId,
    frame_idx: usize,
}

struct AppState {
    aum: Mutex<AumCore>,
    sessions: Mutex<HashMap<FwauId, SessionInfo>>,
}

#[derive(Serialize)]
struct StatusResponse {
    tick: u64,
    dt_ms: u16,
    entities: usize,
    fwau_count: usize,
    ruleset: String,
    frames: Vec<FrameInfo>,
    island_profiler: Option<tbc_engine::islands::IslandProfiler>,
    guardrails: Option<tbc_engine::guardrails::GuardrailReport>,
    archive: Option<tbc_engine::persist::ArchiveStats>,
    rww: tbc_engine::rww::RwwStatus,
    node: tbc_engine::shard::ShardNodeStatus,
}

#[derive(Serialize)]
struct FrameInfo {
    id: u32,
    name: String,
    ruleset: String,
    dt_ms: u16,
}

#[derive(Serialize)]
struct LoginResponse {
    iuoc: u128,
    fwau: u128,
    quality_band: String,
    frame: String,
    message: String,
}

#[derive(Deserialize)]
struct MoveRequest {
    #[serde(with = "tbc_engine::wire_json::compat")]
    fwau: u128,
    dx: f32,
    dy: f32,
    tick: Option<u64>,
}

#[derive(Deserialize)]
struct BlinkRequest {
    #[serde(with = "tbc_engine::wire_json::compat")]
    fwau: u128,
    x: f32,
    y: f32,
}

#[derive(Deserialize)]
struct HandoffRequest {
    #[serde(with = "tbc_engine::wire_json::compat")]
    fwau: u128,
    to_frame: String,
}

#[derive(Deserialize)]
struct PsiRequest {
    #[serde(with = "tbc_engine::wire_json::compat")]
    fwau: u128,
    scope: Option<String>,
}

#[derive(Deserialize)]
struct SpeakRequest {
    #[serde(with = "tbc_engine::wire_json::compat")]
    fwau: u128,
    text: String,
}

#[derive(Deserialize)]
struct ConsentRequest {
    #[serde(with = "tbc_engine::wire_json::compat")]
    fwau: u128,
    #[serde(with = "tbc_engine::wire_json::compat")]
    helper_iuoc: u128,
    scope: String,
    ttl_ticks: Option<u64>,
}

#[derive(Serialize)]
struct ConsentResponse {
    pact: String,
    granted: bool,
    has_consent: bool,
}

#[derive(Deserialize)]
struct WsClientMessage {
    kind: String,
    fwau: Option<u128>,
    dx: Option<f32>,
    dy: Option<f32>,
    tick: Option<u64>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let genesis = *blake3::hash(b"TBC-GENESIS-M2").as_bytes();
    let archive_path =
        std::env::var("TBC_ARCHIVE_PATH").unwrap_or_else(|_| "data/tbc-archive.db".into());
    let archive = SoulArchive::open(&archive_path).expect("open soul archive");
    let node = ShardNodeConfig::from_env();
    info!(
        "Soul archive at {} ({} souls)",
        archive.path(),
        archive.stats().map(|s| s.souls).unwrap_or(0)
    );
    let aum = AumCore::boot_node_with_archive(genesis, archive, node.clone()).expect("boot node");
    info!(
        "Node mode={} shard={:?} RWW backend={} connected={}",
        aum.node.label(),
        aum.node.shard_id,
        aum.rww.status().backend,
        aum.rww.status().connected
    );

    let state = Arc::new(AppState {
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
            } else if i == 2 {
                8
            } else {
                12
            };
            frame.spawn_demo_world(count);
        }
    }

    let web_root = web_root();
    let app = Router::new()
        .route("/", get(index))
        .route("/api/status", get(status))
        .route("/api/login", get(login))
        .route("/api/resume", get(resume))
        .route("/api/move", axum::routing::post(move_player))
        .route("/api/blink", axum::routing::post(blink))
        .route("/api/handoff", axum::routing::post(handoff))
        .route("/api/psi", axum::routing::post(psi_query))
        .route("/api/speak", axum::routing::post(speak))
        .route("/api/consent", axum::routing::post(consent_grant))
        .route("/api/offers", get(offers))
        .route("/api/unbind", axum::routing::post(unbind))
        .route("/api/reincarnate", axum::routing::post(reincarnate))
        .route("/api/attack", axum::routing::post(attack))
        .route("/api/interact", axum::routing::post(interact))
        .route("/api/assist", axum::routing::post(assist))
        .route("/api/ruleset", get(ruleset_info))
        .route("/api/rww", get(rww_recent))
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/metrics", get(metrics))
        .route("/ws", get(ws_handler))
        .nest_service(
            "/static",
            ServeDir::new(web_root.to_string_lossy().to_string()),
        )
        .layer(CorsLayer::permissive())
        .with_state(state.clone());

    let sim_state = state.clone();
    tokio::spawn(async move {
        sim_loop(sim_state).await;
    });

    let addr = SocketAddr::from(([0, 0, 0, 0], listen_port(&node)));
    info!(
        "TBC server listening on http://{} (mode={})",
        addr,
        node.label()
    );
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn index() -> Html<String> {
    let path = web_root().join("index.html");
    Html(std::fs::read_to_string(path).unwrap_or_else(|_| "<h1>TBC</h1>".into()))
}

fn web_root() -> PathBuf {
    std::env::var("TBC_WEB_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../web"))
}

async fn status(State(state): State<Arc<AppState>>) -> Json<StatusResponse> {
    let mut guard = state.aum.lock().await;
    let frame = guard.frames[0].build_snapshot();
    let frames = guard
        .frames
        .iter()
        .map(|f| FrameInfo {
            id: f.spec.id.0,
            name: f.spec.name.clone(),
            ruleset: f.spec.ruleset.id.clone(),
            dt_ms: f.spec.ruleset.dt_ms,
        })
        .collect();
    Json(StatusResponse {
        tick: frame.tick,
        dt_ms: guard.frames[0].spec.ruleset.dt_ms,
        entities: frame.entities.len(),
        fwau_count: frame.fwau_count,
        ruleset: guard.frames[0].spec.ruleset.id.clone(),
        frames,
        island_profiler: frame.island_profiler.clone(),
        guardrails: frame.guardrails.clone(),
        archive: guard.archive_stats(),
        rww: guard.rww.status(),
        node: guard.node_status(),
    })
}

fn listen_port(node: &ShardNodeConfig) -> u16 {
    if let Ok(port) = std::env::var("TBC_PORT") {
        return port.parse().unwrap_or(6014);
    }
    match node.shard_id {
        Some(0) => 6020,
        Some(1) => 6021,
        _ => 6014,
    }
}

fn spawn_pos_for_node(node: &ShardNodeConfig) -> Vec3 {
    match node.shard_id {
        Some(1) => Vec3::new(80.0, 0.0, 0.0),
        _ => Vec3::new(-80.0, 0.0, 0.0),
    }
}

async fn health(State(state): State<Arc<AppState>>) -> Json<tbc_engine::ops::OpsSnapshot> {
    let sessions = state.sessions.lock().await.len();
    let guard = state.aum.lock().await;
    Json(guard.ops_snapshot(sessions))
}

async fn ready(State(state): State<Arc<AppState>>) -> impl IntoResponse {
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

async fn metrics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let sessions = state.sessions.lock().await.len();
    let guard = state.aum.lock().await;
    let body = guard.ops_snapshot(sessions).prometheus_lines();
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4",
        )],
        body,
    )
}

async fn login(State(state): State<Arc<AppState>>) -> Json<LoginResponse> {
    let mut guard = state.aum.lock().await;
    let iuoc = guard.iuoc.create_soul();
    let pos = spawn_pos_for_node(&guard.node);

    let fwau = guard.bind_player(0, iuoc, pos).unwrap();
    let frame_name = guard.frames[0].spec.ruleset.id.clone();

    state.sessions.lock().await.insert(
        fwau,
        SessionInfo {
            iuoc,
            fwau,
            frame_idx: 0,
        },
    );

    let band = guard
        .iuoc
        .get(iuoc)
        .map(|s| s.quality.band().label().to_string())
        .unwrap_or_else(|| "Settled".to_string());

    Json(LoginResponse {
        iuoc: iuoc.0,
        fwau: fwau.0,
        quality_band: band,
        frame: frame_name,
        message: "IUOC partitioned. FWAU bound. Something settled.".to_string(),
    })
}

#[derive(Deserialize)]
struct AttackRequest {
    #[serde(with = "tbc_engine::wire_json::compat")]
    fwau: u128,
    target_entity: Option<u32>,
}

#[derive(Deserialize)]
struct InteractRequest {
    #[serde(with = "tbc_engine::wire_json::compat")]
    fwau: u128,
}

#[derive(Deserialize)]
struct AssistRequest {
    #[serde(with = "tbc_engine::wire_json::compat")]
    fwau: u128,
    target_entity: Option<u32>,
}

async fn assist(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AssistRequest>,
) -> Json<tbc_engine::assist::AssistResult> {
    let fwau = FwauId(req.fwau);
    let frame_idx = state
        .sessions
        .lock()
        .await
        .get(&fwau)
        .map(|s| s.frame_idx)
        .unwrap_or(0);
    let target = req.target_entity.map(|idx| tbc_engine::types::Entity {
        index: idx,
        generation: 0,
    });
    let mut guard = state.aum.lock().await;
    Json(guard.assist(frame_idx, fwau, target))
}

async fn attack(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AttackRequest>,
) -> Json<tbc_engine::gameplay::AttackResult> {
    let fwau = FwauId(req.fwau);
    let frame_idx = state
        .sessions
        .lock()
        .await
        .get(&fwau)
        .map(|s| s.frame_idx)
        .unwrap_or(0);
    let target = req.target_entity.map(|idx| tbc_engine::types::Entity {
        index: idx,
        generation: 0,
    });
    let mut guard = state.aum.lock().await;
    let result = guard.attack(frame_idx, fwau, target);
    if result.killed {
        let deaths = guard.process_player_deaths(frame_idx);
        if deaths.contains(&fwau) {
            state.sessions.lock().await.remove(&fwau);
        }
    }
    Json(result)
}

async fn interact(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InteractRequest>,
) -> Json<tbc_engine::gameplay::InteractResult> {
    let fwau = FwauId(req.fwau);
    let frame_idx = state
        .sessions
        .lock()
        .await
        .get(&fwau)
        .map(|s| s.frame_idx)
        .unwrap_or(0);
    let mut guard = state.aum.lock().await;
    Json(guard.interact(frame_idx, fwau))
}

async fn ruleset_info(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let guard = state.aum.lock().await;
    let ruleset = &guard.frames[0].spec.ruleset;
    Json(serde_json::json!({
      "id": ruleset.id,
      "title": ruleset.title,
      "verbs": ruleset.verbs,
      "conservation": ruleset.conservation,
      "death": ruleset.death,
    }))
}

#[derive(Deserialize)]
struct ResumeQuery {
    iuoc: u128,
}

async fn resume(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ResumeQuery>,
) -> Json<LoginResponse> {
    let iuoc = IuocId(q.iuoc);
    let mut guard = state.aum.lock().await;
    let pos = spawn_pos_for_node(&guard.node);
    let result = guard.resume_soul(iuoc, 0, pos);
    match result {
        Ok(fwau) => {
            let frame_name = guard.frames[0].spec.ruleset.id.clone();
            state.sessions.lock().await.insert(
                fwau,
                SessionInfo {
                    iuoc,
                    fwau,
                    frame_idx: 0,
                },
            );
            let band = guard
                .iuoc
                .get(iuoc)
                .map(|s| s.quality.band().label().to_string())
                .unwrap_or_else(|| "Settled".to_string());
            let inc = guard.iuoc.get(iuoc).map(|s| s.incarnations).unwrap_or(0);
            Json(LoginResponse {
                iuoc: iuoc.0,
                fwau: fwau.0,
                quality_band: band,
                frame: frame_name,
                message: format!("Soul resumed. Incarnation {} continues.", inc),
            })
        }
        Err(e) => Json(LoginResponse {
            iuoc: iuoc.0,
            fwau: 0,
            quality_band: "—".to_string(),
            frame: "—".to_string(),
            message: e,
        }),
    }
}

fn submit_move(frame: &mut tbc_engine::frame::Frame, fwau: FwauId, dx: f32, dy: f32, tick: Tick) {
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
    if let Err(e) = frame.submit_intent(intent) {
        info!("intent rejected: {:?}", e);
    }
}

async fn move_player(
    State(state): State<Arc<AppState>>,
    Json(req): Json<MoveRequest>,
) -> Json<FrameSnapshot> {
    let mut guard = state.aum.lock().await;
    let fwau = FwauId(req.fwau);
    let frame_idx = state
        .sessions
        .lock()
        .await
        .get(&fwau)
        .map(|s| s.frame_idx)
        .unwrap_or(0);
    let frame = &mut guard.frames[frame_idx];
    let tick = Tick(req.tick.unwrap_or_else(|| frame.now().0));
    submit_move(frame, fwau, req.dx, req.dy, tick);
    Json(frame.build_snapshot())
}

async fn blink(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BlinkRequest>,
) -> Json<FrameSnapshot> {
    let mut guard = state.aum.lock().await;
    let fwau = FwauId(req.fwau);
    let frame_idx = state
        .sessions
        .lock()
        .await
        .get(&fwau)
        .map(|s| s.frame_idx)
        .unwrap_or(2);
    let frame = &mut guard.frames[frame_idx];
    let tick = frame.now();
    let mut payload = Vec::with_capacity(8);
    payload.extend_from_slice(&req.x.to_le_bytes());
    payload.extend_from_slice(&req.y.to_le_bytes());
    let intent = Intent {
        fwau,
        tick,
        seq: tick.0 as u32,
        verb: Verb::Blink,
        payload,
        consent: None,
        checksum: 0,
    };
    frame.submit_intent(intent).ok();
    Json(frame.build_snapshot())
}

async fn handoff(
    State(state): State<Arc<AppState>>,
    Json(req): Json<HandoffRequest>,
) -> Json<LoginResponse> {
    let mut guard = state.aum.lock().await;
    let old_fwau = FwauId(req.fwau);
    let session = state.sessions.lock().await.get(&old_fwau).cloned();
    let from_idx = session.as_ref().map(|s| s.frame_idx).unwrap_or(0);
    let to_idx = guard
        .frames
        .iter()
        .position(|f| f.spec.ruleset.id == req.to_frame)
        .unwrap_or(1);

    let handoff_result = guard.handoff_fwau(old_fwau, from_idx, to_idx);
    if let Err(e) = handoff_result {
        return Json(LoginResponse {
            iuoc: session.as_ref().map(|s| s.iuoc.0).unwrap_or(0),
            fwau: old_fwau.0,
            quality_band: "—".to_string(),
            frame: req.to_frame,
            message: e,
        });
    }

    let iuoc = session.as_ref().map(|s| s.iuoc).unwrap_or(IuocId(0));
    let new_fwau = guard
        .iuoc
        .get(iuoc)
        .and_then(|s| s.bound_fwau)
        .unwrap_or(old_fwau);
    let frame_name = guard.frames[to_idx].spec.ruleset.id.clone();

    state.sessions.lock().await.insert(
        new_fwau,
        SessionInfo {
            iuoc,
            fwau: new_fwau,
            frame_idx: to_idx,
        },
    );
    state.sessions.lock().await.remove(&old_fwau);

    let band = guard
        .iuoc
        .get(iuoc)
        .map(|s| s.quality.band().label().to_string())
        .unwrap_or_else(|| "Settled".to_string());

    Json(LoginResponse {
        iuoc: iuoc.0,
        fwau: new_fwau.0,
        quality_band: band,
        frame: frame_name,
        message: "Handoff complete. Frame boundary crossed.".to_string(),
    })
}

async fn psi_query(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PsiRequest>,
) -> Json<EnginePsiResponse> {
    let mut guard = state.aum.lock().await;
    let fwau = FwauId(req.fwau);
    let session = state.sessions.lock().await.get(&fwau).cloned();
    let frame_idx = session.as_ref().map(|s| s.frame_idx).unwrap_or(0);
    let iuoc = session.as_ref().map(|s| s.iuoc).unwrap_or(IuocId(0));
    let scope = req.scope.as_deref().unwrap_or("FutureSelf");
    Json(guard.query_psi(frame_idx, fwau, iuoc, scope))
}

async fn speak(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SpeakRequest>,
) -> Json<SpeakResult> {
    let mut guard = state.aum.lock().await;
    let fwau = FwauId(req.fwau);
    let frame_idx = state
        .sessions
        .lock()
        .await
        .get(&fwau)
        .map(|s| s.frame_idx)
        .unwrap_or(0);
    Json(guard.speak(frame_idx, fwau, &req.text))
}

async fn consent_grant(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ConsentRequest>,
) -> Json<ConsentResponse> {
    let mut guard = state.aum.lock().await;
    let fwau = FwauId(req.fwau);
    let session = state.sessions.lock().await.get(&fwau).cloned();
    let from = session.as_ref().map(|s| s.iuoc).unwrap_or(IuocId(0));
    let helper = IuocId(req.helper_iuoc);
    let ttl = req.ttl_ticks.unwrap_or(20_000);
    match guard.grant_consent(from, helper, &req.scope, ttl) {
        Ok(pact) => {
            let has = guard.has_consent(from, helper, &req.scope);
            Json(ConsentResponse {
                pact,
                granted: true,
                has_consent: has,
            })
        }
        Err(e) => Json(ConsentResponse {
            pact: e,
            granted: false,
            has_consent: false,
        }),
    }
}

async fn offers(
    State(state): State<Arc<AppState>>,
    Query(q): Query<OffersQuery>,
) -> Json<Vec<ReincarnationOffer>> {
    let guard = state.aum.lock().await;
    let iuoc = if let Some(fw) = q.fwau {
        state
            .sessions
            .lock()
            .await
            .get(&FwauId(fw))
            .map(|s| s.iuoc)
            .unwrap_or(IuocId(q.iuoc.unwrap_or(0)))
    } else {
        IuocId(q.iuoc.unwrap_or(0))
    };
    Json(guard.reincarnation_offers(iuoc))
}

#[derive(Deserialize)]
struct OffersQuery {
    fwau: Option<u128>,
    iuoc: Option<u128>,
}

#[derive(Deserialize)]
struct UnbindRequest {
    #[serde(with = "tbc_engine::wire_json::compat")]
    fwau: u128,
}

async fn unbind(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UnbindRequest>,
) -> Json<serde_json::Value> {
    let fwau = FwauId(req.fwau);
    let frame_idx = state
        .sessions
        .lock()
        .await
        .get(&fwau)
        .map(|s| s.frame_idx)
        .unwrap_or(0);
    let mut guard = state.aum.lock().await;
    let packet = guard.unbind_death(fwau, frame_idx);
    state.sessions.lock().await.remove(&fwau);
    let offers = guard.reincarnation_offers(packet.as_ref().map(|p| p.iuoc).unwrap_or(IuocId(0)));
    Json(serde_json::json!({
      "packet": packet,
      "offers": offers,
      "message": "Something settled. Between-lives offers await."
    }))
}

#[derive(Deserialize)]
struct ReincarnateRequest {
    #[serde(with = "tbc_engine::wire_json::compat")]
    iuoc: u128,
    template_id: String,
}

async fn reincarnate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ReincarnateRequest>,
) -> Json<LoginResponse> {
    let iuoc = IuocId(req.iuoc);
    let mut guard = state.aum.lock().await;
    let result = guard.accept_reincarnation(iuoc, &req.template_id);
    match result {
        Ok((fwau, frame_idx)) => {
            let frame_name = guard.frames[frame_idx].spec.ruleset.id.clone();
            state.sessions.lock().await.insert(
                fwau,
                SessionInfo {
                    iuoc,
                    fwau,
                    frame_idx,
                },
            );
            let band = guard
                .iuoc
                .get(iuoc)
                .map(|s| s.quality.band().label().to_string())
                .unwrap_or_else(|| "Settled".to_string());
            Json(LoginResponse {
                iuoc: iuoc.0,
                fwau: fwau.0,
                quality_band: band,
                frame: frame_name,
                message: format!("Reincarnated into {}.", req.template_id),
            })
        }
        Err(e) => Json(LoginResponse {
            iuoc: iuoc.0,
            fwau: 0,
            quality_band: "—".to_string(),
            frame: "—".to_string(),
            message: e,
        }),
    }
}

#[derive(Deserialize)]
struct RwwQuery {
    subject: Option<String>,
    limit: Option<usize>,
}

async fn rww_recent(
    State(state): State<Arc<AppState>>,
    Query(q): Query<RwwQuery>,
) -> Json<serde_json::Value> {
    let guard = state.aum.lock().await;
    let subject = q.subject.as_deref().unwrap_or("rww.handoff");
    let limit = q.limit.unwrap_or(10);
    let msgs = guard.rww.recent(subject, limit);
    Json(serde_json::json!({
      "status": guard.rww.status(),
      "subject": subject,
      "messages": msgs,
    }))
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let send_state = state.clone();
    let tracked_fwau: Arc<std::sync::Mutex<Option<FwauId>>> = Arc::new(std::sync::Mutex::new(None));

    let tracked_send = tracked_fwau.clone();
    let send_task = tokio::spawn(async move {
        loop {
            let snap = {
                let mut guard = send_state.aum.lock().await;
                let fw_opt = tracked_send.lock().ok().and_then(|g| *g);
                let frame_idx = match fw_opt {
                    Some(fw) => send_state
                        .sessions
                        .lock()
                        .await
                        .get(&fw)
                        .map(|s| s.frame_idx)
                        .unwrap_or(0),
                    None => 0,
                };
                if frame_idx < guard.frames.len() {
                    Some(guard.frames[frame_idx].build_snapshot())
                } else {
                    None
                }
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
    let tracked_recv = tracked_fwau.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            if let Ok(Message::Text(text)) = msg {
                if let Ok(client_msg) = serde_json::from_str::<WsClientMessage>(&text) {
                    if client_msg.kind == "move" {
                        if let Some(fwau) = client_msg.fwau {
                            if let Ok(mut g) = tracked_recv.lock() {
                                *g = Some(FwauId(fwau));
                            }
                        }
                        let mut guard = recv_state.aum.lock().await;
                        if let (Some(fwau), Some(dx), Some(dy)) =
                            (client_msg.fwau, client_msg.dx, client_msg.dy)
                        {
                            let fwau_id = FwauId(fwau);
                            let frame_idx = recv_state
                                .sessions
                                .lock()
                                .await
                                .get(&fwau_id)
                                .map(|s| s.frame_idx)
                                .unwrap_or(0);
                            let tick = Tick(
                                client_msg
                                    .tick
                                    .unwrap_or_else(|| guard.frames[frame_idx].now().0),
                            );
                            submit_move(&mut guard.frames[frame_idx], fwau_id, dx, dy, tick);
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
    loop {
        let mut guard = state.aum.lock().await;
        for i in 0..guard.frames.len() {
            let dt = Duration::from_millis(guard.frames[i].spec.ruleset.dt_ms as u64);
            let steps = guard.frames[i].drain_elapsed(dt);
            if steps > 0 {
                guard.run_frame_ticks(i, steps);
                let deaths = guard.process_player_deaths(i);
                drop(guard);
                if !deaths.is_empty() {
                    let mut sessions = state.sessions.lock().await;
                    for fwau in deaths {
                        sessions.remove(&fwau);
                    }
                }
                guard = state.aum.lock().await;
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
                info!("Inbound shard cross bound fwau={} iuoc={}", fwau.0, iuoc.0);
            }
            for (old_fwau, new_fwau, to_idx) in handoffs {
                if new_fwau.0 == 0 {
                    if let Some(info) = sessions.remove(&old_fwau) {
                        info!(
                            "Outbound shard cross: iuoc={} left shard (connect target shard node)",
                            info.iuoc.0
                        );
                    }
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
}
