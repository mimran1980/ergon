//! The TLS configuration the record layer needs is checked up front: a stock
//! rustls config, or a server that only speaks TLS 1.2, fails with an error
//! that says what to change.
#![allow(missing_docs)]

mod support;

use std::sync::Arc;

use ergon_io::{HttpDial, HttpOptions, WsConn, WsDial};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use support::{Lab, R};

/// A client config as rustls builds it by default: TLS 1.2 and 1.3, no
/// secret extraction.
fn stock_config(lab: &Lab) -> R<Arc<ClientConfig>> {
    use rustls::pki_types::pem::PemObject;
    let mut roots = RootCertStore::empty();
    roots.add(CertificateDer::from_pem_slice(lab.cert_pem.as_bytes())?)?;
    Ok(Arc::new(
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()?
            .with_root_certificates(roots)
            .with_no_client_auth(),
    ))
}

#[test]
fn a_config_without_secret_extraction_is_rejected_at_once() -> R {
    let lab = Lab::new()?;
    let cfg = stock_config(&lab)?;
    let ws = WsDial::new(lab.target("/")?, cfg.clone(), 1 << 20).err();
    let http = HttpDial::new(lab.target("")?, cfg, HttpOptions::default()).err();
    for e in [ws, http] {
        let e = e.ok_or("accepted a config without secret extraction")?;
        assert!(e.to_string().contains("enable_secret_extraction"), "{e}");
    }
    Ok(())
}

#[test]
fn a_tls12_handshake_is_refused_clearly() -> R {
    use rustls::pki_types::pem::PemObject;
    let lab = Lab::new()?;
    // The same certificate, served over TLS 1.2 only.
    let ck = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
    let cert = CertificateDer::from(ck.cert.der().to_vec());
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(ck.signing_key.serialize_der()));
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let server = Arc::new(
        ServerConfig::builder_with_provider(provider.clone())
            .with_protocol_versions(&[&rustls::version::TLS12])?
            .with_no_client_auth()
            .with_single_cert(vec![cert.clone()], key)?,
    );
    let mut roots = RootCertStore::empty();
    roots.add(CertificateDer::from_pem_slice(ck.cert.pem().as_bytes())?)?;
    let mut client = ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS12, &rustls::version::TLS13])?
        .with_root_certificates(roots)
        .with_no_client_auth();
    client.enable_secret_extraction = true;
    let client = Arc::new(client);
    std::thread::scope(|s| -> R {
        let srv = s.spawn(|| -> R {
            let (mut sock, _) = lab.listener.accept()?;
            let mut conn = rustls::ServerConnection::new(server.clone())?;
            // Complete the server side of the handshake; the client refuses
            // afterwards and closes.
            while conn.is_handshaking() {
                if conn.wants_write() {
                    conn.write_tls(&mut sock)?;
                    continue;
                }
                if conn.read_tls(&mut sock)? == 0 {
                    break;
                }
                conn.process_new_packets()?;
            }
            while conn.wants_write() {
                conn.write_tls(&mut sock)?;
            }
            Ok(())
        });
        let e = WsConn::connect(&lab.target("/")?, client.clone(), 1 << 20)
            .err()
            .ok_or("a TLS 1.2 connection was accepted")?;
        assert!(e.to_string().contains("TLS 1.2"), "{e}");
        srv.join().map_err(|_| "server panicked")??;
        Ok(())
    })
}
