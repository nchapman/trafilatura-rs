// Port of go-trafilatura/internal/selector/selector.go

pub(crate) mod comments;
pub(crate) mod content;
pub(crate) mod discard;
pub(crate) mod metadata;
pub(crate) mod utils;

use crate::dom::{Document, NodeId};

/// A selector rule: a predicate that tests a single element node.
///
/// Port of Go's `type Rule func(*html.Node) bool`.
pub(crate) type Rule = fn(&Document, NodeId) -> bool;

/// Returns the first element in the subtree of `root` that satisfies any rule,
/// searching in document order.
///
/// Port of `Query`. Note: Go's `Query` takes a single `Rule`; in Rust we accept a
/// slice and OR them together here. This matches the behavior at Go call sites where
/// callers pass rule slices (e.g. `selector.Content`) as a composite predicate.
pub(crate) fn query(doc: &Document, root: NodeId, rules: &[Rule]) -> Option<NodeId> {
    for id in doc.get_elements_by_tag_name(root, "*") {
        for rule in rules {
            if rule(doc, id) {
                return Some(id);
            }
        }
    }
    None
}

/// Returns all elements in the subtree of `root` that satisfy any rule,
/// in document order.
///
/// Port of `QueryAll`.
pub(crate) fn query_all(doc: &Document, root: NodeId, rules: &[Rule]) -> Vec<NodeId> {
    let mut matches = Vec::new();
    for id in doc.get_elements_by_tag_name(root, "*") {
        for rule in rules {
            if rule(doc, id) {
                matches.push(id);
                break;
            }
        }
    }
    matches
}
