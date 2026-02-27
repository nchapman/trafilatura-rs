// Port of go-trafilatura/utils-extractor.go (languageClassifier, checkHtmlLanguage)

use whatlang::Lang;

use crate::dom::{Document, NodeId};
use crate::options::Options;
use crate::utils::regex_patterns::HTML_LANG;

/// Maps a whatlang ISO 639-3 code to ISO 639-1 (2-letter) code.
/// The whatlang crate only provides 3-letter codes; the Go version returns 2-letter.
fn to_iso639_1(lang: Lang) -> &'static str {
    match lang {
        Lang::Afr => "af",
        Lang::Aka => "ak",
        Lang::Amh => "am",
        Lang::Ara => "ar",
        Lang::Aze => "az",
        Lang::Bel => "be",
        Lang::Ben => "bn",
        Lang::Bul => "bg",
        Lang::Cat => "ca",
        Lang::Ces => "cs",
        Lang::Cmn => "zh",
        Lang::Dan => "da",
        Lang::Deu => "de",
        Lang::Ell => "el",
        Lang::Eng => "en",
        Lang::Epo => "eo",
        Lang::Est => "et",
        Lang::Fin => "fi",
        Lang::Fra => "fr",
        Lang::Guj => "gu",
        Lang::Heb => "he",
        Lang::Hin => "hi",
        Lang::Hrv => "hr",
        Lang::Hun => "hu",
        Lang::Hye => "hy",
        Lang::Ind => "id",
        Lang::Ita => "it",
        Lang::Jav => "jv",
        Lang::Jpn => "ja",
        Lang::Kan => "kn",
        Lang::Kat => "ka",
        Lang::Khm => "km",
        Lang::Kor => "ko",
        Lang::Lat => "la",
        Lang::Lav => "lv",
        Lang::Lit => "lt",
        Lang::Mal => "ml",
        Lang::Mar => "mr",
        Lang::Mkd => "mk",
        Lang::Mya => "my",
        Lang::Nep => "ne",
        Lang::Nld => "nl",
        Lang::Nob => "nb",
        Lang::Ori => "or",
        Lang::Pan => "pa",
        Lang::Pes => "fa",
        Lang::Pol => "pl",
        Lang::Por => "pt",
        Lang::Ron => "ro",
        Lang::Rus => "ru",
        Lang::Sin => "si",
        Lang::Slk => "sk",
        Lang::Slv => "sl",
        Lang::Sna => "sn",
        Lang::Spa => "es",
        Lang::Srp => "sr",
        Lang::Swe => "sv",
        Lang::Tam => "ta",
        Lang::Tel => "te",
        Lang::Tgl => "tl",
        Lang::Tha => "th",
        Lang::Tuk => "tk",
        Lang::Tur => "tr",
        Lang::Ukr => "uk",
        Lang::Urd => "ur",
        Lang::Uzb => "uz",
        Lang::Vie => "vi",
        Lang::Yid => "yi",
        Lang::Zul => "zu",
    }
}

/// Detects the language of the given content, returning an ISO 639-1 code.
/// Uses the longer of `content_text` or `comments_text` for detection.
/// Returns an empty string if detection fails.
///
/// Note: the Rust `whatlang` crate covers 69 languages; Go's `whatlanggo`
/// covers ~87. Languages present in whatlanggo but absent from whatlang
/// (e.g. Cebuano, Ilocano) will return `""` here via `None` rather than via
/// an empty ISO 639-1 code. The observable output is identical.
///
/// Port of `languageClassifier`.
pub(crate) fn language_classifier(content_text: &str, comments_text: &str) -> String {
    let len_content = content_text.chars().count();
    let len_comments = comments_text.chars().count();

    let lang_test = if len_comments > len_content {
        comments_text
    } else {
        content_text
    };

    whatlang::detect_lang(lang_test)
        .map(|lang| to_iso639_1(lang).to_string())
        .unwrap_or_default()
}

/// Checks HTML meta elements and the `<html lang>` attribute to determine
/// whether the document's language matches `opts.target_language`.
/// Returns `true` if the language matches or cannot be determined.
///
/// Port of `checkHtmlLanguage`.
pub fn check_html_language(doc: &Document, opts: &Options, strict: bool) -> bool {
    let target = match opts.target_language.as_deref() {
        Some(t) if !t.is_empty() => t,
        _ => return true,
    };

    // Find the <html> element to check the lang attribute.
    let html_node = find_html_node(doc);

    // Check HTTP Content-Language and og:locale meta tags.
    let meta_selectors = [
        r#"meta[http-equiv="content-language"][content]"#,
        r#"meta[property="og:locale"][content]"#,
    ];

    for selector in &meta_selectors {
        let meta_nodes = doc.query_selector_all(doc.root(), selector);
        if meta_nodes.is_empty() {
            continue;
        }

        for meta_id in &meta_nodes {
            if let Some(content) = doc.get_attribute(*meta_id, "content") {
                for lang in HTML_LANG.find_iter(&content) {
                    if lang.as_str().to_lowercase() == target {
                        return true;
                    }
                }
            }
        }

        tracing::warn!("html language detection in meta failed");
        return false;
    }

    // HTML lang attribute: sometimes a wrong indicator, only used in strict mode.
    if strict {
        if let Some(html_id) = html_node {
            if let Some(lang_attr) = doc.get_attribute(html_id, "lang") {
                for lang in HTML_LANG.find_iter(&lang_attr) {
                    if lang.as_str().to_lowercase() == target {
                        return true;
                    }
                }
                tracing::warn!("html language detection failed");
                return false;
            }
        }
    }

    tracing::warn!("no html language elements found");
    true
}

/// Finds the `<html>` element node starting from the document root.
fn find_html_node(doc: &Document) -> Option<NodeId> {
    let root = doc.root();
    // The root is a Document node (not an Element), so we use children()
    // to iterate its element children and find the <html> element directly.
    doc.children(root)
        .into_iter()
        .find(|&child| doc.tag_name(child) == "html")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::Document;
    use crate::options::Options;

    #[test]
    fn test_language_classifier_english() {
        let text = "The quick brown fox jumps over the lazy dog. \
                    This is a simple English sentence to test language detection.";
        let result = language_classifier(text, "");
        assert_eq!(result, "en");
    }

    #[test]
    fn test_language_classifier_uses_longer() {
        // Comments text is longer (more German words), so it should win.
        let content = "Hello";
        let comments = "Das ist ein sehr langer deutscher Kommentar mit vielen Wörtern \
                        und grammatikalisch korrekten Sätzen für die Spracherkennung.";
        let result = language_classifier(content, comments);
        assert_eq!(result, "de");
    }

    #[test]
    fn test_language_classifier_empty() {
        let result = language_classifier("", "");
        assert_eq!(result, "");
    }

    #[test]
    fn test_check_html_language_no_target() {
        let doc = Document::parse("<html><body></body></html>");
        let opts = Options::default(); // target_language = None
        assert!(check_html_language(&doc, &opts, false));
    }

    #[test]
    fn test_check_html_language_meta_match() {
        let html = r#"<html><head><meta http-equiv="content-language" content="en-US"/></head><body></body></html>"#;
        let doc = Document::parse(html);
        let opts = Options {
            target_language: Some("en".to_string()),
            ..Options::default()
        };
        assert!(check_html_language(&doc, &opts, false));
    }

    #[test]
    fn test_check_html_language_meta_no_match() {
        let html = r#"<html><head><meta http-equiv="content-language" content="de"/></head><body></body></html>"#;
        let doc = Document::parse(html);
        let opts = Options {
            target_language: Some("en".to_string()),
            ..Options::default()
        };
        assert!(!check_html_language(&doc, &opts, false));
    }

    #[test]
    fn test_check_html_language_strict_html_lang_match() {
        let html = r#"<html lang="fr"><body></body></html>"#;
        let doc = Document::parse(html);
        let opts = Options {
            target_language: Some("fr".to_string()),
            ..Options::default()
        };
        assert!(check_html_language(&doc, &opts, true));
    }

    #[test]
    fn test_check_html_language_strict_html_lang_no_match() {
        let html = r#"<html lang="fr"><body></body></html>"#;
        let doc = Document::parse(html);
        let opts = Options {
            target_language: Some("en".to_string()),
            ..Options::default()
        };
        assert!(!check_html_language(&doc, &opts, true));
    }

    #[test]
    fn test_check_html_language_non_strict_ignores_html_lang() {
        // In non-strict mode, html lang attribute is not checked → falls through → true.
        let html = r#"<html lang="fr"><body></body></html>"#;
        let doc = Document::parse(html);
        let opts = Options {
            target_language: Some("en".to_string()),
            ..Options::default()
        };
        assert!(check_html_language(&doc, &opts, false));
    }

    #[test]
    fn test_to_iso639_1_spot_check() {
        assert_eq!(to_iso639_1(Lang::Eng), "en");
        assert_eq!(to_iso639_1(Lang::Deu), "de");
        assert_eq!(to_iso639_1(Lang::Fra), "fr");
        assert_eq!(to_iso639_1(Lang::Jpn), "ja");
        assert_eq!(to_iso639_1(Lang::Zul), "zu");
        assert_eq!(to_iso639_1(Lang::Cmn), "zh");
        // Non-obvious: ISO 639-3 code differs substantially from ISO 639-1.
        assert_eq!(to_iso639_1(Lang::Pes), "fa");  // Persian
        assert_eq!(to_iso639_1(Lang::Nob), "nb");  // Norwegian Bokmål (not "no")
        assert_eq!(to_iso639_1(Lang::Hye), "hy");  // Armenian
        assert_eq!(to_iso639_1(Lang::Kan), "kn");  // Kannada
    }

    #[test]
    fn test_check_html_language_og_locale_match() {
        // og:locale uses underscore format (e.g. "en_US"); HTML_LANG extracts "en".
        let html = r#"<html><head><meta property="og:locale" content="en_US"/></head><body></body></html>"#;
        let doc = Document::parse(html);
        let opts = Options {
            target_language: Some("en".to_string()),
            ..Options::default()
        };
        assert!(check_html_language(&doc, &opts, false));
    }

    #[test]
    fn test_check_html_language_og_locale_no_match() {
        let html = r#"<html><head><meta property="og:locale" content="de_DE"/></head><body></body></html>"#;
        let doc = Document::parse(html);
        let opts = Options {
            target_language: Some("en".to_string()),
            ..Options::default()
        };
        assert!(!check_html_language(&doc, &opts, false));
    }
}
