/// Current state of the server process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerState {
    /// Server is not running
    Stopped,
    /// Server is starting up
    Starting,
    /// Server is running and healthy
    Running { port: u16 },
    /// Server is restarting after crash
    Restarting { attempt: u32 },
    /// Server is shutting down gracefully
    ShuttingDown,
    /// Server has failed and won't restart
    Failed { error: String },
}
