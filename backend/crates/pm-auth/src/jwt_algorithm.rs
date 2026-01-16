/// Supported JWT algorithms
#[derive(Debug, Clone)]
pub enum JwtAlgorithm {
    /// HMAC with SHA-256 (symmetric key)
    HS256 { secret: Vec<u8> },
    /// RSA with SHA-256 (asymmetric key)
    RS256 { public_key_pem: String },
}
