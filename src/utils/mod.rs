// Port of go-trafilatura/utils-common.go

pub mod language;
pub(crate) mod lru;
pub(crate) mod regex_patterns;
pub(crate) mod text;
pub(crate) mod url;

use unicode_normalization::UnicodeNormalization;

use crate::dom::{Document, NodeId};

/// Collapses internal whitespace and trims leading/trailing spaces.
/// Equivalent to Go's `strings.Join(strings.Fields(s), " ")`.
///
/// Also strips U+00AD (SOFT HYPHEN) characters, matching Go's html.Parse behavior
/// which removes them from text nodes during parsing.
///
/// Applies Unicode NFC normalization, matching Go's html.Parse which normalizes
/// NFD-encoded text (e.g. `u + U+0308` → `ü`) to NFC.
///
/// Port of `trim`.
pub fn trim(s: &str) -> String {
    // Strip soft hyphens (U+00AD) — Go's HTML parser removes them during tokenization.
    let no_soft_hyphen: String = s.chars().filter(|&c| c != '\u{00AD}').collect();
    // Collapse whitespace (split_whitespace handles all Unicode whitespace).
    let joined = no_soft_hyphen
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    // NFC normalization — Go's html.Parse normalizes NFD decomposed chars to NFC.
    joined.nfc().collect()
}

/// Counts words (whitespace-delimited tokens) in a string.
///
/// Port of `strWordCount`.
pub(crate) fn str_word_count(s: &str) -> usize {
    s.split_whitespace().count()
}

/// Returns the first non-empty string from the arguments.
///
/// Port of `strOr`.
pub(crate) fn str_or<'a>(args: &[&'a str]) -> &'a str {
    args.iter().find(|&&s| !s.is_empty()).copied().unwrap_or("")
}

/// Checks if an element has a valid image `src` or `data-src` attribute.
///
/// Port of `isImageElement`.
pub(crate) fn is_image_element(doc: &Document, id: NodeId) -> bool {
    for attr_name in ["src", "data-src", "data-srcset"] {
        if let Some(val) = doc.get_attribute(id, attr_name) {
            if is_image_file(&val) {
                return true;
            }
        }
    }
    // Also check any attribute starting with "data-src".
    for attr in doc.attribute_names(id) {
        if attr.starts_with("data-src") {
            if let Some(val) = doc.get_attribute(id, &attr) {
                if is_image_file(&val) {
                    return true;
                }
            }
        }
    }
    false
}

/// Checks whether a file path/URL appears to point to an image file.
///
/// Port of `isImageFile`.
pub(crate) fn is_image_file(src: &str) -> bool {
    if src.is_empty() {
        return false;
    }

    // Extract just the path portion before any query string.
    let path_part = src.split('?').next().unwrap_or(src);

    let ext = path_part
        .split('/')
        .next_back()
        .unwrap_or_default()
        .split('.')
        .next_back()
        .unwrap_or_default();

    let ext_lower = ext.to_lowercase();
    matches!(
        ext_lower.as_str(),
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "bmp" | "avif" | "tiff" | "tif" | "ico"
    )
}

/// Deduplicate a list of strings split from comma/semicolon-separated inputs.
/// Strips quotes and trims whitespace from each entry.
///
/// Port of `uniquifyLists`.
pub(crate) fn uniquify_lists(inputs: &[&str]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();

    for &input in inputs {
        // Choose separator: whichever is more common.
        let sep = if input.chars().filter(|&c| c == ';').count()
            > input.chars().filter(|&c| c == ',').count()
        {
            ';'
        } else {
            ','
        };

        for entry in input.split(sep) {
            let entry = trim(entry).replace(['"', '\''], "");
            if !entry.is_empty() && seen.insert(entry.clone()) {
                result.push(entry);
            }
        }
    }

    result
}

/// Decodes common HTML entities in a string.
///
/// Handles named entities (`&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;`, `&nbsp;`, etc.)
/// and numeric references (`&#NNN;`, `&#xHHH;`).
pub(crate) fn unescape_html(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }

    let mut result = String::with_capacity(s.len());
    let mut chars = s.char_indices().peekable();

    while let Some((_, ch)) = chars.next() {
        if ch != '&' {
            result.push(ch);
            continue;
        }

        // Collect entity name up to ';' or a non-entity character.
        // Use peek() so that the terminating character is NOT consumed when
        // the guard fires — the outer loop will process it on the next iteration.
        let mut entity = String::new();
        let mut found_semi = false;

        while let Some(&(_, ec)) = chars.peek() {
            if ec == ';' {
                chars.next();
                found_semi = true;
                break;
            }
            if entity.len() > 12 || (!ec.is_alphanumeric() && ec != '#') {
                // Leave ec unconsumed so the outer loop handles it normally.
                break;
            }
            chars.next();
            entity.push(ec);
        }

        let decoded: Option<&str> = match entity.as_str() {
            "amp" => Some("&"),
            "lt" => Some("<"),
            "gt" => Some(">"),
            "quot" => Some("\""),
            "apos" => Some("'"),
            "nbsp" => Some("\u{00A0}"),
            "ndash" => Some("\u{2013}"),
            "mdash" => Some("\u{2014}"),
            "hellip" => Some("\u{2026}"),
            "laquo" => Some("\u{00AB}"),
            "raquo" => Some("\u{00BB}"),
            "copy" => Some("\u{00A9}"),
            "reg" => Some("\u{00AE}"),
            "trade" => Some("\u{2122}"),
            _ => None,
        };

        if let Some(d) = decoded {
            result.push_str(d);
            continue;
        }

        // Numeric character references — valid both with and without semicolon
        // (HTML5 spec §13.2.5.72 allows omitting ';' for numeric refs).
        if let Some(stripped) = entity.strip_prefix('#') {
            let cp = if let Some(hex) = stripped
                .strip_prefix('x')
                .or_else(|| stripped.strip_prefix('X'))
            {
                u32::from_str_radix(hex, 16).ok()
            } else {
                stripped.parse::<u32>().ok()
            };
            if let Some(c) = cp.and_then(char::from_u32) {
                result.push(c);
                continue;
            }
        }

        if !found_semi {
            // Named entity without semicolon and not a known legacy entity — emit literally.
            result.push('&');
            result.push_str(&entity);
            continue;
        }

        // Unknown entity with semicolon — emit literally.
        result.push('&');
        result.push_str(&entity);
        result.push(';');
    }

    result
}

/// Removes characters in common emoji Unicode ranges.
///
/// Approximates `gomoji.RemoveEmojis` from go-trafilatura.
/// Covers: Misc Symbols/Dingbats (U+2600–U+27BF), Supplemental Arrows &
/// Dingbats (U+2900–U+2BFF), supplementary emoji planes (U+1F000–U+1FFFF),
/// variation selectors (U+FE00–U+FE0F), and the Tags block (U+E0000–U+E007F).
pub(crate) fn remove_emojis(s: &str) -> String {
    s.chars()
        .filter(|&c| {
            let cp = c as u32;
            !matches!(
                cp,
                0x2600..=0x27BF    // Misc Symbols, Dingbats
                | 0x2900..=0x2BFF  // Supplemental Arrows, Misc Symbols & Arrows
                | 0x1F000..=0x1FFFF // Supplementary multilingual plane (emoji)
                | 0xFE00..=0xFE0F  // Variation selectors
                | 0xE0000..=0xE007F // Tags block
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trim() {
        assert_eq!(trim("  hello   world  "), "hello world");
        assert_eq!(trim("single"), "single");
        assert_eq!(trim(""), "");
        assert_eq!(trim("  "), "");
        assert_eq!(trim("a  b\tc"), "a b c");
    }

    #[test]
    fn test_str_word_count() {
        assert_eq!(str_word_count("hello world"), 2);
        assert_eq!(str_word_count("  one  two  three  "), 3);
        assert_eq!(str_word_count(""), 0);
        assert_eq!(str_word_count("single"), 1);
    }

    #[test]
    fn test_str_or() {
        assert_eq!(str_or(&["", "second", "third"]), "second");
        assert_eq!(str_or(&["first", "second"]), "first");
        assert_eq!(str_or(&["", ""]), "");
        assert_eq!(str_or(&[]), "");
    }

    #[test]
    fn test_is_image_file() {
        assert!(is_image_file("photo.jpg"));
        assert!(is_image_file("/photo.jpg"));
        assert!(is_image_file("image.PNG"));
        assert!(is_image_file(
            "https://cdn.example.com/img/photo.webp?size=large"
        ));
        assert!(is_image_file("//cdn.example.com/img/photo.webp?size=large"));
        assert!(!is_image_file("document.pdf"));
        assert!(!is_image_file("script.js"));
        assert!(!is_image_file(""));
        assert!(!is_image_file("noextension"));
    }

    #[test]
    fn test_uniquify_lists() {
        let result = uniquify_lists(&["one, two, three", "two, four"]);
        assert!(result.contains(&"one".to_string()));
        assert!(result.contains(&"two".to_string()));
        assert!(result.contains(&"three".to_string()));
        assert!(result.contains(&"four".to_string()));
        // "two" should appear only once.
        assert_eq!(result.iter().filter(|&s| s == "two").count(), 1);
    }

    #[test]
    fn test_uniquify_lists_semicolon() {
        let result = uniquify_lists(&["alpha; beta; gamma"]);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_uniquify_lists_strips_quotes() {
        let result = uniquify_lists(&[r#""rust", "python""#]);
        assert!(result.contains(&"rust".to_string()));
        assert!(result.contains(&"python".to_string()));
    }

    #[test]
    fn test_unescape_html_common_entities() {
        assert_eq!(unescape_html("&amp;"), "&");
        assert_eq!(unescape_html("&lt;tag&gt;"), "<tag>");
        assert_eq!(unescape_html("&quot;hello&quot;"), "\"hello\"");
        assert_eq!(unescape_html("A &amp; B"), "A & B");
        assert_eq!(unescape_html("no entities"), "no entities");
    }

    #[test]
    fn test_unescape_html_numeric_refs() {
        assert_eq!(unescape_html("&#65;"), "A");
        assert_eq!(unescape_html("&#x41;"), "A");
        assert_eq!(unescape_html("&#x1F600;"), "\u{1F600}");
    }

    #[test]
    fn test_unescape_html_long_entity_no_corruption() {
        // Entity names longer than 12 chars should be emitted literally without
        // swallowing the terminating character.
        let input = "&verylongentityname; rest";
        let result = unescape_html(input);
        // The long unknown entity is emitted literally; " rest" must follow intact.
        assert!(result.ends_with(" rest"), "got: {result}");
        assert!(result.contains('&'), "got: {result}");
    }

    #[test]
    fn test_unescape_html_non_entity_ampersand() {
        // Ampersand not followed by a semicolon should be emitted literally.
        assert_eq!(unescape_html("a & b"), "a & b");
    }

    #[test]
    fn test_remove_emojis_basic() {
        // U+1F600 (😀) — supplementary plane, should be removed.
        assert_eq!(remove_emojis("hello \u{1F600} world"), "hello  world");
        // U+2764 (❤) — in Misc Symbols range, removed.
        assert_eq!(remove_emojis("love \u{2764}"), "love ");
        // ASCII should not be affected.
        assert_eq!(remove_emojis("plain text"), "plain text");
    }

    #[test]
    fn test_remove_emojis_extended_ranges() {
        // U+2B50 (⭐) — in 0x2900–0x2BFF range, should now be removed.
        assert_eq!(remove_emojis("rating \u{2B50}"), "rating ");
        // U+2B06 (⬆) — also in that range.
        assert_eq!(remove_emojis("up \u{2B06}"), "up ");
    }
}
