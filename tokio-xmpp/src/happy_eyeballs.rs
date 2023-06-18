use crate::{ConnecterError, Error};
use idna;
use log::debug;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use trust_dns_resolver::{IntoName, TokioAsyncResolver};

pub async fn connect_to_host(domain: &str, port: u16) -> Result<TcpStream, Error> {
    let ascii_domain = idna::domain_to_ascii(&domain).map_err(|_| Error::Idna)?;

    if let Ok(ip) = ascii_domain.parse() {
        return Ok(TcpStream::connect(&SocketAddr::new(ip, port)).await?);
    }

    let resolver = TokioAsyncResolver::tokio_from_system_conf().map_err(ConnecterError::Resolve)?;

    let ips = resolver
        .lookup_ip(ascii_domain)
        .await
        .map_err(ConnecterError::Resolve)?;
    for ip in ips.iter() {
        match TcpStream::connect(&SocketAddr::new(ip, port)).await {
            Ok(stream) => return Ok(stream),
            Err(_) => {}
        }
    }
    Err(Error::Disconnected)
}

pub async fn connect_with_srv(
    domain: &str,
    srv: &str,
    fallback_port: u16,
) -> Result<TcpStream, Error> {
    let ascii_domain = idna::domain_to_ascii(&domain).map_err(|_| Error::Idna)?;

    if let Ok(ip) = ascii_domain.parse() {
        debug!("Attempting connection to {ip}:{fallback_port}");
        return Ok(TcpStream::connect(&SocketAddr::new(ip, fallback_port)).await?);
    }

    let resolver = TokioAsyncResolver::tokio_from_system_conf().map_err(ConnecterError::Resolve)?;

    let srv_domain = format!("{}.{}.", srv, ascii_domain)
        .into_name()
        .map_err(ConnecterError::Dns)?;
    let srv_records = resolver.srv_lookup(srv_domain.clone()).await.ok();

    match srv_records {
        Some(lookup) => {
            // TODO: sort lookup records by priority/weight
            for srv in lookup.iter() {
                debug!("Attempting connection to {srv_domain} {srv}");
                match connect_to_host(&srv.target().to_ascii(), srv.port()).await {
                    Ok(stream) => return Ok(stream),
                    Err(_) => {}
                }
            }
            Err(Error::Disconnected)
        }
        None => {
            // SRV lookup error, retry with hostname
            debug!("Attempting connection to {domain}:{fallback_port}");
            connect_to_host(domain, fallback_port).await
        }
    }
}
