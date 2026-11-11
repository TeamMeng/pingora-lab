use pingora::lb::LoadBalancer;
use pingora::prelude::*;
use std::sync::Arc;
use std::time::Duration;

pub struct LB(Arc<LoadBalancer<RoundRobin>>);

#[async_trait::async_trait]
impl ProxyHttp for LB {
    type CTX = ();

    fn new_ctx(&self) {}

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let upstream = self
            .0
            .select(b"", 256)
            .ok_or_else(|| Error::new_str("no healthy upstream"))?;

        println!("Turn to {:?}", upstream);

        Ok(Box::new(HttpPeer::new(upstream, false, String::new())))
    }
}

fn main() -> Result<()> {
    let mut my_server = Server::new(None)?;
    my_server.bootstrap();

    let mut upstreams = LoadBalancer::try_from_iter(["127.0.0.1:8000", "127.0.0.1:8001"])
        .map_err(|e| Error::because(ErrorType::InternalError, "invalid upstream address", e))?;

    let hc = TcpHealthCheck::new();
    upstreams.set_health_check(hc);
    upstreams.health_check_frequency = Some(Duration::from_secs(1));

    let background = background_service("health check", upstreams);
    let upstreams = background.task();

    let mut lb = http_proxy_service(&my_server.configuration, LB(upstreams));
    lb.add_tcp("0.0.0.0:6188");

    my_server.add_service(background);
    my_server.add_service(lb);
    my_server.run_forever();
}
