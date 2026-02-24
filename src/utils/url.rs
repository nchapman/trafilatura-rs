// Port of go-trafilatura/url.go

use url::Url;

/// Checks if a URL is a valid, absolute HTTP(S) URL.
/// Returns the parsed URL on success.
///
/// Port of `isAbsoluteURL`.
pub fn is_absolute_url(s: &str) -> bool {
    parse_http_url(s).is_some()
}

fn parse_http_url(s: &str) -> Option<Url> {
    let url = Url::parse(s).ok()?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return None;
    }
    Some(url)
}

/// Convert a relative URL to absolute using a base URL.
/// Hash-prefixed, data URI, and javascript URIs are returned unchanged.
/// If the URL is already absolute, it is returned unchanged.
///
/// Port of `createAbsoluteURL`.
pub fn create_absolute_url(url_str: &str, base: Option<&Url>) -> String {
    if url_str.is_empty() || base.is_none() {
        return url_str.to_string();
    }

    // Passthrough cases.
    if url_str.starts_with('#') || url_str.starts_with("data:") || url_str.starts_with("javascript:") {
        return url_str.to_string();
    }

    // If already absolute, return as-is.
    if let Ok(parsed) = Url::parse(url_str) {
        if !parsed.scheme().is_empty() && parsed.host().is_some() {
            return url_str.to_string();
        }
    }

    // Resolve against base.
    let base = base.unwrap();
    match base.join(url_str) {
        Ok(resolved) => resolved.to_string(),
        Err(_) => url_str.to_string(),
    }
}

/// Extract the hostname from an absolute URL.
/// Returns empty string if the URL is not absolute.
///
/// Port of `getDomainURL`.
pub fn get_domain_url(url_str: &str) -> String {
    parse_http_url(url_str)
        .and_then(|u| u.host_str().map(str::to_string))
        .unwrap_or_default()
}

/// Extract the base URL (scheme + hostname) from an absolute URL.
/// Returns empty string if the URL is not absolute.
///
/// Port of `getBaseURL`.
pub fn get_base_url(url_str: &str) -> String {
    parse_http_url(url_str)
        .map(|u| format!("{}://{}", u.scheme(), u.host_str().unwrap_or("")))
        .unwrap_or_default()
}

/// Validate a URL, optionally converting a relative URL to absolute using `base_url`.
/// Returns `(url_string, is_valid)`.
///
/// Port of `validateURL`.
pub fn validate_url(url_str: &str, base_url: Option<&Url>) -> (String, bool) {
    // Already absolute?
    if is_absolute_url(url_str) {
        return (url_str.to_string(), true);
    }

    // Try absolutizing with base.
    let new_url = create_absolute_url(url_str, base_url);
    if is_absolute_url(&new_url) {
        return (new_url, true);
    }

    (url_str.to_string(), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_absolute_url() {
        assert!(is_absolute_url("https://example.com/path"));
        assert!(is_absolute_url("http://example.com"));
        assert!(!is_absolute_url("//example.com"));
        assert!(!is_absolute_url("/relative/path"));
        assert!(!is_absolute_url("relative/path"));
        assert!(!is_absolute_url("ftp://example.com"));
        assert!(!is_absolute_url(""));
        assert!(!is_absolute_url("not-a-url"));
    }

    #[test]
    fn test_create_absolute_url_passthrough() {
        let base = Url::parse("https://example.com/page").unwrap();
        assert_eq!(create_absolute_url("#section", Some(&base)), "#section");
        assert_eq!(create_absolute_url("data:image/png;base64,xxx", Some(&base)), "data:image/png;base64,xxx");
        assert_eq!(create_absolute_url("javascript:void(0)", Some(&base)), "javascript:void(0)");
        assert_eq!(create_absolute_url("https://other.com/path", Some(&base)), "https://other.com/path");
    }

    #[test]
    fn test_create_absolute_url_relative() {
        let base = Url::parse("https://example.com/dir/page").unwrap();
        let result = create_absolute_url("/other", Some(&base));
        assert_eq!(result, "https://example.com/other");
    }

    #[test]
    fn test_create_absolute_url_no_base() {
        assert_eq!(create_absolute_url("/path", None), "/path");
        assert_eq!(create_absolute_url("", Some(&Url::parse("https://example.com").unwrap())), "");
    }

    #[test]
    fn test_get_domain_url() {
        assert_eq!(get_domain_url("https://www.example.com/path"), "www.example.com");
        assert_eq!(get_domain_url("http://blog.test.org"), "blog.test.org");
        assert_eq!(get_domain_url("/relative"), "");
        assert_eq!(get_domain_url("not-a-url"), "");
    }

    #[test]
    fn test_get_base_url() {
        assert_eq!(get_base_url("https://www.example.com/path?q=1"), "https://www.example.com");
        assert_eq!(get_base_url("http://example.com"), "http://example.com");
        assert_eq!(get_base_url("/relative"), "");
    }

    #[test]
    fn test_validate_url_absolute() {
        let (url, valid) = validate_url("https://example.com", None);
        assert_eq!(url, "https://example.com");
        assert!(valid);
    }

    #[test]
    fn test_validate_url_relative_with_base() {
        let base = Url::parse("https://example.com/").unwrap();
        let (url, valid) = validate_url("/path", Some(&base));
        assert!(valid);
        assert!(url.starts_with("https://example.com"));
    }

    #[test]
    fn test_validate_url_invalid() {
        let (url, valid) = validate_url("not-a-url", None);
        assert!(!valid);
        assert_eq!(url, "not-a-url");
    }
}
