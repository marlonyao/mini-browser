use eframe::egui;
use mini_browser::html::parser::parse_html;
use mini_browser::css::parser::{parse_css, Stylesheet};
use mini_browser::style::style_tree;
use mini_browser::dom::Node;
use mini_browser::layout::{build_layout_tree, layout, Dimensions, Rect};
use mini_browser::paint::DisplayCommand;

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
        Box::new(|_cc| Ok(Box::new(BrowserApp::default()))),
    )
}

#[derive(Default)]
struct BrowserApp {
    url: String,
    page: Option<PageResult>,
    error: Option<String>,
}

struct PageResult {
    display_list: Vec<DisplayCommand>,
}

impl eframe::App for BrowserApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top bar - URL input
        egui::TopBottomPanel::top("url_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("URL:");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.url)
                        .desired_width(600.0)
                        .hint_text("file:///path/to/page.html"),
                );
                if ui.button("Go").clicked()
                    || (ui.input(|i| i.key_pressed(egui::Key::Enter))
                        && response.has_focus())
                {
                    self.load_page();
                }
            });
        });

        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(err) = &self.error {
                ui.colored_label(egui::Color32::RED, err);
            } else if let Some(page) = &self.page {
                self.render_page(ui, page);
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(200.0);
                    ui.heading("Mini Browser");
                    ui.label("Enter a file:// URL and press Go");
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

        // Parse URL
        let path = if url.starts_with("file://") {
            url[7..].to_string()
        } else {
            url.clone()
        };

        match std::fs::read_to_string(&path) {
            Ok(html) => {
                let dom = parse_html(&html);

                // Extract inline stylesheets
                let mut stylesheet = Stylesheet { rules: Vec::new() };
                collect_styles(&dom, &mut stylesheet);

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

                self.page = Some(PageResult { display_list });
                self.error = None;
            }
            Err(e) => {
                self.error = Some(format!("Failed to load: {}", e));
                self.page = None;
            }
        }
    }

    fn render_page(&self, ui: &mut egui::Ui, page: &PageResult) {
        // Use a painter to draw the display commands
        let painter = ui.painter();
        let available = ui.available_rect_before_wrap();

        for cmd in &page.display_list {
            match cmd {
                DisplayCommand::SolidColor(rect, color) => {
                    let egui_rect = egui::Rect::from_min_max(
                        egui::pos2(available.min.x + rect.x, available.min.y + rect.y),
                        egui::pos2(
                            available.min.x + rect.x + rect.width,
                            available.min.y + rect.y + rect.height,
                        ),
                    );
                    let egui_color = egui::Color32::from_rgb(
                        (color.r * 255.0) as u8,
                        (color.g * 255.0) as u8,
                        (color.b * 255.0) as u8,
                    );
                    painter.rect_filled(egui_rect, 0.0, egui_color);
                }
                DisplayCommand::Text(text, rect, color) => {
                    let pos = egui::pos2(
                        available.min.x + rect.x,
                        available.min.y + rect.y,
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
                DisplayCommand::Border(rect, _bw, color) => {
                    let egui_rect = egui::Rect::from_min_max(
                        egui::pos2(available.min.x + rect.x, available.min.y + rect.y),
                        egui::pos2(
                            available.min.x + rect.x + rect.width,
                            available.min.y + rect.y + rect.height,
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
    }
}

fn collect_styles(node: &Node, stylesheet: &mut Stylesheet) {
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
        }
        for child in &element.children {
            collect_styles(child, stylesheet);
        }
    }
}
