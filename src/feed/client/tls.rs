use crate::error;


/// Gets a TLS connection e.g. for the web socket client
pub fn get_connector() -> Result<tokio_rustls::TlsConnector, error::ClientError> {
    let mut root_store = tokio_rustls::rustls::RootCertStore::empty();

    root_store.add_server_trust_anchors(
        webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
            tokio_rustls::rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
                ta.subject,
                ta.spki,
                ta.name_constraints,
            )
        }),
    );
    let config = tokio_rustls::rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    Ok(tokio_rustls::TlsConnector::from(std::sync::Arc::new(config)))
}