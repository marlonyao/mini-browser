use eframe::egui;
use mini_browser::html::parser::parse_html;
use mini_browser::css::parser::{parse_css, Stylesheet};
use mini_browser::style::style_tree;
use mini_browser::dom::Node;
use mini_browser::layout::{build_layout_tree, layout, Dimensions, Rect};
use mini_browser::paint::DisplayCommand;
use std::sync::{Arc, Mutex};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("Mini Browser"),
        ..Default::default()
    };

    eframe::run_native(
        "Mini Browser",
        options,
        Box::new(|cc| {
            // Load system CJK fonts for Chinese/Japanese/Korean support
            let mut fonts = egui::FontDefinitions::default();
            load_cjk_fonts(&mut fonts, cc);
            cc.egui_ctx.set_fonts(fonts);
            Ok(Box::new(BrowserApp::default()))
        }),
    )
}

fn load_cjk_fonts(fonts: &mut egui::FontDefinitions, _cc: &eframe::CreationContext<'_>) {
    // Try to load a CJK-capable font from the system
    let cjk_font_paths = [
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/System/Library/Fonts/PingFang.ttc",           // macOS
        "/System/Library/Fonts/STHeiti Medium.ttc",      // macOS alt
        "/System/Library/Fonts/Hiragino Sans GB.ttc",    // macOS alt
        "C:\\Windows\\Fonts\\msyh.ttc",                     // Windows (Microsoft YaHei)
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc", // Linux WenQuanYi
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",   // Linux WenQuanYi alt
        "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
    ];

    for path in &cjk_font_paths {
        if let Ok(font_data) = std::fs::read(path) {
            fonts.font_data.insert(
                "cjk".into(),
                std::sync::Arc::new(egui::FontData::from_owned(font_data)),
            );
            // Extend the proportional font family with CJK font
            fonts.families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push("cjk".into());
            fonts.families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push("cjk".into());
            break;
        }
    }
}

#[derive(Default)]
struct BrowserApp {
    url: String,
    page: Option<PageResult>,
    error: Option<String>,
    loading: bool,
    // Shared state for background fetch
    fetch_result: Option<Arc<Mutex<Option<FetchResult>>>>,
    ctx_ref: Option<egui::Context>,
    debug_info: String,
}

enum FetchResult {
    Ok(PageResult),
    Err(String),
}

struct PageResult {
    display_list: Vec<DisplayCommand>,
    content_height: f32,  // Total height of page content for scrolling
}

impl eframe::App for BrowserApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Save context for background thread wake-up
        if self.ctx_ref.is_none() {
            self.ctx_ref = Some(ctx.clone());
        }

        // Check if background fetch completed
        if let Some(result_arc) = &self.fetch_result {
            if let Ok(mut guard) = result_arc.lock() {
                if guard.is_some() {
                    let result = guard.take().unwrap();
                    self.loading = false;
                    match result {
                        FetchResult::Ok(page) => {
                            self.debug_info = format!("display_list: {} items, content_height: {:.1}", page.display_list.len(), page.content_height);
                            self.page = Some(page);
                            self.error = None;
                        }
                        FetchResult::Err(e) => {
                            self.debug_info = format!("error: {}", e);
                            self.error = Some(e);
                            self.page = None;
                        }
                    }
                }
            }
        }

        // Top bar - URL input
        egui::TopBottomPanel::top("url_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("URL:");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.url)
                        .desired_width(600.0)
                        .hint_text("https://example.com"),
                );
                let go_clicked = ui.button(if self.loading { "⏳" } else { "Go" }).clicked();
                let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter))
                    && response.has_focus();
                if (go_clicked || enter_pressed) && !self.loading {
                    self.load_page();
                }
            });
            if !self.debug_info.is_empty() {
                ui.label(egui::RichText::new(&self.debug_info).small().color(egui::Color32::GRAY));
            }
        });

        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.loading {
                ui.vertical_centered(|ui| {
                    ui.add_space(200.0);
                    ui.spinner();
                    ui.label("Loading...");
                });
            } else if let Some(err) = &self.error {
                ui.colored_label(egui::Color32::RED, err);
            } else if let Some(page) = &self.page {
                self.render_page(ui, page);
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(200.0);
                    ui.heading("Mini Browser");
                    ui.label("Enter a URL (http/https/file) and press Enter");
                });
            }
        });
    }
}

impl BrowserApp {
    fn load_page(&mut self) {
        let url = self.url.trim().to_string();
        if url.is_empty() {
            return;
        }

        self.loading = true;
        self.error = None;
        self.page = None;

        let result_arc = Arc::new(Mutex::new(None));
        self.fetch_result = Some(result_arc.clone());
        let ctx = self.ctx_ref.clone();

        std::thread::spawn(move || {
            let result = fetch_and_render(&url);
            {
                let mut guard = result_arc.lock().unwrap();
                *guard = Some(result);
            }
            // Wake up the UI thread to repaint
            if let Some(ctx) = ctx {
                ctx.request_repaint();
            }
        });
    }

    fn render_page(&self, ui: &mut egui::Ui, page: &PageResult) {
        let content_height = page.content_height.max(ui.available_height());

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Allocate space for the full page content
                let (_id, rect) = ui.allocate_space(
                    egui::vec2(ui.available_width(), content_height)
                );
                let painter = ui.painter_at(rect);
                let origin = rect.min;

                for cmd in &page.display_list {
                    match cmd {
                        DisplayCommand::SolidColor(cmd_rect, color) => {
                            let egui_rect = egui::Rect::from_min_max(
                                egui::pos2(origin.x + cmd_rect.x, origin.y + cmd_rect.y),
                                egui::pos2(
                                    origin.x + cmd_rect.x + cmd_rect.width,
                                    origin.y + cmd_rect.y + cmd_rect.height,
                                ),
                            );
                            let egui_color = egui::Color32::from_rgb(
                                (color.r * 255.0) as u8,
                                (color.g * 255.0) as u8,
                                (color.b * 255.0) as u8,
                            );
                            painter.rect_filled(egui_rect, 0.0, egui_color);
                        }
                        DisplayCommand::Text(text, cmd_rect, color) => {
                            let pos = egui::pos2(
                                origin.x + cmd_rect.x,
                                origin.y + cmd_rect.y,
                            );
                            let egui_color = egui::Color32::from_rgb(
                                (color.r * 255.0) as u8,
                                (color.g * 255.0) as u8,
                                (color.b * 255.0) as u8,
                            );
                            painter.text(
                                pos,
                                egui::Align2::LEFT_TOP,
                                text,
                                egui::FontId::proportional(16.0),
                                egui_color,
                            );
                        }
                        DisplayCommand::Border(cmd_rect, _bw, color) => {
                            let egui_rect = egui::Rect::from_min_max(
                                egui::pos2(origin.x + cmd_rect.x, origin.y + cmd_rect.y),
                                egui::pos2(
                                    origin.x + cmd_rect.x + cmd_rect.width,
                                    origin.y + cmd_rect.y + cmd_rect.height,
                                ),
                            );
                            let egui_color = egui::Color32::from_rgb(
                                (color.r * 255.0) as u8,
                                (color.g * 255.0) as u8,
                                (color.b * 255.0) as u8,
                            );
                            painter.rect_stroke(egui_rect, 0.0, egui::Stroke::new(2.0, egui_color), egui::StrokeKind::Outside);
                        }
                    }
                }
            });
    }
}

/// Fetch URL and render HTML into a PageResult (runs on background thread)
fn fetch_and_render(url: &str) -> FetchResult {
    let html_result = if url.starts_with("file://") {
        let path = &url[7..];
        std::fs::read_to_string(path).map_err(|e| e.to_string())
    } else if url.starts_with("http://") || url.starts_with("https://") {
        match mini_browser::network::url::Url::parse(url) {
            Ok(parsed) => mini_browser::network::fetch(&parsed).map_err(|e| e.to_string()),
            Err(e) => Err(e.to_string()),
        }
    } else {
        // Treat as file path
        std::fs::read_to_string(url).map_err(|e| e.to_string())
    };

    match html_result {
        Ok(html) => {
            let dom = parse_html(&html);

            // Extract stylesheets (inline + external)
            let mut stylesheet = Stylesheet { rules: Vec::new() };
            collect_styles(&dom, &mut stylesheet, url);

            // Merge UA default styles (lowest specificity)
            mini_browser::style::merge_default_styles(&mut stylesheet);

            // Build styled tree
            let styled = style_tree(&dom, &stylesheet);

            // Build layout tree
            let mut layout_root = build_layout_tree(&styled);
            let viewport = Dimensions {
                content: Rect { x: 0.0, y: 0.0, width: 800.0, height: 600.0 },
                ..Dimensions::default()
            };
            layout(&mut layout_root, viewport);

            // Build display list
            let display_list = mini_browser::paint::build_display_list(&layout_root);

            // Calculate total content height
            let content_height = display_list.iter().map(|cmd| match cmd {
                DisplayCommand::SolidColor(rect, _) | DisplayCommand::Text(_, rect, _) | DisplayCommand::Border(rect, _, _) => {
                    rect.y + rect.height
                }
            }).fold(0.0f32, f32::max);

            FetchResult::Ok(PageResult { display_list, content_height })
        }
        Err(e) => FetchResult::Err(format!("Failed to load: {}", e)),
    }
}

fn collect_styles(node: &Node, stylesheet: &mut Stylesheet, base_url: &str) {
    if let Node::Element(element) = node {
        if element.tag == "style" {
            let mut css_text = String::new();
            for child in &element.children {
                if let Node::Text(text) = child {
                    css_text.push_str(text);
                }
            }
            let parsed = parse_css(&css_text);
            stylesheet.rules.extend(parsed.rules);
        } else if element.tag == "link" {
            if let Some(rel) = element.attrs.get("rel") {
                if rel == "stylesheet" {
                    if let Some(href) = element.attrs.get("href") {
                        let absolute_url = resolve_url(base_url, href);
                        match fetch_css(&absolute_url) {
                            Ok(css_text) => {
                                let parsed = parse_css(&css_text);
                                stylesheet.rules.extend(parsed.rules);
                            }
                            Err(e) => {
                                eprintln!("Failed to load stylesheet {}: {}", absolute_url, e);
                            }
                        }
                    }
                }
            }
        }
        for child in &element.children {
            collect_styles(child, stylesheet, base_url);
        }
    }
}

fn resolve_url(base: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") || href.starts_with("file://") {
        return href.to_string();
    }
    if href.starts_with("//") {
        // Protocol-relative URL
        if let Some(scheme_end) = base.find("://") {
            let scheme = &base[..scheme_end];
            return format!("{}:{}", scheme, href);
        }
        return format!("https:{}", href);
    }
    if href.starts_with('/') {
        // Absolute path
        match mini_browser::network::url::Url::parse(base) {
            Ok(parsed) => {
                return format!("{}://{}:{}{}", parsed.scheme, parsed.host, parsed.port, href);
            }
            Err(_) => return href.to_string(),
        }
    }
    // Relative path
    match mini_browser::network::url::Url::parse(base) {
        Ok(parsed) => {
            let mut path = parsed.path.clone();
            // Ensure path ends with /
            if !path.ends_with('/') {
                if let Some(last_slash) = path.rfind('/') {
                    path = path[..=last_slash].to_string();
                } else {
                    path.push('/');
                }
            }
            return format!("{}://{}:{}{}{}", parsed.scheme, parsed.host, parsed.port, path, href);
        }
        Err(_) => return href.to_string(),
    }
}

fn fetch_css(url: &str) -> Result<String, String> {
    if url.starts_with("file://") {
        let path = &url[7..];
        std::fs::read_to_string(path).map_err(|e| e.to_string())
    } else if url.starts_with("http://") || url.starts_with("https://") {
        match mini_browser::network::url::Url::parse(url) {
            Ok(parsed) => mini_browser::network::fetch(&parsed).map_err(|e| e.to_string()),
            Err(e) => Err(e.to_string()),
        }
    } else {
        std::fs::read_to_string(url).map_err(|e| e.to_string())
    }
}
