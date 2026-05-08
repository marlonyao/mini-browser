pub mod url;

use url::Url;

/// Fetch a URL using reqwest (supports both HTTP and HTTPS).
pub fn fetch(url: &Url) -> Result<String, Box<dyn std::error::Error>> {
    let full_url = format!("{}://{}:{}{}", url.scheme, url.host, url.port, url.path);
    let response = reqwest::blocking::get(&full_url)?;
    let body = response.text()?;
    Ok(body)
}

pub fn parse_http_response(response: &str) -> Result<String, Box<dyn std::error::Error>> {
    let split = match response.find("\r\n\r\n") {
        Some(idx) => idx + 4,
        None => match response.find("\n\n") {
            Some(idx) => idx + 2,
            None => return Err("Invalid HTTP response: no header/body separator".into()),
        }
    };
    Ok(response[split..].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_fetch_example() {
        let url = Url::parse("http://example.com/").unwrap();
        let body = fetch(&url).unwrap();
        let lower = body.to_lowercase();
        assert!(lower.contains("<html") || lower.contains("example"));
    }

    #[test]
    #[ignore]
    fn test_fetch_https() {
        let url = Url::parse("https://example.com/").unwrap();
        let body = fetch(&url).unwrap();
        let lower = body.to_lowercase();
        assert!(lower.contains("<html") || lower.contains("example"));
    }

    #[test]
    fn test_parse_http_response() {
        let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<body>Hello</body>";
        let body = parse_http_response(response).unwrap();
        assert_eq!(body, "<body>Hello</body>");
    }
}
