use monoio::net::TcpStream;
use monoio_rustls::{ClientTlsStream, TlsConnector};
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, RootCertStore};
use std::sync::{Arc, OnceLock};

#[derive(thiserror::Error, Debug)]
pub enum TlsErr {
    #[error("dns name")]
    Dns,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Rustls(#[from] monoio_rustls::TlsError),
}

static GLOBAL_CONNECTOR: OnceLock<TlsConnector> = OnceLock::new();

pub fn default_connector() -> &'static TlsConnector {
    GLOBAL_CONNECTOR.get_or_init(|| {
        // Install default crypto provider
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

        let roots = RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        };
        let cfg = ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        TlsConnector::from(Arc::new(cfg))
    })
}

pub async fn connect_wss(
    host: &str,
    port: u16,
    connector: &TlsConnector,
) -> Result<ClientTlsStream<TcpStream>, TlsErr> {
    let tcp = TcpStream::connect((host, port)).await?;
    let dns = ServerName::try_from(host.to_owned()).map_err(|_| TlsErr::Dns)?;
    let tls = connector.connect(dns, tcp).await?;
    Ok(tls)
}
