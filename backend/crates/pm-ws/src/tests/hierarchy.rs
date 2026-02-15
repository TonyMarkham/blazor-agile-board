//! Unit tests for hierarchy computation module.
//!
//! Tests verify correct ancestor/descendant computation for:
//! - Empty input (no items)
//! - Linear chains (A -> B -> C)
//! - Branching trees (A -> {B, C}, B -> {D, E})
//! - Cycle detection (malformed data safety)
//! - Single-item API

use crate::{compute_hierarchy_for_item, compute_hierarchy_maps};

use pm_core::WorkItem;
use uuid::Uuid;

/// Create a minimal WorkItem with only hierarchy-relevant fields.
fn test_item(id: Uuid, parent_id: Option<Uuid>) -> WorkItem {
    WorkItem {
        id,
        parent_id,
        item_type: pm_core::WorkItemType::Task,
        project_id: Uuid::new_v4(),
        position: 0,
        title: String::new(),
        description: None,
        status: "backlog".into(),
        priority: "medium".into(),
        assignee_id: None,
        story_points: None,
        sprint_id: None,
        item_number: 0,
        version: 1,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        created_by: Uuid::nil(),
        updated_by: Uuid::nil(),
        deleted_at: None,
    }
}

/// Helper: sort a Vec<String> for order-independent comparison.
fn sorted(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v
}

// =========================================================================
// Tests
// =========================================================================

/// WHAT: Empty item list returns empty maps
/// WHY: Ensures no panics on degenerate input
#[test]
fn given_empty_list_when_computing_hierarchy_then_returns_empty_map() {
    // Given: No items
    let items: Vec<WorkItem> = vec![];

    // When: Computing hierarchy
    let result = compute_hierarchy_maps(&items);

    // Then: Empty map
    assert!(result.is_empty());
}

/// WHAT: Linear chain A->B->C produces correct ancestors and descendants
/// WHY: Validates basic parent traversal and BFS descent
#[test]
fn given_linear_chain_when_computing_hierarchy_then_correct_ancestors_and_descendants() {
    // Given: A -> B -> C
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();
    let items = vec![
        test_item(a, None),
        test_item(b, Some(a)),
        test_item(c, Some(b)),
    ];

    // When
    let result = compute_hierarchy_maps(&items);

    // Then: A has no ancestors, descendants = {B, C}
    let a_h = &result[&a];
    assert!(a_h.ancestor_ids.is_empty());
    assert_eq!(
        sorted(a_h.descendant_ids.clone()),
        sorted(vec![b.to_string(), c.to_string()])
    );

    // Then: B has ancestor [A], descendant [C]
    let b_h = &result[&b];
    assert_eq!(b_h.ancestor_ids, vec![a.to_string()]);
    assert_eq!(b_h.descendant_ids, vec![c.to_string()]);

    // Then: C has ancestors [B, A] (immediate parent first), no descendants
    let c_h = &result[&c];
    assert_eq!(c_h.ancestor_ids, vec![b.to_string(), a.to_string()]);
    assert!(c_h.descendant_ids.is_empty());
}

/// WHAT: Branching tree produces correct hierarchy for all nodes
/// WHY: Validates BFS handles multiple children correctly
#[test]
fn given_branching_tree_when_computing_hierarchy_then_all_branches_included() {
    // Given: A -> {B, C}, B -> {D, E}
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();
    let d = Uuid::new_v4();
    let e = Uuid::new_v4();
    let items = vec![
        test_item(a, None),
        test_item(b, Some(a)),
        test_item(c, Some(a)),
        test_item(d, Some(b)),
        test_item(e, Some(b)),
    ];

    // When
    let result = compute_hierarchy_maps(&items);

    // Then: A has 4 descendants {B, C, D, E}
    assert_eq!(
        sorted(result[&a].descendant_ids.clone()),
        sorted(vec![
            b.to_string(),
            c.to_string(),
            d.to_string(),
            e.to_string()
        ])
    );

    // Then: B has ancestor [A], descendants {D, E}
    assert_eq!(result[&b].ancestor_ids, vec![a.to_string()]);
    assert_eq!(
        sorted(result[&b].descendant_ids.clone()),
        sorted(vec![d.to_string(), e.to_string()])
    );

    // Then: C has ancestor [A], no descendants
    assert_eq!(result[&c].ancestor_ids, vec![a.to_string()]);
    assert!(result[&c].descendant_ids.is_empty());

    // Then: D has ancestors [B, A], no descendants
    assert_eq!(result[&d].ancestor_ids, vec![b.to_string(), a.to_string()]);
    assert!(result[&d].descendant_ids.is_empty());
}

/// WHAT: Circular parent reference does not cause infinite loop
/// WHY: Malformed data must not crash the server
#[test]
fn given_circular_reference_when_computing_hierarchy_then_no_infinite_loop() {
    // Given: A -> B -> A (cycle)
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let items = vec![test_item(a, Some(b)), test_item(b, Some(a))];

    // When: Should complete without hanging
    let result = compute_hierarchy_maps(&items);

    // Then: Both items in result (cycle was broken by visited set)
    assert_eq!(result.len(), 2);
}

/// WHAT: Single-item API returns correct data for a middle node
/// WHY: Validates the create/update broadcast path
#[test]
fn given_tree_when_computing_for_single_item_then_correct_ancestors_and_descendants() {
    // Given: A -> B -> C
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();
    let items = vec![
        test_item(a, None),
        test_item(b, Some(a)),
        test_item(c, Some(b)),
    ];

    // When: Computing for B only
    let b_h = compute_hierarchy_for_item(&items, b);

    // Then: B has ancestor [A] and descendant [C]
    assert_eq!(b_h.ancestor_ids, vec![a.to_string()]);
    assert_eq!(b_h.descendant_ids, vec![c.to_string()]);
}

/// WHAT: Unknown item ID returns empty hierarchy
/// WHY: Ensures graceful handling when item is not in the list
#[test]
fn given_tree_when_computing_for_unknown_item_then_empty_hierarchy() {
    // Given: A -> B
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let items = vec![test_item(a, None), test_item(b, Some(a))];

    // When: Computing for unknown ID
    let result = compute_hierarchy_for_item(&items, Uuid::new_v4());

    // Then: Empty hierarchy
    assert!(result.ancestor_ids.is_empty());
    assert!(result.descendant_ids.is_empty());
}
