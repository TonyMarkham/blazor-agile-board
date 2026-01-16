use crate::ClientSubscriptions;

#[test]
fn given_new_subscriptions_when_created_then_is_empty() {
    let subs = ClientSubscriptions::new();
    assert!(subs.is_empty());
    assert_eq!(subs.total_count(), 0);
}

#[test]
fn given_project_subscription_when_added_then_is_subscribed() {
    let mut subs = ClientSubscriptions::new();
    subs.subscribe_project("project-123".to_string());

    assert!(subs.is_subscribed_to_project("project-123"));
    assert!(!subs.is_subscribed_to_project("project-456"));
    assert_eq!(subs.total_count(), 1);
}

#[test]
fn given_subscribed_project_when_unsubscribed_then_not_subscribed() {
    let mut subs = ClientSubscriptions::new();
    subs.subscribe_project("project-123".to_string());
    subs.unsubscribe_project("project-123");

    assert!(!subs.is_subscribed_to_project("project-123"));
    assert!(subs.is_empty());
}

#[test]
fn given_multiple_subscriptions_when_cleared_then_is_empty() {
    let mut subs = ClientSubscriptions::new();
    subs.subscribe_project("project-1".to_string());
    subs.subscribe_sprint("sprint-1".to_string());
    subs.subscribe_work_item("item-1".to_string());

    assert_eq!(subs.total_count(), 3);

    subs.clear();

    assert!(subs.is_empty());
    assert_eq!(subs.total_count(), 0);
}

#[test]
fn given_subscriptions_when_get_projects_then_returns_list() {
    let mut subs = ClientSubscriptions::new();
    subs.subscribe_project("project-1".to_string());
    subs.subscribe_project("project-2".to_string());

    let projects = subs.get_projects();
    assert_eq!(projects.len(), 2);
    assert!(projects.contains(&"project-1".to_string()));
    assert!(projects.contains(&"project-2".to_string()));
}
