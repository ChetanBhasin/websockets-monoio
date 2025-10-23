#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    Ws,
    Wss,
}

#[derive(Debug, Clone)]
pub struct WsUrl<'a> {
    pub scheme: Scheme,
    pub host: &'a str,
    pub port: u16,
    pub path_and_query: &'a str,
}

#[derive(thiserror::Error, Debug)]
pub enum UrlError {
    #[error("URL must start with ws:// or wss://")]
    Scheme,
    #[error("invalid port")]
    Port,
}

pub fn parse_ws_or_wss(input: &str) -> Result<WsUrl<'_>, UrlError> {
    let (scheme, rest) = if let Some(s) = input.strip_prefix("wss://") {
        (Scheme::Wss, s)
    } else if let Some(s) = input.strip_prefix("ws://") {
        (Scheme::Ws, s)
    } else {
        return Err(UrlError::Scheme);
    };

    let (host_port, path_and_query) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };

    let default_port = match scheme {
        Scheme::Ws => 80,
        Scheme::Wss => 443,
    };
    let (host, port) = match host_port.rsplit_once(':') {
        Some((h, p)) => (h, p.parse().map_err(|_| UrlError::Port)?),
        None => (host_port, default_port),
    };

    Ok(WsUrl {
        scheme,
        host,
        port,
        path_and_query,
    })
}
