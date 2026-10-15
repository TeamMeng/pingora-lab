use pingora::prelude::*;

pub struct HelloProxy;

#[async_trait::async_trait]
impl ProxyHttp for HelloProxy {
    type CTX = ();

    fn new_ctx(&self) -> Self::CTX {}

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let peer = Box::new(HttpPeer::new(
            "httpbin.org:443",
            true,
            "httpbin.org".to_string(),
        ));
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
        upstream_request.insert_header("Host", "httpbin.org")?;
        Ok(())
    }
}

fn main() -> Result<()> {
    env_logger::init();

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
