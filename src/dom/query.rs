// Port of CSS selector queries (dom.QuerySelector, dom.QuerySelectorAll, dom.GetElementsByTagName)

use ego_tree::NodeId;
use scraper::{Node, Selector};

use super::Document;

/// Maximum recursion depth for query traversals.
const MAX_TREE_DEPTH: usize = 500;

impl Document {
    /// Find the first element (in document order) within the subtree rooted at
    /// `root` that matches the given CSS selector string.
    ///
    /// Returns `None` if no match found or if the selector is invalid.
    ///
    /// Port of `dom.QuerySelector`.
    pub fn query_selector(&self, root: NodeId, selector: &str) -> Option<NodeId> {
        let sel = Selector::parse(selector).ok()?;
        self.query_first_compiled(root, &sel)
    }

    /// Find all elements within the subtree rooted at `root` that match the
    /// given CSS selector string, in document order.
    ///
    /// Port of `dom.QuerySelectorAll`.
    pub fn query_selector_all(&self, root: NodeId, selector: &str) -> Vec<NodeId> {
        let Ok(sel) = Selector::parse(selector) else {
            return Vec::new();
        };
        self.query_selector_all_compiled(root, &sel)
    }

    /// Find the first element matching a pre-compiled selector (early termination).
    pub(crate) fn query_first_compiled(&self, root: NodeId, sel: &Selector) -> Option<NodeId> {
        self.find_first_matching(root, sel, 0)
    }

    /// Find all elements matching a pre-compiled selector. Exposed for hot-path callers
    /// in the selector module that reuse compiled selectors across documents.
    pub(crate) fn query_selector_all_compiled(&self, root: NodeId, sel: &Selector) -> Vec<NodeId> {
        let mut result = Vec::new();
        self.collect_matching(root, sel, &mut result, 0);
        result
    }

    fn find_first_matching(&self, id: NodeId, sel: &Selector, depth: usize) -> Option<NodeId> {
        if depth >= MAX_TREE_DEPTH {
            return None;
        }
        let node_ref = self.tree.get(id)?;
        if let Some(elem_ref) = scraper::ElementRef::wrap(node_ref) {
            if sel.matches(&elem_ref) {
                return Some(id);
            }
        }
        for child in node_ref.children() {
            if let Some(found) = self.find_first_matching(child.id(), sel, depth + 1) {
                return Some(found);
            }
        }
        None
    }

    fn collect_matching(&self, id: NodeId, sel: &Selector, out: &mut Vec<NodeId>, depth: usize) {
        if depth >= MAX_TREE_DEPTH {
            return;
        }
        let Some(node_ref) = self.tree.get(id) else {
            return;
        };
        if let Some(elem_ref) = scraper::ElementRef::wrap(node_ref) {
            if sel.matches(&elem_ref) {
                out.push(id);
            }
        }
        for child in node_ref.children() {
            self.collect_matching(child.id(), sel, out, depth + 1);
        }
    }

    /// Returns all descendant elements with a given tag name, in document order.
    ///
    /// Pass `"*"` to get all elements.
    ///
    /// Port of `dom.GetElementsByTagName`.
    pub fn get_elements_by_tag_name(&self, root: NodeId, tag: &str) -> Vec<NodeId> {
        let mut result = Vec::new();
        self.collect_by_tag(root, tag, false, &mut result, 0);
        result
    }

    fn collect_by_tag(
        &self,
        id: NodeId,
        tag: &str,
        include_self: bool,
        out: &mut Vec<NodeId>,
        depth: usize,
    ) {
        if depth >= MAX_TREE_DEPTH {
            return;
        }
        let Some(node) = self.tree.get(id) else {
            return;
        };

        if let Node::Element(e) = node.value() {
            let matches = tag == "*" || e.name.local.as_ref() == tag;
            if include_self && matches {
                out.push(id);
            }
        }

        for child in node.children() {
            self.collect_by_tag(child.id(), tag, true, out, depth + 1);
        }
    }

    /// Replaces the inner content of an element by parsing `html` as a fragment
    /// and appending the resulting nodes.
    ///
    /// Used in baseline extraction to parse `articleBody` HTML into a subtree.
    pub fn set_inner_html(&mut self, id: NodeId, html: &str) {
        // Remove all existing children.
        let children: Vec<NodeId> = self.child_nodes(id);
        for child_id in children {
            self.tree
                .get_mut(child_id)
                .expect("child NodeId from same tree")
                .detach();
        }

        // Parse fragment as a full document and transfer body children.
        let fragment = Document::parse(html);
        let Some(frag_body) = fragment.body() else {
            return;
        };

        let frag_children: Vec<NodeId> = fragment
            .tree
            .get(frag_body)
            .map(|n| n.children().map(|c| c.id()).collect())
            .unwrap_or_default();

        for child_id in frag_children {
            self.clone_from_tree(&fragment.tree, child_id, id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(html: &str) -> Document {
        Document::parse(html)
    }

    #[test]
    fn test_query_selector() {
        let d = doc("<div><p class=\"target\">text</p><p>other</p></div>");
        let body = d.body().unwrap();
        let result = d.query_selector(body, "p.target");
        assert!(result.is_some());
        assert_eq!(d.tag_name(result.unwrap()), "p");
    }

    #[test]
    fn test_query_selector_all() {
        let d = doc("<div><p>one</p><p>two</p><span>three</span></div>");
        let body = d.body().unwrap();
        let results = d.query_selector_all(body, "p");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_get_elements_by_tag_name() {
        let d = doc("<div><p>one</p><p>two</p><span>three</span></div>");
        let body = d.body().unwrap();
        let div = d.children(body)[0];
        let ps = d.get_elements_by_tag_name(div, "p");
        assert_eq!(ps.len(), 2);
    }

    #[test]
    fn test_get_elements_by_tag_name_wildcard() {
        let d = doc("<div><p>one</p><span>two</span></div>");
        let body = d.body().unwrap();
        let div = d.children(body)[0];
        let all = d.get_elements_by_tag_name(div, "*");
        // p + span = 2
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_set_inner_html() {
        let mut d = doc("<div><p>old</p></div>");
        let body = d.body().unwrap();
        let div = d.children(body)[0];
        d.set_inner_html(div, "<span>new</span>");
        let children = d.children(div);
        assert_eq!(children.len(), 1);
        assert_eq!(d.tag_name(children[0]), "span");
        assert_eq!(d.text_content(children[0]), "new");
    }

    #[test]
    fn test_query_selector_nested() {
        let d = doc("<article><div><p id=\"main\">content</p></div></article>");
        let body = d.body().unwrap();
        let result = d.query_selector(body, "article p");
        assert!(result.is_some());
        assert_eq!(d.id_attr(result.unwrap()), "main");
    }
}
