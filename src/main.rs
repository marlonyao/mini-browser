use mini_browser::network::url::Url;
use mini_browser::network::fetch;
use mini_browser::html::parser::parse_html;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: mini-browser <url>");
        std::process::exit(1);
    }

    let url = Url::parse(&args[1])?;
    println!("Fetching {}://{}{}...", url.scheme, url.host, url.path);

    let body = fetch(&url)?;
    let dom = parse_html(&body);
    println!("{}", dom);

    Ok(())
}
