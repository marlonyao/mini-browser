use mini_browser::css::parser::parse_css;
use std::fs;

fn main() {
    let css = fs::read_to_string("baidu.css").expect("Failed to read baidu.css");
    let stylesheet = parse_css(&css);
    println!("Parsed {} CSS rules from baidu.css", stylesheet.rules.len());
}
