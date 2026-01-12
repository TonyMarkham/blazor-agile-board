use crate::{ClientSubscriptions, SubscriptionFilter};

#[test]
fn given_project_subscription_when_work_item_event_then_receives() {
    let mut subs = ClientSubscriptions::new();
    subs.subscribe_project("project-1".to_string());

    assert!(SubscriptionFilter::should_receive_work_item_event(
        &subs,
        "project-1",
        "item-123"
    ));
}

#[test]
fn given_work_item_subscription_when_event_then_receives() {
    let mut subs = ClientSubscriptions::new();
    subs.subscribe_work_item("item-123".to_string());

    // Receives even without project subscription                                                                                                                            
    assert!(SubscriptionFilter::should_receive_work_item_event(
        &subs,
        "project-1",
        "item-123"
    ));
}

#[test]
fn given_no_subscription_when_event_then_does_not_receive() {
    let subs = ClientSubscriptions::new();

    assert!(!SubscriptionFilter::should_receive_work_item_event(
        &subs,
        "project-1",
        "item-123"
    ));
} 