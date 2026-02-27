// Port of go-trafilatura/internal/re2go/ source patterns and regex patterns from metadata.go
//
// All patterns compiled once at startup via LazyLock.

use std::sync::LazyLock;
use regex::Regex;

// ---------------------------------------------------------------------------
// Text filter (port of re2go/utils-extractor.re)
// ---------------------------------------------------------------------------

/// Matches social media sharing / print lines that should be filtered out.
/// Port of the re2c source pattern for `IsTextFilter`.
pub(crate) static TEXT_FILTER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\W*(Drucken|E-?Mail|Facebook|Flipboard|Google|Instagram|Linkedin|Mail|PDF|Pinterest|Pocket|Print|QQ|Reddit|Twitter|WeChat|WeiBo|Whatsapp|Xing|Mehr zum Thema:?|More on this.{0,8})$"
    ).unwrap()
});

// ---------------------------------------------------------------------------
// HTML language detection (port of utils-extractor.go)
// ---------------------------------------------------------------------------

/// Matches 2-letter language codes (e.g. "en", "de").
pub(crate) static HTML_LANG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)[a-z]{2}").unwrap()
});

// ---------------------------------------------------------------------------
// Metadata patterns (port of metadata.go var block)
// ---------------------------------------------------------------------------

/// Splits author lists on commas or semicolons.
pub(crate) static COMMA_SEPARATOR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\s*[,;]\s*").unwrap()
});

/// Strips site name suffixes from titles (e.g. "Article – Site Name").
pub(crate) static TITLE_CLEANER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(.+)?\s+[–•·—|⁄*⋆~‹«<›»>:-]\s+(.+)$").unwrap()
});

/// Detects JSON-like curly braces and backslashes (used to detect non-name strings).
pub(crate) static JSON_SYMBOL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"[{\\}]"#).unwrap()
});

/// Extracts a "name" value from a JSON-LD fragment.
pub(crate) static NAME_JSON: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)"name\\?":\s*\\?"([^"\\]+)"#).unwrap()
});

/// Checks if a string contains an HTTP(S) URL prefix.
pub(crate) static URL_CHECK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)https?://").unwrap()
});

/// Extracts the site name from a URL (between the protocol and first path segment).
pub(crate) static SITENAME_FINDER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)https?://(?:www\.|w[0-9]+\.)?([^/]+)").unwrap()
});

/// Strips HTML tags and comments from a string.
pub(crate) static HTML_STRIP_TAG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(<!--.*?-->|<[^>]*>)").unwrap()
});

/// Identifies category URLs by path segment.
pub(crate) static CATEGORY_HREF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)/categor(?:y|ies)/").unwrap()
});

/// Identifies tag URLs by path segment.
pub(crate) static TAG_HREF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)/tags?/").unwrap()
});

/// Extracts Creative Commons license type and version from a URL path.
pub(crate) static CC_LICENSE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)/(by-nc-nd|by-nc-sa|by-nc|by-nd|by-sa|by|zero)/([1-9]\.[0-9])").unwrap()
});

/// Detects Creative Commons license references in text.
pub(crate) static CC_LICENSE_TEXT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(cc|creative commons) (by-nc-nd|by-nc-sa|by-nc|by-nd|by-sa|by|zero) ?([1-9]\.[0-9])?").unwrap()
});

// ---------------------------------------------------------------------------
// Author normalization patterns (port of metadata.go var block)
// ---------------------------------------------------------------------------

/// Removes "by", "written by", "von", "from" prefixes from author strings.
pub(crate) static AUTHOR_PREFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^([a-zäöüß]+(ed|t))? ?(written by|words by|words|by|von|from) ").unwrap()
});

/// Removes digits and trailing text when they appear in author names.
pub(crate) static AUTHOR_DIGITS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\p{N}.+?$").unwrap()
});

/// Removes social media handles (@username) from author strings.
pub(crate) static AUTHOR_SOCIAL_MEDIA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)@\S+").unwrap()
});

/// Replaces dots, underscores, and plus signs with spaces in author strings.
pub(crate) static AUTHOR_SPACE_CHARS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)[._+]").unwrap()
});

/// Removes nicknames in quotes/parentheses from author strings.
pub(crate) static AUTHOR_NICKNAME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)["'({\['\'][^"]+?[''"'\)\]}]"#).unwrap()
});

/// Removes trailing special characters from author names.
pub(crate) static AUTHOR_SPECIAL_CHARS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)[^\p{L}\p{M}\p{N}_]+$|[:()?*$#!%/<>{}~¿]").unwrap()
});

/// Removes prepositions and trailing location text from author names.
pub(crate) static AUTHOR_PREPOSITION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b\s+(am|on|for|at|in|to|from|of|via|with|—|-|–)\s+(.*)").unwrap()
});

/// Detects email addresses embedded in author strings.
pub(crate) static AUTHOR_EMAIL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b").unwrap()
});

/// Splits multiple authors on various separators.
pub(crate) static AUTHOR_SEPARATOR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)/|;|,|\||&|(?:^|[^\p{L}\p{M}\p{N}_])[ua]nd(?:$|[^\p{L}\p{M}\p{N}_])").unwrap()
});

/// Strips HTML tags from author strings.
pub(crate) static AUTHOR_HTML: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)<[^>]+>").unwrap()
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_filter_matches() {
        assert!(TEXT_FILTER.is_match("Facebook"));
        assert!(TEXT_FILTER.is_match("  Twitter"));
        assert!(!TEXT_FILTER.is_match("Print this article")); // "Print" not at end of line
        assert!(TEXT_FILTER.is_match("Drucken"));
        assert!(TEXT_FILTER.is_match("More on this topic"));
        assert!(TEXT_FILTER.is_match("Mehr zum Thema:"));
    }

    #[test]
    fn test_text_filter_no_match() {
        assert!(!TEXT_FILTER.is_match("This is regular text"));
        assert!(!TEXT_FILTER.is_match("The article discusses Linkedin policies"));
    }

    #[test]
    fn test_html_lang() {
        let m: Vec<_> = HTML_LANG.find_iter("en-US").map(|m| m.as_str()).collect();
        assert!(m.contains(&"en"));
    }

    #[test]
    fn test_author_prefix() {
        let result = AUTHOR_PREFIX.replace("by John Doe", "");
        assert_eq!(result.trim(), "John Doe");

        let result = AUTHOR_PREFIX.replace("written by Jane Smith", "");
        assert_eq!(result.trim(), "Jane Smith");
    }

    #[test]
    fn test_sitename_finder() {
        let cap = SITENAME_FINDER.captures("https://www.example.com/path").unwrap();
        assert_eq!(&cap[1], "example.com");

        let cap = SITENAME_FINDER.captures("https://w3.blog.org/article").unwrap();
        assert_eq!(&cap[1], "blog.org");
    }

    #[test]
    fn test_cc_license() {
        let cap = CC_LICENSE.captures("https://creativecommons.org/licenses/by-sa/4.0/").unwrap();
        assert_eq!(&cap[1], "by-sa");
        assert_eq!(&cap[2], "4.0");
    }

    #[test]
    fn test_category_href() {
        assert!(CATEGORY_HREF.is_match("/category/tech/"));
        assert!(CATEGORY_HREF.is_match("/categories/news/"));
        assert!(!CATEGORY_HREF.is_match("/tag/rust/"));
    }

    #[test]
    fn test_tag_href() {
        assert!(TAG_HREF.is_match("/tags/rust/"));
        assert!(TAG_HREF.is_match("/tag/programming/"));
        assert!(!TAG_HREF.is_match("/category/news/"));
    }

    #[test]
    fn test_all_patterns_compile() {
        // Force initialization of all LazyLocks by touching them.
        let _ = &*TEXT_FILTER;
        let _ = &*HTML_LANG;
        let _ = &*COMMA_SEPARATOR;
        let _ = &*TITLE_CLEANER;
        let _ = &*JSON_SYMBOL;
        let _ = &*NAME_JSON;
        let _ = &*URL_CHECK;
        let _ = &*SITENAME_FINDER;
        let _ = &*HTML_STRIP_TAG;
        let _ = &*CATEGORY_HREF;
        let _ = &*TAG_HREF;
        let _ = &*CC_LICENSE;
        let _ = &*CC_LICENSE_TEXT;
        let _ = &*AUTHOR_PREFIX;
        let _ = &*AUTHOR_DIGITS;
        let _ = &*AUTHOR_SOCIAL_MEDIA;
        let _ = &*AUTHOR_SPACE_CHARS;
        let _ = &*AUTHOR_NICKNAME;
        let _ = &*AUTHOR_SPECIAL_CHARS;
        let _ = &*AUTHOR_PREPOSITION;
        let _ = &*AUTHOR_EMAIL;
        let _ = &*AUTHOR_SEPARATOR;
        let _ = &*AUTHOR_HTML;
    }
}
