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
}

struct PageResult {
    display_list: Vec<DisplayCommand>,
    content_height: f32,  // Total height of page content for scrolling
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
                        .hint_text("https://example.com"),
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

        // Parse URL
        let html = if url.starts_with("file://") {
            let path = &url[7..];
            std::fs::read_to_string(path).map_err(|e| e.to_string())
        } else if url.starts_with("http://") || url.starts_with("https://") {
            match mini_browser::network::url::Url::parse(&url) {
                Ok(parsed) => mini_browser::network::fetch(&parsed).map_err(|e| e.to_string()),
                Err(e) => Err(e.to_string()),
            }
        } else {
            // Treat as file path
            std::fs::read_to_string(&url).map_err(|e| e.to_string())
        };

        match html {
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

                // Calculate total content height
                let content_height = display_list.iter().map(|cmd| match cmd {
                    DisplayCommand::SolidColor(rect, _) | DisplayCommand::Text(_, rect, _) | DisplayCommand::Border(rect, _, _) => {
                        rect.y + rect.height
                    }
                }).fold(0.0f32, f32::max);

                self.page = Some(PageResult { display_list, content_height });
                self.error = None;
            }
            Err(e) => {
                self.error = Some(format!("Failed to load: {}", e));
                self.page = None;
            }
        }
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
