pub mod admin;
pub mod api;
pub mod error;
pub mod health;
pub mod logger;
pub mod routes;

pub use api::{
    comments::{
        comment_dto::CommentDto,
        comment_list_response::CommentListResponse,
        comment_response::CommentResponse,
        comments::{create_comment, delete_comment, list_comments, update_comment},
        create_comment_request::CreateCommentRequest,
        update_comment_request::UpdateCommentRequest,
    },
    delete_response::DeleteResponse,
    error::ApiError,
    error::Result as ApiResult,
    extractors::user_id::UserId,
    projects::{
        project_dto::ProjectDto,
        project_list_response::ProjectListResponse,
        project_response::ProjectResponse,
        projects::{get_project, list_projects},
    },
    work_items::{
        create_work_item_request::CreateWorkItemRequest,
        list_work_item_query::ListWorkItemsQuery,
        update_work_item_request::UpdateWorkItemRequest,
        work_item_dto::WorkItemDto,
        work_item_list_response::WorkItemListResponse,
        work_item_response::WorkItemResponse,
        work_items::{
            create_work_item, delete_work_item, get_work_item, list_work_items, update_work_item,
        },
    },
};

pub use crate::routes::build_router;
