// Port of go-trafilatura/utils-extractor.go (textFilter, textCharsTest, duplicateTest)

use crate::dom::{Document, NodeId};
use crate::options::Options;
use crate::utils::lru::LruCache;
use crate::utils::regex_patterns::TEXT_FILTER;
use crate::utils::trim;

/// Filters out unwanted text nodes (social sharing lines, empty text, etc.).
/// Returns `true` if the node's text should be filtered out.
///
/// Port of `textFilter`.
pub(crate) fn text_filter(doc: &Document, id: NodeId) -> bool {
    let text = doc.text(id);
    let tail = doc.tail(id);

    let test_text = if text.is_empty() { tail } else { text };

    if !text_chars_test(&test_text) {
        return true;
    }

    test_text.split('\n').any(|line| TEXT_FILTER.is_match(line))
}

/// Returns `true` if the string contains meaningful (non-whitespace) content.
///
/// Port of `textCharsTest`.
pub(crate) fn text_chars_test(s: &str) -> bool {
    !trim(s).is_empty()
}

/// Checks a node's text against the deduplication cache.
/// Returns `true` if the text is a duplicate (seen too many times already).
/// Always increments the count in the cache when the text is long enough.
///
/// Port of `duplicateTest`.
pub(crate) fn duplicate_test(doc: &Document, id: NodeId, cache: &mut LruCache, opts: &Options) -> bool {
    let test_string = trim(&doc.iter_text(id, " "));

    if test_string.chars().count() > opts.config.min_duplicate_check_size {
        let cache_val = cache.get(&test_string).unwrap_or(0);
        let is_duplicate = cache_val > opts.config.max_duplicate_count;
        cache.put(test_string, cache_val + 1);
        return is_duplicate;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::Document;
    use crate::options::{Config, Options};

    fn make_doc_with_text(tag: &str, text: &str) -> (Document, NodeId) {
        let html = format!("<html><body><{tag}>{text}</{tag}></body></html>");
        let doc = Document::parse(&html);
        let body = doc.body().unwrap();
        // iter includes the starting node; use children() to get only direct child elements.
        let children = doc.children(body);
        (doc, children[0])
    }

    #[test]
    fn test_text_chars_test_basic() {
        assert!(text_chars_test("hello"));
        assert!(text_chars_test("  hello  "));
        assert!(!text_chars_test(""));
        assert!(!text_chars_test("   "));
        assert!(!text_chars_test("\t\n"));
    }

    #[test]
    fn test_text_filter_social_media() {
        let (doc, id) = make_doc_with_text("p", "Facebook");
        assert!(text_filter(&doc, id));

        let (doc, id) = make_doc_with_text("p", "Twitter");
        assert!(text_filter(&doc, id));

        let (doc, id) = make_doc_with_text("p", "Print");
        assert!(text_filter(&doc, id));
    }

    #[test]
    fn test_text_filter_normal_text() {
        let (doc, id) = make_doc_with_text("p", "This is a normal article paragraph.");
        assert!(!text_filter(&doc, id));
    }

    #[test]
    fn test_text_filter_empty_node() {
        // Empty text: text_chars_test fails, so filter returns true.
        let (doc, id) = make_doc_with_text("p", "");
        assert!(text_filter(&doc, id));
    }

    #[test]
    fn test_text_filter_keyword_on_second_line() {
        // Only the second line matches — the whole node should be filtered.
        let (doc, id) = make_doc_with_text("p", "Normal text here\nFacebook");
        assert!(text_filter(&doc, id));
    }

    #[test]
    fn test_text_filter_keyword_not_at_line_end() {
        // "Facebook" is in the middle of the line, not at the end — should not match.
        let (doc, id) = make_doc_with_text("p", "The article discusses Facebook policies");
        assert!(!text_filter(&doc, id));
    }

    #[test]
    fn test_duplicate_test_not_duplicate() {
        let opts = Options {
            config: Config {
                min_duplicate_check_size: 10,
                max_duplicate_count: 2,
                ..Config::default()
            },
            ..Options::default()
        };
        let mut cache = LruCache::new(100);
        let (doc, id) = make_doc_with_text("p", "This is a unique sentence with enough characters.");
        assert!(!duplicate_test(&doc, id, &mut cache, &opts));
    }

    #[test]
    fn test_duplicate_test_becomes_duplicate() {
        let opts = Options {
            config: Config {
                min_duplicate_check_size: 10,
                max_duplicate_count: 2,
                ..Config::default()
            },
            ..Options::default()
        };
        let mut cache = LruCache::new(100);
        let text = "This is a repeated sentence with enough characters.";
        let html = format!("<html><body><p>{text}</p></body></html>");
        let doc = Document::parse(&html);
        let body = doc.body().unwrap();
        let id = doc.children(body)[0];

        // max_duplicate_count = 2; duplicate when cache_val > 2 (i.e., >= 3rd seen).
        assert!(!duplicate_test(&doc, id, &mut cache, &opts)); // count: 0→1
        assert!(!duplicate_test(&doc, id, &mut cache, &opts)); // count: 1→2
        assert!(!duplicate_test(&doc, id, &mut cache, &opts)); // count: 2→3 (2 > 2 is false)
        assert!(duplicate_test(&doc, id, &mut cache, &opts));  // count: 3 > 2 → duplicate
    }

    #[test]
    fn test_duplicate_test_short_text_skipped() {
        let opts = Options {
            config: Config {
                min_duplicate_check_size: 100,
                max_duplicate_count: 2,
                ..Config::default()
            },
            ..Options::default()
        };
        let mut cache = LruCache::new(100);
        let (doc, id) = make_doc_with_text("p", "Short.");
        // Below threshold: cache never touched.
        for _ in 0..10 {
            assert!(!duplicate_test(&doc, id, &mut cache, &opts));
        }
        assert_eq!(cache.get("Short."), None);
    }
}
