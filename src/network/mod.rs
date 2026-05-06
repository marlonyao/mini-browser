pub mod url;

use std::io::{Read, Write};
use std::net::TcpStream;
use url::Url;

pub fn fetch(url: &Url) -> Result<String, Box<dyn std::error::Error>> {
    let address = format!("{}:{}", url.host, url.port);
    let mut stream = TcpStream::connect(address)?;

    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        url.path, url.host
    );
    stream.write_all(request.as_bytes())?;

    let mut response = String::new();
    stream.read_to_string(&mut response)?;

    parse_http_response(&response)
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
    fn test_parse_http_response() {
        let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<body>Hello</body>";
        let body = parse_http_response(response).unwrap();
        assert_eq!(body, "<body>Hello</body>");
    }
}
