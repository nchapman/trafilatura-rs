// Port of go-trafilatura/internal/selector/utils.go

use crate::dom::{Document, NodeId};

/// Returns all ancestor elements of `id` that match `ancestor_tag`.
///
/// Port of `getNodeAncestors`.
pub fn get_node_ancestors(doc: &Document, id: NodeId, ancestor_tag: &str) -> Vec<NodeId> {
    let mut ancestors = Vec::new();
    let mut cur = id;
    while let Some(parent) = doc.parent(cur) {
        if doc.tag_name(parent) == ancestor_tag {
            ancestors.push(parent);
        }
        cur = parent;
    }
    ancestors
}

/// Port of Go's `contains(s, substr)`.
#[inline]
pub fn contains(s: &str, substr: &str) -> bool {
    s.contains(substr)
}

/// Port of Go's `startsWith(s, prefix)`.
#[inline]
pub fn starts_with(s: &str, prefix: &str) -> bool {
    s.starts_with(prefix)
}

/// Port of Go's `lower(s)`.
#[inline]
pub fn lower(s: &str) -> String {
    s.to_lowercase()
}
