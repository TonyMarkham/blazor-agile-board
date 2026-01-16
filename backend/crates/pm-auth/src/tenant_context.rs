use crate::Claims;

/// Extracted tenant context available to handlers
/// This is the validated, trusted context after JWT verification
#[derive(Debug, Clone)]
pub struct TenantContext {
    pub tenant_id: String,
    pub user_id: String,
    pub roles: Vec<String>,
}

impl TenantContext {
    pub fn from_claims(claims: Claims) -> Self {
        Self {
            tenant_id: claims.tenant_id,
            user_id: claims.sub,
            roles: claims.roles,
        }
    }
}
