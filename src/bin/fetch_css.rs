use mini_browser::html::parser::parse_html;
use mini_browser::html::tokenizer::tokenize;
use mini_browser::dom::{Node, Element};
use std::fs;
use std::io::Write;

fn extract_stylesheets(html: &str) -> Vec<String> {
    let mut css_contents = Vec::new();
    let tokens = tokenize(html);
    let dom = parse_html(html);

    // Extract inline <style> tags
    fn extract_style_tags(node: &Node, contents: &mut Vec<String>) {
        match node {
            Node::Element(el) => {
                if el.tag == "style" {
                    for child in &el.children {
                        if let Node::Text(text) = child {
                            contents.push(text.clone());
                        }
                    }
                }
                for child in &el.children {
                    extract_style_tags(child, contents);
                }
            }
            _ => {}
        }
    }
    extract_style_tags(&dom, &mut css_contents);

    // Extract link rel=stylesheet hrefs
    fn extract_link_hrefs(node: &Node, hrefs: &mut Vec<String>) {
        match node {
            Node::Element(el) => {
                if el.tag == "link" {
                    let rel = el.attrs.get("rel").map(|s| s.as_str());
                    let href = el.attrs.get("href");
                    let type_ = el.attrs.get("type").map(|s| s.as_str());
                    if rel == Some("stylesheet") || type_ == Some("text/css") {
                        if let Some(h) = href {
                            hrefs.push(h.clone());
                        }
                    }
                }
                for child in &el.children {
                    extract_link_hrefs(child, hrefs);
                }
            }
            _ => {}
        }
    }
    let mut hrefs = Vec::new();
    extract_link_hrefs(&dom, &mut hrefs);

    println!("Found {} inline <style> blocks", css_contents.len());
    println!("Found {} external stylesheet links:", hrefs.len());
    for href in &hrefs {
        println!("  - {}", href);
    }

    // Download external CSS
    for href in &hrefs {
        let url = if href.starts_with("http") {
            href.clone()
        } else if href.starts_with("//") {
            format!("https:{}", href)
        } else {
            // Relative URL — construct from base
            format!("https://www.baidu.com{}", href)
        };
        println!("Downloading: {}", url);
        match fetch_url(&url) {
            Ok(content) => {
                let size = content.len();
                css_contents.push(content);
                println!("  OK — {} bytes", size);
            }
            Err(e) => {
                println!("  FAILED: {}", e);
            }
        }
    }

    css_contents
}

fn fetch_url(url: &str) -> Result<String, String> {
    use std::process::Command;
    let output = Command::new("curl")
        .args([
            "-sL", "--max-time", "30", "--connect-timeout", "10",
            "-H", "User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36",
            url,
        ])
        .output()
        .map_err(|e| format!("curl failed: {}", e))?;

    if !output.status.success() {
        return Err(format!("curl exited with code: {:?}", output.status.code()));
    }

    String::from_utf8(output.stdout)
        .map_err(|e| format!("Invalid UTF-8: {}", e))
}

fn main() {
    let html = fs::read_to_string("baidu_real.html").expect("Failed to read baidu_real.html");
    println!("=== Fetching all CSS for baidu.com ===");
    println!("HTML size: {} bytes", html.len());

    let css_contents = extract_stylesheets(&html);

    if css_contents.is_empty() {
        println!("No CSS found!");
        return;
    }

    let total_size: usize = css_contents.iter().map(|c| c.len()).sum();
    println!("\n=== Total CSS size: {} bytes ===", total_size);

    // Write merged CSS
    let mut file = fs::File::create("baidu.css").expect("Failed to create baidu.css");
    for content in &css_contents {
        writeln!(file, "{}", content).expect("Write failed");
        writeln!(file, "/* --- --- --- */")
            .expect("Write failed");
    }

    println!("Written to baidu.css ({} chunks merged)", css_contents.len());
}
