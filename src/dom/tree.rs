// Port of go-trafilatura/internal/etree/element.go and etree.go

use ego_tree::NodeId;
use html5ever::{LocalName, QualName, namespace_url, ns};
use scraper::Node;
use scraper::node::{Element, Text};
use tendril::StrTendril;

use super::Document;

/// HTML void elements — those that cannot have children.
const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input",
    "link", "meta", "param", "source", "track", "wbr",
];

// ---------------------------------------------------------------------------
// Helpers for creating node values
// ---------------------------------------------------------------------------

/// Create an Element node value with the given tag name and no attributes.
fn make_element(tag: &str) -> Node {
    Node::Element(Element::new(
        QualName::new(None, ns!(html), LocalName::from(tag)),
        vec![],
    ))
}

/// Create a Text node value.
fn make_text(text: &str) -> Node {
    Node::Text(Text { text: StrTendril::from(text) })
}

// ---------------------------------------------------------------------------
// Element type & attribute access
// ---------------------------------------------------------------------------

impl Document {
    /// Returns true if the node is an Element node.
    pub fn is_element(&self, id: NodeId) -> bool {
        matches!(self.tree.get(id).map(|n| n.value()), Some(Node::Element(_)))
    }

    /// Returns true if the node is a Text node.
    pub fn is_text(&self, id: NodeId) -> bool {
        matches!(self.tree.get(id).map(|n| n.value()), Some(Node::Text(_)))
    }

    /// Returns true if the element is a void element (cannot have children).
    pub fn is_void_element(&self, id: NodeId) -> bool {
        if let Some(Node::Element(e)) = self.tree.get(id).map(|n| n.value()) {
            return VOID_ELEMENTS.contains(&e.name.local.as_ref());
        }
        false
    }

    /// Returns the tag name of an element node (e.g. "p", "div"). Empty for non-elements.
    pub fn tag_name(&self, id: NodeId) -> &str {
        if let Some(Node::Element(e)) = self.tree.get(id).map(|n| n.value()) {
            return e.name.local.as_ref();
        }
        ""
    }

    /// Renames an element's tag in-place.
    ///
    /// Port of Go's `element.Data = "p"` pattern.
    pub fn set_tag_name(&mut self, id: NodeId, tag: &str) {
        if let Some(mut node_mut) = self.tree.get_mut(id) {
            if let Node::Element(e) = node_mut.value() {
                e.name.local = LocalName::from(tag);
                e.name.ns = ns!(html);
            }
        }
    }

    /// Returns the element's `id` attribute value, or empty string.
    pub fn id_attr(&self, id: NodeId) -> String {
        self.get_attribute(id, "id").unwrap_or_default()
    }

    /// Returns the element's `class` attribute value, or empty string.
    pub fn class_name(&self, id: NodeId) -> String {
        self.get_attribute(id, "class").unwrap_or_default()
    }

    /// Returns the value of a named attribute, or None if not present.
    pub fn get_attribute(&self, id: NodeId, name: &str) -> Option<String> {
        if let Some(Node::Element(e)) = self.tree.get(id).map(|n| n.value()) {
            return e.attr(name).map(|s| s.to_string());
        }
        None
    }

    /// Sets (or replaces) an attribute on an element.
    pub fn set_attribute(&mut self, id: NodeId, name: &str, value: &str) {
        if let Some(mut node_mut) = self.tree.get_mut(id) {
            if let Node::Element(e) = node_mut.value() {
                // Update existing or push new entry (attrs is Vec<(QualName, StrTendril)>).
                let local = LocalName::from(name);
                let found = e.attrs.iter_mut()
                    .find(|(k, _)| k.local == local);
                if let Some(entry) = found {
                    entry.1 = StrTendril::from(value);
                } else {
                    let key = QualName::new(None, ns!(), local);
                    e.attrs.push((key, StrTendril::from(value)));
                }
            }
        }
    }

    /// Removes an attribute from an element.
    pub fn remove_attribute(&mut self, id: NodeId, name: &str) {
        if let Some(mut node_mut) = self.tree.get_mut(id) {
            if let Node::Element(e) = node_mut.value() {
                let local = LocalName::from(name);
                e.attrs.retain(|(k, _)| k.local != local);
            }
        }
    }

    /// Removes all attributes from an element.
    pub fn clear_attributes(&mut self, id: NodeId) {
        if let Some(mut node_mut) = self.tree.get_mut(id) {
            if let Node::Element(e) = node_mut.value() {
                e.attrs.clear();
            }
        }
    }

    /// Returns all attribute names for an element.
    pub fn attribute_names(&self, id: NodeId) -> Vec<String> {
        if let Some(Node::Element(e)) = self.tree.get(id).map(|n| n.value()) {
            return e.attrs.iter().map(|(k, _)| k.local.as_ref().to_string()).collect();
        }
        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

impl Document {
    /// Returns the parent NodeId, if any.
    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.tree.get(id)?.parent().map(|n| n.id())
    }

    /// Returns the element children (skipping text/comment nodes).
    pub fn children(&self, id: NodeId) -> Vec<NodeId> {
        self.tree.get(id)
            .map(|n| n.children()
                .filter(|c| matches!(c.value(), Node::Element(_)))
                .map(|c| c.id())
                .collect())
            .unwrap_or_default()
    }

    /// Returns all children including text and comment nodes.
    pub fn child_nodes(&self, id: NodeId) -> Vec<NodeId> {
        self.tree.get(id)
            .map(|n| n.children().map(|c| c.id()).collect())
            .unwrap_or_default()
    }

    /// Returns the next sibling (any kind of node), if any.
    pub fn next_sibling(&self, id: NodeId) -> Option<NodeId> {
        self.tree.get(id)?.next_sibling().map(|n| n.id())
    }

    /// Returns the previous element sibling (skipping text nodes).
    pub fn prev_element_sibling(&self, id: NodeId) -> Option<NodeId> {
        let mut cur = self.tree.get(id)?.prev_sibling()?;
        loop {
            if matches!(cur.value(), Node::Element(_)) {
                return Some(cur.id());
            }
            cur = cur.prev_sibling()?;
        }
    }

    /// Returns the next element sibling (skipping text nodes).
    pub fn next_element_sibling(&self, id: NodeId) -> Option<NodeId> {
        let mut cur = self.tree.get(id)?.next_sibling()?;
        loop {
            if matches!(cur.value(), Node::Element(_)) {
                return Some(cur.id());
            }
            cur = cur.next_sibling()?;
        }
    }
}

// ---------------------------------------------------------------------------
// Text / Tail concept (port of etree/element.go)
// ---------------------------------------------------------------------------

impl Document {
    /// Returns text content before the first child *element*.
    ///
    /// Port of `etree.Text`.
    pub fn text(&self, id: NodeId) -> String {
        let Some(node) = self.tree.get(id) else { return String::new() };
        let mut out = String::new();
        for child in node.children() {
            match child.value() {
                Node::Element(_) => break,
                Node::Text(t) => out.push_str(t.as_ref()),
                _ => {}
            }
        }
        out
    }

    /// Replaces the text before the first child element.
    ///
    /// Port of `etree.SetText`.
    pub fn set_text(&mut self, id: NodeId, text: &str) {
        if self.is_void_element(id) { return; }

        // Remove existing leading text nodes (up to first element child).
        let to_remove: Vec<NodeId> = {
            let Some(node) = self.tree.get(id) else { return };
            node.children()
                .take_while(|c| !matches!(c.value(), Node::Element(_)))
                .filter(|c| matches!(c.value(), Node::Text(_)))
                .map(|c| c.id())
                .collect()
        };
        for tid in to_remove {
            self.tree.get_mut(tid).unwrap().detach();
        }

        // Insert new text node at the front.
        if !text.is_empty() {
            let new_node_val = make_text(text);
            let first_child = self.tree.get(id).unwrap().children().next().map(|c| c.id());
            if let Some(first_id) = first_child {
                self.tree.get_mut(first_id).unwrap().insert_before(new_node_val);
            } else {
                self.tree.get_mut(id).unwrap().append(new_node_val);
            }
        }
    }

    /// Returns the tail text: text nodes that come after this element (before
    /// the next sibling element).
    ///
    /// Port of `etree.Tail`.
    pub fn tail(&self, id: NodeId) -> String {
        let mut out = String::new();
        for tail_id in self.tail_nodes(id) {
            if let Some(Node::Text(t)) = self.tree.get(tail_id).map(|n| n.value()) {
                out.push_str(t.as_ref());
            }
        }
        out
    }

    /// Returns the NodeIds of the tail text nodes (text siblings immediately
    /// after this element).
    ///
    /// Port of `etree.TailNodes`.
    pub fn tail_nodes(&self, id: NodeId) -> Vec<NodeId> {
        let Some(node) = self.tree.get(id) else { return Vec::new() };
        let mut tails = Vec::new();
        let mut cur = node.next_sibling();
        while let Some(sib) = cur {
            match sib.value() {
                Node::Element(_) => break,
                Node::Text(_) => tails.push(sib.id()),
                _ => {}
            }
            cur = sib.next_sibling();
        }
        tails
    }

    /// Replaces the tail text of an element.
    ///
    /// Port of `etree.SetTail`.
    pub fn set_tail(&mut self, id: NodeId, tail: &str) {
        // Parent must exist and not be void.
        let parent_id = match self.parent(id) {
            Some(p) if !self.is_void_element(p) => p,
            _ => return,
        };

        // Remove existing tail text nodes.
        let old_tails = self.tail_nodes(id);
        for tid in old_tails {
            self.tree.get_mut(tid).unwrap().detach();
        }

        if tail.is_empty() { return; }

        // Insert new tail node.
        let new_tail = make_text(tail);
        let next_sib = self.tree.get(id).unwrap().next_sibling().map(|n| n.id());
        if let Some(next_id) = next_sib {
            self.tree.get_mut(next_id).unwrap().insert_before(new_tail);
        } else {
            self.tree.get_mut(parent_id).unwrap().append(new_tail);
        }
    }
}

// ---------------------------------------------------------------------------
// Tree construction
// ---------------------------------------------------------------------------

impl Document {
    /// Creates a new orphan element node (not yet attached to the tree).
    pub fn create_element(&mut self, tag: &str) -> NodeId {
        self.tree.orphan(make_element(tag)).id()
    }

    /// Creates a new orphan text node.
    pub fn create_text_node(&mut self, text: &str) -> NodeId {
        self.tree.orphan(make_text(text)).id()
    }

    /// Creates an element and appends it as a child of `parent`.
    ///
    /// Port of `etree.SubElement`.
    pub fn sub_element(&mut self, parent: NodeId, tag: &str) -> NodeId {
        self.tree.get_mut(parent).unwrap().append(make_element(tag)).id()
    }

    /// Appends `child` (and its tail text nodes) to `parent` by cloning.
    ///
    /// Port of `etree.Append` — moves element + tail into new parent.
    pub fn append_child(&mut self, parent: NodeId, child: NodeId) {
        // Collect tail text before mutating.
        let tail_texts: Vec<String> = self.tail_nodes(child).iter()
            .filter_map(|&tid| {
                if let Some(Node::Text(t)) = self.tree.get(tid).map(|n| n.value()) {
                    Some((**t).to_string())
                } else {
                    None
                }
            })
            .collect();

        let old_tail_ids = self.tail_nodes(child);

        // Clone child subtree into parent.
        self.clone_subtree_into(child, parent);

        // Append cloned tail text nodes.
        for text in &tail_texts {
            self.tree.get_mut(parent).unwrap().append(make_text(text));
        }

        // Detach original tail nodes and original child.
        for tid in old_tail_ids {
            self.tree.get_mut(tid).unwrap().detach();
        }
        self.tree.get_mut(child).unwrap().detach();
    }

    /// Appends multiple children into `parent`.
    ///
    /// Port of `etree.Extend`.
    pub fn extend(&mut self, parent: NodeId, children: &[NodeId]) {
        let children: Vec<NodeId> = children.to_vec();
        for child in children {
            self.append_child(parent, child);
        }
    }
}

// ---------------------------------------------------------------------------
// Tree removal
// ---------------------------------------------------------------------------

impl Document {
    /// Removes an element from the tree, optionally preserving its tail text.
    ///
    /// Port of `etree.Remove`.
    pub fn remove(&mut self, id: NodeId, keep_tail: bool) {
        if self.tree.get(id).is_none() { return; }

        if !keep_tail {
            let tails = self.tail_nodes(id);
            for tid in tails {
                self.tree.get_mut(tid).unwrap().detach();
            }
        }
        self.tree.get_mut(id).unwrap().detach();
    }

    /// Removes an element but preserves its children (merged into parent) and
    /// its tail text.
    ///
    /// Port of `etree.Strip`.
    pub fn strip(&mut self, id: NodeId) {
        if self.parent(id).is_none() { return; }

        let child_ids: Vec<NodeId> = self.child_nodes(id);
        let next_sib = self.tree.get(id).unwrap().next_sibling().map(|n| n.id());

        // Clone each child and insert before `id`'s position.
        for child_id in child_ids {
            if let Some(ns_id) = next_sib {
                self.clone_subtree_before(child_id, ns_id);
            } else {
                let parent = self.parent(id).unwrap();
                self.clone_subtree_into(child_id, parent);
            }
        }

        // Detach the stripped element (leave tail nodes in place).
        self.tree.get_mut(id).unwrap().detach();
    }

    /// Removes elements with any of the given tags but keeps their text/children.
    ///
    /// Port of `etree.StripTags`. Processes in reverse to avoid parent-before-child issues.
    pub fn strip_tags(&mut self, root: NodeId, tags: &[&str]) {
        for &tag in tags {
            let to_strip: Vec<NodeId> = self.get_elements_by_tag_name(root, tag);
            for id in to_strip.into_iter().rev() {
                self.strip(id);
            }
        }
    }

    /// Removes elements with any of the given tags *and* all their children.
    ///
    /// Port of `etree.StripElements`.
    pub fn strip_elements(&mut self, root: NodeId, keep_tail: bool, tags: &[&str]) {
        for &tag in tags {
            let to_remove: Vec<NodeId> = self.get_elements_by_tag_name(root, tag);
            for id in to_remove.into_iter().rev() {
                self.remove(id, keep_tail);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Cloning helpers (pub(crate) for use across the dom module)
// ---------------------------------------------------------------------------

impl Document {
    /// Deep-clone `source` into `target_parent` as a new last child.
    pub(crate) fn clone_subtree_into(&mut self, source: NodeId, target_parent: NodeId) -> NodeId {
        let val = self.tree.get(source).unwrap().value().clone();
        let new_id = self.tree.get_mut(target_parent).unwrap().append(val).id();

        let children: Vec<NodeId> = self.tree.get(source).unwrap()
            .children().map(|c| c.id()).collect();
        for child_id in children {
            self.clone_subtree_into(child_id, new_id);
        }
        new_id
    }

    /// Deep-clone `source` and insert it immediately before `target_sibling`.
    pub(crate) fn clone_subtree_before(&mut self, source: NodeId, target_sibling: NodeId) -> NodeId {
        let val = self.tree.get(source).unwrap().value().clone();
        let new_id = self.tree.get_mut(target_sibling).unwrap().insert_before(val).id();

        let children: Vec<NodeId> = self.tree.get(source).unwrap()
            .children().map(|c| c.id()).collect();
        for child_id in children {
            self.clone_subtree_into(child_id, new_id);
        }
        new_id
    }

    /// Deep-clone source from a *different* tree into target_parent in self.
    pub(crate) fn clone_from_tree(
        &mut self,
        source_tree: &ego_tree::Tree<Node>,
        source_id: NodeId,
        target_parent: NodeId,
    ) -> NodeId {
        let val = source_tree.get(source_id).unwrap().value().clone();
        let new_id = self.tree.get_mut(target_parent).unwrap().append(val).id();

        let children: Vec<NodeId> = source_tree.get(source_id).unwrap()
            .children().map(|c| c.id()).collect();
        for child_id in children {
            self.clone_from_tree(source_tree, child_id, new_id);
        }
        new_id
    }

    /// Creates a fully independent clone of this document.
    ///
    /// Port of `dom.Clone(doc, true)`.
    pub fn clone_document(&self) -> Document {
        Document { tree: self.tree.clone() }
    }
}

// ---------------------------------------------------------------------------
// Traversal (port of etree/element.go:31-92)
// ---------------------------------------------------------------------------

impl Document {
    /// Returns the element itself and all descendant elements with matching tags,
    /// in document order. If `tags` is empty, returns all elements.
    ///
    /// Port of `etree.Iter`.
    pub fn iter(&self, id: NodeId, tags: &[&str]) -> Vec<NodeId> {
        let mut result = Vec::new();
        self.collect_iter(id, tags, true, &mut result);
        result
    }

    /// Like `iter` but excludes the starting node itself.
    ///
    /// Port of `etree.IterDescendants`.
    pub fn iter_descendants(&self, id: NodeId, tags: &[&str]) -> Vec<NodeId> {
        let mut result = Vec::new();
        self.collect_iter(id, tags, false, &mut result);
        result
    }

    fn collect_iter(&self, id: NodeId, tags: &[&str], include_self: bool, out: &mut Vec<NodeId>) {
        let Some(node) = self.tree.get(id) else { return };
        let tag = match node.value() {
            Node::Element(e) => e.name.local.as_ref(),
            _ => return,
        };

        if include_self && (tags.is_empty() || tags.contains(&tag)) {
            out.push(id);
        }

        for child_id in node.children().map(|c| c.id()).collect::<Vec<_>>() {
            self.collect_iter(child_id, tags, true, out);
        }
    }

    /// Collects inner text with whitespace separators when the nesting level changes.
    ///
    /// Port of `etree.IterText`.
    pub fn iter_text(&self, id: NodeId, separator: &str) -> String {
        let mut buffer = String::new();
        let mut last_level: usize = 0;
        self.iter_text_inner(id, 0, separator, &mut buffer, &mut last_level);
        buffer.trim().to_string()
    }

    fn iter_text_inner(
        &self,
        id: NodeId,
        level: usize,
        sep: &str,
        buf: &mut String,
        last_level: &mut usize,
    ) {
        let Some(node) = self.tree.get(id) else { return };

        match node.value() {
            Node::Element(e) => {
                if VOID_ELEMENTS.contains(&e.name.local.as_ref()) {
                    buf.push_str(sep);
                }
            }
            Node::Text(t) => {
                if level != *last_level {
                    buf.push_str(sep);
                }
                buf.push_str(t.as_ref());
            }
            _ => {}
        }

        *last_level = level;

        let children: Vec<NodeId> = node.children().map(|c| c.id()).collect();
        for child_id in children {
            self.iter_text_inner(child_id, level + 1, sep, buf, last_level);
        }
    }

    /// Returns all text content within the subtree (no separators).
    ///
    /// Equivalent to `dom.TextContent` in Go.
    pub fn text_content(&self, id: NodeId) -> String {
        let mut result = String::new();
        self.collect_text(id, &mut result);
        result
    }

    fn collect_text(&self, id: NodeId, out: &mut String) {
        let Some(node) = self.tree.get(id) else { return };
        if let Node::Text(t) = node.value() {
            out.push_str(t.as_ref());
        }
        for child_id in node.children().map(|c| c.id()).collect::<Vec<_>>() {
            self.collect_text(child_id, out);
        }
    }
}

// ---------------------------------------------------------------------------
// Serialization
// ---------------------------------------------------------------------------

impl Document {
    /// Serializes an element to its outer HTML.
    pub fn outer_html(&self, id: NodeId) -> String {
        let Some(node_ref) = self.tree.get(id) else { return String::new() };

        // Handle text nodes (no wrapping element).
        if let Node::Text(t) = node_ref.value() {
            return (**t).to_string();
        }

        // For element nodes, use scraper's ElementRef serialization.
        if let Some(elem_ref) = scraper::ElementRef::wrap(node_ref) {
            return elem_ref.html();
        }
        String::new()
    }

    /// Serializes the inner HTML of an element (its children only).
    pub fn inner_html(&self, id: NodeId) -> String {
        let Some(node_ref) = self.tree.get(id) else { return String::new() };
        if let Some(elem_ref) = scraper::ElementRef::wrap(node_ref) {
            return elem_ref.inner_html();
        }
        String::new()
    }

    /// Serializes element + tail text nodes.
    ///
    /// Port of `etree.ToString`.
    pub fn to_string(&self, id: NodeId) -> String {
        let outer = self.outer_html(id);
        let tail = self.tail(id);
        format!("{}{}", outer, tail)
    }

    /// Parse an HTML string and return the first body child element in a new Document.
    ///
    /// Port of `etree.FromString`.
    pub fn from_string(html: &str) -> Option<(Document, NodeId)> {
        let doc = Document::parse(html);
        let body_id = doc.body()?;
        let first_child = doc.tree.get(body_id)?
            .children()
            .find(|n| matches!(n.value(), Node::Element(_)))?
            .id();
        Some((doc, first_child))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(html: &str) -> Document {
        Document::parse(html)
    }

    #[test]
    fn test_text_and_tail() {
        let d = doc("<p>Hello <b>world</b> and more</p>");
        let body = d.body().unwrap();
        let p = d.children(body)[0];
        let b = d.children(p)[0];

        assert_eq!(d.text(p), "Hello ");
        assert_eq!(d.text(b), "world");
        assert_eq!(d.tail(b), " and more");
    }

    #[test]
    fn test_text_no_children() {
        let d = doc("<p>Just text</p>");
        let body = d.body().unwrap();
        let p = d.children(body)[0];
        assert_eq!(d.text(p), "Just text");
        assert_eq!(d.tail(p), "");
    }

    #[test]
    fn test_set_text() {
        let mut d = doc("<p>Hello <b>world</b></p>");
        let body = d.body().unwrap();
        let p = d.children(body)[0];
        d.set_text(p, "Replaced ");
        assert_eq!(d.text(p), "Replaced ");
        // child <b> should still be there
        assert!(!d.children(p).is_empty());
        assert_eq!(d.text(d.children(p)[0]), "world");
    }

    #[test]
    fn test_set_tail() {
        let mut d = doc("<p>Hello <b>world</b> old tail</p>");
        let body = d.body().unwrap();
        let p = d.children(body)[0];
        let b = d.children(p)[0];
        d.set_tail(b, " new tail");
        assert_eq!(d.tail(b), " new tail");
    }

    #[test]
    fn test_remove_without_keep_tail() {
        let mut d = doc("<div><p>Hello</p> tail text</div>");
        let body = d.body().unwrap();
        let div = d.children(body)[0];
        let p = d.children(div)[0];
        d.remove(p, false);
        // Both p and tail text should be gone.
        assert!(d.children(div).is_empty());
        assert_eq!(d.text_content(div), "");
    }

    #[test]
    fn test_remove_with_keep_tail() {
        let mut d = doc("<div><p>Hello</p> tail text</div>");
        let body = d.body().unwrap();
        let div = d.children(body)[0];
        let p = d.children(div)[0];
        d.remove(p, true);
        // p is gone but tail remains.
        assert!(d.children(div).is_empty());
        assert!(d.text_content(div).contains("tail text"));
    }

    #[test]
    fn test_strip() {
        let mut d = doc("<div><span>inner text</span></div>");
        let body = d.body().unwrap();
        let div = d.children(body)[0];
        let span = d.children(div)[0];
        d.strip(span);
        // span is gone, text "inner text" is now in div.
        assert!(d.children(div).is_empty());
        assert!(d.text_content(div).contains("inner text"));
    }

    #[test]
    fn test_strip_tags() {
        let mut d = doc("<p>Hello <b>world</b> and <i>more</i></p>");
        let body = d.body().unwrap();
        let p = d.children(body)[0];
        d.strip_tags(p, &["b", "i"]);
        let text = d.text_content(p);
        assert!(text.contains("Hello"));
        assert!(text.contains("world"));
        assert!(text.contains("more"));
    }

    #[test]
    fn test_strip_elements() {
        let mut d = doc("<div><p>Keep</p><script>Remove</script></div>");
        let body = d.body().unwrap();
        let div = d.children(body)[0];
        d.strip_elements(div, false, &["script"]);
        let children = d.children(div);
        assert_eq!(children.len(), 1);
        assert_eq!(d.tag_name(children[0]), "p");
    }

    #[test]
    fn test_set_tag_name() {
        let mut d = doc("<div><p>Hello</p></div>");
        let body = d.body().unwrap();
        let div = d.children(body)[0];
        let p = d.children(div)[0];
        d.set_tag_name(p, "done");
        assert_eq!(d.tag_name(p), "done");
    }

    #[test]
    fn test_iter() {
        let d = doc("<div><p>one</p><p>two</p><span>three</span></div>");
        let body = d.body().unwrap();
        let div = d.children(body)[0];

        let ps = d.iter(div, &["p"]);
        assert_eq!(ps.len(), 2);

        let all = d.iter(div, &[]);
        // div + p + p + span = 4
        assert_eq!(all.len(), 4);
    }

    #[test]
    fn test_iter_descendants_excludes_self() {
        let d = doc("<div><p>one</p></div>");
        let body = d.body().unwrap();
        let div = d.children(body)[0];

        let all = d.iter_descendants(div, &[]);
        assert_eq!(all.len(), 1);
        assert_eq!(d.tag_name(all[0]), "p");
    }

    #[test]
    fn test_iter_text() {
        let d = doc("<p>Hello <b>world</b> end</p>");
        let body = d.body().unwrap();
        let p = d.children(body)[0];
        let text = d.iter_text(p, " ");
        assert!(text.contains("Hello"));
        assert!(text.contains("world"));
        assert!(text.contains("end"));
    }

    #[test]
    fn test_text_content() {
        let d = doc("<p>Hello <b>world</b> end</p>");
        let body = d.body().unwrap();
        let p = d.children(body)[0];
        let text = d.text_content(p);
        assert_eq!(text, "Hello world end");
    }

    #[test]
    fn test_clone_document() {
        let d = doc("<p>Hello</p>");
        let d2 = d.clone_document();
        let body1 = d.body().unwrap();
        let body2 = d2.body().unwrap();
        assert_eq!(d.text_content(body1), d2.text_content(body2));
    }

    #[test]
    fn test_clone_independence() {
        let d = doc("<p>Hello</p>");
        let mut d2 = d.clone_document();
        let body2 = d2.body().unwrap();
        let p2 = d2.children(body2)[0];
        d2.set_text(p2, "Changed");
        // Original document should not be affected.
        let body1 = d.body().unwrap();
        let p1 = d.children(body1)[0];
        assert_eq!(d.text(p1), "Hello");
    }

    #[test]
    fn test_get_set_attribute() {
        let mut d = doc("<p>text</p>");
        let body = d.body().unwrap();
        let p = d.children(body)[0];
        d.set_attribute(p, "class", "highlight");
        assert_eq!(d.get_attribute(p, "class"), Some("highlight".to_string()));
        assert_eq!(d.class_name(p), "highlight");
        d.remove_attribute(p, "class");
        assert_eq!(d.get_attribute(p, "class"), None);
    }

    #[test]
    fn test_sub_element() {
        let mut d = doc("<div></div>");
        let body = d.body().unwrap();
        let div = d.children(body)[0];
        let span = d.sub_element(div, "span");
        assert_eq!(d.tag_name(span), "span");
        assert_eq!(d.children(div).len(), 1);
    }

    #[test]
    fn test_is_void_element() {
        let d = doc("<p><br/><img src='x.png'/></p>");
        let body = d.body().unwrap();
        let p = d.children(body)[0];
        let children = d.children(p);
        assert!(children.iter().any(|&c| d.is_void_element(c)));
        assert!(!d.is_void_element(p));
    }

    #[test]
    fn test_from_string() {
        let (d, first) = Document::from_string("<p>Hello world</p>").unwrap();
        assert_eq!(d.tag_name(first), "p");
        assert_eq!(d.text_content(first), "Hello world");
    }

    #[test]
    fn test_outer_html() {
        let d = doc("<p class=\"hi\">text</p>");
        let body = d.body().unwrap();
        let p = d.children(body)[0];
        let html = d.outer_html(p);
        assert!(html.contains("<p"));
        assert!(html.contains("text"));
        assert!(html.contains("</p>"));
    }

    #[test]
    fn test_navigation() {
        let d = doc("<div><p>first</p><span>second</span></div>");
        let body = d.body().unwrap();
        let div = d.children(body)[0];
        let children = d.children(div);
        let p = children[0];
        let span = children[1];

        assert_eq!(d.next_element_sibling(p), Some(span));
        assert_eq!(d.prev_element_sibling(span), Some(p));
        assert_eq!(d.parent(p), Some(div));
    }
}
