// Port of go-trafilatura/internal/selector/content.go

use crate::dom::{Document, NodeId};
use crate::selector::utils::{contains, lower, starts_with};

/// Ordered list of content extraction rules.
/// Applied in sequence; the first match wins.
pub(crate) const CONTENT: &[super::Rule] = &[
    content_rule1,
    content_rule2,
    content_rule3,
    content_rule4,
    content_rule5,
];

/// Matches common article/post content containers by class/id patterns.
///
/// Port of `contentRule1`.
fn content_rule1(doc: &Document, id: NodeId) -> bool {
    let tag = doc.tag_name(id);
    match tag {
        "article" | "div" | "main" | "section" => {}
        _ => return false,
    }

    let class = doc.class_name(id);
    let elem_id = doc.id_attr(id);
    let item_prop = doc.get_attribute(id, "itemprop").unwrap_or_default();

    matches!(class.as_str(), "post" | "entry")
        || contains(&class, "post-text")
        || contains(&class, "post_text")
        || contains(&class, "post-body")
        || contains(&class, "post-entry")
        || contains(&class, "postentry")
        || contains(&class, "post-content")
        || contains(&class, "post_content")
        || contains(&lower(&class), "postcontent")
        || contains(&class, "post_inner_wrapper")
        || contains(&class, "article-text")
        || contains(&lower(&class), "articletext")
        || contains(&elem_id, "entry-content")
        || contains(&class, "entry-content")
        || contains(&elem_id, "article-content")
        || contains(&class, "article-content")
        || contains(&elem_id, "article__content")
        || contains(&class, "article__content")
        || contains(&elem_id, "article-body")
        || contains(&class, "article-body")
        || contains(&elem_id, "article__body")
        || contains(&class, "article__body")
        || item_prop == "articleBody"
        || contains(&lower(&elem_id), "articlebody")
        || contains(&lower(&class), "articlebody")
        || elem_id == "articleContent"
        || contains(&class, "ArticleContent")
        || contains(&class, "page-content")
        || contains(&class, "text-content")
        || contains(&elem_id, "body-text")
        || contains(&class, "body-text")
        || contains(&class, "article__container")
        || contains(&elem_id, "art-content")
        || contains(&class, "art-content")
}

/// Matches any `<article>` element.
///
/// Port of `contentRule2`.
fn content_rule2(doc: &Document, id: NodeId) -> bool {
    doc.tag_name(id) == "article"
}

/// Matches content containers by story/post/blog class patterns, plus `role="article"`.
///
/// Port of `contentRule3`.
fn content_rule3(doc: &Document, id: NodeId) -> bool {
    let tag = doc.tag_name(id);
    match tag {
        "article" | "div" | "main" | "section" => {}
        _ => return false,
    }

    let class = doc.class_name(id);
    let elem_id = doc.id_attr(id);
    let role = doc.get_attribute(id, "role").unwrap_or_default();

    contains(&class, "post-bodycopy")
        || contains(&class, "storycontent")
        || contains(&class, "story-content")
        || class == "postarea"
        || class == "art-postcontent"
        || contains(&class, "theme-content")
        || contains(&class, "blog-content")
        || contains(&class, "section-content")
        || contains(&class, "single-content")
        || contains(&class, "single-post")
        || contains(&class, "main-column")
        || contains(&class, "wpb_text_column")
        || starts_with(&elem_id, "primary")
        || starts_with(&class, "article")
        || class == "text"
        || elem_id == "article"
        || class == "cell"
        || elem_id == "story"
        || class == "story"
        || contains(&class, "story-body")
        || contains(&elem_id, "story-body")
        || contains(&class, "field-body")
        || contains(&lower(&class), "fulltext")
        || role == "article"
}

/// Matches content containers by content-main/content-body/main-content patterns.
///
/// Port of `contentRule4`.
fn content_rule4(doc: &Document, id: NodeId) -> bool {
    let tag = doc.tag_name(id);
    match tag {
        "article" | "div" | "main" | "section" => {}
        _ => return false,
    }

    let class = doc.class_name(id);
    let elem_id = doc.id_attr(id);

    contains(&elem_id, "content-main")
        || contains(&class, "content-main")
        || contains(&class, "content_main")
        || contains(&elem_id, "content-body")
        || contains(&class, "content-body")
        || contains(&elem_id, "contentBody")
        || contains(&class, "content__body")
        || contains(&lower(&elem_id), "main-content")
        || contains(&lower(&class), "main-content")
        || contains(&lower(&class), "page-content")
        || elem_id == "content"
        || class == "content"
}

/// Matches containers starting with "main" in class/id/role, plus `<main>` itself.
///
/// Port of `contentRule5`.
fn content_rule5(doc: &Document, id: NodeId) -> bool {
    let tag = doc.tag_name(id);
    match tag {
        "article" | "div" | "section" => {}
        "main" => return true,
        _ => return false,
    }

    let class = doc.class_name(id);
    let elem_id = doc.id_attr(id);
    let role = doc.get_attribute(id, "role").unwrap_or_default();

    starts_with(&class, "main") || starts_with(&elem_id, "main") || starts_with(&role, "main")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::Document;
    use crate::selector::{query, query_all};

    fn parse(html: &str) -> Document {
        Document::parse(html)
    }

    #[test]
    fn test_content_rule1_post_class() {
        let doc = parse(r#"<html><body><div class="post"><p>text</p></div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, CONTENT).is_some());
    }

    #[test]
    fn test_content_rule1_article_body() {
        let doc = parse(r#"<html><body><div class="article-body">text</div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, CONTENT).is_some());
    }

    #[test]
    fn test_content_rule2_article_tag() {
        let doc = parse(r#"<html><body><article>text</article></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, CONTENT).is_some());
    }

    #[test]
    fn test_content_rule3_story_content() {
        let doc = parse(r#"<html><body><div class="story-content">text</div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, CONTENT).is_some());
    }

    #[test]
    fn test_content_rule4_content_main() {
        let doc = parse(r#"<html><body><div id="content-main">text</div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, CONTENT).is_some());
    }

    #[test]
    fn test_content_rule5_main_tag() {
        let doc = parse(r#"<html><body><main>text</main></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, CONTENT).is_some());
    }

    #[test]
    fn test_content_rule5_main_class() {
        let doc = parse(r#"<html><body><div class="main-container">text</div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, CONTENT).is_some());
    }

    #[test]
    fn test_content_no_match() {
        let doc = parse(r#"<html><body><div class="footer">text</div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, CONTENT).is_none());
    }

    #[test]
    fn test_query_all_returns_multiple() {
        let doc =
            parse(r#"<html><body><article>one</article><article>two</article></body></html>"#);
        let body = doc.body().unwrap();
        assert_eq!(query_all(&doc, body, CONTENT).len(), 2);
    }
}
