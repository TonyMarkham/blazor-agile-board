/// The resolved .pm/ directory path, shared via Tauri managed state.
#[derive(Clone, Debug)]
pub struct PmDir(pub std::path::PathBuf);