// 服务入口：构造调度器、广播通道、HTTP 路由、tick 循环。

mod api;
mod heap;
mod resources;
mod scheduler;
mod term;
mod trie;

use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::broadcast;
use axum::serve;
use tokio::net::TcpListener;

use crate::scheduler::{Scheduler, Shared};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "info,emr_scheduler=debug".into()))
        .init();

    let sched: Shared = Arc::new(RwLock::new(Scheduler::new()));
    let (tx, _rx) = broadcast::channel::<api::SnapshotMsg>(64);

    // tick loop (仿真时间步)
    spawn_tick_loop(sched.clone());

    let app = api::build_router(sched, tx);
    let listener = TcpListener::bind("0.0.0.0:7878").await.expect("bind");
    tracing::info!("EMR Scheduler listening on http://0.0.0.0:7878");
    serve(listener, app).await.unwrap();
}

fn spawn_tick_loop(sched: Shared) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            let (run, interval_ms) = {
                let s = sched.read();
                (s.auto_running, s.tick_ms)
            };
            if !run { continue; }
            tokio::time::sleep(std::time::Duration::from_millis(interval_ms.saturating_sub(200))).await;
            // step 并收集事件
            let events = {
                let mut s = sched.write();
                s.step()
            };
            if !events.is_empty() {
                tracing::debug!(?events, "tick events");
            }
        }
    });
}
