use pm_core::WorkItem;
use pm_core::{Project, ProjectStatus};
use pm_proto::{
    Error as PmProtoError, FieldChange, Project as ProtoProject, ProjectCreated, ProjectDeleted,
    ProjectList, ProjectStatus as ProtoProjectStatus, ProjectUpdated, WebSocketMessage,
    WorkItem as PmProtoWorkItem, WorkItemCreated, WorkItemDeleted, WorkItemUpdated, WorkItemsList,
    web_socket_message::Payload::{
        Error as ProtoError, ProjectCreated as ProtoProjectCreated,
        ProjectDeleted as ProtoProjectDeleted, ProjectList as ProtoProjectList,
        ProjectUpdated as ProtoProjectUpdated, WorkItemCreated as ProtoWorkItemCreated,
        WorkItemDeleted as ProtoWorkItemDeleted, WorkItemUpdated as ProtoWorkItemUpdated,
        WorkItemsList as ProtoWorkItemsList,
    },
};

use chrono::Utc;
use uuid::Uuid;

/// Build WorkItemCreated response
pub fn build_work_item_created_response(
    message_id: &str,
    work_item: &WorkItem,
    actor_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoWorkItemCreated(WorkItemCreated {
            work_item: Some(work_item_to_proto(work_item)),
            user_id: actor_id.to_string(),
        })),
    }
}

/// Build WorkItemUpdated response with field changes
pub fn build_work_item_updated_response(
    message_id: &str,
    work_item: &WorkItem,
    changes: &[FieldChange],
    actor_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoWorkItemUpdated(WorkItemUpdated {
            work_item: Some(work_item_to_proto(work_item)),
            changes: changes.to_vec(),
            user_id: actor_id.to_string(),
        })),
    }
}

/// Build WorkItemDeleted response
pub fn build_work_item_deleted_response(
    message_id: &str,
    work_item_id: Uuid,
    actor_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoWorkItemDeleted(WorkItemDeleted {
            work_item_id: work_item_id.to_string(),
            user_id: actor_id.to_string(),
        })),
    }
}

/// Build WorkItemsList response
pub fn build_work_items_list_response(
    message_id: &str,
    work_items: Vec<WorkItem>,
    as_of_timestamp: i64,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoWorkItemsList(WorkItemsList {
            work_items: work_items.iter().map(work_item_to_proto).collect(),
            as_of_timestamp,
        })),
    }
}

/// Build error response
pub fn build_error_response(message_id: &str, error: PmProtoError) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoError(error)),
    }
}

/// Convert domain WorkItem to proto WorkItem
fn work_item_to_proto(item: &WorkItem) -> PmProtoWorkItem {
    PmProtoWorkItem {
        id: item.id.to_string(),
        item_type: item.item_type.clone() as i32,
        title: item.title.clone(),
        description: item.description.clone(),
        status: item.status.clone(),
        priority: item.priority.clone(),
        parent_id: item.parent_id.map(|id| id.to_string()),
        project_id: item.project_id.to_string(),
        assignee_id: item.assignee_id.map(|id| id.to_string()),
        story_points: item.story_points,
        position: item.position,
        sprint_id: item.sprint_id.map(|id| id.to_string()),
        version: item.version,
        created_at: item.created_at.timestamp(),
        updated_at: item.updated_at.timestamp(),
        created_by: item.created_by.to_string(),
        updated_by: item.updated_by.to_string(),
        deleted_at: item.deleted_at.map(|dt| dt.timestamp()),
    }
}

/// Convert domain Project to proto Project
fn project_to_proto(project: &Project) -> ProtoProject {
    ProtoProject {
        id: project.id.to_string(),
        title: project.title.clone(),
        description: project.description.clone(),
        key: project.key.clone(),
        status: match project.status {
            ProjectStatus::Active => ProtoProjectStatus::Active.into(),
            ProjectStatus::Archived => ProtoProjectStatus::Archived.into(),
        },
        version: project.version,
        created_at: project.created_at.timestamp(),
        updated_at: project.updated_at.timestamp(),
        created_by: project.created_by.to_string(),
        updated_by: project.updated_by.to_string(),
        deleted_at: project.deleted_at.map(|dt| dt.timestamp()),
    }
}

/// Build ProjectCreated response
pub fn build_project_created_response(
    message_id: &str,
    project: &Project,
    actor_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoProjectCreated(ProjectCreated {
            project: Some(project_to_proto(project)),
            user_id: actor_id.to_string(),
        })),
    }
}

/// Build ProjectUpdated response with field changes
pub fn build_project_updated_response(
    message_id: &str,
    project: &Project,
    changes: &[FieldChange],
    actor_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoProjectUpdated(ProjectUpdated {
            project: Some(project_to_proto(project)),
            changes: changes.to_vec(),
            user_id: actor_id.to_string(),
        })),
    }
}

/// Build ProjectDeleted response
pub fn build_project_deleted_response(
    message_id: &str,
    project_id: Uuid,
    actor_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoProjectDeleted(ProjectDeleted {
            project_id: project_id.to_string(),
            user_id: actor_id.to_string(),
        })),
    }
}

/// Build ProjectList response
pub fn build_project_list_response(message_id: &str, projects: &[Project]) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoProjectList(ProjectList {
            projects: projects.iter().map(project_to_proto).collect(),
        })),
    }
}
