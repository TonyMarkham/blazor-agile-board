use pm_core::{
    ActivityLog, Comment, Dependency, DependencyType, LlmContext, Project, ProjectStatus, Sprint,
    SprintStatus, TimeEntry, WorkItem,
};
use pm_proto::{
    ActivityLogCreated, ActivityLogEntry as ProtoActivityLogEntry, ActivityLogList,
    Comment as ProtoComment, CommentCreated, CommentDeleted, CommentUpdated, CommentsList,
    DependenciesList, Dependency as ProtoDependency, DependencyCreated, DependencyDeleted,
    DependencyType as ProtoDependencyType, Error as PmProtoError, FieldChange,
    LlmContextEntry as ProtoLlmContextEntry, LlmContextList, Project as ProtoProject,
    ProjectCreated, ProjectDeleted, ProjectList, ProjectStatus as ProtoProjectStatus,
    ProjectUpdated, RunningTimerResponse, Sprint as ProtoSprint, SprintCreated, SprintDeleted,
    SprintStatus as ProtoSprintStatus, SprintUpdated, SprintsList, TimeEntriesList,
    TimeEntry as ProtoTimeEntry, TimeEntryCreated, TimeEntryDeleted, TimeEntryUpdated,
    TimerStarted, TimerStopped, WebSocketMessage, WorkItem as PmProtoWorkItem, WorkItemCreated,
    WorkItemDeleted, WorkItemUpdated, WorkItemsList,
    web_socket_message::Payload::{
        ActivityLogCreated as ProtoActivityLogCreated, ActivityLogList as ProtoActivityLogList,
        CommentCreated as ProtoCommentCreated, CommentDeleted as ProtoCommentDeleted,
        CommentUpdated as ProtoCommentUpdated, CommentsList as ProtoCommentsList,
        DependenciesList as ProtoDependenciesList, DependencyCreated as ProtoDependencyCreated,
        DependencyDeleted as ProtoDependencyDeleted, Error as ProtoError,
        LlmContextList as ProtoLlmContextList, ProjectCreated as ProtoProjectCreated,
        ProjectDeleted as ProtoProjectDeleted, ProjectList as ProtoProjectList,
        ProjectUpdated as ProtoProjectUpdated, RunningTimerResponse as ProtoRunningTimerResponse,
        SprintCreated as ProtoSprintCreated, SprintDeleted as ProtoSprintDeleted,
        SprintUpdated as ProtoSprintUpdated, SprintsList as ProtoSprintsList,
        TimeEntriesList as ProtoTimeEntriesList, TimeEntryCreated as ProtoTimeEntryCreated,
        TimeEntryDeleted as ProtoTimeEntryDeleted, TimeEntryUpdated as ProtoTimeEntryUpdated,
        TimerStarted as ProtoTimerStarted, TimerStopped as ProtoTimerStopped,
        WorkItemCreated as ProtoWorkItemCreated, WorkItemDeleted as ProtoWorkItemDeleted,
        WorkItemUpdated as ProtoWorkItemUpdated, WorkItemsList as ProtoWorkItemsList,
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
        item_number: item.item_number,
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
        next_work_item_number: project.next_work_item_number,
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

/// Build SprintCreated response
pub fn build_sprint_created_response(
    message_id: &str,
    sprint: &Sprint,
    actor_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoSprintCreated(SprintCreated {
            sprint: Some(sprint_to_proto(sprint)),
            user_id: actor_id.to_string(),
        })),
    }
}

/// Build SprintUpdated response with field changes
pub fn build_sprint_updated_response(
    message_id: &str,
    sprint: &Sprint,
    changes: &[FieldChange],
    actor_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoSprintUpdated(SprintUpdated {
            sprint: Some(sprint_to_proto(sprint)),
            changes: changes.to_vec(),
            user_id: actor_id.to_string(),
        })),
    }
}

/// Build SprintDeleted response
pub fn build_sprint_deleted_response(
    message_id: &str,
    sprint_id: Uuid,
    actor_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoSprintDeleted(SprintDeleted {
            sprint_id: sprint_id.to_string(),
            user_id: actor_id.to_string(),
        })),
    }
}

/// Build SprintsList response
pub fn build_sprints_list_response(message_id: &str, sprints: Vec<Sprint>) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoSprintsList(SprintsList {
            sprints: sprints.iter().map(sprint_to_proto).collect(),
        })),
    }
}

/// Convert domain Sprint to proto Sprint
fn sprint_to_proto(sprint: &Sprint) -> ProtoSprint {
    ProtoSprint {
        id: sprint.id.to_string(),
        project_id: sprint.project_id.to_string(),
        name: sprint.name.clone(),
        goal: sprint.goal.clone(),
        start_date: sprint.start_date.timestamp(),
        end_date: sprint.end_date.timestamp(),
        status: sprint_status_to_proto(&sprint.status) as i32,
        version: sprint.version,
        created_at: sprint.created_at.timestamp(),
        updated_at: sprint.updated_at.timestamp(),
        created_by: sprint.created_by.to_string(),
        updated_by: sprint.updated_by.to_string(),
        deleted_at: sprint.deleted_at.map(|dt| dt.timestamp()),
    }
}

/// Convert domain SprintStatus to proto SprintStatus
fn sprint_status_to_proto(status: &SprintStatus) -> ProtoSprintStatus {
    match status {
        SprintStatus::Planned => ProtoSprintStatus::Planned,
        SprintStatus::Active => ProtoSprintStatus::Active,
        SprintStatus::Completed => ProtoSprintStatus::Completed,
        SprintStatus::Cancelled => ProtoSprintStatus::Cancelled,
    }
}

fn comment_to_proto(comment: &Comment) -> ProtoComment {
    ProtoComment {
        id: comment.id.to_string(),
        work_item_id: comment.work_item_id.to_string(),
        content: comment.content.clone(),
        created_at: comment.created_at.timestamp(),
        updated_at: comment.updated_at.timestamp(),
        created_by: comment.created_by.to_string(),
        updated_by: comment.updated_by.to_string(),
        deleted_at: comment.deleted_at.map(|dt| dt.timestamp()),
    }
}

pub fn build_comment_created_response(
    message_id: &str,
    comment: &Comment,
    actor_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoCommentCreated(CommentCreated {
            comment: Some(comment_to_proto(comment)),
            user_id: actor_id.to_string(),
        })),
    }
}

pub fn build_comment_updated_response(
    message_id: &str,
    comment: &Comment,
    actor_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoCommentUpdated(CommentUpdated {
            comment: Some(comment_to_proto(comment)),
            user_id: actor_id.to_string(),
        })),
    }
}

pub fn build_comment_deleted_response(
    message_id: &str,
    comment_id: Uuid,
    actor_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoCommentDeleted(CommentDeleted {
            comment_id: comment_id.to_string(),
            user_id: actor_id.to_string(),
        })),
    }
}

pub fn build_comments_list_response(message_id: &str, comments: Vec<Comment>) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoCommentsList(CommentsList {
            comments: comments.iter().map(comment_to_proto).collect(),
        })),
    }
}

// === Time Entry Converters ===

/// Convert domain TimeEntry to protobuf TimeEntry
pub fn time_entry_to_proto(entry: &TimeEntry) -> ProtoTimeEntry {
    ProtoTimeEntry {
        id: entry.id.to_string(),
        work_item_id: entry.work_item_id.to_string(),
        user_id: entry.user_id.to_string(),
        started_at: entry.started_at.timestamp(),
        ended_at: entry.ended_at.map(|dt| dt.timestamp()),
        duration_seconds: entry.duration_seconds,
        description: entry.description.clone(),
        created_at: entry.created_at.timestamp(),
        updated_at: entry.updated_at.timestamp(),
        deleted_at: entry.deleted_at.map(|dt| dt.timestamp()),
    }
}

/// Convert domain Dependency to protobuf Dependency
pub fn dependency_to_proto(dep: &Dependency) -> ProtoDependency {
    ProtoDependency {
        id: dep.id.to_string(),
        blocking_item_id: dep.blocking_item_id.to_string(),
        blocked_item_id: dep.blocked_item_id.to_string(),
        dependency_type: match dep.dependency_type {
            DependencyType::Blocks => ProtoDependencyType::Blocks as i32,
            DependencyType::RelatesTo => ProtoDependencyType::RelatesTo as i32,
        },
        created_at: dep.created_at.timestamp(),
        created_by: dep.created_by.to_string(),
        deleted_at: dep.deleted_at.map(|dt| dt.timestamp()),
    }
}

// === Time Entry Response Builders ===

/// Build TimerStarted response with optional stopped entry
pub fn build_timer_started_response(
    message_id: &str,
    entry: &TimeEntry,
    stopped_entry: Option<&TimeEntry>,
    user_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoTimerStarted(TimerStarted {
            time_entry: Some(time_entry_to_proto(entry)),
            user_id: user_id.to_string(),
            stopped_entry: stopped_entry.map(time_entry_to_proto),
        })),
    }
}

/// Build TimerStopped response
pub fn build_timer_stopped_response(
    message_id: &str,
    entry: &TimeEntry,
    user_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoTimerStopped(TimerStopped {
            time_entry: Some(time_entry_to_proto(entry)),
            user_id: user_id.to_string(),
        })),
    }
}

/// Build TimeEntryCreated response
pub fn build_time_entry_created_response(
    message_id: &str,
    entry: &TimeEntry,
    user_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoTimeEntryCreated(TimeEntryCreated {
            time_entry: Some(time_entry_to_proto(entry)),
            user_id: user_id.to_string(),
        })),
    }
}

/// Build TimeEntryUpdated response
pub fn build_time_entry_updated_response(
    message_id: &str,
    entry: &TimeEntry,
    user_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoTimeEntryUpdated(TimeEntryUpdated {
            time_entry: Some(time_entry_to_proto(entry)),
            user_id: user_id.to_string(),
        })),
    }
}

/// Build TimeEntryDeleted response
pub fn build_time_entry_deleted_response(
    message_id: &str,
    time_entry_id: Uuid,
    work_item_id: Uuid,
    user_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoTimeEntryDeleted(TimeEntryDeleted {
            time_entry_id: time_entry_id.to_string(),
            work_item_id: work_item_id.to_string(),
            user_id: user_id.to_string(),
        })),
    }
}

/// Build TimeEntriesList response with pagination info
pub fn build_time_entries_list_response(
    message_id: &str,
    entries: &[TimeEntry],
    total_count: i32,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoTimeEntriesList(TimeEntriesList {
            time_entries: entries.iter().map(time_entry_to_proto).collect(),
            total_count,
        })),
    }
}

/// Build RunningTimerResponse (may be empty if no running timer)
pub fn build_running_timer_response(
    message_id: &str,
    entry: Option<&TimeEntry>,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoRunningTimerResponse(RunningTimerResponse {
            time_entry: entry.map(time_entry_to_proto),
        })),
    }
}

// === Dependency Response Builders ===

/// Build DependencyCreated response
pub fn build_dependency_created_response(
    message_id: &str,
    dependency: &Dependency,
    user_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoDependencyCreated(DependencyCreated {
            dependency: Some(dependency_to_proto(dependency)),
            user_id: user_id.to_string(),
        })),
    }
}

/// Build DependencyDeleted response
pub fn build_dependency_deleted_response(
    message_id: &str,
    dependency_id: Uuid,
    blocking_item_id: Uuid,
    blocked_item_id: Uuid,
    user_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoDependencyDeleted(DependencyDeleted {
            dependency_id: dependency_id.to_string(),
            blocking_item_id: blocking_item_id.to_string(),
            blocked_item_id: blocked_item_id.to_string(),
            user_id: user_id.to_string(),
        })),
    }
}

/// Build DependenciesList response with both directions
pub fn build_dependencies_list_response(
    message_id: &str,
    blocking: &[Dependency],
    blocked: &[Dependency],
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoDependenciesList(DependenciesList {
            blocking: blocking.iter().map(dependency_to_proto).collect(),
            blocked: blocked.iter().map(dependency_to_proto).collect(),
        })),
    }
}

// === Activity Log + LLM Context Response Builders ===

fn activity_log_to_proto(entry: &ActivityLog) -> ProtoActivityLogEntry {
    ProtoActivityLogEntry {
        id: entry.id.to_string(),
        entity_type: entry.entity_type.clone(),
        entity_id: entry.entity_id.to_string(),
        action: entry.action.clone(),
        field_name: entry.field_name.clone(),
        old_value: entry.old_value.clone(),
        new_value: entry.new_value.clone(),
        user_id: entry.user_id.to_string(),
        timestamp: entry.timestamp.timestamp(),
        comment: entry.comment.clone(),
    }
}

pub fn build_activity_log_list_response(
    message_id: &str,
    entries: Vec<ActivityLog>,
    total_count: i64,
    limit: i64,
    offset: i64,
) -> WebSocketMessage {
    let has_more = (offset + limit) < total_count;

    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoActivityLogList(ActivityLogList {
            entries: entries.iter().map(activity_log_to_proto).collect(),
            total_count: total_count as i32,
            has_more,
        })),
    }
}

pub fn build_activity_log_created_event(entry: &ActivityLog) -> WebSocketMessage {
    WebSocketMessage {
        message_id: Uuid::new_v4().to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoActivityLogCreated(ActivityLogCreated {
            entry: Some(activity_log_to_proto(entry)),
        })),
    }
}

fn llm_context_to_proto(entry: &LlmContext) -> ProtoLlmContextEntry {
    ProtoLlmContextEntry {
        id: entry.id.to_string(),
        context_type: entry.context_type.as_str().to_string(),
        category: entry.category.clone(),
        title: entry.title.clone(),
        content: entry.content.clone(),
        example_sql: entry.example_sql.clone(),
        example_description: entry.example_description.clone(),
        priority: entry.priority,
    }
}

pub fn build_llm_context_list_response(
    message_id: &str,
    entries: Vec<LlmContext>,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoLlmContextList(LlmContextList {
            entries: entries.iter().map(llm_context_to_proto).collect(),
        })),
    }
}
