use log::info;
use pingora::prelude::*;
use std::time::Instant;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

pub struct HelloProxy;

pub struct Ctx {
    start: Instant,
}

#[async_trait::async_trait]
impl ProxyHttp for HelloProxy {
    type CTX = Ctx;

    fn new_ctx(&self) -> Self::CTX {
        Ctx {
            start: Instant::now(),
        }
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let peer = Box::new(HttpPeer::new("127.0.0.1:8080", false, String::new()));
        Ok(peer)
    }

    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        _ctx: &mut Self::CTX,
    ) -> Result<()>
    where
        Self::CTX: Send + Sync,
    {
        upstream_request.insert_header("Host", "127.0.0.1:8080")?;
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
}

fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let mut server = Server::new(None)?;

    server.bootstrap();

    let proxy = HelloProxy;

    let mut proxy_service = http_proxy_service(&server.configuration, proxy);

    proxy_service.add_tcp("0.0.0.0:6188");

    server.add_service(proxy_service);

    server.run_forever();
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(1 + 1, 2);
    }
}
