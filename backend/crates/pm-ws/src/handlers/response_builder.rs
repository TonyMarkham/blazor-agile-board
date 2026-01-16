use pm_core::models::work_item::WorkItem;
use pm_proto::{
    Error as PmProtoError, FieldChange, WebSocketMessage, WorkItem as PmProtoWorkItem,
    WorkItemCreated, WorkItemDeleted, WorkItemUpdated, WorkItemsList,
    web_socket_message::Payload::{
        Error as ProtoError, WorkItemCreated as ProtoWorkItemCreated,
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
        parent_id: item.parent_id.map(|id| id.to_string()),
        project_id: item.project_id.to_string(),
        assignee_id: item.assignee_id.map(|id| id.to_string()),
        position: item.position,
        sprint_id: item.sprint_id.map(|id| id.to_string()),
        created_at: item.created_at.timestamp(),
        updated_at: item.updated_at.timestamp(),
        created_by: item.created_by.to_string(),
        updated_by: item.updated_by.to_string(),
        deleted_at: item.deleted_at.map(|dt| dt.timestamp()),
    }
}
