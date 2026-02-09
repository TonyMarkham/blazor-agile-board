use crate::{CliClientResult, ClientError};

use std::panic::Location;

use error_location::ErrorLocation;
use reqwest::{Client as ReqwestClient, Method};
use serde::Serialize;
use serde_json::Value;

/// HTTP client for the pm-server REST API
pub struct Client {
    pub base_url: String,
    pub user_id: Option<String>,
    client: ReqwestClient,
}

impl Client {
    /// Create a new client
    ///
    /// # Arguments
    /// * `base_url` - Server URL (e.g., "http://127.0.0.1:8080")
    /// * `user_id` - Optional user ID to include in X-User-Id header
    pub fn new(base_url: &str, user_id: Option<&str>) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            user_id: user_id.map(String::from),
            client: ReqwestClient::new(),
        }
    }

    /// Build a request with optional user ID header
    fn request(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.request(method, &url);

        if let Some(ref user_id) = self.user_id {
            req = req.header("X-User-Id", user_id);
        }

        req
    }

    /// Execute request and handle errors
    async fn execute(&self, req: reqwest::RequestBuilder) -> CliClientResult<Value> {
        let response = req.send().await?;
        let status = response.status();
        let body: Value = response.json().await?;

        // Check for error response
        #[allow(clippy::collapsible_if)]
        if !status.is_success() {
            if let Some(error) = body.get("error") {
                let code = error
                    .get("code")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UNKNOWN")
                    .to_string();
                let message = error
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error")
                    .to_string();
                return Err(ClientError::Api {
                    code,
                    message,
                    location: ErrorLocation::from(Location::caller()),
                });
            }
        }

        Ok(body)
    }

    // =========================================================================
    // Project Operations
    // =========================================================================

    /// List all projects
    pub async fn list_projects(&self) -> CliClientResult<Value> {
        let req = self.request(Method::GET, "/api/v1/projects");
        self.execute(req).await
    }

    /// Get a project by ID
    pub async fn get_project(&self, id: &str) -> CliClientResult<Value> {
        let req = self.request(Method::GET, &format!("/api/v1/projects/{}", id));
        self.execute(req).await
    }

    /// Create a new project
    pub async fn create_project(
        &self,
        title: &str,
        key: &str,
        description: Option<&str>,
    ) -> CliClientResult<Value> {
        #[derive(Serialize)]
        struct CreateRequest<'a> {
            title: &'a str,
            key: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            description: Option<&'a str>,
        }

        let body = CreateRequest {
            title,
            key,
            description,
        };
        let req = self.request(Method::POST, "/api/v1/projects").json(&body);
        self.execute(req).await
    }

    /// Update a project
    pub async fn update_project(
        &self,
        id: &str,
        title: Option<&str>,
        description: Option<&str>,
        status: Option<&str>,
        expected_version: i32,
    ) -> CliClientResult<Value> {
        #[derive(Serialize)]
        struct UpdateRequest<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            title: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            description: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            status: Option<&'a str>,
            expected_version: i32,
        }

        let body = UpdateRequest {
            title,
            description,
            status,
            expected_version,
        };
        let req = self
            .request(Method::PUT, &format!("/api/v1/projects/{}", id))
            .json(&body);
        self.execute(req).await
    }

    /// Delete a project
    pub async fn delete_project(&self, id: &str) -> CliClientResult<Value> {
        let req = self.request(Method::DELETE, &format!("/api/v1/projects/{}", id));
        self.execute(req).await
    }

    // =========================================================================
    // Sprint Operations
    // =========================================================================

    /// List sprints in a project
    pub async fn list_sprints(&self, project_id: &str) -> CliClientResult<Value> {
        let req = self.request(
            Method::GET,
            &format!("/api/v1/projects/{}/sprints", project_id),
        );
        self.execute(req).await
    }

    /// Get a sprint by ID
    pub async fn get_sprint(&self, id: &str) -> CliClientResult<Value> {
        let req = self.request(Method::GET, &format!("/api/v1/sprints/{}", id));
        self.execute(req).await
    }

    /// Create a new sprint
    pub async fn create_sprint(
        &self,
        project_id: &str,
        name: &str,
        start_date: i64,
        end_date: i64,
        goal: Option<&str>,
    ) -> CliClientResult<Value> {
        #[derive(Serialize)]
        struct CreateSprintRequest<'a> {
            project_id: &'a str,
            name: &'a str,
            start_date: i64,
            end_date: i64,
            #[serde(skip_serializing_if = "Option::is_none")]
            goal: Option<&'a str>,
        }

        let body = CreateSprintRequest {
            project_id,
            name,
            start_date,
            end_date,
            goal,
        };

        let req = self.request(Method::POST, "/api/v1/sprints").json(&body);
        self.execute(req).await
    }

    /// Update a sprint
    #[allow(clippy::too_many_arguments)]
    pub async fn update_sprint(
        &self,
        id: &str,
        name: Option<&str>,
        goal: Option<&str>,
        start_date: Option<i64>,
        end_date: Option<i64>,
        status: Option<&str>,
        expected_version: i32,
    ) -> CliClientResult<Value> {
        #[derive(Serialize)]
        struct UpdateSprintRequest<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            name: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            goal: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            start_date: Option<i64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            end_date: Option<i64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            status: Option<&'a str>,
            expected_version: i32,
        }

        let body = UpdateSprintRequest {
            name,
            goal,
            start_date,
            end_date,
            status,
            expected_version,
        };

        let req = self
            .request(Method::PUT, &format!("/api/v1/sprints/{}", id))
            .json(&body);
        self.execute(req).await
    }

    /// Delete a sprint
    pub async fn delete_sprint(&self, id: &str) -> CliClientResult<Value> {
        let req = self.request(Method::DELETE, &format!("/api/v1/sprints/{}", id));
        self.execute(req).await
    }

    // =========================================================================
    // Work Item Operations
    // =========================================================================

    /// Get a work item by ID
    pub async fn get_work_item(&self, id: &str) -> CliClientResult<Value> {
        let req = self.request(Method::GET, &format!("/api/v1/work-items/{}", id));
        self.execute(req).await
    }

    /// List work items in a project
    pub async fn list_work_items(
        &self,
        project_id: &str,
        item_type: Option<&str>,
        status: Option<&str>,
    ) -> CliClientResult<Value> {
        let mut url = format!("/api/v1/projects/{}/work-items", project_id);

        // Build query string
        let mut params = vec![];
        if let Some(t) = item_type {
            params.push(format!("type={}", t));
        }
        if let Some(s) = status {
            params.push(format!("status={}", s));
        }
        if !params.is_empty() {
            url.push_str(&format!("?{}", params.join("&")));
        }

        let req = self.request(Method::GET, &url);
        self.execute(req).await
    }

    /// Create a new work item
    #[allow(clippy::too_many_arguments)]
    pub async fn create_work_item(
        &self,
        project_id: &str,
        item_type: &str,
        title: &str,
        description: Option<&str>,
        parent_id: Option<&str>,
        status: Option<&str>,
        priority: Option<&str>,
    ) -> CliClientResult<Value> {
        #[derive(Serialize)]
        struct CreateRequest<'a> {
            project_id: &'a str,
            item_type: &'a str,
            title: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            description: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            parent_id: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            status: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            priority: Option<&'a str>,
        }

        let body = CreateRequest {
            project_id,
            item_type,
            title,
            description,
            parent_id,
            status,
            priority,
        };

        let req = self.request(Method::POST, "/api/v1/work-items").json(&body);
        self.execute(req).await
    }

    /// Update a work item
    #[allow(clippy::too_many_arguments)]
    pub async fn update_work_item(
        &self,
        id: &str,
        title: Option<&str>,
        description: Option<&str>,
        status: Option<&str>,
        priority: Option<&str>,
        assignee_id: Option<&str>,
        sprint_id: Option<&str>,
        story_points: Option<i32>,
        expected_version: i32,
    ) -> CliClientResult<Value> {
        #[derive(Serialize)]
        struct UpdateRequest<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            title: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            description: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            status: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            priority: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            assignee_id: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            sprint_id: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            story_points: Option<i32>,
            expected_version: i32,
        }

        let body = UpdateRequest {
            title,
            description,
            status,
            priority,
            assignee_id,
            sprint_id,
            story_points,
            expected_version,
        };

        let req = self
            .request(Method::PUT, &format!("/api/v1/work-items/{}", id))
            .json(&body);
        self.execute(req).await
    }

    /// Delete a work item
    pub async fn delete_work_item(&self, id: &str) -> CliClientResult<Value> {
        let req = self.request(Method::DELETE, &format!("/api/v1/work-items/{}", id));
        self.execute(req).await
    }

    // =========================================================================
    // Comment Operations
    // =========================================================================

    /// List comments on a work item
    pub async fn list_comments(&self, work_item_id: &str) -> CliClientResult<Value> {
        let req = self.request(
            Method::GET,
            &format!("/api/v1/work-items/{}/comments", work_item_id),
        );
        self.execute(req).await
    }

    /// Create a comment on a work item
    pub async fn create_comment(
        &self,
        work_item_id: &str,
        content: &str,
    ) -> CliClientResult<Value> {
        #[derive(Serialize)]
        struct CreateCommentRequest<'a> {
            content: &'a str,
        }

        let body = CreateCommentRequest { content };
        let req = self
            .request(
                Method::POST,
                &format!("/api/v1/work-items/{}/comments", work_item_id),
            )
            .json(&body);
        self.execute(req).await
    }

    /// Update a comment
    pub async fn update_comment(&self, id: &str, content: &str) -> CliClientResult<Value> {
        #[derive(Serialize)]
        struct UpdateCommentRequest<'a> {
            content: &'a str,
        }

        let body = UpdateCommentRequest { content };
        let req = self
            .request(Method::PUT, &format!("/api/v1/comments/{}", id))
            .json(&body);
        self.execute(req).await
    }

    /// Delete a comment
    pub async fn delete_comment(&self, id: &str) -> CliClientResult<Value> {
        let req = self.request(Method::DELETE, &format!("/api/v1/comments/{}", id));
        self.execute(req).await
    }

    // =========================================================================
    // Dependency Operations
    // =========================================================================

    /// List dependencies for a work item (both blocking and blocked)
    pub async fn list_dependencies(&self, work_item_id: &str) -> CliClientResult<Value> {
        let req = self.request(
            Method::GET,
            &format!("/api/v1/work-items/{}/dependencies", work_item_id),
        );
        self.execute(req).await
    }

    /// Create a dependency link
    pub async fn create_dependency(
        &self,
        blocking_item_id: &str,
        blocked_item_id: &str,
        dependency_type: &str,
    ) -> CliClientResult<Value> {
        #[derive(Serialize)]
        struct CreateRequest<'a> {
            blocking_item_id: &'a str,
            blocked_item_id: &'a str,
            dependency_type: &'a str,
        }

        let body = CreateRequest {
            blocking_item_id,
            blocked_item_id,
            dependency_type,
        };
        let req = self
            .request(Method::POST, "/api/v1/dependencies")
            .json(&body);
        self.execute(req).await
    }

    /// Delete a dependency
    pub async fn delete_dependency(&self, id: &str) -> CliClientResult<Value> {
        let req = self.request(Method::DELETE, &format!("/api/v1/dependencies/{}", id));
        self.execute(req).await
    }

    // =========================================================================
    // Swim Lane Operations (read-only — swim lanes are fixed configuration)
    // =========================================================================

    /// List swim lanes for a project (ordered by position)
    pub async fn list_swim_lanes(&self, project_id: &str) -> CliClientResult<Value> {
        let req = self.request(
            Method::GET,
            &format!("/api/v1/projects/{}/swim-lanes", project_id),
        );
        self.execute(req).await
    }
}
