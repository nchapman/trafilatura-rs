// Port of go-trafilatura/internal/etree/

mod tree;
mod query;

pub use ego_tree::NodeId;
use scraper::Node;

/// A parsed HTML document backed by an ego-tree.
///
/// Wraps `ego_tree::Tree<scraper::Node>` and adds text/tail semantics
/// (from Python's ElementTree / Go's etree package), mutable tree operations,
/// and CSS selector queries.
///
/// NodeId handles are stable as long as the node is not detached.
/// Detached nodes remain in the backing store (they're never freed), so
/// previously captured NodeIds remain valid but the node has no parent.
pub struct Document {
    pub(crate) tree: ego_tree::Tree<Node>,
}

impl Document {
    /// Parse an HTML string into a mutable Document.
    /// Uses html5ever's browser-grade HTML parser (same as scraper).
    pub fn parse(html: &str) -> Self {
        let html_doc = scraper::Html::parse_document(html);
        Self { tree: html_doc.tree }
    }

    /// Return the NodeId of the tree root (a Document node, not an Element).
    pub fn root(&self) -> NodeId {
        self.tree.root().id()
    }

    /// Return the NodeId of the `<body>` element, if present.
    pub fn body(&self) -> Option<NodeId> {
        // Walk: root → html → body
        let html_id = self.tree.root()
            .children()
            .find(|n| matches!(n.value(), Node::Element(e) if e.name.local.as_ref() == "html"))?
            .id();

        self.tree.get(html_id)?
            .children()
            .find(|n| matches!(n.value(), Node::Element(e) if e.name.local.as_ref() == "body"))
            .map(|n| n.id())
    }
}
