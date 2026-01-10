//! MITM Proxy for intercepting Claude API traffic
//!
//! This module provides an HTTP/HTTPS proxy that intercepts traffic to
//! api.anthropic.com and logs it for monitoring.

use crate::traffic::TrafficLog;
use anyhow::Result;
use rcgen::{Certificate, CertificateParams, DistinguishedName, DnType, KeyPair, PKCS_ECDSA_P256_SHA256};
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

/// Event sent from proxy to TUI
#[derive(Debug, Clone)]
pub enum ProxyEvent {
    RequestStarted { id: u64, model: String, stream: bool },
    RequestCompleted { id: u64, tokens_in: u64, tokens_out: u64, latency_ms: u64 },
    RequestFailed { id: u64, error: String },
    StreamChunk { id: u64, text: String },
}

/// MITM Proxy configuration
pub struct ProxyConfig {
    pub listen_addr: SocketAddr,
    pub ca_cert_path: PathBuf,
    pub ca_key_path: PathBuf,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("claudev");

        Self {
            listen_addr: "127.0.0.1:8080".parse().unwrap(),
            ca_cert_path: config_dir.join("ca.crt"),
            ca_key_path: config_dir.join("ca.key"),
        }
    }
}

/// Generated CA certificate data
pub struct CaCert {
    pub cert_pem: String,
    pub key_pem: String,
}

/// MITM Proxy server
pub struct MitmProxy {
    config: ProxyConfig,
    traffic_log: TrafficLog,
    event_tx: mpsc::UnboundedSender<ProxyEvent>,
}

impl MitmProxy {
    pub fn new(
        config: ProxyConfig,
        traffic_log: TrafficLog,
        event_tx: mpsc::UnboundedSender<ProxyEvent>,
    ) -> Self {
        Self {
            config,
            traffic_log,
            event_tx,
        }
    }

    /// Initialize or load CA certificate
    pub fn init_ca(&mut self) -> Result<()> {
        // Create config directory if needed
        if let Some(parent) = self.config.ca_cert_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Check if CA already exists
        if self.config.ca_cert_path.exists() && self.config.ca_key_path.exists() {
            tracing::info!("CA certificate exists at {:?}", self.config.ca_cert_path);
            return Ok(());
        }

        tracing::info!("Generating new CA certificate");
        let ca = generate_ca()?;

        // Save CA cert and key
        fs::write(&self.config.ca_cert_path, &ca.cert_pem)?;
        fs::write(&self.config.ca_key_path, &ca.key_pem)?;

        tracing::info!("CA certificate saved to {:?}", self.config.ca_cert_path);
        tracing::info!(
            "Add this to your trust store or set SSL_CERT_FILE={}",
            self.config.ca_cert_path.display()
        );

        Ok(())
    }

    /// Run the proxy server
    pub async fn run(&self) -> Result<()> {
        let listener = TcpListener::bind(&self.config.listen_addr).await?;
        tracing::info!("MITM Proxy listening on {}", self.config.listen_addr);

        loop {
            let (stream, addr) = listener.accept().await?;
            let traffic_log = self.traffic_log.clone();
            let event_tx = self.event_tx.clone();

            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, addr, traffic_log, event_tx).await {
                    tracing::debug!("Connection error from {}: {}", addr, e);
                }
            });
        }
    }
}

/// Generate a self-signed CA certificate
fn generate_ca() -> Result<CaCert> {
    let mut params = CertificateParams::default();

    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "claudev MITM CA");
    dn.push(DnType::OrganizationName, "claudev");
    params.distinguished_name = dn;

    params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    params.key_usages = vec![
        rcgen::KeyUsagePurpose::KeyCertSign,
        rcgen::KeyUsagePurpose::CrlSign,
        rcgen::KeyUsagePurpose::DigitalSignature,
    ];

    // Generate key pair and set it in params
    let key_pair = KeyPair::generate(&PKCS_ECDSA_P256_SHA256)?;
    params.key_pair = Some(key_pair);

    // Generate self-signed certificate
    let cert = Certificate::from_params(params)?;

    Ok(CaCert {
        cert_pem: cert.serialize_pem()?,
        key_pem: cert.serialize_private_key_pem(),
    })
}

/// Handle a single client connection
async fn handle_connection(
    mut stream: TcpStream,
    _addr: SocketAddr,
    _traffic_log: TrafficLog,
    _event_tx: mpsc::UnboundedSender<ProxyEvent>,
) -> Result<()> {
    let mut buf = vec![0u8; 8192];
    let n = stream.read(&mut buf).await?;
    if n == 0 {
        return Ok(());
    }

    let request_line = String::from_utf8_lossy(&buf[..n]);

    // Check if this is a CONNECT request (HTTPS proxy)
    if request_line.starts_with("CONNECT ") {
        handle_connect(stream, &request_line).await
    } else {
        // Regular HTTP proxy - just forward
        handle_http(stream, &buf[..n]).await
    }
}

/// Handle CONNECT method for HTTPS proxying
async fn handle_connect(
    mut client_stream: TcpStream,
    request: &str,
) -> Result<()> {
    // Parse CONNECT host:port
    let parts: Vec<&str> = request.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(anyhow::anyhow!("Invalid CONNECT request"));
    }

    let host_port = parts[1];
    let (host, port) = if let Some(idx) = host_port.rfind(':') {
        (&host_port[..idx], host_port[idx + 1..].parse().unwrap_or(443))
    } else {
        (host_port, 443)
    };

    tracing::debug!("CONNECT to {}:{}", host, port);

    // Send 200 Connection Established
    client_stream
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        .await?;

    // Check if this is an Anthropic endpoint we want to intercept
    if host.contains("anthropic.com") {
        tracing::info!("Intercepting traffic to {}:{}", host, port);
        // For now, just tunnel - full TLS MITM requires more setup
    }

    // Connect to target and tunnel
    let server_addr = format!("{}:{}", host, port);
    let server_stream = TcpStream::connect(&server_addr).await?;
    tunnel_streams(client_stream, server_stream).await
}

/// Pipe two streams together
async fn tunnel_streams(mut client: TcpStream, mut server: TcpStream) -> Result<()> {
    let (mut client_read, mut client_write) = client.split();
    let (mut server_read, mut server_write) = server.split();

    let client_to_server = tokio::io::copy(&mut client_read, &mut server_write);
    let server_to_client = tokio::io::copy(&mut server_read, &mut client_write);

    tokio::select! {
        result = client_to_server => {
            result?;
        }
        result = server_to_client => {
            result?;
        }
    }

    Ok(())
}

/// Handle regular HTTP proxy request
async fn handle_http(
    mut client_stream: TcpStream,
    initial_data: &[u8],
) -> Result<()> {
    // Parse the HTTP request
    let request_str = String::from_utf8_lossy(initial_data);
    let lines: Vec<&str> = request_str.lines().collect();

    if lines.is_empty() {
        return Ok(());
    }

    // Parse request line
    let parts: Vec<&str> = lines[0].split_whitespace().collect();
    if parts.len() < 2 {
        return Ok(());
    }

    let _method = parts[0];
    let url = parts[1];

    // Extract host from URL or Host header
    let host = if url.starts_with("http://") {
        url.strip_prefix("http://")
            .and_then(|s| s.split('/').next())
            .unwrap_or("localhost")
    } else {
        lines
            .iter()
            .find(|l| l.to_lowercase().starts_with("host:"))
            .and_then(|l| l.split(':').nth(1))
            .map(|h| h.trim())
            .unwrap_or("localhost")
    };

    tracing::debug!("HTTP request to {}", host);

    // Connect to target and forward
    let port = 80;
    let server_addr = format!("{}:{}", host, port);

    if let Ok(mut server_stream) = TcpStream::connect(&server_addr).await {
        server_stream.write_all(initial_data).await?;
        tunnel_streams(client_stream, server_stream).await?;
    }

    Ok(())
}

/// Get the CA certificate path
pub fn get_ca_cert_path() -> PathBuf {
    ProxyConfig::default().ca_cert_path
}

/// Print setup instructions
pub fn print_setup_instructions() {
    let ca_path = get_ca_cert_path();
    println!("\n=== Claude Traffic Monitor Setup ===\n");
    println!("1. Start the monitor in one terminal:");
    println!("   claudev monitor\n");
    println!("2. In another terminal, run Claude with proxy:");
    println!("   export HTTPS_PROXY=http://127.0.0.1:8080");
    println!("   export SSL_CERT_FILE={}", ca_path.display());
    println!("   claude\n");
    println!("Or use the patch command:");
    println!("   claudev patch\n");
    println!("Then just run 'claude' normally.\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_ca() {
        let ca = generate_ca().unwrap();
        assert!(ca.cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(ca.key_pem.contains("BEGIN PRIVATE KEY"));
    }

    #[test]
    fn test_default_config() {
        let config = ProxyConfig::default();
        assert_eq!(config.listen_addr.port(), 8080);
    }
}
