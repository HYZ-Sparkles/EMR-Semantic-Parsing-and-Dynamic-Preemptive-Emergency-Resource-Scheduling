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

    // tick loop (仿真时间步) —— 必须和 HTTP 入口共享同一把 tx，
    // 否则 auto 跑起来后前端 WS 永远收不到新快照
    spawn_tick_loop(sched.clone(), tx.clone());

    let app = api::build_router(sched, tx);
    let listener = TcpListener::bind("0.0.0.0:7878").await.expect("bind");
    tracing::info!("EMR Scheduler listening on http://0.0.0.0:7878");
    serve(listener, app).await.unwrap();
}

fn spawn_tick_loop(sched: Shared, tx: tokio::sync::broadcast::Sender<api::SnapshotMsg>) {
    tokio::spawn(async move {
        loop {
            // 200ms 轮询一次 auto_running，避免 step 线程独占 scheduler 锁
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            let (run, interval_ms) = {
                let s = sched.read();
                (s.auto_running, s.tick_ms)
            };
            if !run { continue; }
            tokio::time::sleep(std::time::Duration::from_millis(interval_ms.saturating_sub(200))).await;
            // 通过 api.rs 提供的工具函数 step + broadcast，
            // 走与 tick_handler 完全相同的路径，杜绝“step 了但忘了 broadcast”
            api::step_and_broadcast(sched.clone(), tx.clone());
        }
    });
}
