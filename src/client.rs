use ::errors::*;
use hyper_proxy::{Intercept, Proxy, ProxyConnector};
use hyper::Uri;
use hyper_tls::HttpsConnector;

// TODO fix to more generic type
pub type HttpConnector = hyper_proxy::ProxyConnector<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>;

pub fn new_connector() -> Result<HttpConnector> {
    if let Ok(proxy_url) = std::env::var("http_proxy") {
        let mut proxy = Proxy::new(Intercept::All, proxy_url.parse::<Uri>()?);
        let connector = HttpsConnector::new(4)?;
        return Ok(ProxyConnector::from_proxy(connector, proxy)?);
    } else {
        let connector = HttpsConnector::new(4)?;
        return Ok(ProxyConnector::new(connector)?);
    }
}