/// Fields that can be loaded from a TOML file for work item create/update.
///
/// All fields are optional so a TOML file only needs to contain the fields
/// relevant to the specific operation. CLI flags always take precedence.
///
/// Unknown fields (e.g. `id`, `created_at`, `version` from a full API response
/// written by `--output-toml`) are silently ignored, enabling a permission-free
/// round-trip workflow: `work-item get --output-toml` → edit → `work-item update
/// --from-toml`.  The `--version` flag must still be provided on the CLI.
///
/// Example TOML file:
///
/// ```toml
/// project_id = "8d96310e-1e69-4dc5-9529-5c173674ab90"
/// type = "task"
/// title = "My Task"
///
/// description = """
/// # Task Description
///
/// Multi-line markdown with `backticks` and code blocks.
/// """
///
/// status = "todo"
/// priority = "high"
/// ```
#[derive(serde::Deserialize, Default, Debug)]
pub struct WorkItemToml {
    pub project_id: Option<String>,

    /// Maps from TOML key "type" (renamed because `type` is a Rust keyword)
    #[serde(rename = "type")]
    pub item_type: Option<String>,

    pub title: Option<String>,
    pub description: Option<String>,
    pub parent_id: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,

    // Update-only fields (silently ignored on create)
    pub assignee_id: Option<String>,
    pub sprint_id: Option<String>,
    pub story_points: Option<i32>,
    pub position: Option<i32>,
}
