use crate::RequestContext;
use crate::handlers::response_builder::build_error_response;

use pm_proto::WebSocketMessage;

use log::error;

/// Execute a handler with panic recovery
pub async fn with_error_boundary<F, Fut>(
    ctx: &RequestContext,
    handler_name: &str,
    handler: F,
) -> WebSocketMessage
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = WebSocketMessage> + Send + 'static,
{
    // Spawn the handler in a separate task for panic isolation
    let correlation_id = ctx.correlation_id.clone();
    let handler_name_owned = handler_name.to_string();
    let log_prefix = ctx.log_prefix();

    let result = tokio::spawn(async move { handler().await }).await;

    match result {
        Ok(response) => response,
        Err(join_error) => {
            // Task panicked or was cancelled
            let panic_msg = if join_error.is_panic() {
                match join_error.into_panic().downcast::<String>() {
                    Ok(msg) => *msg,
                    Err(any) => match any.downcast::<&str>() {
                        Ok(msg) => msg.to_string(),
                        Err(_) => "Unknown panic".to_string(),
                    },
                }
            } else {
                "Task cancelled".to_string()
            };

            error!(
                "{} Handler {} panicked: {}",
                log_prefix, handler_name_owned, panic_msg
            );

            build_error_response(
                &correlation_id,
                pm_proto::Error {
                    code: "INTERNAL_ERROR".to_string(),
                    message: "An unexpected error occurred. Please try again.".to_string(),
                    field: None,
                },
            )
        }
    }
}

/// Wrapper that ensures we never leak internal error details to clients
pub fn sanitize_error_message(internal_error: &str) -> String {
    // Never expose internal details like file paths, SQL, stack traces
    if internal_error.contains("SQLITE")
        || internal_error.contains("sqlx")
        || internal_error.contains("panicked")
        || internal_error.contains("/Users")
        || internal_error.contains("\\Users")
        || internal_error.contains("/home")
    {
        "An internal error occurred. Please try again later.".to_string()
    } else {
        // Truncate to reasonable length
        internal_error.chars().take(200).collect()
    }
}
