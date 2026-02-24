// Port of go-trafilatura/main-extractor.go (orchestration)

pub mod baseline;
pub mod elements;
pub mod external;
pub mod html_processing;

use std::collections::HashSet;

use crate::dom::{Document, NodeId};
use crate::options::{ExtractionFocus, Options};
use crate::selector;
use crate::settings::{TAG_CATALOG, XML_HEAD_TAGS, XML_LB_TAGS, XML_LIST_TAGS, XML_REF_TAGS};
use crate::utils::lru::LruCache;
use crate::utils::trim;

use elements::handle_text_elem;
use html_processing::{
    delete_by_link_density, handle_text_node, link_density_test_tables, prune_unwanted_nodes,
};

// ---------------------------------------------------------------------------
// Pruning
// ---------------------------------------------------------------------------

/// Rule-based deletion of targeted document sections.
///
/// Port of `pruneUnwantedSections`.
pub fn prune_unwanted_sections(
    doc: &Document,
    potential_tags: &HashSet<&str>,
    opts: &Options,
) -> Document {
    // Prune overall discarded content (with backup in case too many nodes removed).
    let mut work = prune_unwanted_nodes(doc, selector::discard::OVERALL_DISCARDED_CONTENT, true);

    // Prune images.
    if !opts.include_images {
        work = prune_unwanted_nodes(&work, selector::discard::DISCARDED_IMAGE, false);
    }

    // Balance precision / recall.
    if opts.focus != ExtractionFocus::FavorRecall {
        work = prune_unwanted_nodes(&work, selector::discard::DISCARDED_TEASER, false);
        if opts.focus == ExtractionFocus::FavorPrecision {
            work = prune_unwanted_nodes(
                &work,
                selector::discard::PRECISION_DISCARDED_CONTENT,
                false,
            );
        }
    }

    // Use the body element (an actual Element node) as the subtree root for iter()-based
    // operations. work.root() returns the virtual Document node which is not an Element,
    // causing iter() to return empty (collect_iter returns early for non-Element nodes).
    let subtree = work.body().unwrap_or_else(|| work.root());

    // Remove elements by link density (two passes).
    for _ in 0..2 {
        delete_by_link_density(&mut work, subtree, opts, true, &["div"]);
        delete_by_link_density(&mut work, subtree, opts, false, &["ul", "ol", "dl"]);
        delete_by_link_density(&mut work, subtree, opts, false, &["p"]);
    }

    // Remove tables by link density.
    if potential_tags.contains("table") || opts.focus == ExtractionFocus::FavorPrecision {
        let tables = work.iter(subtree, &["table"]);
        for &table_id in tables.iter().rev() {
            if link_density_test_tables(&work, table_id, opts) {
                work.remove(table_id, false);
            }
        }
    }

    // Precision-specific cleanup.
    if opts.focus == ExtractionFocus::FavorPrecision {
        // Delete trailing title elements from the subtree's direct children.
        let children = work.children(subtree);
        for &child_id in children.iter().rev() {
            if XML_HEAD_TAGS.contains(work.tag_name(child_id)) {
                work.remove(child_id, false);
            } else {
                break;
            }
        }

        delete_by_link_density(&mut work, subtree, opts, false, &["h1", "h2", "h3", "h4", "h5", "h6", "summary"]);
        delete_by_link_density(&mut work, subtree, opts, false, &["blockquote", "pre", "q"]);
    }

    work
}

// ---------------------------------------------------------------------------
// Wild-text recovery
// ---------------------------------------------------------------------------

/// Looks for unconsidered elements throughout the document to recover missing text.
///
/// Port of `recoverWildText`.
fn recover_wild_text(
    doc: &Document,
    result_elems: &mut Vec<(String, String)>,
    potential_tags: &mut HashSet<&'static str>,
    cache: &mut LruCache,
    opts: &Options,
) {
    tracing::info!("recovering wild text elements");

    let mut selector_parts: Vec<&str> =
        vec!["blockquote", "pre", "q", "code", "p", "table", "div[class*=\"w3-code\"]"];

    if opts.focus == ExtractionFocus::FavorRecall {
        potential_tags.insert("div");
        for &t in XML_LB_TAGS.iter() {
            potential_tags.insert(t);
            selector_parts.push(t);
        }
        selector_parts.push("div");
        for &t in XML_LIST_TAGS.iter() {
            selector_parts.push(t);
        }
    }

    // Prune the backup doc.
    let mut search_doc = prune_unwanted_sections(doc, potential_tags, opts);

    let root = search_doc.root();

    // Strip links/spans.
    if potential_tags.contains("a") {
        search_doc.strip_tags(root, &["span"]);
    } else {
        search_doc.strip_tags(root, &["a", "ref", "span"]);
    }

    let selector_css = selector_parts.join(", ");
    let elements = search_doc.query_selector_all(root, &selector_css);

    for &elem_id in &elements {
        if let Some(html) = handle_text_elem(&mut search_doc, elem_id, potential_tags, cache, opts) {
            let tag = search_doc.tag_name(elem_id).to_string();
            result_elems.push((html, tag));
        }
    }
}

// ---------------------------------------------------------------------------
// Content extraction
// ---------------------------------------------------------------------------

/// Extracts the main content from the document.
///
/// Returns the result body as a Document and the extracted text.
///
/// Port of `extractContent`.
pub fn extract_content(
    doc: &Document,
    cache: &mut LruCache,
    opts: &Options,
) -> (Document, String) {
    // Prepare potential tags.
    let mut potential_tags: HashSet<&'static str> = TAG_CATALOG.iter().copied().collect();

    if !opts.exclude_tables {
        potential_tags.insert("table");
        potential_tags.insert("tr");
        potential_tags.insert("th");
        potential_tags.insert("td");
    }
    if opts.include_images {
        potential_tags.insert("img");
    }
    if opts.include_links {
        potential_tags.insert("a");
    }

    let mut result_elems: Vec<(String, String)> = Vec::new();

    // Try each content selector in priority order.
    'selector_loop: for &rule in selector::content::CONTENT {
        // Find the matching subtree in the original (unmodified) doc.
        let sub_id = match selector::query(doc, doc.root(), std::slice::from_ref(&rule)) {
            Some(id) => id,
            None => continue,
        };

        // Extract just the matched subtree's inner content into a standalone document.
        // This mirrors Go's behavior: `pruneUnwantedSections(subTree, ...)` clones and
        // prunes only the subtree, not the full document.  Pruning the full document
        // incorrectly applies link-density tests across unrelated page sections, which
        // can remove the very element we just selected.
        let subtree_inner = doc.inner_html(sub_id);
        let subtree_html = format!("<html><body>{subtree_inner}</body></html>");
        let subtree_doc = Document::parse(&subtree_html);

        // Prune unwanted content within the subtree.
        let mut work = prune_unwanted_sections(&subtree_doc, &potential_tags, opts);
        let work_body = work.body().unwrap_or_else(|| work.root());

        // Skip if the subtree is now empty.
        if work.children(work_body).is_empty() {
            continue;
        }

        // Count paragraph text from the original document (before pruning),
        // scanning the whole doc — this matches Go's `dom.GetElementsByTagName(doc, "p")`.
        let paragraph_text: String = doc
            .iter(doc.root(), &["p"])
            .into_iter()
            .map(|id| doc.text_content(id))
            .collect();

        let factor: usize = if opts.focus == ExtractionFocus::FavorPrecision { 1 } else { 3 };
        if paragraph_text.is_empty()
            || paragraph_text.chars().count() < opts.config.min_extracted_size * factor
        {
            potential_tags.insert("div");
        }

        // Strip irrelevant inline tags from the subtree.
        if !potential_tags.contains("a") {
            work.strip_tags(work_body, &["a"]);
        }
        if !potential_tags.contains("span") {
            work.strip_tags(work_body, &["span"]);
        }

        // Collect all descendant elements of the subtree.
        let mut sub_elements = work.get_elements_by_tag_name(work_body, "*");

        // If the only sub-elements are <br>, process the subtree root directly.
        let tag_set: HashSet<&str> = sub_elements
            .iter()
            .map(|&id| work.tag_name(id))
            .collect();
        if tag_set.len() == 1 && tag_set.contains("br") {
            sub_elements = vec![work_body];
        }

        // Process each element.
        let batch_start = result_elems.len();
        for &elem_id in &sub_elements {
            if let Some(html) =
                handle_text_elem(&mut work, elem_id, &potential_tags, cache, opts)
            {
                let tag = work.tag_name(elem_id).to_string();
                result_elems.push((html, tag));
            }
        }

        // Remove trailing title / ref elements from the batch.
        while let Some((_, tag)) = result_elems.last() {
            if XML_HEAD_TAGS.contains(tag.as_str()) || XML_REF_TAGS.contains(tag.as_str()) {
                result_elems.pop();
            } else {
                break;
            }
        }

        // If we have more than one result element, stop trying selectors.
        if result_elems.len() - batch_start > 1 {
            break 'selector_loop;
        }
    }

    // Fall back to wild text recovery if content is too short.
    let tmp_text_chars: usize = result_elems.iter().map(|(h, _)| h.chars().count()).sum();
    if result_elems.is_empty() || tmp_text_chars < opts.config.min_extracted_size {
        result_elems.clear();
        recover_wild_text(doc, &mut result_elems, &mut potential_tags, cache, opts);
    }

    // Build result document from collected HTML fragments.
    let body_html: String = result_elems.into_iter().map(|(h, _)| h).collect();
    let full_html = format!("<html><body>{body_html}</body></html>");
    let mut result_doc = Document::parse(&full_html);

    let body_id = result_doc.body().unwrap_or_else(|| result_doc.root());

    // Strip "done" placeholder elements (subtree removed).
    result_doc.strip_elements(body_id, false, &["done"]);
    // Strip bare <div> wrappers (keep children).
    result_doc.strip_tags(body_id, &["div"]);

    let tmp_text = trim(&result_doc.iter_text(body_id, " "));

    (result_doc, tmp_text)
}

// ---------------------------------------------------------------------------
// Comments
// ---------------------------------------------------------------------------

/// Processes a single node for comment extraction.
///
/// Port of `processCommentsNode`.
pub fn process_comments_node(
    doc: &mut Document,
    id: NodeId,
    potential_tags: &HashSet<&str>,
    cache: &mut LruCache,
    opts: &Options,
) -> Option<String> {
    let tag = doc.tag_name(id).to_string();

    if !potential_tags.contains(tag.as_str()) {
        return None;
    }

    // Process: dedup and filter check (fix_comments=true, preserve_spaces=false).
    handle_text_node(doc, id, Some(cache), true, false, opts)?;

    // Clear attributes.
    doc.clear_attributes(id);

    let inner = doc.inner_html(id);
    Some(format!("<{tag}>{inner}</{tag}>"))
}

/// Extracts comments from the document, removing the comment section from `doc`.
///
/// Returns the comments as a Document and the raw comment text, or `(None, "")` if none found.
///
/// Port of `extractComments`.
pub fn extract_comments(
    doc: &mut Document,
    cache: &mut LruCache,
    opts: &Options,
) -> (Option<Document>, String) {
    let potential_tags: HashSet<&str> = TAG_CATALOG.iter().copied().collect();
    let mut result_elems: Vec<String> = Vec::new();

    'comment_loop: for &rule in selector::comments::COMMENTS {
        let sub_id = match selector::query(doc, doc.root(), std::slice::from_ref(&rule)) {
            Some(id) => id,
            None => continue,
        };

        // Clone and prune a working copy.  NodeIds are preserved.
        let mut work =
            prune_unwanted_nodes(doc, selector::discard::DISCARDED_COMMENTS, false);
        work.strip_tags(sub_id, &["a", "span"]);

        // Extract comment nodes.
        let batch_start = result_elems.len();
        let descendants = work.get_elements_by_tag_name(sub_id, "*");
        for &elem_id in &descendants {
            if let Some(html) =
                process_comments_node(&mut work, elem_id, &potential_tags, cache, opts)
            {
                result_elems.push(html);
            }
        }

        if result_elems.len() > batch_start {
            // Remove the comment section from the original document.
            doc.remove(sub_id, false);
            break 'comment_loop;
        }
    }

    if result_elems.is_empty() {
        return (None, String::new());
    }

    let body_html: String = result_elems.join("");
    let full_html = format!("<html><body>{body_html}</body></html>");
    let result_doc = Document::parse(&full_html);
    let body_id = result_doc.body().unwrap_or_else(|| result_doc.root());
    let tmp_comments = result_doc.iter_text(body_id, " ");

    (Some(result_doc), tmp_comments)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cache() -> LruCache {
        LruCache::new(500)
    }

    fn default_opts() -> Options {
        Options::default()
    }

    // ---------------------------------------------------------------------------
    // prune_unwanted_sections
    // ---------------------------------------------------------------------------

    #[test]
    fn test_prune_removes_nav_and_footer() {
        let html = r#"<html><body>
            <nav>Site navigation</nav>
            <article>Main article content that is long enough to keep.</article>
            <footer>Footer text</footer>
        </body></html>"#;
        let doc = Document::parse(html);
        let potential_tags: HashSet<&str> = TAG_CATALOG.iter().copied().collect();
        let pruned = prune_unwanted_sections(&doc, &potential_tags, &default_opts());
        let text = pruned.iter_text(pruned.root(), " ");
        // Navigation and footer should be removed by OverallDiscardedContent rules.
        // Article content should remain.
        assert!(
            text.contains("Main article content"),
            "article content missing: {text}"
        );
    }

    #[test]
    fn test_prune_preserves_content_when_too_much_removed() {
        // If OverallDiscardedContent would remove > 6/7 of text, the backup is restored.
        // Create a doc where almost everything is "discarded" content.
        let html = r#"<html><body>
            <p>Short article.</p>
        </body></html>"#;
        let doc = Document::parse(html);
        let potential_tags: HashSet<&str> = TAG_CATALOG.iter().copied().collect();
        let pruned = prune_unwanted_sections(&doc, &potential_tags, &default_opts());
        let text = pruned.iter_text(pruned.root(), " ");
        assert!(text.contains("Short article"), "content lost: {text}");
    }

    // ---------------------------------------------------------------------------
    // extract_content
    // ---------------------------------------------------------------------------

    #[test]
    fn test_extract_content_article_tag() {
        let html = r#"<html><body>
            <article id="main">
                <h1>Article Title</h1>
                <p>This is the main article content that is long enough to pass the minimum size check and provides substantial text.</p>
                <p>Second paragraph with more content to ensure we exceed the minimum threshold.</p>
            </article>
            <nav>Nav garbage</nav>
        </body></html>"#;
        let doc = Document::parse(html);
        let mut cache = make_cache();
        let (result_doc, text) = extract_content(&doc, &mut cache, &default_opts());
        let body = result_doc.body().unwrap_or(result_doc.root());
        let result_text = result_doc.iter_text(body, " ");
        assert!(
            result_text.contains("main article content"),
            "content missing: {result_text}"
        );
        assert!(
            !result_text.contains("Nav garbage"),
            "nav should be removed: {result_text}"
        );
        assert!(!text.is_empty(), "extracted text should not be empty");
    }

    #[test]
    fn test_extract_content_falls_back_to_wild_recovery() {
        // No known content selector matches → falls back to wild recovery.
        let html = r#"<html><body>
            <div>
                <p>Some standalone paragraph content that is substantial enough for extraction and passes the minimum size threshold for extraction purposes.</p>
            </div>
        </body></html>"#;
        let doc = Document::parse(html);
        let mut cache = make_cache();
        let (result_doc, text) = extract_content(&doc, &mut cache, &default_opts());
        let body = result_doc.body().unwrap_or(result_doc.root());
        let result_text = result_doc.iter_text(body, " ");
        assert!(
            result_text.contains("standalone paragraph"),
            "content missing: {result_text}"
        );
        assert!(!text.is_empty(), "extracted text should not be empty");
    }

    // ---------------------------------------------------------------------------
    // process_comments_node
    // ---------------------------------------------------------------------------

    #[test]
    fn test_process_comments_node_valid() {
        let html = "<html><body><p>A comment text here.</p></body></html>";
        let mut doc = Document::parse(html);
        let body = doc.body().unwrap();
        let p_id = doc.children(body)[0];
        let potential_tags: HashSet<&str> = TAG_CATALOG.iter().copied().collect();
        let mut cache = make_cache();
        let result = process_comments_node(&mut doc, p_id, &potential_tags, &mut cache, &default_opts());
        assert!(result.is_some(), "expected Some, got None");
        let html = result.unwrap();
        assert!(html.contains("A comment text"), "got: {html}");
    }

    #[test]
    fn test_process_comments_node_not_in_potential_tags() {
        let html = "<html><body><nav>Navigation</nav></body></html>";
        let mut doc = Document::parse(html);
        let body = doc.body().unwrap();
        let nav_id = doc.children(body)[0];
        let potential_tags: HashSet<&str> = TAG_CATALOG.iter().copied().collect();
        let mut cache = make_cache();
        // "nav" is not in TAG_CATALOG (potential_tags), so it should return None.
        let result = process_comments_node(&mut doc, nav_id, &potential_tags, &mut cache, &default_opts());
        assert!(result.is_none(), "nav should not be included in comments");
    }

    // ---------------------------------------------------------------------------
    // extract_comments
    // ---------------------------------------------------------------------------

    #[test]
    fn test_extract_comments_basic() {
        // Use id="comments-section" which matches commentsRule2 (starts_with "comments").
        let html = r#"<html><body>
            <article><p>Main content here.</p></article>
            <div id="comments-section">
                <p>First comment text that is meaningful and long enough to pass filters.</p>
                <p>Second comment with more words to make it substantial content.</p>
            </div>
        </body></html>"#;
        let mut doc = Document::parse(html);
        let mut cache = make_cache();
        let (result, text) = extract_comments(&mut doc, &mut cache, &default_opts());
        assert!(result.is_some(), "expected comments to be found");
        assert!(!text.is_empty(), "comment text should not be empty");
        // The comments section should have been removed from doc.
        let doc_text = doc.iter_text(doc.root(), " ");
        assert!(
            doc_text.contains("Main content"),
            "main content should remain after comment extraction: {doc_text}"
        );
        assert!(
            !doc_text.contains("First comment"),
            "comment section should be removed from doc: {doc_text}"
        );
    }

    #[test]
    fn test_extract_content_strips_trailing_titles() {
        // Articles that end with a heading like "See Also" should have the
        // trailing title removed from the result.
        let html = r#"<html><body>
            <article class="post-content">
                <p>This is meaningful article content that passes the minimum size threshold.
                It is long enough to be extracted by the content pipeline without issues.</p>
                <p>Second paragraph with more content to ensure we hit the threshold.</p>
                <h2>See Also</h2>
            </article>
        </body></html>"#;
        let doc = Document::parse(html);
        let mut cache = make_cache();
        let (result_doc, _text) = extract_content(&doc, &mut cache, &default_opts());
        let body = result_doc.body().unwrap_or(result_doc.root());
        let result_text = result_doc.iter_text(body, " ");
        assert!(
            result_text.contains("meaningful article content"),
            "content missing: {result_text}"
        );
        assert!(
            !result_text.contains("See Also"),
            "trailing title should be stripped: {result_text}"
        );
    }

    #[test]
    fn test_extract_comments_no_comments() {
        let html = "<html><body><article><p>Just content, no comments.</p></article></body></html>";
        let mut doc = Document::parse(html);
        let mut cache = make_cache();
        let (result, text) = extract_comments(&mut doc, &mut cache, &default_opts());
        assert!(result.is_none(), "expected no comments");
        assert!(text.is_empty(), "expected empty text");
    }
}
