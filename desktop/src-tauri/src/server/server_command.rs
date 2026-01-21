/// Commands from health monitor to lifecycle manager.
///
/// We use a channel because the health monitor runs in a
/// separate task without access to the app handle needed
/// for spawning new processes.
#[derive(Debug)]
pub enum ServerCommand {
    /// Request server restart after crash/unhealthy detection
    Restart { attempt: u32 },
    /// Signal that max restarts exceeded
    MaxRestartsExceeded { count: u32 },
}
