// PQC (Post-Quantum Cryptography) abstraction logic.
use std::error::Error;

#[derive(Debug, Clone)]
pub struct PqcTlsSettings {
    pub cert_path: String,
    pub key_path: String,
    pub enforce_tls1_3: bool,
    pub kem_algorithm: Option<String>,
}

/// Enforces robust downstream TLS configuration. 
/// If `pqc-kem` is enabled, explicitly attempts to append Kyber algorithm curves to the Context.
pub fn get_optimized_tls_settings(cert_path: &str, key_path: &str) -> Result<PqcTlsSettings, Box<dyn Error>> {
    let kem_algorithm = None;
    
    #[cfg(feature = "pqc-kem")]
    {
        tracing::warn!("PQC enabled: Injecting X25519Kyber768 algorithm directives into Context");
        kem_algorithm = Some("X25519Kyber768Draft00:X25519".to_string());
    }
    
    #[cfg(not(feature = "pqc-kem"))]
    {
        tracing::debug!("PQC disabled: Enforcing standard TLS 1.3 algorithm context.");
    }
    
    Ok(PqcTlsSettings {
        cert_path: cert_path.to_string(),
        key_path: key_path.to_string(),
        enforce_tls1_3: true,
        kem_algorithm,
    })
}
