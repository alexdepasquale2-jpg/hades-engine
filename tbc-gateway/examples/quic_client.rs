//! Smoke test for TBC QUIC gateway — login + reliable move + datagram move.

use quinn::{ClientConfig, Endpoint};
use rustls::pki_types::CertificateDer;
use std::net::SocketAddr;
use std::sync::Arc;
use tbc_engine::transport::{
  decode_reliable, encode_move_datagram, encode_reliable, WireMessage,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let addr: SocketAddr = "127.0.0.1:4433".parse()?;
  let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
  endpoint.set_default_client_config(client_config());

  let connection = endpoint
    .connect(addr, "localhost")?
    .await
    .map_err(|e| format!("connect failed (is tbc-gateway running?): {}", e))?;

  println!("Connected to {}", addr);

  let (mut send, mut recv) = connection.open_bi().await?;

  send.write_all(&encode_reliable(&WireMessage::login())?).await?;
  send.finish()?;

  let chunk = recv.read_to_end(1_000_000).await?;
  let reply = decode_reliable(&chunk)?;
  assert_eq!(reply.kind, "login_ok");
  let fwau = reply.fwau.expect("fwau in login_ok");
  let tick = reply
    .payload
    .get("snapshot")
    .and_then(|s| s.get("tick"))
    .and_then(|t| t.as_u64())
    .unwrap_or(1);
  println!("Login OK fwau={} tick={}", fwau, tick);

  let (mut move_send, mut move_recv) = connection.open_bi().await?;
  move_send
    .write_all(&encode_reliable(&WireMessage::move_intent(fwau, tick + 1, 1.0, 0.0))?)
    .await?;
  move_send.finish()?;

  let move_chunk = move_recv.read_to_end(1_000_000).await?;
  let move_reply = decode_reliable(&move_chunk)?;
  assert_eq!(move_reply.kind, "snapshot");
  println!("Reliable move → snapshot tick={:?}", move_reply.payload.get("tick"));

  let dg = encode_move_datagram(fwau, tick + 2, 0.5, 0.0);
  connection.send_datagram(dg.into())?;
  println!("Sent unreliable move datagram (36 bytes)");

  println!("QUIC transport smoke test OK");
  Ok(())
}

fn client_config() -> ClientConfig {
  let crypto = rustls::ClientConfig::builder()
    .dangerous()
    .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
    .with_no_client_auth();

  ClientConfig::new(Arc::new(
    quinn::crypto::rustls::QuicClientConfig::try_from(crypto).unwrap(),
  ))
}

#[derive(Debug)]
struct SkipServerVerification;

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
  fn verify_server_cert(
    &self,
    _end_entity: &CertificateDer<'_>,
    _intermediates: &[CertificateDer<'_>],
    _server_name: &rustls::pki_types::ServerName<'_>,
    _ocsp_response: &[u8],
    _now: rustls::pki_types::UnixTime,
  ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
    Ok(rustls::client::danger::ServerCertVerified::assertion())
  }

  fn verify_tls12_signature(
    &self,
    _message: &[u8],
    _cert: &CertificateDer<'_>,
    _dss: &rustls::DigitallySignedStruct,
  ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
    Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
  }

  fn verify_tls13_signature(
    &self,
    _message: &[u8],
    _cert: &CertificateDer<'_>,
    _dss: &rustls::DigitallySignedStruct,
  ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
    Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
  }

  fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
    rustls::crypto::ring::default_provider()
      .signature_verification_algorithms
      .supported_schemes()
  }
}
