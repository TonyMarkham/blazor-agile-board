use crate::WorkItemType;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItem {
    pub id: Uuid,
    pub item_type: WorkItemType,

    // Hierarchy
    pub parent_id: Option<Uuid>,
    pub project_id: Uuid,
    pub position: i32,

    // Core fields
    pub title: String,
    pub description: Option<String>,

    // Workflow
    pub status: String,
    pub priority: String,

    // Assignment
    pub assignee_id: Option<Uuid>,

    // Agile
    pub story_points: Option<i32>,

    // Sprint
    pub sprint_id: Option<Uuid>,

    // Concurrency control
    pub version: i32,

    // Audit
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Uuid,
    pub updated_by: Uuid,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl WorkItem {
    pub fn new(
        item_type: WorkItemType,
        title: String,
        description: Option<String>,
        parent_id: Option<Uuid>,
        project_id: Uuid,
        created_by: Uuid,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            item_type,
            parent_id,
            project_id,
            position: 0,
            title,
            description,
            status: "backlog".to_string(),
            priority: "medium".to_string(),
            assignee_id: None,
            story_points: None,
            sprint_id: None,
            version: 0,
            created_at: now,
            updated_at: now,
            created_by,
            updated_by: created_by,
            deleted_at: None,
        }
    }
}
