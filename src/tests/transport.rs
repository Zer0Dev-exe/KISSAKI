use super::*;

#[test]
fn mime_policy_is_case_insensitive_and_never_sniffs_binary_bodies() {
    for mime in [
        "text/html",
        "TEXT/HTML; charset=utf-8",
        " application/xhtml+xml ",
    ] {
        assert!(is_extractable(Some(mime), false));
    }
    for mime in [
        "application/javascript",
        "text/javascript; charset=utf-8",
        "APPLICATION/X-JAVASCRIPT",
    ] {
        assert!(is_extractable(Some(mime), true));
        assert!(!is_extractable(Some(mime), false));
    }
    for mime in [
        None,
        Some(""),
        Some("application/octet-stream"),
        Some("application/json"),
        Some("image/png"),
    ] {
        assert!(!is_extractable(mime, true));
    }
}
