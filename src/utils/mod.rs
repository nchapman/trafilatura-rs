// Port of go-trafilatura/utils-common.go

pub mod language;
pub mod lru;
pub mod regex_patterns;
pub mod text;
pub mod url;

use std::path::Path;

use crate::dom::{Document, NodeId};

/// Collapses internal whitespace and trims leading/trailing spaces.
/// Equivalent to Go's `strings.Join(strings.Fields(s), " ")`.
///
/// Port of `trim`.
pub fn trim(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Counts words (whitespace-delimited tokens) in a string.
///
/// Port of `strWordCount`.
pub fn str_word_count(s: &str) -> usize {
    s.split_whitespace().count()
}

/// Returns the first non-empty string from the arguments.
///
/// Port of `strOr`.
pub fn str_or<'a>(args: &[&'a str]) -> &'a str {
    args.iter().find(|&&s| !s.is_empty()).copied().unwrap_or("")
}

/// Checks if an element has a valid image `src` or `data-src` attribute.
///
/// Port of `isImageElement`.
pub fn is_image_element(doc: &Document, id: NodeId) -> bool {
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
pub fn is_image_file(src: &str) -> bool {
    if src.is_empty() {
        return false;
    }

    // Extract just the path portion before any query string.
    let path_part = src.split('?').next().unwrap_or(src);

    let ext = Path::new(path_part)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let ext_lower = ext.to_lowercase();
    matches!(ext_lower.as_str(), "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "bmp" | "avif" | "tiff" | "tif" | "ico")
}

/// Deduplicate a list of strings split from comma/semicolon-separated inputs.
/// Strips quotes and trims whitespace from each entry.
///
/// Port of `uniquifyLists`.
pub fn uniquify_lists(inputs: &[&str]) -> Vec<String> {
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
            let entry = trim(entry)
                .replace(['"', '\''], "");
            if !entry.is_empty() && seen.insert(entry.clone()) {
                result.push(entry);
            }
        }
    }

    result
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
        assert!(is_image_file("image.PNG"));
        assert!(is_image_file("https://cdn.example.com/img/photo.webp?size=large"));
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
}
