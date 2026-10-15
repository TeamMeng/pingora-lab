use async_trait::async_trait;
use pingora::{prelude::*, server::ShutdownWatch, services::background::BackgroundService};
use std::time::Duration;
use tokio::time::sleep;

struct HeartbeatTask {
    interval_secs: u64,
}

impl HeartbeatTask {
    pub fn new(interval_secs: u64) -> Self {
        Self { interval_secs }
    }
}

#[async_trait]
impl BackgroundService for HeartbeatTask {
    async fn start(&self, mut shutdown: ShutdownWatch) {
        loop {
            tokio::select! {
                _ = shutdown.changed() => {
                    println!("HeartbeatTask: 收到关闭信号，停止心跳");
                    break;
                }
                _ = sleep(Duration::from_secs(self.interval_secs)) => {
                    println!("HeartbeatTask: 心跳正常，服务运行中...");
                }
            }
        }
    }
}

fn main() -> Result<()> {
    let mut server = Server::new(None)?;

    let heartbeat = background_service("heartbeat_task", HeartbeatTask::new(5));

    server.add_service(heartbeat);

    server.run_forever();
}
