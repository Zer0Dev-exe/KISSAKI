use super::*;

#[test]
fn content_encoding_accepts_only_absent_or_identity_codings() {
    assert!(validate_content_encoding(&HeaderMap::new()).is_ok());
    for value in ["", "identity", " Identity ", ", identity,,IDENTITY,"] {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_ENCODING, HeaderValue::from_str(value).unwrap());
        assert!(validate_content_encoding(&headers).is_ok());
        headers.append(CONTENT_ENCODING, HeaderValue::from_static("identity"));
        assert!(validate_content_encoding(&headers).is_ok());
    }
    for value in ["gzip", "br", "zstd", "deflate", "unknown", "identity, gzip"] {
        let mut headers = HeaderMap::new();
        headers.append(CONTENT_ENCODING, HeaderValue::from_static("identity"));
        headers.append(CONTENT_ENCODING, HeaderValue::from_str(value).unwrap());
        let error = validate_content_encoding(&headers).unwrap_err();
        assert_eq!(
            error.to_string(),
            "unsupported Content-Encoding; encoded response bodies are not decoded"
        );
    }
}

#[test]
fn content_encoding_rejects_non_ascii_without_echoing_remote_values() {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_ENCODING, HeaderValue::from_bytes(b"\xff").unwrap());
    assert_eq!(
        validate_content_encoding(&headers).unwrap_err().to_string(),
        "invalid Content-Encoding header"
    );
}

#[test]
fn mime_policy_is_case_insensitive_and_never_sniffs_binary_bodies() {
    for mime in [
        "text/html",
        "TEXT/HTML; charset=utf-8",
        " application/xhtml+xml ",
    ] {
        for javascript in [false, true] {
            assert_eq!(
                extraction_mode(Some(mime), javascript),
                Some(ExtractionMode::Html { javascript })
            );
        }
    }
    for mime in [
        "application/javascript",
        "text/javascript; charset=utf-8",
        "APPLICATION/X-JAVASCRIPT",
    ] {
        assert_eq!(
            extraction_mode(Some(mime), true),
            Some(ExtractionMode::JavaScript)
        );
        assert_eq!(extraction_mode(Some(mime), false), None);
    }
    for mime in [
        None,
        Some(""),
        Some("application/octet-stream"),
        Some("application/json"),
        Some("image/png"),
    ] {
        assert_eq!(extraction_mode(mime, true), None);
    }
}
