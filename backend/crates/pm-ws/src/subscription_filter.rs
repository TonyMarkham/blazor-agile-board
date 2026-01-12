use crate::ClientSubscriptions;

/// Helper for filtering broadcast events based on subscriptions                                                                                                                 
pub struct SubscriptionFilter;

impl SubscriptionFilter {
    /// Check if client should receive an event about a work item                                                                                                                
    pub fn should_receive_work_item_event(
        subscriptions: &ClientSubscriptions,
        project_id: &str,
        work_item_id: &str,
    ) -> bool {
        // Receive if subscribed to the project OR the specific work item                                                                                                        
        subscriptions.is_subscribed_to_project(project_id)
            || subscriptions.is_subscribed_to_work_item(work_item_id)
    }

    /// Check if client should receive an event about a sprint                                                                                                                   
    pub fn should_receive_sprint_event(
        subscriptions: &ClientSubscriptions,
        project_id: &str,
        sprint_id: &str,
    ) -> bool {
        // Receive if subscribed to the project OR the specific sprint                                                                                                           
        subscriptions.is_subscribed_to_project(project_id)
            || subscriptions.is_subscribed_to_sprint(sprint_id)
    }

    /// Check if client should receive a comment event                                                                                                                           
    pub fn should_receive_comment_event(
        subscriptions: &ClientSubscriptions,
        project_id: &str,
        work_item_id: &str,
    ) -> bool {
        // Comments follow work item subscription rules                                                                                                                          
        Self::should_receive_work_item_event(subscriptions, project_id, work_item_id)
    }
}  