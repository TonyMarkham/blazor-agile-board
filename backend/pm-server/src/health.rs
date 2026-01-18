use pm_ws::AppState;

use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<DatabaseHealth>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub circuit_breaker: Option<CircuitBreakerHealth>,
}

#[derive(Serialize)]
pub struct DatabaseHealth {
    pub status: &'static str,
    pub latency_ms: u64,
}

#[derive(Serialize)]
pub struct CircuitBreakerHealth {
    pub state: String,
}

/// Liveness probe - is the process running?
/// Used by Kubernetes/container orchestrators
pub async fn liveness() -> StatusCode {
    StatusCode::OK
}

/// Readiness probe - can we serve requests?
/// Checks database connectivity and circuit breaker state
pub async fn readiness(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let start = std::time::Instant::now();

    // Check database connection
    let db_result = sqlx::query("SELECT 1").execute(&state.pool).await;

    let db_health = match db_result {
        Ok(_) => DatabaseHealth {
            status: "healthy",
            latency_ms: start.elapsed().as_millis() as u64,
        },
        Err(_) => DatabaseHealth {
            status: "unhealthy",
            latency_ms: start.elapsed().as_millis() as u64,
        },
    };

    // Check circuit breaker state
    let cb_state = format!("{:?}", state.circuit_breaker.state());

    let overall_status = if db_health.status == "healthy" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let response = HealthResponse {
        status: if overall_status == StatusCode::OK {
            "healthy"
        } else {
            "unhealthy"
        },
        version: env!("CARGO_PKG_VERSION"),
        database: Some(db_health),
        circuit_breaker: Some(CircuitBreakerHealth { state: cb_state }),
    };

    (overall_status, Json(response))
}

/// Simple health check for load balancers
/// Returns immediately without checking dependencies
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        database: None,
        circuit_breaker: None,
    })
}
