use std::collections::HashSet;

/// Tracks what resources a client has subscribed to
#[derive(Debug, Clone, Default)]
pub struct ClientSubscriptions {
    /// Project IDs the client is interested in
    projects: HashSet<String>,
    /// Sprint IDs the client is interested in
    sprints: HashSet<String>,
    /// Work item IDs the client is interested in (optional granular subscriptions)
    work_items: HashSet<String>,
}

impl ClientSubscriptions {
    pub fn new() -> Self {
        Self::default()
    }

    /// Subscribe to a project (and implicitly all its contents)
    pub fn subscribe_project(&mut self, project_id: String) {
        self.projects.insert(project_id);
    }

    /// Unsubscribe from a project
    pub fn unsubscribe_project(&mut self, project_id: &str) {
        self.projects.remove(project_id);
    }

    /// Subscribe to a specific sprint
    pub fn subscribe_sprint(&mut self, sprint_id: String) {
        self.sprints.insert(sprint_id);
    }

    /// Unsubscribe from a sprint
    pub fn unsubscribe_sprint(&mut self, sprint_id: &str) {
        self.sprints.remove(sprint_id);
    }

    /// Subscribe to a specific work item
    pub fn subscribe_work_item(&mut self, work_item_id: String) {
        self.work_items.insert(work_item_id);
    }

    /// Unsubscribe from a work item
    pub fn unsubscribe_work_item(&mut self, work_item_id: &str) {
        self.work_items.remove(work_item_id);
    }

    /// Check if client is interested in a project
    pub fn is_subscribed_to_project(&self, project_id: &str) -> bool {
        self.projects.contains(project_id)
    }

    /// Check if client is interested in a sprint
    pub fn is_subscribed_to_sprint(&self, sprint_id: &str) -> bool {
        self.sprints.contains(sprint_id)
    }

    /// Check if client is interested in a work item
    pub fn is_subscribed_to_work_item(&self, work_item_id: &str) -> bool {
        self.work_items.contains(work_item_id)
    }

    /// Get all subscribed project IDs
    pub fn get_projects(&self) -> Vec<String> {
        self.projects.iter().cloned().collect()
    }

    /// Get all subscribed sprint IDs
    pub fn get_sprints(&self) -> Vec<String> {
        self.sprints.iter().cloned().collect()
    }

    /// Get all subscribed work item IDs
    pub fn get_work_items(&self) -> Vec<String> {
        self.work_items.iter().cloned().collect()
    }

    /// Clear all subscriptions
    pub fn clear(&mut self) {
        self.projects.clear();
        self.sprints.clear();
        self.work_items.clear();
    }

    /// Get total subscription count (for metrics/debugging)
    pub fn total_count(&self) -> usize {
        self.projects.len() + self.sprints.len() + self.work_items.len()
    }

    /// Check if client has any subscriptions
    pub fn is_empty(&self) -> bool {
        self.projects.is_empty() && self.sprints.is_empty() && self.work_items.is_empty()
    }
}
