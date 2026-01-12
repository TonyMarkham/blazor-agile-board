use crate::error::{WsError, Result as WsErrorResult};

use pm_core::ErrorLocation;

use std::collections::HashMap;
use std::panic::Location;
use std::sync::Arc;

use tokio::sync::RwLock;

/// Configuration for connection limits
#[derive(Debug, Clone)]
pub struct ConnectionLimits {
    /// Maximum connections per tenant
    pub max_per_tenant: usize,
    /// Maximum total connections across all tenants
    pub max_total: usize,
}

impl Default for ConnectionLimits {
    fn default() -> Self {
        Self {
            max_per_tenant: 1000,
            max_total: 10000,
        }
    }
}