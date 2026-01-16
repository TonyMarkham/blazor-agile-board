use pm_core::WorkItem;
use pm_proto::{FieldChange, UpdateWorkItemRequest};

/// Track which fields changed between current state and update request.
/// Returns list of FieldChange for the WorkItemUpdated event.
pub fn track_changes(current: &WorkItem, request: &UpdateWorkItemRequest) -> Vec<FieldChange> {
    let mut changes = Vec::new();

    if let Some(ref new_title) = request.title
        && &current.title != new_title
    {
        changes.push(FieldChange {
            field_name: "title".to_string(),
            old_value: Some(current.title.clone()),
            new_value: Some(new_title.clone()),
        });
    }

    if let Some(ref new_desc) = request.description {
        let current_desc = current.description.as_deref().unwrap_or("");
        if current_desc != new_desc {
            changes.push(FieldChange {
                field_name: "description".to_string(),
                old_value: Some(current_desc.to_string()),
                new_value: Some(new_desc.clone()),
            });
        }
    }

    if let Some(ref new_status) = request.status
        && &current.status != new_status
    {
        changes.push(FieldChange {
            field_name: "status".to_string(),
            old_value: Some(current.status.clone()),
            new_value: Some(new_status.clone()),
        });
    }

    if let Some(ref new_assignee) = request.assignee_id {
        let current_assignee = current
            .assignee_id
            .map(|id| id.to_string())
            .unwrap_or_default();
        if &current_assignee != new_assignee {
            changes.push(FieldChange {
                field_name: "assignee_id".to_string(),
                old_value: Some(current_assignee),
                new_value: Some(new_assignee.clone()),
            });
        }
    }

    if let Some(new_position) = request.position
        && current.position != new_position
    {
        changes.push(FieldChange {
            field_name: "position".to_string(),
            old_value: Some(format!("{}", current.position)),
            new_value: Some(format!("{new_position}")),
        });
    }

    changes
}
