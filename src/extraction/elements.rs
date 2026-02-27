// Port of go-trafilatura/main-extractor.go (element handler functions)

use std::collections::HashSet;

use crate::dom::{Document, NodeId};
use crate::options::{ExtractionFocus, Options};
use crate::settings::{
    XML_CELL_TAGS, XML_GRAPHIC_TAGS, XML_HEAD_TAGS, XML_HI_TAGS, XML_ITEM_TAGS, XML_LB_TAGS,
    XML_LIST_TAGS, XML_QUOTE_TAGS, XML_REF_TAGS,
};
use crate::utils::lru::LruCache;
use crate::utils::text::text_chars_test;
use crate::utils::{is_image_file, trim};

use super::html_processing::{handle_text_node, process_node};

/// Maximum nesting depth for mutually recursive list handlers.
const MAX_LIST_DEPTH: usize = 100;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Escapes special characters in an HTML attribute value.
fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

// ---------------------------------------------------------------------------
// Titles
// ---------------------------------------------------------------------------

/// Port of `handleTitles`.
pub(crate) fn handle_titles(
    doc: &mut Document,
    id: NodeId,
    cache: &mut LruCache,
    opts: &Options,
) -> Option<String> {
    // In original trafilatura, summary is treated as heading; rename to <b>.
    let orig_tag = doc.tag_name(id).to_string();
    if orig_tag == "summary" {
        doc.set_tag_name(id, "b");
    }
    let result_tag = if orig_tag == "summary" { "b" } else { orig_tag.as_str() };

    let children = doc.children(id);

    let title_valid = if children.is_empty() {
        // No children: process the node (filter/dedup check).
        process_node(doc, id, cache, opts).is_some()
    } else {
        // Has children: call handleTextNode on each child.
        // Go always appends something for every child — either the processed version
        // or the original clone if processing failed.  Since we work in-place, we
        // snapshot text/tail and restore them if handleTextNode returns None, so that
        // the child's original content is still serialized via inner_html.
        for &child_id in &children {
            let saved_text = doc.text(child_id);
            let saved_tail = doc.tail(child_id);
            let result = handle_text_node(doc, child_id, cache, false, false, opts);
            if result.is_none() {
                // Child was discarded: restore original text and tail so that
                // inner_html still includes whatever the original element contributed
                // (Go appends the original clone even when processedChild is nil).
                doc.set_text(child_id, &saved_text);
                doc.set_tail(child_id, &saved_tail);
            } else {
                // Child was kept: restore the tail verbatim to preserve
                // inter-element text (e.g. " Operator" after <code>in</code>).
                // Go works with separate text nodes so trim() never removes the
                // leading space; in our text/tail model we must do this explicitly.
                doc.set_tail(child_id, &saved_tail);
            }
            // Note: children are marked "done" AFTER inner_html is captured below.
        }
        true
    };

    if !title_valid {
        return None;
    }

    let full_text = trim(&doc.iter_text(id, ""));
    if !text_chars_test(&full_text) {
        return None;
    }

    // Capture inner HTML BEFORE marking children as "done" (mirrors Go clone-before-mark).
    let inner = doc.inner_html(id);
    for &child_id in &children {
        doc.set_tag_name(child_id, "done");
    }
    Some(format!("<{result_tag}>{inner}</{result_tag}>"))
}

// ---------------------------------------------------------------------------
// Formatting
// ---------------------------------------------------------------------------

/// Port of `handleFormatting`.
///
/// Handles formatting elements (b, i, em, etc.) found outside of paragraphs.
pub(crate) fn handle_formatting(
    doc: &mut Document,
    id: NodeId,
    cache: &mut LruCache,
    opts: &Options,
) -> Option<String> {
    let has_children = !doc.children(id).is_empty();
    let processed = process_node(doc, id, cache, opts);

    if !has_children && processed.is_none() {
        return None;
    }

    // Determine whether to wrap in <p>.
    // Go: uses element.Parent; if nil, uses element.PrevSibling.
    let parent_id = doc.parent(id).or_else(|| doc.prev_element_sibling(id));
    let should_wrap = match parent_id {
        None => true,
        Some(p_id) => {
            let ptag = doc.tag_name(p_id).to_string();
            !XML_CELL_TAGS.contains(ptag.as_str())
                && !XML_HEAD_TAGS.contains(ptag.as_str())
                && !XML_HI_TAGS.contains(ptag.as_str())
                && !XML_ITEM_TAGS.contains(ptag.as_str())
                && !XML_QUOTE_TAGS.contains(ptag.as_str())
                && ptag != "p"
        }
    };

    let tag = doc.tag_name(id).to_string();
    let inner = doc.inner_html(id);
    let elem_html = format!("<{tag}>{inner}</{tag}>");

    if should_wrap {
        Some(format!("<p>{elem_html}</p>"))
    } else {
        Some(elem_html)
    }
}

// ---------------------------------------------------------------------------
// Lists
// ---------------------------------------------------------------------------

/// Port of `processNestedElement`.
///
/// Builds the inner HTML string for a list item that has child elements.
fn process_nested_element(
    doc: &mut Document,
    child_id: NodeId,
    cache: &mut LruCache,
    opts: &Options,
    depth: usize,
) -> String {
    let mut inner = doc.text(child_id);

    let sub_elements = doc.get_elements_by_tag_name(child_id, "*");
    for &sub_id in &sub_elements {
        if doc.tag_name(sub_id) == "done" {
            continue;
        }
        let sub_tag = doc.tag_name(sub_id).to_string();

        if XML_LIST_TAGS.contains(sub_tag.as_str()) {
            // Nested list — recurse.
            if let Some(nested_html) = handle_lists_inner(doc, sub_id, cache, opts, depth + 1) {
                inner.push_str(&nested_html);
            }
        } else {
            let processed = handle_text_node(doc, sub_id, cache, false, false, opts);
            if processed.is_some() {
                let sub_text = doc.text(sub_id);
                let tail = trim(&doc.tail(sub_id));
                let stag = doc.tag_name(sub_id).to_string();
                if !sub_text.is_empty() {
                    // Port of addSubElement: copy attributes from the original element
                    // to the serialised sub-element HTML (Go: subChildElement.Attr = subElement.Attr).
                    // For link elements this preserves href/target; for others the
                    // attribute list is typically empty after handle_text_node cleared them.
                    let attrs_html: String = doc
                        .attribute_names(sub_id)
                        .into_iter()
                        .filter_map(|name| {
                            doc.get_attribute(sub_id, &name)
                                .map(|val| format!(" {}=\"{}\"", name, escape_attr(&val)))
                        })
                        .collect();
                    inner.push_str(&format!("<{stag}{attrs_html}>{sub_text}</{stag}>"));
                }
                if !tail.is_empty() {
                    inner.push(' ');
                    inner.push_str(&tail);
                }
            }
        }
        doc.set_tag_name(sub_id, "done");
    }

    // Handle tail of the item (text after it, before the next sibling element).
    // Go appends tail to the last sub-child's tail; we append to inner content.
    let item_tail = trim(&doc.tail(child_id));
    if !item_tail.is_empty() {
        inner.push(' ');
        inner.push_str(&item_tail);
    }

    inner
}

/// Port of `handleLists`.
pub(crate) fn handle_lists(
    doc: &mut Document,
    id: NodeId,
    cache: &mut LruCache,
    opts: &Options,
) -> Option<String> {
    handle_lists_inner(doc, id, cache, opts, 0)
}

fn handle_lists_inner(
    doc: &mut Document,
    id: NodeId,
    cache: &mut LruCache,
    opts: &Options,
    depth: usize,
) -> Option<String> {
    if depth >= MAX_LIST_DEPTH { return None; }
    let tag = doc.tag_name(id).to_string();
    let mut items: Vec<String> = Vec::new();

    // If the element has direct text (before any children), add it as <li>.
    let direct_text = trim(&doc.text(id));
    if !direct_text.is_empty() {
        items.push(format!("<li>{direct_text}</li>"));
    }

    // Iterate all descendants looking for list item tags (li, dt, dd).
    let descendants = doc.get_elements_by_tag_name(id, "*");
    for &desc_id in &descendants {
        let desc_tag = doc.tag_name(desc_id).to_string();
        if desc_tag == "done" || !XML_ITEM_TAGS.contains(desc_tag.as_str()) {
            continue;
        }

        let item_inner = if doc.children(desc_id).is_empty() {
            // No children: process the node directly.
            let processed = process_node(doc, desc_id, cache, opts);
            if processed.is_none() {
                doc.set_tag_name(desc_id, "done");
                continue;
            }
            let text = trim(&doc.text(desc_id));
            let tail = trim(&doc.tail(desc_id));
            let content = match (!text.is_empty(), !tail.is_empty()) {
                (true, true) => format!("{text} {tail}"),
                (true, false) => text,
                (false, true) => tail,
                (false, false) => {
                    doc.set_tag_name(desc_id, "done");
                    continue;
                }
            };
            content
        } else {
            // Has children: build inner content recursively.
            process_nested_element(doc, desc_id, cache, opts, depth)
        };

        if !item_inner.is_empty() {
            items.push(format!("<{desc_tag}>{item_inner}</{desc_tag}>"));
        }
        doc.set_tag_name(desc_id, "done");
    }

    doc.set_tag_name(id, "done");

    if items.is_empty() {
        return None;
    }

    let inner = items.join("");
    if text_chars_test(&inner) {
        Some(format!("<{tag}>{inner}</{tag}>"))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Code blocks
// ---------------------------------------------------------------------------

/// Port of `isCodeBlockElement`.
pub(crate) fn is_code_block_element(doc: &Document, id: NodeId) -> bool {
    // Pip: lang attr or code tag.
    if doc.get_attribute(id, "lang").is_some() || doc.tag_name(id) == "code" {
        return true;
    }

    // GitHub: parent has "highlight" class.
    if let Some(parent_id) = doc.parent(id) {
        if doc.class_name(parent_id).contains("highlight") {
            return true;
        }
    }

    // Highlight.js: single child that is <code>.
    let children = doc.children(id);
    if children.len() == 1 && doc.tag_name(children[0]) == "code" {
        return true;
    }

    false
}

/// Port of `handleCodeBlocks`.
///
/// Go clones the element, renames the root to `<code>`, and strips attributes
/// from all nodes in the clone.  We replicate this by clearing attributes on
/// all descendants in-place, serializing with `inner_html`, then marking
/// descendants as "done".
pub(crate) fn handle_code_blocks(doc: &mut Document, id: NodeId) -> Option<String> {
    if doc.iter_text(id, "").is_empty() {
        return None;
    }

    // Strip attributes from all descendants (Go strips attrs on every node in the clone).
    let descendants = doc.get_elements_by_tag_name(id, "*");
    for &desc_id in &descendants {
        doc.clear_attributes(desc_id);
    }

    let inner = doc.inner_html(id);

    // Mark descendants as "done" so the outer loop skips them.
    for &desc_id in &descendants {
        doc.set_tag_name(desc_id, "done");
    }

    Some(format!("<code>{inner}</code>"))
}

// ---------------------------------------------------------------------------
// Quotes
// ---------------------------------------------------------------------------

/// Port of `handleQuotes`.
pub(crate) fn handle_quotes(
    doc: &mut Document,
    id: NodeId,
    cache: &mut LruCache,
    opts: &Options,
) -> Option<String> {
    // Handle code blocks first.
    if is_code_block_element(doc, id) {
        return handle_code_blocks(doc, id);
    }

    let tag = doc.tag_name(id).to_string();

    // Process each descendant (text trimming / filter).
    let descendants = doc.get_elements_by_tag_name(id, "*");
    for &child_id in &descendants {
        if doc.tag_name(child_id) != "done" {
            let _ = process_node(doc, child_id, cache, opts);
        }
    }

    let full_text = trim(&doc.iter_text(id, ""));
    if !text_chars_test(&full_text) {
        // Mark as done even on discard so the outer loop skips these children.
        for &child_id in &descendants {
            if doc.tag_name(child_id) != "done" {
                doc.set_tag_name(child_id, "done");
            }
        }
        return None;
    }

    // Capture inner HTML BEFORE marking children as "done" — mirrors Go's clone-before-mark
    // pattern. Go builds a fresh element from processed children so the returned node never
    // contains "done" markers. We reproduce that by serialising first, then marking.
    let inner = doc.inner_html(id);
    for &child_id in &descendants {
        if doc.tag_name(child_id) != "done" {
            doc.set_tag_name(child_id, "done");
        }
    }
    Some(format!("<{tag}>{inner}</{tag}>"))
}

// ---------------------------------------------------------------------------
// Images
// ---------------------------------------------------------------------------

/// Port of `handleImage`.
///
/// Returns processed image HTML, or None if the image src is invalid.
pub(crate) fn handle_image(doc: &Document, id: NodeId) -> Option<String> {
    let tag = doc.tag_name(id).to_string();

    // Determine the best src attribute.
    let src = {
        let data_src = doc.get_attribute(id, "data-src").unwrap_or_default();
        if is_image_file(&data_src) {
            data_src
        } else {
            let src_val = doc.get_attribute(id, "src").unwrap_or_default();
            if is_image_file(&src_val) {
                src_val
            } else {
                // Check any data-src* attribute.
                let mut found = String::new();
                for attr in doc.attribute_names(id) {
                    if attr.starts_with("data-src") {
                        if let Some(val) = doc.get_attribute(id, &attr) {
                            if is_image_file(&val) {
                                found = val;
                                break;
                            }
                        }
                    }
                }
                found
            }
        }
    };

    if src.is_empty() {
        return None;
    }

    // Fix protocol-relative URLs.
    let src = if src.starts_with("//") {
        format!("http://{}", src.trim_start_matches("//"))
    } else {
        src
    };

    let mut attrs = format!(" src=\"{}\"", escape_attr(&src));
    if let Some(alt) = doc.get_attribute(id, "alt") {
        if !alt.is_empty() {
            attrs.push_str(&format!(" alt=\"{}\"", escape_attr(&alt)));
        }
    }
    if let Some(title) = doc.get_attribute(id, "title") {
        if !title.is_empty() {
            attrs.push_str(&format!(" title=\"{}\"", escape_attr(&title)));
        }
    }

    Some(format!("<{tag}{attrs}/>"))
}

/// Transforms an image element in-place for use inside paragraphs.
/// Port of the image-handling branch in `handleParagraphs`.
fn transform_image_in_place(doc: &mut Document, id: NodeId) {
    let data_src = doc.get_attribute(id, "data-src").unwrap_or_default();
    let src_val = doc.get_attribute(id, "src").unwrap_or_default();
    let alt = doc.get_attribute(id, "alt").unwrap_or_default();
    let title = doc.get_attribute(id, "title").unwrap_or_default();

    let best_src = if is_image_file(&data_src) {
        data_src
    } else if is_image_file(&src_val) {
        src_val
    } else {
        let mut found = String::new();
        for attr in doc.attribute_names(id) {
            if attr.starts_with("data-src") {
                if let Some(val) = doc.get_attribute(id, &attr) {
                    if is_image_file(&val) {
                        found = val;
                        break;
                    }
                }
            }
        }
        found
    };

    doc.clear_attributes(id);

    if best_src.is_empty() {
        return;
    }
    let src = if best_src.starts_with("//") {
        format!("http://{}", best_src.trim_start_matches("//"))
    } else {
        best_src
    };

    doc.set_attribute(id, "src", &src);
    if !alt.is_empty() {
        doc.set_attribute(id, "alt", &alt);
    }
    if !title.is_empty() {
        doc.set_attribute(id, "title", &title);
    }
}

// ---------------------------------------------------------------------------
// Other elements
// ---------------------------------------------------------------------------

/// Port of `handleOtherElements`.
pub(crate) fn handle_other_elements(
    doc: &mut Document,
    id: NodeId,
    potential_tags: &HashSet<&str>,
    cache: &mut LruCache,
    opts: &Options,
) -> Option<String> {
    let tag = doc.tag_name(id).to_string();

    // Handle W3Schools code block.
    if tag == "div" && doc.class_name(id).contains("w3-code") {
        return handle_code_blocks(doc, id);
    }

    // Skip elements not in potential tags (or "done" silently).
    if !potential_tags.contains(tag.as_str()) {
        if tag != "done" {
            tracing::debug!("discarding element: {} {:?}", tag, doc.text_content(id));
        }
        return None;
    }

    // div and details: handle as paragraphs-light.
    if tag == "div" || tag == "details" {
        let processed = handle_text_node(doc, id, cache, false, true, opts);
        if processed.is_some() {
            let text = trim(&doc.text(id));
            if text_chars_test(&text) {
                doc.clear_attributes(id);
                let result_tag = if tag == "div" { "p" } else { tag.as_str() };
                return Some(format!("<{result_tag}>{text}</{result_tag}>"));
            }
        }
    }

    tracing::debug!("unexpected element seen: {} {:?}", tag, doc.text(id));
    None
}

// ---------------------------------------------------------------------------
// Paragraphs
// ---------------------------------------------------------------------------

/// Port of `handleParagraphs`.
pub(crate) fn handle_paragraphs(
    doc: &mut Document,
    id: NodeId,
    potential_tags: &HashSet<&str>,
    cache: &mut LruCache,
    opts: &Options,
) -> Option<String> {
    // Clear attributes on the paragraph element.
    doc.clear_attributes(id);

    let children = doc.children(id);

    // Simple case: no children.
    if children.is_empty() {
        process_node(doc, id, cache, opts)?;
        let text = trim(&doc.iter_text(id, ""));
        if text.is_empty() {
            return None;
        }
        // Mirror Go's `etree.SetTail(processedElement, etree.Tail(element))`:
        // include element's tail so sibling text (e.g. after a nested <p>) is preserved.
        let tail = trim(&doc.tail(id));
        let tail_part = if !tail.is_empty() { format!(" {tail}") } else { String::new() };
        return Some(format!("<p>{text}</p>{tail_part}"));
    }

    // Complex case: paragraph has child elements.
    let all_elem_children = doc.get_elements_by_tag_name(id, "*");
    let mut unwanted: Vec<NodeId> = Vec::new();

    for &child_id in &all_elem_children {
        let child_tag = doc.tag_name(child_id).to_string();
        if child_tag == "done" {
            continue;
        }

        if !potential_tags.contains(child_tag.as_str()) {
            tracing::debug!(
                "unexpected in p: {} {:?} {:?}",
                child_tag,
                doc.text(child_id),
                doc.tail(child_id)
            );
            unwanted.push(child_id);
            continue;
        }

        let processed = handle_text_node(doc, child_id, cache, false, true, opts);
        if processed.is_none() {
            doc.set_tag_name(child_id, "done");
            continue;
        }

        match child_tag.as_str() {
            "p" => {
                // Nested <p>: merge content into parent paragraph.
                tracing::warn!(
                    "extra p within p: {} {:?} {:?}",
                    child_tag,
                    doc.text(child_id),
                    doc.tail(child_id)
                );
                let child_text = doc.text(child_id);
                let parent_text = doc.text(id);
                if !parent_text.is_empty() && !child_text.is_empty() {
                    doc.set_text(child_id, &format!(" {child_text}"));
                }
                doc.strip(child_id);
            }
            t if XML_REF_TAGS.contains(t) => {
                // Links: keep only href and target attributes.
                let href = trim(&doc.get_attribute(child_id, "href").unwrap_or_default());
                let target = trim(&doc.get_attribute(child_id, "target").unwrap_or_default());
                doc.clear_attributes(child_id);
                if !href.is_empty() {
                    doc.set_attribute(child_id, "href", &href);
                }
                if !target.is_empty() {
                    doc.set_attribute(child_id, "target", &target);
                }
            }
            t if XML_GRAPHIC_TAGS.contains(t) => {
                // Images: transform attributes in-place.
                transform_image_in_place(doc, child_id);
            }
            _ => {}
        }
    }

    // Remove unwanted children (reverse order to keep indices valid).
    for &uc in unwanted.iter().rev() {
        doc.remove(uc, false);
    }

    // Remove empty non-void children (backward).
    let remaining = doc.get_elements_by_tag_name(id, "*");
    for &child_id in remaining.iter().rev() {
        if doc.is_void_element(child_id) {
            continue;
        }
        if !text_chars_test(&doc.text(child_id)) {
            doc.strip(child_id);
        }
    }

    // Clean trailing line breaks.
    // Go condition: `br.NextSibling == nil || etree.Tail(br) == ""`
    // In Go's node model, NextSibling is a text node when there is tail text, so
    // NextSibling == nil implies tail == "".  The combined condition simplifies to
    // just checking whether the tail is empty — remove the <br> only when it has
    // no text content after it (tail is empty).
    let line_breaks: Vec<NodeId> = doc
        .get_elements_by_tag_name(id, "*")
        .into_iter()
        .filter(|&cid| matches!(doc.tag_name(cid), "br" | "hr"))
        .collect();
    for &br_id in line_breaks.iter().rev() {
        if doc.tail(br_id).is_empty() {
            doc.remove(br_id, false);
        }
    }

    // Build result.
    let result_text = doc.text(id);
    let result_children: Vec<NodeId> = doc
        .children(id)
        .into_iter()
        .filter(|&cid| doc.tag_name(cid) != "done")
        .collect();

    if !result_text.is_empty() || !result_children.is_empty() {
        // Capture HTML before marking children as "done" — mirrors Go's clone-before-mark
        // pattern in handleParagraphs. In Go, `dom.Clone(element, true)` captures the state
        // (including <br> tail text) before marking originals as "done". We reproduce that by
        // serialising the inner HTML first, then marking the in-doc nodes as done.
        let inner = doc.inner_html(id);
        tracing::debug!("keeping p-child: {inner}");
        // Mirror Go's `etree.SetTail(processedElement, etree.Tail(element))`:
        // include the element's tail text in the result so that sibling text nodes
        // (e.g. ", CCC." after a nested <p> in malformed HTML) are not lost.
        let tail = trim(&doc.tail(id));
        let tail_part = if !tail.is_empty() {
            format!(" {tail}")
        } else {
            String::new()
        };
        // Mark all element children as done to prevent re-processing.
        for &child_id in &all_elem_children {
            if doc.tag_name(child_id) != "done" {
                doc.set_tag_name(child_id, "done");
            }
        }
        return Some(format!("<p>{inner}</p>{tail_part}"));
    }

    tracing::debug!("discarding p-child: {}", doc.outer_html(id));
    None
}

// ---------------------------------------------------------------------------
// Tables
// ---------------------------------------------------------------------------

/// Port of `handleTable`.
pub(crate) fn handle_table(
    doc: &mut Document,
    id: NodeId,
    potential_tags: &HashSet<&str>,
    cache: &mut LruCache,
    opts: &Options,
) -> Option<String> {
    // Potential tags with "div" added for table cell content.
    let mut potential_tags_with_div = potential_tags.clone();
    potential_tags_with_div.insert("div");

    // Strip structural wrappers (thead, tbody, tfoot).
    doc.strip_tags(id, &["thead", "tbody", "tfoot"]);

    let mut rows: Vec<String> = Vec::new();
    let mut current_row_cells: Vec<String> = Vec::new();

    let descendants = doc.get_elements_by_tag_name(id, "*");
    for &sub_id in &descendants {
        let sub_tag = doc.tag_name(sub_id).to_string();
        if sub_tag == "done" {
            continue;
        }

        if sub_tag == "tr" {
            if !current_row_cells.is_empty() {
                rows.push(format!("<tr>{}</tr>", current_row_cells.join("")));
                current_row_cells.clear();
            }
        } else if sub_tag == "td" || sub_tag == "th" {
            if let Some(cell_content) =
                build_cell_content(doc, sub_id, &potential_tags_with_div, cache, opts)
            {
                current_row_cells.push(format!("<{sub_tag}>{cell_content}</{sub_tag}>"));
            }
        } else if sub_tag == "table" {
            // Nested table: stop processing.
            break;
        }

        doc.set_tag_name(sub_id, "done");
    }

    if !current_row_cells.is_empty() {
        rows.push(format!("<tr>{}</tr>", current_row_cells.join("")));
    }

    if rows.is_empty() {
        return None;
    }

    Some(format!("<table>{}</table>", rows.join("")))
}

/// Builds content string for a single table cell (td/th).
fn build_cell_content(
    doc: &mut Document,
    cell_id: NodeId,
    potential_tags: &HashSet<&str>,
    cache: &mut LruCache,
    opts: &Options,
) -> Option<String> {
    let children = doc.children(cell_id);

    if children.is_empty() {
        // Childless cell: processNode.
        process_node(doc, cell_id, cache, opts)?;
        let text = trim(&doc.text(cell_id));
        let tail = trim(&doc.tail(cell_id));
        let content = if !text.is_empty() && !tail.is_empty() {
            format!("{text} {tail}")
        } else if !text.is_empty() {
            text
        } else {
            tail
        };
        return if content.is_empty() { None } else { Some(content) };
    }

    // Cell has children: iterate and build content.
    let mut cell_inner = doc.text(cell_id);
    // Mark cell as done to prevent double-processing of cell element itself.
    doc.set_tag_name(cell_id, "done");

    let sub_elements = doc.get_elements_by_tag_name(cell_id, "*");
    for &child_id in &sub_elements {
        if doc.tag_name(child_id) == "done" {
            continue;
        }
        let child_tag = doc.tag_name(child_id).to_string();

        let sub_html =
            if XML_CELL_TAGS.contains(child_tag.as_str()) || XML_HI_TAGS.contains(child_tag.as_str()) {
                let processed = handle_text_node(doc, child_id, cache, true, false, opts);
                if processed.is_some() {
                    let text = trim(&doc.text(child_id));
                    let tail = trim(&doc.tail(child_id));
                    let t = &child_tag;
                    if !text.is_empty() {
                        Some(if !tail.is_empty() {
                            format!("<{t}>{text}</{t}> {tail}")
                        } else {
                            format!("<{t}>{text}</{t}>")
                        })
                    } else if !tail.is_empty() {
                        Some(tail)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else if XML_LIST_TAGS.contains(child_tag.as_str())
                && opts.focus == ExtractionFocus::FavorRecall
            {
                handle_lists(doc, child_id, cache, opts)
            } else {
                handle_text_elem(doc, child_id, potential_tags, cache, opts)
            };

        if let Some(html) = sub_html {
            cell_inner.push_str(&html);
        }
        doc.set_tag_name(child_id, "done");
    }

    if cell_inner.is_empty() {
        None
    } else {
        Some(cell_inner)
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

/// Port of `handleTextElem`.
///
/// Dispatches to the appropriate handler based on the element's tag name.
pub(crate) fn handle_text_elem(
    doc: &mut Document,
    id: NodeId,
    potential_tags: &HashSet<&str>,
    cache: &mut LruCache,
    opts: &Options,
) -> Option<String> {
    let tag = doc.tag_name(id).to_string();

    if XML_LIST_TAGS.contains(tag.as_str()) {
        handle_lists(doc, id, cache, opts)
    } else if XML_QUOTE_TAGS.contains(tag.as_str()) || tag == "code" {
        handle_quotes(doc, id, cache, opts)
    } else if XML_HEAD_TAGS.contains(tag.as_str()) {
        handle_titles(doc, id, cache, opts)
    } else if tag == "p" {
        handle_paragraphs(doc, id, potential_tags, cache, opts)
    } else if XML_LB_TAGS.contains(tag.as_str()) {
        // Line break: if tail has text content, promote it to a <p>.
        let tail = trim(&doc.tail(id));
        if text_chars_test(&tail) && process_node(doc, id, cache, opts).is_some() {
            let tail_text = trim(&doc.tail(id));
            if !tail_text.is_empty() {
                return Some(format!("<p>{tail_text}</p>"));
            }
        }
        None
    } else if XML_HI_TAGS.contains(tag.as_str())
        || XML_REF_TAGS.contains(tag.as_str())
        || tag == "span"
    {
        handle_formatting(doc, id, cache, opts)
    } else if tag == "table" {
        if potential_tags.contains("table") {
            handle_table(doc, id, potential_tags, cache, opts)
        } else {
            None
        }
    } else if XML_GRAPHIC_TAGS.contains(tag.as_str()) {
        if potential_tags.contains("img") {
            handle_image(doc, id)
        } else {
            None
        }
    } else {
        handle_other_elements(doc, id, potential_tags, cache, opts)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::Document;
    use crate::options::Options;
    use crate::utils::lru::LruCache;

    fn make_cache() -> LruCache {
        LruCache::new(500)
    }

    fn parse_elem(html: &str) -> (Document, NodeId) {
        let doc = Document::parse(&format!("<html><body>{html}</body></html>"));
        let body = doc.body().unwrap();
        let first = doc.children(body)[0];
        (doc, first)
    }

    // ---------------------------------------------------------------------------
    // handle_titles
    // ---------------------------------------------------------------------------

    #[test]
    fn test_handle_titles_simple() {
        let (mut doc, id) = parse_elem("<h1>Hello World</h1>");
        let opts = Options::default();
        let mut cache = make_cache();
        let result = handle_titles(&mut doc, id, &mut cache, &opts);
        let html = result.expect("expected Some");
        assert!(html.contains("Hello World"), "got: {html}");
        assert!(html.starts_with("<h1>"), "got: {html}");
    }

    #[test]
    fn test_handle_titles_empty() {
        let (mut doc, id) = parse_elem("<h2></h2>");
        let opts = Options::default();
        let mut cache = make_cache();
        assert!(handle_titles(&mut doc, id, &mut cache, &opts).is_none());
    }

    #[test]
    fn test_handle_titles_summary_renamed() {
        let (mut doc, id) = parse_elem("<summary>Collapsed</summary>");
        let opts = Options::default();
        let mut cache = make_cache();
        let html = handle_titles(&mut doc, id, &mut cache, &opts).expect("expected Some");
        assert!(html.starts_with("<b>"), "summary should become <b>, got: {html}");
    }

    #[test]
    fn test_handle_titles_with_children() {
        let (mut doc, id) = parse_elem("<h1>Article <strong>Title</strong></h1>");
        let opts = Options::default();
        let mut cache = make_cache();
        let html = handle_titles(&mut doc, id, &mut cache, &opts).expect("expected Some");
        assert!(html.contains("Article"), "got: {html}");
        assert!(html.contains("Title"), "got: {html}");
    }

    // ---------------------------------------------------------------------------
    // handle_image
    // ---------------------------------------------------------------------------

    #[test]
    fn test_handle_image_src() {
        let (doc, id) = parse_elem(r#"<img src="photo.jpg" alt="A photo"/>"#);
        let html = handle_image(&doc, id).expect("expected Some");
        assert!(html.contains("photo.jpg"), "got: {html}");
        assert!(html.contains(r#"alt="A photo""#), "got: {html}");
    }

    #[test]
    fn test_handle_image_data_src() {
        let (doc, id) = parse_elem(r#"<img data-src="lazy.png"/>"#);
        let html = handle_image(&doc, id).expect("expected Some");
        assert!(html.contains("lazy.png"), "got: {html}");
    }

    #[test]
    fn test_handle_image_no_src() {
        let (doc, id) = parse_elem(r#"<img alt="no src"/>"#);
        assert!(handle_image(&doc, id).is_none());
    }

    #[test]
    fn test_handle_image_protocol_relative() {
        let (doc, id) = parse_elem(r#"<img src="//example.com/img.jpg"/>"#);
        let html = handle_image(&doc, id).expect("expected Some");
        assert!(html.contains("http://example.com/img.jpg"), "got: {html}");
    }

    // ---------------------------------------------------------------------------
    // handle_lists
    // ---------------------------------------------------------------------------

    #[test]
    fn test_handle_lists_simple() {
        let (mut doc, id) = parse_elem("<ul><li>Item 1</li><li>Item 2</li></ul>");
        let opts = Options::default();
        let mut cache = make_cache();
        let html = handle_lists(&mut doc, id, &mut cache, &opts).expect("expected Some");
        assert!(html.starts_with("<ul>"), "got: {html}");
        assert!(html.contains("Item 1"), "got: {html}");
        assert!(html.contains("Item 2"), "got: {html}");
    }

    #[test]
    fn test_handle_lists_empty() {
        let (mut doc, id) = parse_elem("<ul><li>  </li></ul>");
        let opts = Options::default();
        let mut cache = make_cache();
        assert!(handle_lists(&mut doc, id, &mut cache, &opts).is_none());
    }

    #[test]
    fn test_handle_lists_ordered() {
        let (mut doc, id) = parse_elem("<ol><li>First</li><li>Second</li></ol>");
        let opts = Options::default();
        let mut cache = make_cache();
        let html = handle_lists(&mut doc, id, &mut cache, &opts).expect("expected Some");
        assert!(html.starts_with("<ol>"), "got: {html}");
    }

    // ---------------------------------------------------------------------------
    // handle_paragraphs
    // ---------------------------------------------------------------------------

    #[test]
    fn test_handle_paragraphs_simple() {
        let (mut doc, id) = parse_elem("<p>Hello world of content.</p>");
        let opts = Options::default();
        let mut cache = make_cache();
        let potential_tags: HashSet<&str> = crate::settings::TAG_CATALOG.iter().copied().collect();
        let html = handle_paragraphs(&mut doc, id, &potential_tags, &mut cache, &opts)
            .expect("expected Some");
        assert!(html.contains("Hello world"), "got: {html}");
    }

    #[test]
    fn test_handle_paragraphs_with_link() {
        let (mut doc, id) =
            parse_elem(r#"<p>Read <a href="http://example.com" class="x">more</a> here.</p>"#);
        let opts = Options::default();
        let mut cache = make_cache();
        let mut potential_tags: HashSet<&str> =
            crate::settings::TAG_CATALOG.iter().copied().collect();
        potential_tags.insert("a");
        let html = handle_paragraphs(&mut doc, id, &potential_tags, &mut cache, &opts)
            .expect("expected Some");
        // href preserved, class attribute stripped
        assert!(html.contains(r#"href="http://example.com""#), "got: {html}");
        assert!(!html.contains("class="), "class should be stripped, got: {html}");
    }

    // ---------------------------------------------------------------------------
    // is_code_block_element
    // ---------------------------------------------------------------------------

    #[test]
    fn test_is_code_block_element_code_tag() {
        let (doc, id) = parse_elem("<code>x = 1</code>");
        assert!(is_code_block_element(&doc, id));
    }

    #[test]
    fn test_is_code_block_element_lang_attr() {
        let (doc, id) = parse_elem(r#"<pre lang="python">x = 1</pre>"#);
        assert!(is_code_block_element(&doc, id));
    }

    #[test]
    fn test_is_code_block_element_not_code() {
        let (doc, id) = parse_elem("<pre>plain preformatted</pre>");
        assert!(!is_code_block_element(&doc, id));
    }

    // ---------------------------------------------------------------------------
    // handle_text_elem (dispatch)
    // ---------------------------------------------------------------------------

    #[test]
    fn test_handle_text_elem_heading() {
        let (mut doc, id) = parse_elem("<h2>Section Title</h2>");
        let opts = Options::default();
        let mut cache = make_cache();
        let tags: HashSet<&str> = crate::settings::TAG_CATALOG.iter().copied().collect();
        let html = handle_text_elem(&mut doc, id, &tags, &mut cache, &opts).expect("expected Some");
        assert!(html.contains("Section Title"), "got: {html}");
    }

    #[test]
    fn test_handle_text_elem_list() {
        let (mut doc, id) = parse_elem("<ul><li>Apple</li><li>Banana</li></ul>");
        let opts = Options::default();
        let mut cache = make_cache();
        let tags: HashSet<&str> = crate::settings::TAG_CATALOG.iter().copied().collect();
        let html = handle_text_elem(&mut doc, id, &tags, &mut cache, &opts).expect("expected Some");
        assert!(html.contains("Apple"), "got: {html}");
    }

    #[test]
    fn test_handle_text_elem_done() {
        let (mut doc, id) = parse_elem("<div>content</div>");
        doc.set_tag_name(id, "done");
        let opts = Options::default();
        let mut cache = make_cache();
        let tags: HashSet<&str> = crate::settings::TAG_CATALOG.iter().copied().collect();
        assert!(handle_text_elem(&mut doc, id, &tags, &mut cache, &opts).is_none());
    }

    // ---------------------------------------------------------------------------
    // handle_code_blocks
    // ---------------------------------------------------------------------------

    #[test]
    fn test_handle_code_blocks_plain_text() {
        let (mut doc, id) = parse_elem("<pre>let x = 1;</pre>");
        let html = handle_code_blocks(&mut doc, id).expect("expected Some");
        assert!(html.starts_with("<code>"), "got: {html}");
        assert!(html.contains("let x = 1;"), "got: {html}");
    }

    #[test]
    fn test_handle_code_blocks_strips_attrs_preserves_structure() {
        // Go preserves nested element structure but strips attributes.
        let (mut doc, id) =
            parse_elem(r#"<pre><span class="kw">if</span> x == 1 {}</pre>"#);
        let html = handle_code_blocks(&mut doc, id).expect("expected Some");
        assert!(html.starts_with("<code>"), "got: {html}");
        // Nested <span> structure should be preserved.
        assert!(html.contains("<span>"), "span structure lost: {html}");
        // But class attribute should be stripped.
        assert!(!html.contains("class="), "class attr should be stripped: {html}");
        assert!(html.contains("if"), "text content lost: {html}");
    }

    #[test]
    fn test_handle_code_blocks_empty_returns_none() {
        let (mut doc, id) = parse_elem("<pre></pre>");
        assert!(handle_code_blocks(&mut doc, id).is_none());
    }

    // ---------------------------------------------------------------------------
    // handle_formatting (wrap heuristic)
    // ---------------------------------------------------------------------------

    #[test]
    fn test_handle_formatting_wrap_in_p_when_orphaned() {
        // Formatting element with no parent context → should be wrapped in <p>.
        let (mut doc, id) = parse_elem("<b>Bold text here for testing.</b>");
        let opts = Options::default();
        let mut cache = make_cache();
        let html = handle_formatting(&mut doc, id, &mut cache, &opts).expect("expected Some");
        assert!(html.starts_with("<p>"), "should be wrapped in <p>, got: {html}");
        assert!(html.contains("<b>"), "got: {html}");
    }

    // ---------------------------------------------------------------------------
    // handle_lists (nested)
    // ---------------------------------------------------------------------------

    #[test]
    fn test_handle_lists_nested() {
        let (mut doc, id) = parse_elem(
            "<ul><li>Item <ul><li>Sub-item</li></ul></li></ul>",
        );
        let opts = Options::default();
        let mut cache = make_cache();
        let html = handle_lists(&mut doc, id, &mut cache, &opts).expect("expected Some");
        assert!(html.contains("Item"), "outer item missing: {html}");
        assert!(html.contains("Sub-item"), "nested item missing: {html}");
    }

    // ---------------------------------------------------------------------------
    // handle_table
    // ---------------------------------------------------------------------------

    #[test]
    fn test_handle_table_basic() {
        let (mut doc, id) = parse_elem(
            "<table><tr><td>Cell A</td><td>Cell B</td></tr><tr><td>Cell C</td></tr></table>",
        );
        let opts = Options::default();
        let mut cache = make_cache();
        let potential_tags: HashSet<&str> = crate::settings::TAG_CATALOG.iter().copied().collect();
        let html = handle_table(&mut doc, id, &potential_tags, &mut cache, &opts)
            .expect("expected Some");
        assert!(html.starts_with("<table>"), "got: {html}");
        assert!(html.contains("Cell A"), "got: {html}");
        assert!(html.contains("Cell B"), "got: {html}");
        assert!(html.contains("Cell C"), "got: {html}");
        assert!(html.contains("<tr>"), "rows missing: {html}");
        assert!(html.contains("<td>"), "cells missing: {html}");
    }

    #[test]
    fn test_handle_table_empty_returns_none() {
        let (mut doc, id) = parse_elem("<table><tr><td>  </td></tr></table>");
        let opts = Options::default();
        let mut cache = make_cache();
        let potential_tags: HashSet<&str> = crate::settings::TAG_CATALOG.iter().copied().collect();
        assert!(handle_table(&mut doc, id, &potential_tags, &mut cache, &opts).is_none());
    }

    // ---------------------------------------------------------------------------
    // handle_image (attribute escaping)
    // ---------------------------------------------------------------------------

    #[test]
    fn test_handle_image_escapes_alt_attribute() {
        let (doc, id) = parse_elem(r#"<img src="photo.jpg" alt='Say "hello" & goodbye'/>"#);
        let html = handle_image(&doc, id).expect("expected Some");
        // Quotes in alt must be escaped so the HTML is valid.
        assert!(html.contains("&amp;"), "& not escaped: {html}");
        assert!(!html.contains(" & "), "raw & in attribute: {html}");
    }
}
