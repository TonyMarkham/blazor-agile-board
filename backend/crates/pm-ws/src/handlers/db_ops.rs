use crate::Result as WsErrorResult;
use crate::handlers::context::HandlerContext;
use crate::retry::with_retry;

use std::future::Future;

/// Execute a database read operation with circuit breaker and retry
pub async fn db_read<F, Fut, T>(
    ctx: &HandlerContext,
    operation_name: &str,
    operation: F,
) -> WsErrorResult<T>
where
    F: Fn() -> Fut + Clone,
    Fut: Future<Output = WsErrorResult<T>>,
{
    // Check circuit breaker first
    ctx.check_circuit()?;

    let result = with_retry(&ctx.retry_config, operation_name, operation).await;

    match &result {
        Ok(_) => ctx.record_db_success(),
        Err(_) => ctx.record_db_failure(),
    }

    result
}

/// Execute a database write operation with circuit breaker (no retry for writes)
pub async fn db_write<F, Fut, T>(
    ctx: &HandlerContext,
    operation_name: &str,
    operation: F,
) -> WsErrorResult<T>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = WsErrorResult<T>>,
{
    // Check circuit breaker first
    ctx.check_circuit()?;

    log::debug!("{} Starting {}", ctx.log_prefix(), operation_name);

    let result = operation().await;

    match &result {
        Ok(_) => {
            ctx.record_db_success();
            log::debug!("{} {} succeeded", ctx.log_prefix(), operation_name);
        }
        Err(e) => {
            ctx.record_db_failure();
            log::warn!("{} {} failed: {}", ctx.log_prefix(), operation_name, e);
        }
    }

    result
}

/// Execute a database transaction with circuit breaker
pub async fn db_transaction<F, Fut, T>(
    ctx: &HandlerContext,
    operation_name: &str,
    operation: F,
) -> WsErrorResult<T>
where
    F: FnOnce(sqlx::Transaction<'_, sqlx::Sqlite>) -> Fut,
    Fut: Future<Output = WsErrorResult<(T, sqlx::Transaction<'static, sqlx::Sqlite>)>>,
{
    // Check circuit breaker first
    ctx.check_circuit()?;

    log::debug!(
        "{} Starting transaction: {}",
        ctx.log_prefix(),
        operation_name
    );

    let tx = ctx.pool.begin().await?;

    match operation(tx).await {
        Ok((result, tx)) => {
            tx.commit().await?;
            ctx.record_db_success();
            log::debug!(
                "{} Transaction {} committed",
                ctx.log_prefix(),
                operation_name
            );
            Ok(result)
        }
        Err(e) => {
            ctx.record_db_failure();
            log::warn!(
                "{} Transaction {} failed: {}",
                ctx.log_prefix(),
                operation_name,
                e
            );
            // Transaction automatically rolled back on drop
            Err(e)
        }
    }
}
