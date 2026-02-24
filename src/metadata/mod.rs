// Port of go-trafilatura/metadata.go

pub mod json_ld;

use std::collections::HashSet;
use std::sync::LazyLock;

use crate::dom::Document;
use crate::options::Options;
use crate::result::Metadata;
use crate::selector::metadata::{
    META_AUTHOR, META_AUTHOR_DISCARD, META_CATEGORIES, META_TAGS, META_TITLE,
};
use crate::selector::query_all;
use crate::utils::regex_patterns::{
    AUTHOR_DIGITS, AUTHOR_EMAIL, AUTHOR_HTML, AUTHOR_NICKNAME, AUTHOR_PREFIX, AUTHOR_PREPOSITION,
    AUTHOR_SEPARATOR, AUTHOR_SOCIAL_MEDIA, AUTHOR_SPACE_CHARS, AUTHOR_SPECIAL_CHARS, CC_LICENSE,
    CC_LICENSE_TEXT, CATEGORY_HREF, HTML_STRIP_TAG, SITENAME_FINDER, TAG_HREF, TITLE_CLEANER,
    URL_CHECK,
};
use crate::utils::url::{get_base_url, validate_url};
use crate::utils::{remove_emojis, str_or, trim, unescape_html, uniquify_lists};
use crate::extraction::html_processing::prune_unwanted_nodes;

// ---------------------------------------------------------------------------
// Static meta-name lookup sets
// ---------------------------------------------------------------------------

static META_NAME_AUTHOR: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "article:author", "atc-metaauthor", "author", "authors", "byl",
        "citation_author", "creator", "dc.creator", "dc.creator.aut", "dc:creator",
        "dcterms.creator", "dcterms.creator.aut", "dcsext.author", "parsely-author",
        "rbauthors", "sailthru.author", "shareaholic:article_author_name",
    ]
    .into_iter()
    .collect()
});

static META_NAME_TITLE: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "citation_title", "dc.title", "dcterms.title", "fb_title",
        "headline", "parsely-title", "sailthru.title", "shareaholic:title",
        "rbtitle", "title", "twitter:title",
    ]
    .into_iter()
    .collect()
});

static META_NAME_DESCRIPTION: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "dc.description", "dc:description",
        "dcterms.abstract", "dcterms.description",
        "description", "sailthru.description", "twitter:description",
    ]
    .into_iter()
    .collect()
});

static META_NAME_PUBLISHER: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "article:publisher", "citation_journal_title", "copyright",
        "dc.publisher", "dc:publisher", "dcterms.publisher",
        "publisher", "sailthru.publisher", "rbpubname", "twitter:site",
    ]
    .into_iter()
    .collect()
});

static META_NAME_TAG: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "citation_keywords", "dcterms.subject", "keywords",
        "parsely-tags", "shareaholic:keywords", "tags",
    ]
    .into_iter()
    .collect()
});

static META_NAME_IMAGE: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "image", "og:image", "og:image:url", "og:image:secure_url",
        "twitter:image", "twitter:image:src",
    ]
    .into_iter()
    .collect()
});

static URL_SELECTORS: &[&str] = &[
    r#"head link[rel="canonical"]"#,
    "head base",
    r#"head link[rel="alternate"][hreflang="x-default"]"#,
];

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Extracts all metadata from the document.
///
/// Port of `extractMetadata`.
pub fn extract_metadata(doc: &Document, opts: &Options) -> Metadata {
    // Extract from <meta> tags (includes OpenGraph).
    let mut metadata = examine_meta(doc);
    metadata.author = remove_blacklisted_authors(&metadata.author, opts);

    // Extract from JSON-LD and override.
    metadata = json_ld::extract_json_ld(opts, doc, metadata);
    metadata.author = remove_blacklisted_authors(&metadata.author, opts);

    // Title fallback via DOM selectors.
    if metadata.title.is_empty() {
        metadata.title = extract_dom_title(doc);
    }

    // Author fallback via DOM selectors.
    if metadata.author.is_empty() {
        metadata.author = extract_dom_author(doc);
        metadata.author = remove_blacklisted_authors(&metadata.author, opts);
    }

    // URL fallback via canonical link.
    if metadata.url.is_empty() {
        metadata.url = extract_dom_url(doc);
    }

    // Validate URL — must be absolute.
    if !metadata.url.is_empty() {
        let (valid, is_abs) = validate_url(&metadata.url, opts.original_url.as_ref());
        if !valid.is_empty() && is_abs {
            metadata.url = valid;
        } else {
            metadata.url = String::new();
        }
    }

    // If URL not found but original URL given, use that.
    if metadata.url.is_empty() {
        if let Some(orig) = &opts.original_url {
            metadata.url = orig.to_string();
        }
    }

    // Hostname from URL.
    if !metadata.url.is_empty() {
        use crate::utils::url::get_domain_url;
        metadata.hostname = get_domain_url(&metadata.url);
    }

    // Validate image URL — must be absolute.
    if !metadata.image.is_empty() {
        let (valid, is_abs) = validate_url(&metadata.image, opts.original_url.as_ref());
        if !valid.is_empty() && is_abs {
            metadata.image = valid;
        } else {
            metadata.image = String::new();
        }
    }

    // Date extraction: we don't have an htmldate equivalent yet.
    // metadata.date remains None.

    // Sitename fallback via DOM.
    if metadata.sitename.is_empty() {
        metadata.sitename = extract_dom_sitename(doc);
    }

    if !metadata.sitename.is_empty() {
        // Strip leading Twitter @.
        metadata.sitename = metadata.sitename.trim_start_matches('@').to_string();

        // Title-case if it looks like a word (no dot, not already titled).
        let first = metadata.sitename.chars().next();
        if !metadata.sitename.contains('.')
            && !first.map(|c| c.is_uppercase()).unwrap_or(false)
        {
            metadata.sitename = title_case(&metadata.sitename);
        }
    } else if !metadata.url.is_empty() {
        // Last resort: extract sitename from URL.
        if let Some(caps) = SITENAME_FINDER.captures(&metadata.url) {
            metadata.sitename = caps[1].to_string();
        }
    }

    // Categories fallback via DOM.
    if metadata.categories.is_empty() {
        metadata.categories = extract_dom_categories(doc);
    }
    if !metadata.categories.is_empty() {
        metadata.categories = clean_cat_tags(metadata.categories);
    }

    // Tags fallback via DOM.
    if metadata.tags.is_empty() {
        metadata.tags = extract_dom_tags(doc);
    }
    if !metadata.tags.is_empty() {
        metadata.tags = clean_cat_tags(metadata.tags);
    }

    // License.
    metadata.license = extract_license(doc);

    metadata
}

// ---------------------------------------------------------------------------
// Meta tag scanning
// ---------------------------------------------------------------------------

/// Port of `examineMeta`.
fn examine_meta(doc: &Document) -> Metadata {
    let mut metadata = extract_open_graph_meta(doc);

    // Short-circuit if all fields filled.
    if !metadata.title.is_empty()
        && !metadata.author.is_empty()
        && !metadata.url.is_empty()
        && !metadata.description.is_empty()
        && !metadata.sitename.is_empty()
        && !metadata.image.is_empty()
        && !metadata.page_type.is_empty()
    {
        return metadata;
    }

    let mut tmp_sitename = String::new();

    for node_id in doc.query_selector_all(doc.root(), "head meta[content]") {
        let content = doc.get_attribute(node_id, "content").unwrap_or_default();
        let content = HTML_STRIP_TAG.replace_all(&content, "");
        let content = unescape_html(&content);
        let content = trim(&content);
        if content.is_empty() {
            continue;
        }

        // Handle property attribute.
        let property = trim(&doc.get_attribute(node_id, "property").unwrap_or_default());
        if !property.is_empty() {
            if property.starts_with("og:") {
                // Already handled by extract_open_graph_meta.
            } else if property == "article:tag" {
                metadata.tags.push(content);
            } else if property == "author" || property == "article:author" {
                metadata.author = normalize_authors(&metadata.author, &content);
            } else if property == "article:publisher" {
                metadata.sitename = str_or(&[&metadata.sitename, &content]).to_string();
            } else if META_NAME_IMAGE.contains(property.as_str()) {
                metadata.image = str_or(&[&metadata.image, &content]).to_string();
            }
            continue;
        }

        // Handle name attribute.
        let name = doc.get_attribute(node_id, "name").unwrap_or_default();
        let name = trim(&name.to_lowercase());
        if !name.is_empty() {
            if META_NAME_AUTHOR.contains(name.as_str()) {
                let content = HTML_STRIP_TAG.replace_all(&content, "").to_string();
                metadata.author = normalize_authors(&metadata.author, &content);
            } else if META_NAME_TITLE.contains(name.as_str()) {
                metadata.title = str_or(&[&metadata.title, &content]).to_string();
            } else if META_NAME_DESCRIPTION.contains(name.as_str()) {
                metadata.description = str_or(&[&metadata.description, &content]).to_string();
            } else if META_NAME_PUBLISHER.contains(name.as_str()) {
                metadata.sitename = str_or(&[&metadata.sitename, &content]).to_string();
            } else if name == "twitter:site"
                || name == "application-name"
                || name.contains("twitter:app:name")
            {
                tmp_sitename = content;
            } else if name == "twitter:url" {
                if metadata.url.is_empty() {
                    let (_, is_abs) = validate_url(&content, None);
                    if is_abs {
                        metadata.url = content;
                    }
                }
            } else if META_NAME_TAG.contains(name.as_str()) {
                metadata.tags.push(content);
            }
            continue;
        }

        // Handle itemprop attribute.
        let itemprop = trim(&doc.get_attribute(node_id, "itemprop").unwrap_or_default());
        if !itemprop.is_empty() {
            match itemprop.as_str() {
                "author" => {
                    metadata.author = normalize_authors(&metadata.author, &content);
                }
                "description" => {
                    metadata.description =
                        str_or(&[&metadata.description, &content]).to_string();
                }
                "headline" => {
                    metadata.title = str_or(&[&metadata.title, &content]).to_string();
                }
                _ => {}
            }
        }
    }

    // Use temporary sitename if necessary.
    if metadata.sitename.is_empty() && !tmp_sitename.is_empty() {
        metadata.sitename = tmp_sitename;
    }

    metadata.author = validate_metadata_name(&metadata.author);

    let cat_strs: Vec<&str> = metadata.categories.iter().map(|s| s.as_str()).collect();
    metadata.categories = uniquify_lists(&cat_strs);
    let tag_strs: Vec<&str> = metadata.tags.iter().map(|s| s.as_str()).collect();
    metadata.tags = uniquify_lists(&tag_strs);

    metadata
}

/// Port of `extractOpenGraphMeta`.
fn extract_open_graph_meta(doc: &Document) -> Metadata {
    let mut metadata = Metadata::default();

    for node_id in doc.query_selector_all(doc.root(), r#"meta[property^="og:"]"#) {
        let prop = trim(&doc.get_attribute(node_id, "property").unwrap_or_default());
        let content = trim(&unescape_html(
            &doc.get_attribute(node_id, "content").unwrap_or_default(),
        ));
        if content.is_empty() {
            continue;
        }

        match prop.as_str() {
            "og:site_name" => metadata.sitename = content,
            "og:title" => metadata.title = content,
            "og:description" => metadata.description = content,
            "og:author" | "og:article:author" => {
                metadata.author = normalize_authors("", &content);
            }
            "og:image" | "og:image:url" | "og:image:secure_url" => {
                metadata.image = content;
            }
            "og:url" => {
                let (_, is_abs) = validate_url(&content, None);
                if is_abs {
                    metadata.url = content;
                }
            }
            "og:article:tag" => {
                metadata.tags = uniquify_lists(&[&content]);
            }
            "og:type" => metadata.page_type = content,
            _ => {}
        }
    }

    metadata
}

// ---------------------------------------------------------------------------
// Metadata name validation
// ---------------------------------------------------------------------------

/// Port of `validateMetadataName`.
pub fn validate_metadata_name(name: &str) -> String {
    if name.is_empty() {
        return String::new();
    }

    // Must contain a space (multi-word) and not start with "http".
    if !name.contains(' ') || name.starts_with("http") {
        return String::new();
    }

    // Reject strings that look like JSON (contain JSON special characters).
    use crate::utils::regex_patterns::JSON_SYMBOL;
    if JSON_SYMBOL.is_match(name) {
        return String::new();
    }

    name.to_string()
}

// ---------------------------------------------------------------------------
// DOM-based title / author / URL / sitename extraction
// ---------------------------------------------------------------------------

/// Examines `<title>` element and returns (full_title, first_part, second_part).
///
/// Port of `examineTitleElement`.
fn examine_title_element(doc: &Document) -> (String, String, String) {
    let Some(title_id) = doc.query_selector(doc.root(), "head > title") else {
        return (String::new(), String::new(), String::new());
    };

    let title = trim(&doc.text_content(title_id));
    if title.is_empty() {
        return (String::new(), String::new(), String::new());
    }

    if let Some(caps) = TITLE_CLEANER.captures(&title) {
        let first = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
        let second = caps.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
        (title, first, second)
    } else {
        (title, String::new(), String::new())
    }
}

/// Port of `extractDomTitle`.
fn extract_dom_title(doc: &Document) -> String {
    // Single H1 → use as title.
    let h1_nodes = doc.query_selector_all(doc.root(), "h1");
    if h1_nodes.len() == 1 {
        let title = trim(&doc.text_content(h1_nodes[0]));
        if !title.is_empty() {
            return title;
        }
    }

    // DOM meta selectors.
    let title = extract_dom_meta_selectors(doc, 200, META_TITLE);
    if !title.is_empty() {
        return title;
    }

    // <title> element.
    let (full_title, first, second) = examine_title_element(doc);
    if !first.is_empty() && !first.contains('.') {
        return first;
    } else if !second.is_empty() && !second.contains('.') {
        return second;
    } else if !full_title.is_empty() {
        return full_title;
    }

    // First H1 as fallback.
    if !h1_nodes.is_empty() {
        return trim(&doc.text_content(h1_nodes[0]));
    }

    // First H2 as last resort.
    if let Some(h2_id) = doc.query_selector(doc.root(), "h2") {
        return trim(&doc.text_content(h2_id));
    }

    String::new()
}

/// Port of `extractDomAuthor`.
fn extract_dom_author(doc: &Document) -> String {
    // Prune elements that might confuse author detection.
    let clone = prune_unwanted_nodes(doc, META_AUTHOR_DISCARD, false);

    let author = extract_dom_meta_selectors(&clone, 120, META_AUTHOR);
    if !author.is_empty() {
        return normalize_authors("", &author);
    }

    String::new()
}

/// Port of `extractDomURL`.
fn extract_dom_url(doc: &Document) -> String {
    let mut url = String::new();

    for &sel in URL_SELECTORS {
        if let Some(elem_id) = doc.query_selector(doc.root(), sel) {
            let href = trim(&doc.get_attribute(elem_id, "href").unwrap_or_default());
            if !href.is_empty() {
                url = href;
                break;
            }
        }
    }

    // Fix relative URLs — add domain name if missing.
    if !url.is_empty() && url.starts_with('/') {
        for node_id in doc.query_selector_all(doc.root(), "head meta[content]") {
            let node_name = trim(&doc.get_attribute(node_id, "name").unwrap_or_default());
            let node_property =
                trim(&doc.get_attribute(node_id, "property").unwrap_or_default());
            let attr_type = str_or(&[&node_name, &node_property]).to_string();
            if attr_type.is_empty() {
                continue;
            }

            if attr_type.starts_with("og:") || attr_type.starts_with("twitter:") {
                let content = trim(&doc.get_attribute(node_id, "content").unwrap_or_default());
                let base = get_base_url(&content);
                if !base.is_empty() {
                    url = format!("{base}{url}");
                    break;
                }
            }
        }
    }

    url
}

/// Port of `extractDomSitename`.
fn extract_dom_sitename(doc: &Document) -> String {
    let (_, first, second) = examine_title_element(doc);
    if !first.is_empty() && first.contains('.') {
        return first;
    } else if !second.is_empty() && second.contains('.') {
        return second;
    }
    String::new()
}

// ---------------------------------------------------------------------------
// Categories and tags
// ---------------------------------------------------------------------------

/// Port of `extractDomCategories`.
fn extract_dom_categories(doc: &Document) -> Vec<String> {
    let mut categories = Vec::new();

    for &rule in META_CATEGORIES {
        let root = doc.root();
        for node_id in query_all(doc, root, &[rule]) {
            let href = trim(&doc.get_attribute(node_id, "href").unwrap_or_default());
            if !href.is_empty() && CATEGORY_HREF.is_match(&href) {
                let text = trim(&doc.text_content(node_id));
                if !text.is_empty() {
                    categories.push(text);
                }
            }
        }
        if !categories.is_empty() {
            break;
        }
    }

    // Fallback: article:section and subject meta tags.
    if categories.is_empty() {
        for node_id in doc.query_selector_all(
            doc.root(),
            r#"head meta[property="article:section"], head meta[name*="subject"]"#,
        ) {
            let content = trim(&doc.get_attribute(node_id, "content").unwrap_or_default());
            if !content.is_empty() {
                categories.push(content);
            }
        }
    }

    let strs: Vec<&str> = categories.iter().map(|s| s.as_str()).collect();
    uniquify_lists(&strs)
}

/// Port of `extractDomTags`.
fn extract_dom_tags(doc: &Document) -> Vec<String> {
    let mut tags = Vec::new();

    for &rule in META_TAGS {
        let root = doc.root();
        for node_id in query_all(doc, root, &[rule]) {
            let href = trim(&doc.get_attribute(node_id, "href").unwrap_or_default());
            if !href.is_empty() && TAG_HREF.is_match(&href) {
                let text = trim(&doc.text_content(node_id));
                if !text.is_empty() {
                    tags.push(text);
                }
            }
        }
        if !tags.is_empty() {
            break;
        }
    }

    let strs: Vec<&str> = tags.iter().map(|s| s.as_str()).collect();
    uniquify_lists(&strs)
}

/// Port of `cleanCatTags`.
fn clean_cat_tags(cat_tags: Vec<String>) -> Vec<String> {
    use crate::utils::regex_patterns::COMMA_SEPARATOR;
    let mut cleaned = Vec::new();
    for entry in cat_tags {
        for item in COMMA_SEPARATOR.split(&entry) {
            let item = trim(item);
            if !item.is_empty() {
                cleaned.push(item);
            }
        }
    }
    cleaned
}

/// Port of `extractDomMetaSelectors`.
fn extract_dom_meta_selectors(doc: &Document, limit: usize, rules: &[crate::selector::Rule]) -> String {
    let root = doc.root();
    for &rule in rules {
        for node_id in query_all(doc, root, &[rule]) {
            let text = trim(&doc.iter_text(node_id, " "));
            let len = text.chars().count();
            if len > 2 && len < limit {
                return text;
            }
        }
    }
    String::new()
}

// ---------------------------------------------------------------------------
// License extraction
// ---------------------------------------------------------------------------

/// Port of `extractLicense`.
fn extract_license(doc: &Document) -> String {
    // Look for links labeled as license.
    for node_id in doc.query_selector_all(doc.root(), r#"a[rel="license"][href]"#) {
        if let Some(result) = parse_license_element(doc, node_id, false) {
            return result;
        }
    }

    // Probe footer elements for CC links.
    let sel = r#"footer a[href], div[class*="footer"] a[href], div[id*="footer"] a[href]"#;
    for node_id in doc.query_selector_all(doc.root(), sel) {
        if let Some(result) = parse_license_element(doc, node_id, true) {
            return result;
        }
    }

    String::new()
}

/// Port of `parseLicenseElement`.
fn parse_license_element(doc: &Document, node_id: crate::dom::NodeId, strict: bool) -> Option<String> {
    // Check href for CC license.
    let href = trim(&doc.get_attribute(node_id, "href").unwrap_or_default());
    if !href.is_empty() {
        if let Some(caps) = CC_LICENSE.captures(&href) {
            return Some(format!("CC {} {}", caps[1].to_uppercase(), &caps[2]));
        }
    }

    // Check link text.
    let text = trim(&doc.text(node_id));
    if !text.is_empty() {
        if !strict {
            return Some(text);
        }
        if let Some(caps) = CC_LICENSE_TEXT.captures(&text) {
            return Some(caps[0].to_string());
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Author normalization
// ---------------------------------------------------------------------------

/// Port of `normalizeAuthors`.
pub fn normalize_authors(authors: &str, input: &str) -> String {
    // Skip URLs and email addresses.
    if URL_CHECK.is_match(input) || AUTHOR_EMAIL.is_match(input) {
        return authors.to_string();
    }

    let mut input = trim(input);
    input = unescape_html(&input);
    input = remove_emojis(&input);
    input = AUTHOR_DIGITS.replace_all(&input, "").to_string();
    input = AUTHOR_SOCIAL_MEDIA.replace_all(&input, "").to_string();
    input = AUTHOR_SPACE_CHARS.replace_all(&input, " ").to_string();

    // Second unescape pass for double-encoded entities.
    if input.contains("&#") || input.contains("&amp;") {
        input = unescape_html(&input);
    }

    // Strip HTML tags.
    input = AUTHOR_HTML.replace_all(&input, "").to_string();

    // Build current author list.
    let mut list_author: Vec<String> = if authors.is_empty() {
        Vec::new()
    } else {
        authors.split("; ").map(|s| s.to_string()).collect()
    };

    let tracker: HashSet<String> = list_author.iter().cloned().collect();
    let mut tracker = tracker;

    for a in AUTHOR_SEPARATOR.split(&input) {
        let a = AUTHOR_NICKNAME.replace_all(a, "").to_string();
        let a = AUTHOR_SPECIAL_CHARS.replace_all(&a, "").to_string();
        let a = AUTHOR_PREFIX.replace_all(&a, "").to_string();
        let a = AUTHOR_PREPOSITION.replace_all(&a, "").to_string();
        let a = trim(&a);

        let length = a.chars().count();
        let has_dash = a.contains('-');
        let has_space = a.contains(' ');

        if length == 0 || (!has_dash && !has_space && length >= 50) {
            continue;
        }

        // Title-case if not already.
        let a = {
            let first = a.chars().next();
            if !first.map(|c| c.is_uppercase()).unwrap_or(false)
                || a.to_lowercase() == a
            {
                title_case(&a)
            } else {
                a
            }
        };

        if !authors.contains(&a) && !tracker.contains(&a) {
            tracker.insert(a.clone());
            list_author.push(a);
        }
    }

    list_author.join("; ")
}

/// Port of `removeBlacklistedAuthors`.
pub fn remove_blacklisted_authors(current: &str, opts: &Options) -> String {
    if current.is_empty() {
        return String::new();
    }

    let blacklisted: HashSet<String> = opts
        .blacklisted_authors
        .iter()
        .map(|a| a.to_lowercase())
        .collect();

    let allowed: Vec<&str> = current
        .split(';')
        .map(|a| a.trim())
        .filter(|a| !blacklisted.contains(&a.to_lowercase()))
        .collect();

    if !allowed.is_empty() {
        allowed.join("; ")
    } else {
        String::new()
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Simple Unicode-aware title case.
///
/// Capitalizes the first letter of each word, where words are delimited by
/// whitespace or hyphens — matching Go's `cases.Title(language.English)` behavior
/// for the hyphenated-name case (e.g. "anne-marie" → "Anne-Marie").
fn title_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut capitalize_next = true;
    for ch in s.chars() {
        if ch.is_whitespace() || ch == '-' {
            capitalize_next = true;
            result.push(ch);
        } else if capitalize_next {
            result.extend(ch.to_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::Document;
    use crate::options::Options;

    fn parse(html: &str) -> Document {
        Document::parse(html)
    }

    // ---------------------------------------------------------------------------
    // validate_metadata_name
    // ---------------------------------------------------------------------------

    #[test]
    fn test_validate_metadata_name_valid() {
        assert_eq!(validate_metadata_name("John Doe"), "John Doe");
        assert_eq!(validate_metadata_name("Jane Smith"), "Jane Smith");
    }

    #[test]
    fn test_validate_metadata_name_single_word() {
        assert_eq!(validate_metadata_name("Alice"), "");
    }

    #[test]
    fn test_validate_metadata_name_url() {
        assert_eq!(validate_metadata_name("http://example.com"), "");
    }

    #[test]
    fn test_validate_metadata_name_json() {
        assert_eq!(validate_metadata_name(r#"{"name": "value"}"#), "");
    }

    // ---------------------------------------------------------------------------
    // normalize_authors
    // ---------------------------------------------------------------------------

    #[test]
    fn test_normalize_authors_basic() {
        let result = normalize_authors("", "by John Doe");
        assert!(result.contains("John Doe"), "got: {result}");
    }

    #[test]
    fn test_normalize_authors_url_skipped() {
        let result = normalize_authors("Alice", "https://example.com/author");
        assert_eq!(result, "Alice");
    }

    #[test]
    fn test_normalize_authors_email_skipped() {
        let result = normalize_authors("Alice", "john@example.com");
        assert_eq!(result, "Alice");
    }

    #[test]
    fn test_normalize_authors_dedup() {
        let result = normalize_authors("Jane Doe", "Jane Doe");
        // Should not duplicate.
        assert_eq!(result.matches("Jane Doe").count(), 1);
    }

    #[test]
    fn test_normalize_authors_multiple() {
        let result = normalize_authors("", "Alice Smith and Bob Jones");
        assert!(result.contains("Alice Smith"), "got: {result}");
        assert!(result.contains("Bob Jones"), "got: {result}");
    }

    // ---------------------------------------------------------------------------
    // remove_blacklisted_authors
    // ---------------------------------------------------------------------------

    #[test]
    fn test_remove_blacklisted_authors() {
        let opts = Options {
            blacklisted_authors: vec!["Staff Reporter".to_string()],
            ..Default::default()
        };
        let result = remove_blacklisted_authors("Staff Reporter; Jane Doe", &opts);
        assert!(!result.contains("Staff Reporter"));
        assert!(result.contains("Jane Doe"));
    }

    // ---------------------------------------------------------------------------
    // examine_meta (OpenGraph)
    // ---------------------------------------------------------------------------

    #[test]
    fn test_examine_meta_og_title() {
        let doc = parse(
            r#"<html><head><meta property="og:title" content="My Article"/></head></html>"#,
        );
        let meta = examine_meta(&doc);
        assert_eq!(meta.title, "My Article");
    }

    #[test]
    fn test_examine_meta_og_author() {
        let doc = parse(
            r#"<html><head><meta property="og:author" content="Jane Doe"/></head></html>"#,
        );
        let meta = examine_meta(&doc);
        assert_eq!(meta.author, "Jane Doe");
    }

    #[test]
    fn test_examine_meta_name_author() {
        let doc = parse(
            r#"<html><head><meta name="author" content="John Smith"/></head></html>"#,
        );
        let meta = examine_meta(&doc);
        assert!(meta.author.contains("John Smith"), "got: {}", meta.author);
    }

    #[test]
    fn test_examine_meta_description() {
        let doc = parse(
            r#"<html><head><meta name="description" content="Article description"/></head></html>"#,
        );
        let meta = examine_meta(&doc);
        assert_eq!(meta.description, "Article description");
    }

    // ---------------------------------------------------------------------------
    // extract_dom_title
    // ---------------------------------------------------------------------------

    #[test]
    fn test_extract_dom_title_single_h1() {
        let doc =
            parse(r#"<html><body><h1>Single Heading</h1><p>text</p></body></html>"#);
        let title = extract_dom_title(&doc);
        assert_eq!(title, "Single Heading");
    }

    #[test]
    fn test_extract_dom_title_from_title_tag() {
        let doc = parse(r#"<html><head><title>Article – Site Name</title></head><body><h1>A</h1><h1>B</h1></body></html>"#);
        let title = extract_dom_title(&doc);
        // TITLE_CLEANER splits on – and returns "Article".
        assert_eq!(title, "Article");
    }

    // ---------------------------------------------------------------------------
    // extract_license
    // ---------------------------------------------------------------------------

    #[test]
    fn test_extract_license_cc_href() {
        let doc = parse(r#"<html><body><a rel="license" href="https://creativecommons.org/licenses/by-sa/4.0/">CC</a></body></html>"#);
        let lic = extract_license(&doc);
        assert!(lic.starts_with("CC BY-SA"), "got: {lic}");
    }

    // ---------------------------------------------------------------------------
    // extract_metadata integration
    // ---------------------------------------------------------------------------

    #[test]
    fn test_extract_metadata_og_basic() {
        let doc = parse(r#"<html><head>
            <meta property="og:title" content="Test Title"/>
            <meta property="og:description" content="Test Description"/>
            <meta name="author" content="Test Author"/>
        </head><body></body></html>"#);
        let meta = extract_metadata(&doc, &Options::default());
        assert_eq!(meta.title, "Test Title");
        assert_eq!(meta.description, "Test Description");
        assert!(meta.author.contains("Test Author"), "got: {}", meta.author);
    }

    #[test]
    fn test_extract_metadata_json_ld_overrides_og() {
        let doc = parse(r#"<html><head>
            <meta property="og:title" content="OG Title"/>
            <script type="application/ld+json">
            {"@type":"Article","name":"LD Title","author":{"@type":"Person","name":"LD Author"}}
            </script>
        </head><body></body></html>"#);
        let meta = extract_metadata(&doc, &Options::default());
        // JSON-LD title should fill in since OG title is already set.
        // Depending on order: OG is extracted first, JSON-LD uses strOr (keeps non-empty OG).
        assert!(!meta.title.is_empty());
        // JSON-LD author takes precedence.
        assert!(meta.author.contains("LD Author"), "got: {}", meta.author);
    }

    #[test]
    fn test_title_case() {
        assert_eq!(title_case("hello world"), "Hello World");
        assert_eq!(title_case("already Title"), "Already Title");
        assert_eq!(title_case(""), "");
    }

    #[test]
    fn test_title_case_hyphen() {
        // Matches Go's cases.Title(language.English) which treats hyphens as word boundaries.
        assert_eq!(title_case("anne-marie"), "Anne-Marie");
        assert_eq!(title_case("jean-luc picard"), "Jean-Luc Picard");
    }

    #[test]
    fn test_normalize_authors_hyphenated_name() {
        let result = normalize_authors("", "anne-marie dupont");
        assert!(result.contains("Anne-Marie"), "got: {result}");
    }
}
