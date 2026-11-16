use log::info;
use pingora::prelude::*;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

pub struct HelloProxy {
    upstreams: Arc<LoadBalancer<RoundRobin>>,
}

pub struct Ctx {
    start: Instant,
    upstream: String,
}

fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let mut server = Server::new(None)?;
    server.bootstrap();

    let mut upstreams =
        LoadBalancer::<RoundRobin>::try_from_iter(["127.0.0.1:9000", "127.0.0.1:9001"])
            .map_err(|e| Error::because(ErrorType::InternalError, "invalid upstream address", e))?;
    upstreams.set_health_check(TcpHealthCheck::new());
    upstreams.health_check_frequency = Some(Duration::from_secs(1));

    let background = background_service("health check", upstreams);
    let upstreams = background.task();

    let proxy = HelloProxy { upstreams };

    let mut proxy_service = http_proxy_service(&server.configuration, proxy);

    proxy_service.add_tcp("0.0.0.0:6188");

    server.add_service(background);
    server.add_service(proxy_service);

    server.run_forever();
}

#[async_trait::async_trait]
impl ProxyHttp for HelloProxy {
    type CTX = Ctx;

    fn new_ctx(&self) -> Self::CTX {
        Ctx {
            start: Instant::now(),
            upstream: String::new(),
        }
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let backend = self
            .upstreams
            .select(b"", 256)
            .ok_or_else(|| Error::new_str("no healthy upstream"))?;

        ctx.upstream = backend.addr.to_string();

        Ok(Box::new(HttpPeer::new(backend, false, String::new())))
    }

    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()>
    where
        Self::CTX: Send + Sync,
    {
        upstream_request.insert_header("Host", ctx.upstream.as_str())?;
        Ok(())
    }

    async fn response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut ResponseHeader,
        _ctx: &mut Self::CTX,
    ) -> Result<()>
    where
        Self::CTX: Send + Sync,
    {
        upstream_response.insert_header("X-Gateway", "pingora-lab")?;
        Ok(())
    }

    async fn logging(&self, session: &mut Session, e: Option<&Error>, ctx: &mut Self::CTX)
    where
        Self::CTX: Send + Sync,
    {
        let status = session
            .response_written()
            .map_or(0, |resp| resp.status.as_u16());
        info!(
            "{} {} status={} error={} {}ms",
            session.req_header().method,
            session.req_header().uri.path(),
            status,
            e.is_some(),
            ctx.start.elapsed().as_millis()
        );
    }

    async fn request_filter(&self, session: &mut Session, _ctx: &mut Self::CTX) -> Result<bool>
    where
        Self::CTX: Send + Sync,
    {
        let path = session.req_header().uri.path().to_string();

        if path.starts_with("/admin") {
            session.respond_error(403).await?;
            return Ok(true);
        }

        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(1 + 1, 2);
    }
}
