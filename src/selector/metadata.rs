// Port of go-trafilatura/internal/selector/meta-author.go,
// meta-author-discard.go, meta-title.go, meta-categories.go, meta-tags.go

use crate::dom::{Document, NodeId};
use crate::selector::utils::{contains, get_node_ancestors, lower, starts_with};

/// Rules to find author metadata elements.
///
/// Port of `MetaAuthor`.
pub const META_AUTHOR: &[super::Rule] = &[meta_author_rule1, meta_author_rule2, meta_author_rule3];

/// Rules to discard false-positive author matches.
///
/// Port of `MetaAuthorDiscard`.
pub const META_AUTHOR_DISCARD: &[super::Rule] =
    &[meta_author_discard_rule1, meta_author_discard_rule2];

/// Rules to find title metadata elements.
///
/// Port of `MetaTitle`.
pub const META_TITLE: &[super::Rule] = &[meta_title_rule1, meta_title_rule2, meta_title_rule3];

/// Rules to find category metadata links.
///
/// Port of `MetaCategories`.
pub const META_CATEGORIES: &[super::Rule] = &[
    meta_categories_rule1,
    meta_categories_rule2,
    meta_categories_rule3,
    meta_categories_rule4,
    meta_categories_rule5,
    meta_categories_rule6,
];

/// Rules to find tag metadata links.
///
/// Port of `MetaTags`.
pub const META_TAGS: &[super::Rule] = &[
    meta_tags_rule1,
    meta_tags_rule2,
    meta_tags_rule3,
    meta_tags_rule4,
];

// ---------------------------------------------------------------------------
// MetaAuthor rules
// ---------------------------------------------------------------------------

/// Matches specific author elements by rel, id, class, itemprop, or data-testid.
///
/// Port of `metaAuthorRule1`.
fn meta_author_rule1(doc: &Document, id: NodeId) -> bool {
    let tag = doc.tag_name(id);
    match tag {
        "a" | "address" | "div" | "link" | "p" | "span" | "strong" => {}
        "author" => return true,
        _ => return false,
    }

    let class = doc.class_name(id);
    let elem_id = doc.id_attr(id);
    let rel = doc.get_attribute(id, "rel").unwrap_or_default();
    let item_prop = doc.get_attribute(id, "itemprop").unwrap_or_default();
    let data_test_id = doc.get_attribute(id, "data-testid").unwrap_or_default();

    rel == "author"
        || elem_id == "author"
        || class == "author"
        || item_prop == "author name"
        || rel == "me"
        || contains(&class, "author-name")
        || contains(&class, "AuthorName")
        || contains(&class, "authorName")
        || contains(&class, "author name")
        || data_test_id == "AuthorCard"
        || data_test_id == "AuthorURL"
}

/// Matches generic author containers by class/id/itemprop patterns.
///
/// Port of `metaAuthorRule2`.
fn meta_author_rule2(doc: &Document, id: NodeId) -> bool {
    let tag = doc.tag_name(id);
    match tag {
        "a" | "div" | "h3" | "h4" | "p" | "span" => {}
        _ => return false,
    }

    let class = doc.class_name(id);
    let elem_id = doc.id_attr(id);
    let item_prop = doc.get_attribute(id, "itemprop").unwrap_or_default();

    contains(&class, "author")
        || contains(&elem_id, "author")
        || contains(&item_prop, "author")
        || class == "byline"
        || contains(&class, "channel-name")
        || contains(&elem_id, "zuozhe")
        || contains(&class, "zuozhe")
        || contains(&elem_id, "bianji")
        || contains(&class, "bianji")
        || contains(&elem_id, "xiaobian")
        || contains(&class, "xiaobian")
        || contains(&class, "submitted-by")
        || contains(&class, "posted-by")
        || class == "username"
        || class == "byl"
        || class == "BBL"
        || contains(&class, "journalist-name")
}

/// Matches any element with author-related attributes (last-resort).
///
/// Port of `metaAuthorRule3`.
fn meta_author_rule3(doc: &Document, id: NodeId) -> bool {
    let class = doc.class_name(id);
    let elem_id = doc.id_attr(id);
    let data_component = doc.get_attribute(id, "data-component").unwrap_or_default();
    let item_prop = doc.get_attribute(id, "itemprop").unwrap_or_default();

    contains(&lower(&elem_id), "author")
        || contains(&lower(&class), "author")
        || contains(&class, "screenname")
        || contains(&lower(&data_component), "byline")
        || contains(&item_prop, "author")
        || contains(&class, "writer")
        || contains(&lower(&class), "byline")
}

// ---------------------------------------------------------------------------
// MetaAuthorDiscard rules
// ---------------------------------------------------------------------------

/// Discards comment sections, sidebars, dates, and figures that may contain author-like text.
///
/// Port of `metaAuthorDiscardRule1`.
fn meta_author_discard_rule1(doc: &Document, id: NodeId) -> bool {
    let tag = doc.tag_name(id);
    match tag {
        "a" | "div" | "section" | "span" => {}
        _ => return false,
    }

    let class = doc.class_name(id);
    let elem_id = doc.id_attr(id);
    let data_component = doc.get_attribute(id, "data-component").unwrap_or_default();

    elem_id == "comments"
        || class == "comments"
        || class == "title"
        || class == "date"
        || contains(&elem_id, "commentlist")
        || contains(&class, "commentlist")
        || contains(&class, "sidebar")
        || contains(&class, "is-hidden")
        || contains(&class, "quote")
        || contains(&elem_id, "comment-list")
        || contains(&class, "comment-list")
        || contains(&class, "embedly-instagram")
        || contains(&elem_id, "ProductReviews")
        || starts_with(&elem_id, "comments")
        || contains(&data_component, "Figure")
        || contains(&class, "article-share")
        || contains(&class, "article-support")
        || contains(&class, "print")
        || contains(&class, "category")
        || contains(&class, "meta-date")
        || contains(&class, "meta-reviewer")
        || starts_with(&class, "comments")
        || starts_with(&class, "Comments")
}

/// Discards `<time>` and `<figure>` elements.
///
/// Port of `metaAuthorDiscardRule2`.
fn meta_author_discard_rule2(doc: &Document, id: NodeId) -> bool {
    let tag = doc.tag_name(id);
    tag == "time" || tag == "figure"
}

// ---------------------------------------------------------------------------
// MetaTitle rules
// ---------------------------------------------------------------------------

/// Matches h1/h2 with headline-related class/itemprop.
///
/// Port of `metaTitleRule1`.
fn meta_title_rule1(doc: &Document, id: NodeId) -> bool {
    let tag = doc.tag_name(id);
    match tag {
        "h1" | "h2" => {}
        _ => return false,
    }

    let class = doc.class_name(id);
    let elem_id = doc.id_attr(id);
    let item_prop = doc.get_attribute(id, "itemprop").unwrap_or_default();

    contains(&class, "post-title")
        || contains(&class, "entry-title")
        || contains(&class, "headline")
        || contains(&elem_id, "headline")
        || contains(&item_prop, "headline")
        || contains(&class, "post__title")
        || contains(&class, "article-title")
}

/// Matches any element with class "entry-title" or "post-title".
///
/// Port of `metaTitleRule2`.
fn meta_title_rule2(doc: &Document, id: NodeId) -> bool {
    let class = doc.class_name(id);
    class == "entry-title" || class == "post-title"
}

/// Matches h1/h2/h3 with "title" in class or id.
///
/// Port of `metaTitleRule3`.
fn meta_title_rule3(doc: &Document, id: NodeId) -> bool {
    let tag = doc.tag_name(id);
    match tag {
        "h1" | "h2" | "h3" => {}
        _ => return false,
    }

    let class = doc.class_name(id);
    let elem_id = doc.id_attr(id);

    contains(&class, "title") || contains(&elem_id, "title")
}

// ---------------------------------------------------------------------------
// MetaCategories rules
// ---------------------------------------------------------------------------

/// Matches `<a href>` inside div ancestors with post-meta/entry-meta class prefixes.
///
/// Port of `metaCategoriesRule1`.
fn meta_categories_rule1(doc: &Document, id: NodeId) -> bool {
    if doc.tag_name(id) != "a" || doc.get_attribute(id, "href").is_none() {
        return false;
    }

    for ancestor in get_node_ancestors(doc, id, "div") {
        let elem_id = doc.id_attr(ancestor);
        let class = doc.class_name(ancestor);

        if starts_with(&class, "post-info")
            || starts_with(&class, "postinfo")
            || starts_with(&class, "post-meta")
            || starts_with(&class, "postmeta")
            || starts_with(&class, "meta")
            || starts_with(&class, "entry-meta")
            || starts_with(&class, "entry-info")
            || starts_with(&class, "entry-utility")
            || starts_with(&elem_id, "postpath")
        {
            return true;
        }
    }

    false
}

/// Matches `<a href>` inside p ancestors with postmeta/entry-categories class prefix.
///
/// Port of `metaCategoriesRule2`.
fn meta_categories_rule2(doc: &Document, id: NodeId) -> bool {
    if doc.tag_name(id) != "a" || doc.get_attribute(id, "href").is_none() {
        return false;
    }

    for ancestor in get_node_ancestors(doc, id, "p") {
        let elem_id = doc.id_attr(ancestor);
        let class = doc.class_name(ancestor);

        if starts_with(&class, "postmeta")
            || starts_with(&class, "entry-categories")
            || class == "postinfo"
            || elem_id == "filedunder"
        {
            return true;
        }
    }

    false
}

/// Matches `<a href>` inside footer ancestors with entry-meta/entry-footer class prefix.
///
/// Port of `metaCategoriesRule3`.
fn meta_categories_rule3(doc: &Document, id: NodeId) -> bool {
    if doc.tag_name(id) != "a" || doc.get_attribute(id, "href").is_none() {
        return false;
    }

    for ancestor in get_node_ancestors(doc, id, "footer") {
        let class = doc.class_name(ancestor);

        if starts_with(&class, "entry-meta") || starts_with(&class, "entry-footer") {
            return true;
        }
    }

    false
}

/// Matches `<a href>` inside li/span ancestors with post-category/cat-links class.
///
/// Port of `metaCategoriesRule4`.
fn meta_categories_rule4(doc: &Document, id: NodeId) -> bool {
    if doc.tag_name(id) != "a" || doc.get_attribute(id, "href").is_none() {
        return false;
    }

    let mut ancestors = get_node_ancestors(doc, id, "li");
    ancestors.extend(get_node_ancestors(doc, id, "span"));

    for ancestor in ancestors {
        let class = doc.class_name(ancestor);
        if class == "post-category"
            || class == "postcategory"
            || class == "entry-category"
            || contains(&class, "cat-links")
        {
            return true;
        }
    }

    false
}

/// Matches `<a href>` inside header ancestors with class "entry-header".
///
/// Port of `metaCategoriesRule5`.
fn meta_categories_rule5(doc: &Document, id: NodeId) -> bool {
    if doc.tag_name(id) != "a" || doc.get_attribute(id, "href").is_none() {
        return false;
    }

    for ancestor in get_node_ancestors(doc, id, "header") {
        if doc.class_name(ancestor) == "entry-header" {
            return true;
        }
    }

    false
}

/// Matches `<a href>` inside div ancestors with class "row" or "tags".
///
/// Port of `metaCategoriesRule6`.
fn meta_categories_rule6(doc: &Document, id: NodeId) -> bool {
    if doc.tag_name(id) != "a" || doc.get_attribute(id, "href").is_none() {
        return false;
    }

    for ancestor in get_node_ancestors(doc, id, "div") {
        let class = doc.class_name(ancestor);
        if class == "row" || class == "tags" {
            return true;
        }
    }

    false
}

// ---------------------------------------------------------------------------
// MetaTags rules
// ---------------------------------------------------------------------------

/// Matches `<a href>` inside div with class "tags".
///
/// Port of `metaTagsRule1`.
fn meta_tags_rule1(doc: &Document, id: NodeId) -> bool {
    if doc.tag_name(id) != "a" || doc.get_attribute(id, "href").is_none() {
        return false;
    }

    for ancestor in get_node_ancestors(doc, id, "div") {
        if doc.class_name(ancestor) == "tags" {
            return true;
        }
    }

    false
}

/// Matches `<a href>` inside p ancestors with "entry-tags" class prefix.
///
/// Port of `metaTagsRule2`.
fn meta_tags_rule2(doc: &Document, id: NodeId) -> bool {
    if doc.tag_name(id) != "a" || doc.get_attribute(id, "href").is_none() {
        return false;
    }

    for ancestor in get_node_ancestors(doc, id, "p") {
        if starts_with(&doc.class_name(ancestor), "entry-tags") {
            return true;
        }
    }

    false
}

/// Matches `<a href>` inside div ancestors with tag/meta-related classes.
///
/// Port of `metaTagsRule3`.
fn meta_tags_rule3(doc: &Document, id: NodeId) -> bool {
    if doc.tag_name(id) != "a" || doc.get_attribute(id, "href").is_none() {
        return false;
    }

    for ancestor in get_node_ancestors(doc, id, "div") {
        let class = doc.class_name(ancestor);
        if class == "row"
            || class == "jp-relatedposts"
            || class == "entry-utility"
            || starts_with(&class, "tag")
            || starts_with(&class, "postmeta")
            || starts_with(&class, "meta")
        {
            return true;
        }
    }

    false
}

/// Matches `<a href>` inside any ancestor with entry-meta, topics, or tags-links class.
///
/// Port of `metaTagsRule4`.
fn meta_tags_rule4(doc: &Document, id: NodeId) -> bool {
    if doc.tag_name(id) != "a" || doc.get_attribute(id, "href").is_none() {
        return false;
    }

    let mut cur = id;
    while let Some(parent) = doc.parent(cur) {
        let class = doc.class_name(parent);
        if class == "entry-meta"
            || contains(&class, "topics")
            || contains(&class, "tags-links")
        {
            return true;
        }
        cur = parent;
    }

    false
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
    fn test_meta_author_rule1_rel_author() {
        let doc = parse(r#"<html><body><a rel="author">Author</a></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, META_AUTHOR).is_some());
    }

    #[test]
    fn test_meta_author_rule1_id_author() {
        let doc = parse(r#"<html><body><div id="author">Author Name</div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, META_AUTHOR).is_some());
    }

    #[test]
    fn test_meta_author_rule2_class_author() {
        let doc = parse(r#"<html><body><span class="article-author">Jane</span></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, META_AUTHOR).is_some());
    }

    #[test]
    fn test_meta_author_discard_time() {
        let doc = parse(r#"<html><body><time>2024-01-01</time></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, META_AUTHOR_DISCARD).is_some());
    }

    #[test]
    fn test_meta_title_rule1_entry_title_h1() {
        let doc = parse(r#"<html><body><h1 class="entry-title">Title</h1></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, META_TITLE).is_some());
    }

    #[test]
    fn test_meta_title_rule2_exact_class() {
        let doc = parse(r#"<html><body><div class="post-title">My Post</div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, META_TITLE).is_some());
    }

    #[test]
    fn test_meta_title_rule3_h2_title() {
        let doc = parse(r#"<html><body><h2 id="article-title">Headline</h2></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, META_TITLE).is_some());
    }

    #[test]
    fn test_meta_categories_rule1_post_meta_ancestor() {
        let doc = parse(r#"<html><body><div class="post-meta"><a href="/cat/news">News</a></div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, META_CATEGORIES).is_some());
    }

    #[test]
    fn test_meta_categories_rule4_cat_links() {
        let doc = parse(r#"<html><body><li class="cat-links"><a href="/category/tech">Tech</a></li></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, META_CATEGORIES).is_some());
    }

    #[test]
    fn test_meta_tags_rule1_tags_div() {
        let doc = parse(r#"<html><body><div class="tags"><a href="/tag/rust">Rust</a></div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, META_TAGS).is_some());
    }

    #[test]
    fn test_meta_tags_rule4_entry_meta() {
        let doc = parse(r#"<html><body><div class="entry-meta"><a href="/tag/web">Web</a></div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, META_TAGS).is_some());
    }

    #[test]
    fn test_meta_author_no_match() {
        let doc = parse(r#"<html><body><p class="article-body">text</p></body></html>"#);
        let body = doc.body().unwrap();
        // Rule 3 is a last resort that catches "author" in class — not present here.
        assert!(query(&doc, body, META_AUTHOR).is_none());
    }

    #[test]
    fn test_meta_categories_rule1_no_href_rejected() {
        // <a> without href must not match even inside correct ancestor.
        let doc = parse(r#"<html><body><div class="post-meta"><a>No href</a></div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, META_CATEGORIES).is_none());
    }

    #[test]
    fn test_meta_categories_rule1_wrong_ancestor_class() {
        // Correct tag ancestor but wrong class should not match.
        let doc = parse(r#"<html><body><div class="sidebar"><a href="/cat">Cat</a></div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, META_CATEGORIES).is_none());
    }

    #[test]
    fn test_meta_tags_rule1_no_href_rejected() {
        // <a> without href must not match.
        let doc = parse(r#"<html><body><div class="tags"><a>No href</a></div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, META_TAGS).is_none());
    }

    #[test]
    fn test_meta_tags_rule1_wrong_ancestor_class() {
        // tags-like ancestor but wrong class should not trigger rule1.
        let doc = parse(r#"<html><body><div class="sidebar"><a href="/tag/rust">Rust</a></div></body></html>"#);
        let body = doc.body().unwrap();
        // rule3 would match "meta" but "sidebar" doesn't match tag/postmeta/meta either,
        // and there is no entry-meta/topics/tags-links ancestor for rule4.
        assert!(query(&doc, body, META_TAGS).is_none());
    }
}
