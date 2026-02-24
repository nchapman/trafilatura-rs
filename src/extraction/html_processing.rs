// Port of go-trafilatura/html-processing.go

use crate::dom::{Document, NodeId};
use crate::options::{ExtractionFocus, Options};
use crate::selector::Rule;
use crate::settings::{
    ALLOWED_ATTRIBUTES, ELEMENT_WITH_SIZE_ATTR, EMPTY_TAGS_TO_REMOVE, TAGS_TO_CLEAN,
    TAGS_TO_STRIP, XML_GRAPHIC_TAGS, XML_LB_TAGS, XML_QUOTE_TAGS,
};
use crate::utils::lru::LruCache;
use crate::utils::text::{duplicate_test, text_chars_test, text_filter};
use crate::utils::trim;
use crate::utils::is_image_element;
use crate::selector::query_all;

/// Cleans the document by discarding unwanted elements.
///
/// Port of `docCleaning` (originally `tree_cleaning` in Python trafilatura).
pub fn doc_cleaning(doc: &mut Document, opts: &Options) {
    // Build cleaning and stripping lists, then modify based on options.
    let mut cleaning_list: std::collections::HashSet<&'static str> = TAGS_TO_CLEAN.clone();
    let mut stripping_list: std::collections::HashSet<&'static str> = TAGS_TO_STRIP.clone();

    if opts.exclude_tables {
        for tag in ["table", "td", "th", "tr"] {
            cleaning_list.insert(tag);
        }
    } else {
        // Convert figure elements that contain tables into divs.
        let figures = doc.query_selector_all(doc.root(), "figure");
        for figure_id in figures {
            let has_table = doc
                .get_elements_by_tag_name(figure_id, "*")
                .iter()
                .any(|&child| doc.tag_name(child) == "table");
            if has_table {
                doc.set_tag_name(figure_id, "div");
            }
        }
    }

    if opts.include_images {
        // Many websites have <img> inside <figure> or <picture> or <source>; keep them.
        cleaning_list.remove("figure");
        cleaning_list.remove("picture");
        cleaning_list.remove("source");
        stripping_list.remove("img");
    }

    // Strip tags in stripping list (remove tag, keep children and tail).
    let strip_tags: Vec<&str> = stripping_list.iter().copied().collect();
    for tag in &strip_tags {
        doc.strip_tags(doc.root(), &[tag]);
    }

    // Prevent removal of paragraphs when in recall mode.
    if opts.focus == ExtractionFocus::FavorRecall
        && !doc.get_elements_by_tag_name(doc.root(), "p").is_empty()
    {
        let backup = doc.clone_document();
        for tag in &cleaning_list.iter().copied().collect::<Vec<_>>() {
            doc.strip_elements(doc.root(), false, &[tag]);
        }

        // If paragraphs were removed, restore backup.
        if doc.get_elements_by_tag_name(doc.root(), "p").is_empty() {
            *doc = backup;
        }
    } else {
        for tag in &cleaning_list.iter().copied().collect::<Vec<_>>() {
            doc.strip_elements(doc.root(), false, &[tag]);
        }
    }

    // Remove HTML comment nodes.
    remove_html_comment_nodes(doc);
    prune_html(doc, opts);
}

/// Removes all HTML comment nodes from the document.
///
/// Port of `removeHtmlCommentNode`.
pub fn remove_html_comment_nodes(doc: &mut Document) {
    // Collect all comment node IDs first (can't mutate while iterating).
    let comment_ids: Vec<NodeId> = doc.collect_comment_nodes(doc.root());
    for id in comment_ids.into_iter().rev() {
        // Remove comment without keeping tail (comments don't have tails).
        doc.remove_comment(id);
    }
}

/// Deletes selected empty elements to save space and processing time.
///
/// Port of `pruneHTML`.
pub fn prune_html(doc: &mut Document, opts: &Options) {
    let keep_tail = opts.focus != ExtractionFocus::FavorPrecision;
    let all_elements = doc.get_elements_by_tag_name(doc.root(), "*");

    // Iterate in reverse so children are removed before parents.
    for &id in all_elements.iter().rev() {
        let tag = doc.tag_name(id);
        if !EMPTY_TAGS_TO_REMOVE.contains(tag) {
            continue;
        }
        if doc.child_nodes(id).is_empty() {
            doc.remove(id, keep_tail);
        }
    }
}

/// Prunes the HTML tree by removing elements matched by any selector rule.
/// Optionally restores the original tree if too much text would be lost.
/// Always works on a clone of the input; the original is never mutated.
///
/// Port of `pruneUnwantedNodes`.
pub fn prune_unwanted_nodes(doc: &Document, rules: &[Rule], with_backup: bool) -> Document {
    let mut cloned = doc.clone_document();

    let (backup, old_len) = if with_backup {
        let text_len = cloned.iter_text(cloned.root(), " ").chars().count();
        (Some(cloned.clone_document()), text_len)
    } else {
        (None, 0)
    };

    // Collect all matches upfront, then remove in reverse order.
    let root = cloned.root();
    let matches = query_all(&cloned, root, rules);
    for &id in matches.iter().rev() {
        // Preserve tail text before removing.
        // Go always uses SetTail on the previous sibling (if any) or on the parent:
        //   previous = subElement.PrevSibling or subElement.Parent
        //   etree.SetTail(previous, previousTail + " " + tail)
        let tail = cloned.tail(id);
        if !tail.is_empty() {
            let target = cloned.prev_element_sibling(id).or_else(|| cloned.parent(id));
            if let Some(target_id) = target {
                let prev_tail = cloned.tail(target_id);
                let new_tail = if prev_tail.is_empty() {
                    tail
                } else {
                    format!("{prev_tail} {tail}")
                };
                cloned.set_tail(target_id, &new_tail);
            }
        }
        cloned.remove(id, false);
    }

    if with_backup {
        let new_len = cloned.iter_text(cloned.root(), " ").chars().count();
        if new_len <= old_len / 7 {
            return backup.unwrap();
        }
    }

    cloned
}

/// Converts, formats, and probes potential text elements.
/// Returns `None` if the node should be discarded.
///
/// Port of `handleTextNode`.
pub fn handle_text_node(
    doc: &mut Document,
    id: NodeId,
    cache: Option<&mut LruCache>,
    fix_comments: bool,
    preserve_spaces: bool,
    opts: &Options,
) -> Option<NodeId> {
    let tag = doc.tag_name(id).to_string();

    // Image element bypass.
    if XML_GRAPHIC_TAGS.contains(tag.as_str()) && is_image_element(doc, id) {
        return Some(id);
    }

    // If the tag is "done" or the node is empty (no children, no text, no tail), discard.
    let text = doc.text(id);
    let tail = doc.tail(id);
    // children is captured before any mutations below; it is only used for emptiness
    // checks that precede any structural changes, so the snapshot remains valid.
    let children = doc.children(id);
    if tag == "done" || (children.is_empty() && text.is_empty() && tail.is_empty()) {
        return None;
    }

    // Line break bypass.
    if !fix_comments && XML_LB_TAGS.contains(tag.as_str()) {
        if !preserve_spaces {
            let trimmed = trim(&tail);
            doc.set_tail(id, &trimmed);
        }
        return Some(id);
    }

    // If text is empty but there's tail content (and no children), promote tail to text.
    let text = doc.text(id);
    let tail = doc.tail(id);
    if text.is_empty() && children.is_empty() {
        doc.set_text(id, &tail);
        doc.set_tail(id, "");

        // Handle br/hr specially in fix_comments mode.
        if fix_comments && XML_LB_TAGS.contains(tag.as_str()) {
            doc.set_tag_name(id, "p");
        }
    }

    // Trim whitespace unless preserving spaces.
    if !preserve_spaces {
        let t = trim(&doc.text(id));
        doc.set_text(id, &t);
        let tl = trim(&doc.tail(id));
        doc.set_tail(id, &tl);
    }

    let text = doc.text(id);
    if text.is_empty() && text_filter(doc, id) {
        return None;
    }

    if opts.deduplicate {
        if let Some(cache) = cache {
            if duplicate_test(doc, id, cache, opts) {
                return None;
            }
        }
    }

    Some(id)
}

/// Checks whether an element is link-heavy (probably boilerplate).
/// Returns `(non_empty_links, is_high_density)`.
///
/// Port of `linkDensityTest`.
pub fn link_density_test(
    doc: &Document,
    element: NodeId,
    opts: &Options,
) -> (Vec<NodeId>, bool) {
    let links = doc.get_elements_by_tag_name(element, "a");
    let n_links = links.len();
    if n_links == 0 {
        return (Vec::new(), false);
    }

    let text = trim(&doc.text_content(element));
    let text_length = text.chars().count();

    // Shortcut for single-link elements.
    if n_links == 1 {
        let threshold: usize = if opts.focus == ExtractionFocus::FavorPrecision {
            10
        } else {
            100
        };
        let link_text = trim(&doc.text_content(links[0]));
        let link_text_length = link_text.chars().count();
        if link_text_length > threshold
            && link_text_length as f64 > text_length as f64 * 0.9
        {
            return (Vec::new(), true);
        }
    }

    // Determine limit based on tag and sibling position.
    let limit_length: usize = if doc.tag_name(element) == "p" {
        if doc.next_element_sibling(element).is_none() { 60 } else { 30 }
    } else if doc.next_element_sibling(element).is_none() { 300 } else { 100 };

    if text_length < limit_length {
        let (link_length, n_short_links, non_empty_links) = collect_link_info(doc, &links);
        let n_non_empty = non_empty_links.len();
        if n_non_empty == 0 {
            return (non_empty_links, true);
        }

        tracing::debug!(
            "list link text/total: {}/{}",
            link_length,
            text_length
        );
        tracing::debug!(
            "short elems/total: {}/{}",
            n_short_links,
            n_non_empty
        );

        if link_length as f64 > text_length as f64 * 0.8
            || (n_non_empty > 1
                && n_short_links as f64 / n_non_empty as f64 > 0.8)
        {
            return (non_empty_links, true);
        }
    }

    (Vec::new(), false)
}

/// Checks whether a table will be removed because it's link-heavy (probably boilerplate).
///
/// Port of `linkDensityTestTables`.
pub fn link_density_test_tables(doc: &Document, table: NodeId, _opts: &Options) -> bool {
    let links = doc.get_elements_by_tag_name(table, "a");
    if links.is_empty() {
        return false;
    }

    let text = trim(&doc.text_content(table));
    let text_length = text.chars().count();
    if text_length < 200 {
        return false;
    }

    let (link_length, _, non_empty_links) = collect_link_info(doc, &links);
    let n_non_empty = non_empty_links.len();
    if n_non_empty == 0 {
        return true;
    }

    tracing::debug!("table link text: {} / total: {}", link_length, text_length);

    if text_length < 1000 {
        link_length as f64 > text_length as f64 * 0.8
    } else {
        link_length as f64 > text_length as f64 * 0.5
    }
}

/// Collects heuristics on link text within a set of link nodes.
/// Returns `(total_link_length, n_short_links, non_empty_links)`.
///
/// Port of `collectLinkInfo`.
pub fn collect_link_info(
    doc: &Document,
    links: &[NodeId],
) -> (usize, usize, Vec<NodeId>) {
    let mut link_length = 0usize;
    let mut n_short_links = 0usize;
    let mut non_empty_links = Vec::new();

    for &link in links {
        let text = trim(&doc.text_content(link));
        let text_length = text.chars().count();
        if text_length == 0 {
            continue;
        }

        link_length += text_length;
        if text_length < 10 {
            n_short_links += 1;
        }
        non_empty_links.push(link);
    }

    (link_length, n_short_links, non_empty_links)
}

/// Converts, formats, and probes potential text elements (light format).
/// Returns `None` if the node should be discarded.
///
/// Port of `processNode`.
pub fn process_node(
    doc: &mut Document,
    id: NodeId,
    cache: Option<&mut LruCache>,
    opts: &Options,
) -> Option<NodeId> {
    let tag = doc.tag_name(id).to_string();
    let text = doc.text(id);
    let tail = doc.tail(id);
    let children = doc.children(id);

    if tag == "done" || (children.is_empty() && text.is_empty() && tail.is_empty()) {
        return None;
    }

    // Trim.
    let text = trim(&text);
    let tail = trim(&tail);
    doc.set_text(id, &text);
    doc.set_tail(id, &tail);

    // Promote tail to text if it's not a line-break tag and text is empty.
    let text = doc.text(id);
    let tail = doc.tail(id);
    if !XML_LB_TAGS.contains(tag.as_str()) && text.is_empty() && !tail.is_empty() {
        doc.set_text(id, &tail);
        doc.set_tail(id, "");
    }

    // Content checks.
    let text = doc.text(id);
    let tail = doc.tail(id);
    if !text.is_empty() || !tail.is_empty() {
        if text_filter(doc, id) {
            return None;
        }
        if let Some(cache) = cache {
            if opts.deduplicate && duplicate_test(doc, id, cache, opts) {
                return None;
            }
        }
    }

    Some(id)
}

/// Prunes unwanted nodes from a subtree IN-PLACE (no cloning).
///
/// Unlike `prune_unwanted_nodes`, this function does not clone the document —
/// it modifies `doc` directly. It also has no backup mechanism. Used by
/// `extract_content` so that "done" marks on processed elements persist across
/// selector-rule iterations, matching Go's in-place behavior.
pub fn prune_unwanted_nodes_inplace(doc: &mut Document, subtree_id: NodeId, rules: &[Rule]) {
    let matches = query_all(doc, subtree_id, rules);
    for &id in matches.iter().rev() {
        let tail = doc.tail(id);
        if !tail.is_empty() {
            let target = doc.prev_element_sibling(id).or_else(|| doc.parent(id));
            if let Some(target_id) = target {
                let prev_tail = doc.tail(target_id);
                let new_tail = if prev_tail.is_empty() {
                    tail
                } else {
                    format!("{prev_tail} {tail}")
                };
                doc.set_tail(target_id, &new_tail);
            }
        }
        doc.remove(id, false);
    }
}

/// Cleans the extracted content document.
/// Removes empty nodes and strips disallowed attributes.
///
/// Port of `postCleaning`.
pub fn post_cleaning(doc: &mut Document) {
    // Remove empty nodes (backward, so children are removed before parents).
    let children = doc.get_elements_by_tag_name(doc.root(), "*");
    for &id in children.iter().rev() {
        let grandchildren = doc.children(id);
        let is_void = doc.is_void_element(id);
        let is_empty = !text_chars_test(&doc.text(id));
        if grandchildren.is_empty() && is_empty && !is_void {
            doc.strip(id);
        }
    }

    // Remove disallowed attributes from every element.
    let elements = doc.get_elements_by_tag_name(doc.root(), "*");
    for &id in &elements {
        let tag = doc.tag_name(id).to_string();
        let allow_size = ELEMENT_WITH_SIZE_ATTR.contains(tag.as_str());
        let attr_names: Vec<String> = doc.attribute_names(id);

        for attr in attr_names {
            // Specific arms take precedence over the allowlist. Several of these attrs
            // (e.g. "style", "id", "class") appear in ALLOWED_ATTRIBUTES but must still
            // be stripped — the arms below shadow the allowlist for those cases.
            let keep = match attr.as_str() {
                // Always strip presentational/identification attrs.
                "id" | "class" | "align" | "background" | "bgcolor" | "border"
                | "cellpadding" | "cellspacing" | "frame" | "hspace" | "rules"
                | "style" | "valign" | "vspace" => false,
                // Strip width/height from non-size elements.
                "width" | "height" => allow_size,
                // Keep only if in allowlist.
                other => ALLOWED_ATTRIBUTES.contains(other),
            };

            if !keep {
                doc.remove_attribute(id, &attr);
            }
        }
    }
}

/// Deletes link-dense elements from a subtree.
///
/// Port of `deleteByLinkDensity`.
pub fn delete_by_link_density(
    doc: &mut Document,
    subtree: NodeId,
    opts: &Options,
    backtracking: bool,
    tag_names: &[&str],
) {
    let threshold: usize = if opts.focus == ExtractionFocus::FavorPrecision {
        200
    } else {
        100
    };
    let n_child_limit: usize = if opts.focus == ExtractionFocus::FavorPrecision {
        1
    } else {
        3
    };

    // iter() includes subtree itself; if the root element is link-dense it will be removed.
    let elements = if tag_names.is_empty() {
        doc.iter(subtree, &[])
    } else {
        doc.iter(subtree, tag_names)
    };

    let mut to_delete: Vec<NodeId> = Vec::new();
    for &elem in &elements {
        let (non_empty_links, is_high_density) = link_density_test(doc, elem, opts);
        if is_high_density {
            to_delete.push(elem);
        } else if backtracking && !non_empty_links.is_empty() {
            let text = trim(&doc.text_content(elem));
            let text_length = text.chars().count();
            if text_length > 0
                && text_length < threshold
                && doc.children(elem).len() >= n_child_limit
            {
                to_delete.push(elem);
            }
        }
    }

    for &id in to_delete.iter().rev() {
        doc.remove(id, false);
    }
}

/// Simplifies HTML markup by handling link conversion and code detection.
///
/// Port of `convertTags`.
pub fn convert_tags(doc: &mut Document, opts: &Options) {
    if !opts.include_links {
        // Protect links inside relevant containers.
        let css_selector = if opts.exclude_tables {
            "div a, ul a, ol a, dl a, p a".to_string()
        } else {
            "div a, ul a, ol a, dl a, p a, table a".to_string()
        };

        let important_links = doc.query_selector_all(doc.root(), &css_selector);
        for &id in &important_links {
            doc.set_tag_name(id, "protected-a");
        }

        doc.strip_tags(doc.root(), &["a"]);

        for &id in &important_links {
            doc.set_tag_name(id, "a");
        }
    } else {
        // Convert relative URLs to absolute.
        let links = doc.query_selector_all(doc.root(), "a");
        for &id in &links {
            let href = trim(&doc.get_attribute(id, "href").unwrap_or_default());
            let target = trim(&doc.get_attribute(id, "target").unwrap_or_default());

            doc.clear_attributes(id);

            if !href.is_empty() {
                let abs_href = crate::utils::url::create_absolute_url(
                    &href,
                    opts.original_url.as_ref(),
                );
                doc.set_attribute(id, "href", &abs_href);
            }
            if !target.is_empty() {
                // Note: Go source also resolves target as a URL; this mirrors upstream
                // behavior even though `target` is typically a frame name, not a URL.
                let abs_target = crate::utils::url::create_absolute_url(
                    &target,
                    opts.original_url.as_ref(),
                );
                doc.set_attribute(id, "target", &abs_target);
            }
        }
    }

    // Detect and mark code blocks.
    let quote_tags: Vec<&str> = XML_QUOTE_TAGS.iter().copied().collect();
    let quote_elems = doc.iter(doc.root(), &quote_tags);
    for &id in &quote_elems {
        let mut code_flag = false;

        // <pre> with a single <span> child is likely code.
        if doc.tag_name(id) == "pre" {
            let ch = doc.children(id);
            if ch.len() == 1 && doc.tag_name(ch[0]) == "span" {
                code_flag = true;
            }
        }

        // Find hljs span elements.
        let hljs_elems = doc.query_selector_all(
            id,
            r#"span[class*=" hljs"], span[class^="hljs"]"#,
        );
        if !hljs_elems.is_empty() {
            code_flag = true;
            for &hljs_id in &hljs_elems {
                doc.clear_attributes(hljs_id);
            }
        }

        if code_flag {
            doc.set_tag_name(id, "code");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::Document;
    use crate::options::Options;

    fn parse(html: &str) -> Document {
        Document::parse(html)
    }

    #[test]
    fn test_doc_cleaning_removes_script() {
        let mut doc = parse(r#"<html><body><script>alert(1)</script><p>text</p></body></html>"#);
        doc_cleaning(&mut doc, &Options::default());
        assert!(doc.query_selector(doc.root(), "script").is_none());
        assert!(doc.query_selector(doc.root(), "p").is_some());
    }

    #[test]
    fn test_doc_cleaning_removes_footer() {
        let mut doc = parse(r#"<html><body><footer>nav</footer><p>article</p></body></html>"#);
        doc_cleaning(&mut doc, &Options::default());
        assert!(doc.query_selector(doc.root(), "footer").is_none());
    }

    #[test]
    fn test_doc_cleaning_strips_abbr() {
        // <abbr> is in TAGS_TO_STRIP: tag is removed but children are kept.
        let mut doc = parse(r#"<html><body><p><abbr>stuff</abbr></p></body></html>"#);
        doc_cleaning(&mut doc, &Options::default());
        assert!(doc.query_selector(doc.root(), "abbr").is_none());
        // Text "stuff" should still be there (as part of the p).
        let body = doc.body().unwrap();
        assert!(doc.text_content(body).contains("stuff"));
    }

    #[test]
    fn test_doc_cleaning_include_images_keeps_figure() {
        let opts = Options { include_images: true, ..Options::default() };
        let mut doc = parse(r#"<html><body><figure><img src="x.jpg"/></figure></body></html>"#);
        doc_cleaning(&mut doc, &opts);
        assert!(doc.query_selector(doc.root(), "figure").is_some());
    }

    #[test]
    fn test_doc_cleaning_exclude_tables() {
        let opts = Options { exclude_tables: true, ..Options::default() };
        let mut doc = parse(r#"<html><body><table><tr><td>data</td></tr></table><p>text</p></body></html>"#);
        doc_cleaning(&mut doc, &opts);
        assert!(doc.query_selector(doc.root(), "table").is_none());
    }

    #[test]
    fn test_prune_html_removes_empty_div() {
        let mut doc = parse(r#"<html><body><div></div><p>text</p></body></html>"#);
        prune_html(&mut doc, &Options::default());
        assert!(doc.query_selector(doc.root(), "div").is_none());
    }

    #[test]
    fn test_prune_unwanted_nodes_removes_matched() {
        use crate::selector::discard::OVERALL_DISCARDED_CONTENT;
        let doc = parse(r#"<html><body><div class="footer">foot</div><p>text</p></body></html>"#);
        let result = prune_unwanted_nodes(&doc, OVERALL_DISCARDED_CONTENT, false);
        // Footer div should be removed.
        assert!(result.query_selector(result.root(), "div").is_none());
        // Paragraph should remain.
        assert!(result.query_selector(result.root(), "p").is_some());
    }

    #[test]
    fn test_prune_unwanted_nodes_preserves_tail_text() {
        use crate::selector::discard::OVERALL_DISCARDED_CONTENT;
        // footer div has tail text "after footer" in the body
        let doc = parse(
            r#"<html><body><p>before</p><div class="footer">nav</div>after footer</body></html>"#,
        );
        let result = prune_unwanted_nodes(&doc, OVERALL_DISCARDED_CONTENT, false);
        // Footer removed, but its tail text should survive somewhere in the tree.
        let body = result.body().unwrap();
        assert!(result.text_content(body).contains("after footer"));
    }

    #[test]
    fn test_prune_unwanted_nodes_backup_restored_on_large_removal() {
        use crate::selector::discard::OVERALL_DISCARDED_CONTENT;
        // All content is in a footer div → if removed, backup would be restored.
        let doc = parse(r#"<html><body><div class="footer">a lot of text here that will be counted as content and removed by the discard rules because it has footer class</div></body></html>"#);
        let result = prune_unwanted_nodes(&doc, OVERALL_DISCARDED_CONTENT, true);
        // Original should be restored since too much was removed.
        assert!(result.query_selector(result.root(), "div").is_some());
    }

    #[test]
    fn test_post_cleaning_strips_disallowed_attrs() {
        let mut doc = parse(r#"<html><body><p class="foo" style="color:red">text</p></body></html>"#);
        post_cleaning(&mut doc);
        let p = doc.query_selector(doc.root(), "p").unwrap();
        assert!(doc.get_attribute(p, "class").is_none());
        assert!(doc.get_attribute(p, "style").is_none());
    }

    #[test]
    fn test_post_cleaning_removes_empty_span() {
        let mut doc = parse(r#"<html><body><p><span></span>text</p></body></html>"#);
        post_cleaning(&mut doc);
        assert!(doc.query_selector(doc.root(), "span").is_none());
    }

    #[test]
    fn test_link_density_test_high_density() {
        let doc = parse(r#"<html><body><p><a href="/1">link</a></p></body></html>"#);
        let body = doc.body().unwrap();
        let p = doc.query_selector(body, "p").unwrap();
        let (_, is_dense) = link_density_test(&doc, p, &Options::default());
        // Single link where link text ≈ total text → high density.
        assert!(is_dense);
    }

    #[test]
    fn test_link_density_test_normal() {
        let doc = parse(r#"<html><body><p>This is a long paragraph with some text and <a href="x">one link</a> that is not dominant.</p></body></html>"#);
        let body = doc.body().unwrap();
        let p = doc.query_selector(body, "p").unwrap();
        let (_, is_dense) = link_density_test(&doc, p, &Options::default());
        assert!(!is_dense);
    }

    #[test]
    fn test_convert_tags_strips_links_when_not_included() {
        // Links inside p/div/ul/ol/dl are preserved (renamed to protected-a, then back).
        // Standalone links (outside those containers) are stripped.
        let opts = Options { include_links: false, ..Options::default() };
        let mut doc = parse(
            r#"<html><body><p><a href="x">inline</a></p><a href="nav">nav</a></body></html>"#,
        );
        convert_tags(&mut doc, &opts);
        let body = doc.body().unwrap();
        // Inline link inside <p> is kept as <a>.
        let p = doc.query_selector(body, "p").unwrap();
        assert!(doc.query_selector(p, "a").is_some(), "<a> inside <p> should be preserved");
        // Both text contents survive.
        let text = doc.text_content(body);
        assert!(text.contains("inline"));
        assert!(text.contains("nav"));
        // Standalone <a> directly in body should have its tag stripped.
        // After stripping, "nav" text lives directly in body, not inside an <a>.
        let direct_links = doc.get_elements_by_tag_name(body, "a");
        // Only the one inside <p> remains; the naked body-level <a> was stripped.
        assert_eq!(direct_links.len(), 1);
    }

    #[test]
    fn test_remove_html_comment_nodes() {
        let mut doc = parse(r#"<html><body><!-- comment --><p>text</p></body></html>"#);
        remove_html_comment_nodes(&mut doc);
        // Comments should be gone; paragraph should remain.
        assert!(doc.query_selector(doc.root(), "p").is_some());
    }
}
