//! Hierarchy computation for work item ancestor/descendant resolution.
//!
//! Provides bulk and single-item hierarchy computation used by
//! response builders and broadcast handlers to populate the
//! ancestor_ids and descendant_ids proto fields.

use std::collections::{HashMap, HashSet, VecDeque};

use pm_core::WorkItem;
use uuid::Uuid;

/// Pre-computed hierarchy data for a single work item.
/// Uses Vec<String> (not Vec<Uuid>) because proto fields are strings.
pub struct HierarchyData {
    /// UUIDs of all ancestors, from immediate parent up to root.
    pub ancestor_ids: Vec<String>,
    /// UUIDs of all descendants: children, grandchildren, etc.
    pub descendant_ids: Vec<String>,
}

/// Compute hierarchy data for ALL items in a project.
///
/// Algorithm:
/// 1. Build parent-to-children and child-to-parent indices in O(N)
/// 2. For each item, walk UP the parent chain to collect ancestors
/// 3. For each item, BFS DOWN from children to collect descendants
/// 4. Cycle guards (visited sets) prevent infinite loops on malformed data
///
/// For a typical 3-level hierarchy (Epic -> Story -> Task), depth
/// traversal is bounded by 3, making effective complexity O(N).
pub fn compute_hierarchy_maps(items: &[WorkItem]) -> HashMap<Uuid, HierarchyData> {
    if items.is_empty() {
        return HashMap::new();
    }

    // Phase 1: Build adjacency indices in O(N)
    let mut children_of: HashMap<Uuid, Vec<Uuid>> = HashMap::with_capacity(items.len());
    let mut parent_of: HashMap<Uuid, Uuid> = HashMap::with_capacity(items.len());

    for item in items {
        if let Some(pid) = item.parent_id {
            children_of.entry(pid).or_default().push(item.id);
            parent_of.insert(item.id, pid);
        }
    }

    // Phase 2: For each item, compute ancestors (walk up) and descendants (BFS down)
    let mut result = HashMap::with_capacity(items.len());

    for item in items {
        let ancestors = collect_ancestors(item.id, &parent_of);
        let descendants = collect_descendants(item.id, &children_of);

        result.insert(
            item.id,
            HierarchyData {
                ancestor_ids: ancestors,
                descendant_ids: descendants,
            },
        );
    }

    result
}

/// Compute hierarchy for a SINGLE item given all project items.
///
/// Used in create/update broadcast paths. Delegates to
/// compute_hierarchy_maps internally and extracts the entry
/// for the specified item, avoiding logic duplication.
pub fn compute_hierarchy_for_item(items: &[WorkItem], item_id: Uuid) -> HierarchyData {
    let mut maps = compute_hierarchy_maps(items);
    maps.remove(&item_id).unwrap_or_else(|| HierarchyData {
        ancestor_ids: Vec::new(),
        descendant_ids: Vec::new(),
    })
}

/// Walk up the parent chain to collect all ancestor IDs.
/// Uses a visited set to guard against cycles in malformed data.
fn collect_ancestors(item_id: Uuid, parent_of: &HashMap<Uuid, Uuid>) -> Vec<String> {
    let mut ancestors = Vec::new();
    let mut current = item_id;
    let mut visited = HashSet::new();
    visited.insert(current);

    while let Some(&pid) = parent_of.get(&current) {
        if !visited.insert(pid) {
            break; // Cycle detected — stop walking
        }
        ancestors.push(pid.to_string());
        current = pid;
    }

    ancestors
}

/// BFS down from an item to collect all descendant IDs.
/// Uses a visited set to guard against cycles in malformed data.
fn collect_descendants(item_id: Uuid, children_of: &HashMap<Uuid, Vec<Uuid>>) -> Vec<String> {
    let mut descendants = Vec::new();
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    visited.insert(item_id);

    // Seed queue with direct children
    if let Some(direct_children) = children_of.get(&item_id) {
        for &child_id in direct_children {
            queue.push_back(child_id);
        }
    }

    while let Some(child_id) = queue.pop_front() {
        if !visited.insert(child_id) {
            continue; // Already visited — cycle guard
        }
        descendants.push(child_id.to_string());
        if let Some(grandchildren) = children_of.get(&child_id) {
            for &gc_id in grandchildren {
                queue.push_back(gc_id);
            }
        }
    }

    descendants
}
