// Port of go-trafilatura/internal/selector/comments.go

use crate::dom::{Document, NodeId};
use crate::selector::utils::{contains, starts_with};

/// Ordered list of comment section detection rules.
pub(crate) const COMMENTS: &[super::Rule] = &[
    comments_rule1,
    comments_rule2,
    comments_rule3,
    comments_rule4,
];

/// Matches comment list containers by id/class patterns.
///
/// Port of `commentsRule1`.
fn comments_rule1(doc: &Document, id: NodeId) -> bool {
    let tag = doc.tag_name(id);
    match tag {
        "div" | "ol" | "ul" | "dl" | "section" => {}
        _ => return false,
    }

    let class = doc.class_name(id);
    let elem_id = doc.id_attr(id);
    let id_class = format!("{elem_id}{class}");

    contains(&id_class, "commentlist")
        || contains(&class, "comment-page")
        || contains(&id_class, "comment-list")
        || contains(&class, "comments-content")
        || contains(&class, "post-comments")
}

/// Matches containers starting with "comments" or "comment-".
///
/// Port of `commentsRule2`.
fn comments_rule2(doc: &Document, id: NodeId) -> bool {
    let tag = doc.tag_name(id);
    match tag {
        "div" | "section" | "ol" | "ul" | "dl" => {}
        _ => return false,
    }

    let class = doc.class_name(id);
    let elem_id = doc.id_attr(id);
    let id_class = format!("{elem_id}{class}");

    starts_with(&id_class, "comments")
        || starts_with(&class, "Comments")
        || starts_with(&id_class, "comment-")
        || contains(&class, "article-comments")
}

/// Matches Disqus and similar comment thread containers.
///
/// Port of `commentsRule3`.
fn comments_rule3(doc: &Document, id: NodeId) -> bool {
    let tag = doc.tag_name(id);
    match tag {
        "div" | "section" | "ol" | "ul" | "dl" => {}
        _ => return false,
    }

    let elem_id = doc.id_attr(id);

    starts_with(&elem_id, "comol")
        || starts_with(&elem_id, "disqus_thread")
        || starts_with(&elem_id, "dsq_comments")
}

/// Matches social/comment sections by id prefix or class.
///
/// Port of `commentsRule4`.
fn comments_rule4(doc: &Document, id: NodeId) -> bool {
    let tag = doc.tag_name(id);
    match tag {
        "div" | "section" => {}
        _ => return false,
    }

    let class = doc.class_name(id);
    let elem_id = doc.id_attr(id);

    starts_with(&elem_id, "social") || contains(&class, "comment")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::Document;
    use crate::selector::query;

    fn parse(html: &str) -> Document {
        Document::parse(html)
    }

    #[test]
    fn test_comments_rule1_commentlist() {
        let doc = parse(r#"<html><body><div id="commentlist">comments</div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, COMMENTS).is_some());
    }

    #[test]
    fn test_comments_rule2_starts_with_comments() {
        let doc = parse(r#"<html><body><div id="comments-section">comments</div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, COMMENTS).is_some());
    }

    #[test]
    fn test_comments_rule3_disqus() {
        let doc = parse(r#"<html><body><div id="disqus_thread">comments</div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, COMMENTS).is_some());
    }

    #[test]
    fn test_comments_rule3_dsq_comments_underscore() {
        // Verify the underscore variant (dsq_comments), not dsq-comments with hyphen.
        let doc = parse(r#"<html><body><div id="dsq_comments">comments</div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, COMMENTS).is_some());
    }

    #[test]
    fn test_comments_rule4_comment_class() {
        let doc = parse(r#"<html><body><div class="user-comments">comments</div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, COMMENTS).is_some());
    }

    #[test]
    fn test_comments_no_match() {
        let doc = parse(r#"<html><body><div class="article-content">content</div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, COMMENTS).is_none());
    }
}
