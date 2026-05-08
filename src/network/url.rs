#[derive(Debug, PartialEq)]
pub struct Url {
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub path: String,
}

impl Url {
    pub fn parse(url: &str) -> Result<Url, Box<dyn std::error::Error>> {
        let scheme_end = url.find("://").ok_or("Invalid URL: missing scheme")?;
        let scheme = &url[..scheme_end];
        let rest = &url[scheme_end + 3..];

        let (host_port, path) = match rest.find('/') {
            Some(idx) => (&rest[..idx], &rest[idx..]),
            None => (rest, "/"),
        };

        let (host, port) = match host_port.find(':') {
            Some(idx) => {
                let host = &host_port[..idx];
                let port = host_port[idx + 1..].parse::<u16>()?;
                (host, port)
            }
            None => {
                let default_port = match scheme {
                    "https" => 443,
                    _ => 80,
                };
                (host_port, default_port)
            }
        };

        Ok(Url {
            scheme: scheme.to_string(),
            host: host.to_string(),
            port,
            path: if path.is_empty() { "/".to_string() } else { path.to_string() },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_http_url() {
        let url = Url::parse("http://example.com/index.html").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.host, "example.com");
        assert_eq!(url.port, 80);
        assert_eq!(url.path, "/index.html");
    }

    #[test]
    fn test_parse_url_with_port() {
        let url = Url::parse("http://example.com:8080/path").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.host, "example.com");
        assert_eq!(url.port, 8080);
        assert_eq!(url.path, "/path");
    }

    #[test]
    fn test_parse_url_default_port() {
        let url = Url::parse("http://example.com/").unwrap();
        assert_eq!(url.port, 80);
        assert_eq!(url.path, "/");
    }

    #[test]
    fn test_parse_url_with_query() {
        let url = Url::parse("http://example.com/search?q=hello").unwrap();
        assert_eq!(url.path, "/search?q=hello");
    }
}
