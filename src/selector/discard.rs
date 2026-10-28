// Port of go-trafilatura/internal/selector/content-discard-overall.go,
// content-discard-precision.go, teaser-discard.go, image-discard.go,
// comments-discard.go, comments-removed.go

use crate::dom::{Document, NodeId};
use crate::selector::utils::{contains, lower, starts_with};

/// Rules that discard common boilerplate: footers, nav, sidebars, ads, etc.
///
/// Port of `OverallDiscardedContent`.
pub(crate) const OVERALL_DISCARDED_CONTENT: &[super::Rule] = &[
    overall_discarded_content_rule1,
    overall_discarded_content_rule2,
];

/// Rules that discard elements in precision mode: headers and link/border containers.
///
/// Port of `PrecisionDiscardedContent`.
pub(crate) const PRECISION_DISCARDED_CONTENT: &[super::Rule] = &[
    precision_discarded_content_rule1,
    precision_discarded_content_rule2,
];

/// Rules that discard comment-adjacent noise (reply forms, hidden elements, etc.).
///
/// Port of `DiscardedComments`.
pub(crate) const DISCARDED_COMMENTS: &[super::Rule] = &[
    discarded_comments_rule1,
    discarded_comments_rule2,
    discarded_comments_rule3,
];

/// Rules that remove full comment thread sections.
///
/// Port of `RemovedComments`.
pub(crate) const REMOVED_COMMENTS: &[super::Rule] = &[removed_comments_rule1];

/// Containers that readability-style extractors treat as unlikely content
/// candidates: comments, sidebars, headers, footers, promos, and similar
/// boilerplate.
///
/// Port of the unlikely-candidates preprocessing in go-trafilatura's
/// readability-lxml engine (`readabilityLxmlRemoveUnlikely`), itself the
/// classic arc90 readability heuristic: an element whose combined
/// `class`/`id` matches an unlikely token — unless it also matches a
/// "possible content" token such as `main` or `article` — is unlikely to be
/// main content. Exposed as a regular rule so it composes with
/// [`prune_unwanted_nodes`](crate::extraction::prune_unwanted_nodes) like any
/// other discard set.
pub(crate) const UNLIKELY_CANDIDATES: &[super::Rule] = &[unlikely_candidates_rule1];

static UNLIKELY_TOKENS: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(r"(?i)combx|comment|community|disqus|extra|foot|header|menu|remark|rss|shoutbox|sidebar|sponsor|ad-break|agegate|pagination|pager|popup|tweet|twitter").unwrap()
});

static POSSIBLE_CONTENT_TOKENS: std::sync::LazyLock<regex::Regex> =
    std::sync::LazyLock::new(|| {
        regex::Regex::new(r"(?i)and|article|body|column|main|shadow").unwrap()
    });

/// Matches boilerplate-marked containers whose `class`/`id` contains an
/// unlikely token but no possible-content token.
///
/// Port of `readabilityLxmlRemoveUnlikely`. `<html>` and `<body>` are exempt.
fn unlikely_candidates_rule1(doc: &Document, id: NodeId) -> bool {
    let tag = doc.tag_name(id);
    if tag == "html" || tag == "body" {
        return false;
    }
    let attributes = format!("{} {}", doc.class_name(id), doc.id_attr(id));
    UNLIKELY_TOKENS.is_match(&attributes) && !POSSIBLE_CONTENT_TOKENS.is_match(&attributes)
}

/// Rules that discard image caption containers.
///
/// Port of `DiscardedImage`.
pub(crate) const DISCARDED_IMAGE: &[super::Rule] = &[discarded_image_rule1];

/// Rules that discard teaser/preview elements.
///
/// Port of `DiscardedTeaser`.
pub(crate) const DISCARDED_TEASER: &[super::Rule] = &[discarded_teaser_rule1];

// ---------------------------------------------------------------------------
// OverallDiscardedContent rules
// ---------------------------------------------------------------------------

/// Matches navigation, footers, social sharing, ads, paywalls, and other boilerplate.
///
/// Port of `overallDiscardedContentRule1`.
fn overall_discarded_content_rule1(doc: &Document, id: NodeId) -> bool {
    let tag = doc.tag_name(id);
    match tag {
        "div" | "dd" | "dt" | "li" | "ul" | "ol" | "dl" | "p" | "section" | "span" => {}
        _ => return false,
    }

    let class = doc.class_name(id);
    let elem_id = doc.id_attr(id);
    let role = doc.get_attribute(id, "role").unwrap_or_default();
    let data_component = doc.get_attribute(id, "data-component").unwrap_or_default();
    let id_class = format!("{elem_id}{class}");
    // Pre-compute lowercased strings to avoid repeated allocations in the conditions.
    let lower_id = lower(&elem_id);
    let lower_class = lower(&class);
    let lower_role = lower(&role);

    contains(&lower_id, "footer")
        || contains(&lower_class, "footer")
        || contains(&elem_id, "related")
        || contains(&class, "elated")
        || contains(&id_class, "viral")
        || starts_with(&id_class, "shar")
        || contains(&class, "share-")
        || contains(&lower_id, "share")
        || contains(&id_class, "social")
        || contains(&class, "sociable")
        || contains(&id_class, "syndication")
        || starts_with(&elem_id, "jp-")
        || starts_with(&elem_id, "dpsp-content")
        || contains(&class, "embedded")
        || contains(&class, "embed")
        || contains(&id_class, "newsletter")
        || contains(&class, "subnav")
        || contains(&id_class, "cookie")
        || contains(&id_class, "tags")
        || contains(&class, "tag-list")
        || contains(&id_class, "sidebar")
        || contains(&id_class, "banner")
        || contains(&class, "bar")
        || contains(&class, "meta")
        || contains(&elem_id, "menu")
        || contains(&class, "menu")
        || contains(&lower_id, "nav")
        || contains(&lower_role, "nav")
        || starts_with(&class, "nav")
        || contains(&class, "avigation")
        || contains(&class, "navbar")
        || contains(&class, "navbox")
        || starts_with(&class, "post-nav")
        || contains(&id_class, "breadcrumb")
        || contains(&id_class, "bread-crumb")
        || contains(&id_class, "author")
        || contains(&id_class, "button")
        || contains(&lower_class, "byline")
        || contains(&class, "rating")
        || contains(&class, "widget")
        || contains(&class, "attachment")
        || contains(&class, "timestamp")
        || contains(&class, "user-info")
        || contains(&class, "user-profile")
        || contains(&class, "-ad-")
        || contains(&class, "-icon")
        || contains(&class, "article-infos")
        || contains(&class, "nfoline")
        || contains(&data_component, "MostPopularStories")
        || contains(&class, "outbrain")
        || contains(&class, "taboola")
        || contains(&class, "criteo")
        || contains(&class, "options")
        || contains(&class, "expand")
        || contains(&class, "consent")
        || contains(&class, "modal-content")
        || contains(&class, " ad ")
        || contains(&class, "permission")
        || contains(&class, "next-")
        || contains(&class, "-stories")
        || contains(&class, "most-popular")
        || contains(&class, "mol-factbox")
        || starts_with(&class, "ZendeskForm")
        || contains(&id_class, "message-container")
        || contains(&class, "yin")
        || contains(&class, "zlylin")
        || contains(&class, "xg1")
        || contains(&elem_id, "bmdh")
        || contains(&class, "slide")
        || contains(&class, "viewport")
        || doc
            .get_attribute(id, "data-lp-replacement-content")
            .is_some()
        || contains(&elem_id, "premium")
        || contains(&class, "overlay")
        || contains(&class, "paid-content")
        || contains(&class, "paidcontent")
        || contains(&class, "obfuscated")
        || contains(&class, "blurred")
}

/// Matches hidden elements, comment debris, and display:none content.
///
/// Port of `overallDiscardedContentRule2`.
fn overall_discarded_content_rule2(doc: &Document, id: NodeId) -> bool {
    let class = doc.class_name(id);
    let elem_id = doc.id_attr(id);
    let style = doc.get_attribute(id, "style").unwrap_or_default();
    let aria_hidden = doc.get_attribute(id, "aria-hidden").unwrap_or_default();
    let id_class = format!("{elem_id}{class}");
    let id_style = format!("{elem_id}{style}");

    class == "comments-title"
        || contains(&class, "comments-title")
        || contains(&class, "nocomments")
        || starts_with(&id_class, "reply-")
        || contains(&class, "-reply-")
        || contains(&class, "message")
        || contains(&elem_id, "reader-comments")
        || contains(&elem_id, "akismet")
        || contains(&class, "akismet")
        || contains(&class, "suggest-links")
        || starts_with(&class, "hide-")
        || contains(&class, "-hide-")
        || contains(&class, "hide-print")
        || contains(&id_style, "hidden")
        || contains(&class, " hidden")
        || contains(&class, " hide")
        || contains(&class, "noprint")
        || contains(&style, "display:none")
        || contains(&style, "display: none")
        || aria_hidden == "true"
        || contains(&class, "notloaded")
}

// ---------------------------------------------------------------------------
// PrecisionDiscardedContent rules
// ---------------------------------------------------------------------------

/// Matches `<header>` elements.
///
/// Port of `precisionDiscardedContentRule1`.
fn precision_discarded_content_rule1(doc: &Document, id: NodeId) -> bool {
    doc.tag_name(id) == "header"
}

/// Matches containers with "bottom", "link", or border styles.
///
/// Port of `precisionDiscardedContentRule2`.
fn precision_discarded_content_rule2(doc: &Document, id: NodeId) -> bool {
    let tag = doc.tag_name(id);
    match tag {
        "div" | "dd" | "dt" | "li" | "ul" | "ol" | "dl" | "p" | "section" | "span" => {}
        _ => return false,
    }

    let class = doc.class_name(id);
    let elem_id = doc.id_attr(id);
    let style = doc.get_attribute(id, "style").unwrap_or_default();
    let id_class = format!("{elem_id}{class}");

    contains(&id_class, "bottom") || contains(&id_class, "link") || contains(&style, "border")
}

// ---------------------------------------------------------------------------
// DiscardedComments rules
// ---------------------------------------------------------------------------

/// Matches reply form containers (id starts with "respond").
///
/// Port of `discardedCommentsRule1`.
fn discarded_comments_rule1(doc: &Document, id: NodeId) -> bool {
    let tag = doc.tag_name(id);
    match tag {
        "div" | "section" => {}
        _ => return false,
    }

    starts_with(&doc.id_attr(id), "respond")
}

/// Matches `<cite>` and `<quote>` elements.
///
/// Port of `discardedCommentsRule2`.
fn discarded_comments_rule2(doc: &Document, id: NodeId) -> bool {
    let tag = doc.tag_name(id);
    tag == "cite" || tag == "quote"
}

/// Matches comment noise: titles, nocomments, reply links, hidden, sign-in, akismet.
///
/// Port of `discardedCommentsRule3`.
fn discarded_comments_rule3(doc: &Document, id: NodeId) -> bool {
    let class = doc.class_name(id);
    let elem_id = doc.id_attr(id);
    let style = doc.get_attribute(id, "style").unwrap_or_default();
    let id_class = format!("{elem_id}{class}");

    class == "comments-title"
        || contains(&class, "comments-title")
        || contains(&class, "nocomments")
        || starts_with(&id_class, "reply-")
        || contains(&class, "-reply-")
        || contains(&class, "message")
        || contains(&class, "signin")
        || contains(&id_class, "akismet")
        || contains(&style, "display:none")
}

// ---------------------------------------------------------------------------
// RemovedComments rules
// ---------------------------------------------------------------------------

/// Matches full comment thread sections (starts with "comment" in id or class).
///
/// Port of `removedCommentsRule1`.
fn removed_comments_rule1(doc: &Document, id: NodeId) -> bool {
    let tag = doc.tag_name(id);
    match tag {
        "div" | "ol" | "ul" | "dl" | "section" | "details" => {}
        _ => return false,
    }

    let class = doc.class_name(id);
    let elem_id = doc.id_attr(id);

    starts_with(&elem_id, "comment")
        || starts_with(&elem_id, "Comment")
        || starts_with(&class, "comment")
        || starts_with(&class, "Comment")
        || contains(&class, "article-comments")
        || contains(&class, "post-comments")
        || starts_with(&elem_id, "comol")
        || starts_with(&elem_id, "disqus_thread")
        || starts_with(&elem_id, "dsq-comments")
}

// ---------------------------------------------------------------------------
// DiscardedImage rules
// ---------------------------------------------------------------------------

/// Matches image caption containers.
///
/// Port of `discardedImageRule1`.
fn discarded_image_rule1(doc: &Document, id: NodeId) -> bool {
    let tag = doc.tag_name(id);
    match tag {
        "div" | "dd" | "dt" | "li" | "ul" | "ol" | "dl" | "p" | "section" | "span" => {}
        _ => return false,
    }

    let class = doc.class_name(id);
    let elem_id = doc.id_attr(id);

    contains(&elem_id, "caption") || contains(&class, "caption")
}

// ---------------------------------------------------------------------------
// DiscardedTeaser rules
// ---------------------------------------------------------------------------

/// Matches teaser/preview containers (case-insensitive).
///
/// Port of `discardedTeaserRule1`.
fn discarded_teaser_rule1(doc: &Document, id: NodeId) -> bool {
    let tag = doc.tag_name(id);
    match tag {
        "div" | "dd" | "dt" | "li" | "ul" | "ol" | "dl" | "p" | "section" | "span" => {}
        _ => return false,
    }

    let class = doc.class_name(id);
    let elem_id = doc.id_attr(id);

    contains(&lower(&elem_id), "teaser") || contains(&lower(&class), "teaser")
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
    fn test_overall_discard_footer() {
        let doc = parse(r#"<html><body><div class="footer">footer</div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, OVERALL_DISCARDED_CONTENT).is_some());
    }

    #[test]
    fn test_overall_discard_nav() {
        let doc = parse(r#"<html><body><div class="navbar">nav</div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, OVERALL_DISCARDED_CONTENT).is_some());
    }

    #[test]
    fn test_overall_discard_sidebar() {
        let doc = parse(r#"<html><body><div id="sidebar">side</div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, OVERALL_DISCARDED_CONTENT).is_some());
    }

    #[test]
    fn test_overall_discard_aria_hidden() {
        let doc = parse(r#"<html><body><div aria-hidden="true">hidden</div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, OVERALL_DISCARDED_CONTENT).is_some());
    }

    #[test]
    fn test_overall_discard_display_none() {
        let doc = parse(r#"<html><body><div style="display:none">hidden</div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, OVERALL_DISCARDED_CONTENT).is_some());
    }

    #[test]
    fn test_overall_discard_paywall() {
        let doc = parse(r#"<html><body><div class="paid-content">paywall</div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, OVERALL_DISCARDED_CONTENT).is_some());
    }

    #[test]
    fn test_precision_discard_header() {
        let doc = parse(r#"<html><body><header>header</header></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, PRECISION_DISCARDED_CONTENT).is_some());
    }

    #[test]
    fn test_precision_discard_bottom() {
        let doc = parse(r#"<html><body><div class="bottom-links">links</div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, PRECISION_DISCARDED_CONTENT).is_some());
    }

    #[test]
    fn test_discarded_comments_respond() {
        let doc = parse(r#"<html><body><div id="respond">form</div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, DISCARDED_COMMENTS).is_some());
    }

    #[test]
    fn test_unlikely_candidates_rule() {
        let matches = |html: &str| {
            let doc = parse(html);
            let body = doc.body().unwrap();
            query(&doc, body, UNLIKELY_CANDIDATES).is_some()
        };
        // Boilerplate-marked containers match …
        assert!(matches(
            r#"<html><body><div id="comments" class="comments-area">x</div></body></html>"#
        ));
        assert!(matches(
            r#"<html><body><aside class="sidebar">x</aside></body></html>"#
        ));
        assert!(matches(
            r#"<html><body><div class="promo sponsor-box">x</div></body></html>"#
        ));
        // … unless a possible-content token appears (and|article|body|column|main|shadow).
        assert!(!matches(
            r#"<html><body><div class="comment-body">x</div></body></html>"#
        ));
        assert!(!matches(
            r#"<html><body><div class="comments main-area">x</div></body></html>"#
        ));
        assert!(!matches(
            r#"<html><body><article class="">article body</article></body></html>"#
        ));
    }

    #[test]
    fn test_discarded_comments_cite() {
        let doc = parse(r#"<html><body><cite>quote</cite></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, DISCARDED_COMMENTS).is_some());
    }

    #[test]
    fn test_removed_comments_section() {
        let doc = parse(r#"<html><body><div id="comments">all comments</div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, REMOVED_COMMENTS).is_some());
    }

    #[test]
    fn test_discarded_image_caption() {
        let doc = parse(r#"<html><body><div class="caption">caption text</div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, DISCARDED_IMAGE).is_some());
    }

    #[test]
    fn test_discarded_teaser() {
        let doc = parse(r#"<html><body><div class="Teaser-box">teaser</div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, DISCARDED_TEASER).is_some());
    }

    #[test]
    fn test_overall_no_match_article_content() {
        let doc = parse(r#"<html><body><div class="article-content">text</div></body></html>"#);
        let body = doc.body().unwrap();
        assert!(query(&doc, body, OVERALL_DISCARDED_CONTENT).is_none());
    }
}
