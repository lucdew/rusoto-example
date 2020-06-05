use anyhow::Result;
use hyper::{Client, Uri};
use hyper_proxy::{Intercept, Proxy, ProxyConnector};
use hyper_tls::HttpsConnector;

type HttpConnector =
    hyper_proxy::ProxyConnector<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>;

pub type HttpClient = rusoto_core::HttpClient<HttpConnector>;

pub fn new_client() -> Result<HttpClient> {
    let connector = HttpsConnector::new();

    let http_connector: HttpConnector;
    if let Ok(proxy_url) = std::env::var("http_proxy") {
        let proxy = Proxy::new(Intercept::All, proxy_url.parse::<Uri>()?);
        http_connector = ProxyConnector::from_proxy(connector, proxy)?;
    } else {
        http_connector = ProxyConnector::new(connector)?;
    }
    let mut hyper_builder = Client::builder();

    // disabling due to connection closed issue
    hyper_builder.pool_max_idle_per_host(0);
    /* still getting connection closed before message completed
     * https://github.com/rusoto/rusoto/issues/1766
    hyper_builder
        .pool_idle_timeout(Duration::from_secs(5))
        .pool_max_idle_per_host(4)
        .retry_canceled_requests(true);*/
    Ok(rusoto_core::HttpClient::from_builder(
        hyper_builder,
        http_connector,
    ))
}
