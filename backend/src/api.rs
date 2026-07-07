// HTTP + WebSocket API。
//   POST /api/parse            { text } -> { matches, score, desired_kind }
//   POST /api/admit            { name, complaint } -> { patient, events }
//   GET  /api/snapshot         -> { tick, queue, patients, resources, events }
//   POST /api/tick             -> { events } (手动 step)
//   POST /api/auto             { running: bool, tick_ms?: number } -> ok
//   POST /api/reset            -> ok
//   GET  /api/terms            -> 词表
//   WS   /ws                   -> 推送完整 snapshot (广播通道)

use axum::{
    extract::{ws::WebSocket, ws::Message, State, WebSocketUpgrade},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;

use crate::scheduler::{Scheduler, Shared, ParseResult, SimEvent, Patient, PatientState};
use crate::term::ResourceKind;
use crate::trie::Match;
use crate::resources::ResourceSlot;

#[derive(Clone)]
struct AppState {
    sched: Shared,
    tx: broadcast::Sender<SnapshotMsg>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SnapshotMsg {
    pub tick: u64,
    pub queue: Vec<Patient>,
    pub patients: Vec<Patient>,
    pub resources: Vec<ResourceSlot>,
    pub events: Vec<SimEvent>,
    pub auto_running: bool,
    pub tick_ms: u64,
}

pub fn build_router(sched: Shared, tx: broadcast::Sender<SnapshotMsg>) -> Router {
    let state = AppState { sched, tx };
    Router::new()
        .route("/api/parse",    post(parse_handler))
        .route("/api/admit",    post(admit_handler))
        .route("/api/snapshot", get(snapshot_handler))
        .route("/api/tick",     post(tick_handler))
        .route("/api/auto",     post(auto_handler))
        .route("/api/reset",    post(reset_handler))
        .route("/api/terms",    get(terms_handler))
        .route("/ws",           get(ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

fn make_snapshot(sched: &Scheduler) -> SnapshotMsg {
    let queue: Vec<_> = sched.heap.snapshot().into_iter()
        .filter_map(|e| sched.patients.get(&e.id).cloned())
        .filter(|p| p.state == PatientState::Queued)
        .collect();
    let patients: Vec<_> = sched.patients.values().cloned().collect();
    let resources = sched.resources.snapshot();
    let mut events = sched.events.clone();
    let len = events.len();
    if len > 30 { events = events.split_off(len - 30); }
    SnapshotMsg {
        tick: sched.tick,
        queue,
        patients,
        resources,
        events,
        auto_running: sched.auto_running,
        tick_ms: sched.tick_ms,
    }
}

fn broadcast_snapshot(state: &AppState, sched: &Scheduler) {
    let snap = make_snapshot(sched);
    let _ = state.tx.send(snap);
}

#[derive(Deserialize)]
struct ParseReq { text: String }
#[derive(Serialize)]
struct ParseResp { matches: Vec<Match>, score: u32, desired_kind: Option<ResourceKind> }

async fn parse_handler(State(s): State<AppState>, Json(req): Json<ParseReq>) -> Json<ParseResp> {
    let sched = s.sched.read();
    let ParseResult { matched, score, desired_kind } = sched.parse(&req.text);
    Json(ParseResp { matches: matched, score, desired_kind })
}

#[derive(Deserialize)]
struct AdmitReq { name: String, complaint: String }
#[derive(Serialize)]
struct AdmitResp { patient: Option<Patient>, event: Option<SimEvent> }
async fn admit_handler(State(s): State<AppState>, Json(req): Json<AdmitReq>) -> Json<AdmitResp> {
    let mut sched = s.sched.write();
    let (patient, event) = match sched.admit(&req.name, &req.complaint) {
        Some((p, e)) => (Some(p), Some(e)),
        None => (None, None),
    };
    broadcast_snapshot(&s, &sched);
    Json(AdmitResp { patient, event })
}

async fn snapshot_handler(State(s): State<AppState>) -> Json<SnapshotMsg> {
    let sched = s.sched.read();
    Json(make_snapshot(&sched))
}

async fn tick_handler(State(s): State<AppState>) -> Json<serde_json::Value> {
    let events = {
        let mut sched = s.sched.write();
        sched.step()
    };
    broadcast_snapshot(&s, &s.sched.read());
    Json(serde_json::json!({ "ok": true, "events": events }))
}

#[derive(Deserialize)]
struct AutoReq { running: bool, #[serde(default)] tick_ms: Option<u64> }
async fn auto_handler(State(s): State<AppState>, Json(req): Json<AutoReq>) -> Json<serde_json::Value> {
    {
        let mut sched = s.sched.write();
        sched.auto_running = req.running;
        if let Some(ms) = req.tick_ms { sched.tick_ms = ms.clamp(200, 10_000); }
    }
    broadcast_snapshot(&s, &s.sched.read());
    Json(serde_json::json!({ "ok": true }))
}

async fn reset_handler(State(s): State<AppState>) -> Json<serde_json::Value> {
    {
        let mut sched = s.sched.write();
        *sched = Scheduler::new();
    }
    broadcast_snapshot(&s, &s.sched.read());
    Json(serde_json::json!({ "ok": true }))
}

async fn terms_handler() -> Json<serde_json::Value> {
    let list: Vec<_> = crate::term::TERMS.iter().map(|t| serde_json::json!({
        "word": t.word, "weight": t.weight, "kind": t.kind,
    })).collect();
    Json(serde_json::json!({ "terms": list }))
}

async fn ws_handler(State(s): State<AppState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| ws_loop(socket, s))
}

async fn ws_loop(mut socket: WebSocket, state: AppState) {
    let mut rx = state.tx.subscribe();
    // 立即推一次 (读取快照后立刻释放锁)
    let initial = {
        let sched = state.sched.read();
        make_snapshot(&sched)
    };
    if let Ok(s) = serde_json::to_string(&initial) {
        let _ = socket.send(Message::Text(s)).await;
    }
    loop {
        tokio::select! {
            Ok(msg) = rx.recv() => {
                if let Ok(s) = serde_json::to_string(&msg) {
                    if socket.send(Message::Text(s)).await.is_err() { break; }
                }
            }
            Some(Ok(_)) = socket.recv() => { /* ignore client msgs */ }
            else => break,
        }
    }
}
