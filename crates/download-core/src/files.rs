use percent_encoding::percent_decode_str;
use reqwest::header::{HeaderMap, CONTENT_DISPOSITION};
use url::Url;

pub fn sanitize_filename(input: &str) -> String {
    let clean: String = input
        .chars()
        .filter(|c| {
            !c.is_control() && !matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
        .map(|c| {
            if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
                '_'
            } else {
                c
            }
        })
        .take(160)
        .collect();
    let clean = clean.trim().trim_matches('.').trim().to_string();
    let stem = clean.split('.').next().unwrap_or_default().to_uppercase();
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9", "COM¹",
        "COM²", "COM³", "LPT¹", "LPT²", "LPT³",
    ];
    if clean.is_empty() {
        "download".into()
    } else if reserved.contains(&stem.as_str()) {
        format!("_{clean}")
    } else {
        clean
    }
}

pub fn filename(headers: &HeaderMap, url: &Url) -> String {
    if let Some(value) = headers
        .get(CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
    {
        // Extended UTF-8 filename takes precedence over the ASCII fallback.
        for field in value.split(';').map(str::trim) {
            if let Some((key, value)) = field.split_once('=') {
                if key.eq_ignore_ascii_case("filename*") {
                    let value = value.trim_matches('"');
                    if let Some(rest) = value
                        .get(7..)
                        .filter(|_| value[..7].eq_ignore_ascii_case("utf-8''"))
                    {
                        return sanitize_filename(&percent_decode_str(rest).decode_utf8_lossy());
                    }
                }
            }
        }
        for field in value.split(';').map(str::trim) {
            if let Some((key, value)) = field.split_once('=') {
                if key.eq_ignore_ascii_case("filename") {
                    return sanitize_filename(value.trim_matches('"'));
                }
            }
        }
    }
    sanitize_filename(
        &percent_decode_str(
            url.path_segments()
                .and_then(|mut p| p.next_back())
                .unwrap_or("download"),
        )
        .decode_utf8_lossy(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_names_and_traversal() {
        assert_eq!(sanitize_filename("../../CON.exe"), "_.._CON.exe");
        assert_eq!(sanitize_filename("CON.exe"), "_CON.exe");
        assert_eq!(sanitize_filename("x:y?.zip. "), "x_y_.zip");
        assert_eq!(sanitize_filename(".."), "download");
        assert!(!sanitize_filename("../file.txt").contains('/'));
    }
    #[test]
    fn extended_filename_wins() {
        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_DISPOSITION,
            "attachment; filename=a.zip; filename*=UTF-8''%D9%81.zip"
                .parse()
                .expect("header"),
        );
        assert_eq!(
            filename(&headers, &Url::parse("https://example.com/a").expect("url")),
            "ف.zip"
        );
    }
}
