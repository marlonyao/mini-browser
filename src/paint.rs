use crate::dom::Node;
use crate::layout::{parse_value, BoxType, LayoutBox, Rect};

#[derive(Debug, Clone, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Color { r, g, b, a }
    }

    pub fn black() -> Self { Color::new(0.0, 0.0, 0.0, 1.0) }
    pub fn white() -> Self { Color::new(1.0, 1.0, 1.0, 1.0) }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DisplayCommand {
    SolidColor(Rect, Color),
    Text(String, Rect, Color),
    Border(Rect, f32, Color),
    /// Image placeholder: (url_or_src, rect, alt_text)
    Image(String, Rect, Option<String>),
}

pub fn parse_color(value: &str) -> Option<Color> {
    let value = value.trim().to_lowercase();
    match value.as_str() {
        "black" => Some(Color::new(0.0, 0.0, 0.0, 1.0)),
        "white" => Some(Color::new(1.0, 1.0, 1.0, 1.0)),
        "red" => Some(Color::new(1.0, 0.0, 0.0, 1.0)),
        "green" => Some(Color::new(0.0, 128.0 / 255.0, 0.0, 1.0)),
        "lime" => Some(Color::new(0.0, 1.0, 0.0, 1.0)),
        "blue" => Some(Color::new(0.0, 0.0, 1.0, 1.0)),
        "yellow" => Some(Color::new(1.0, 1.0, 0.0, 1.0)),
        "orange" => Some(Color::new(1.0, 0.64706, 0.0, 1.0)),
        "gray" | "grey" => Some(Color::new(0.50196, 0.50196, 0.50196, 1.0)),
        "transparent" => Some(Color::new(0.0, 0.0, 0.0, 0.0)),
        "purple" => Some(Color::new(0.50196, 0.0, 0.50196, 1.0)),
        "cyan" => Some(Color::new(0.0, 1.0, 1.0, 1.0)),
        "magenta" => Some(Color::new(1.0, 0.0, 1.0, 1.0)),
        "pink" => Some(Color::new(1.0, 0.75294, 0.79608, 1.0)),
        "brown" => Some(Color::new(0.64706, 0.16471, 0.16471, 1.0)),
        "silver" => Some(Color::new(0.75294, 0.75294, 0.75294, 1.0)),
        _ => parse_hex_or_rgb(&value),
    }
}

fn parse_hex_or_rgb(value: &str) -> Option<Color> {
    if value.starts_with('#') {
        parse_hex(value)
    } else if value.starts_with("rgb(") && value.ends_with(')') {
        parse_rgb(value)
    } else {
        None
    }
}

fn parse_hex(value: &str) -> Option<Color> {
    let hex = &value[1..];
    match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()? as f32 / 255.0;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()? as f32 / 255.0;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()? as f32 / 255.0;
            Some(Color::new(r, g, b, 1.0))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
            Some(Color::new(r, g, b, 1.0))
        }
        _ => None,
    }
}

fn parse_rgb(value: &str) -> Option<Color> {
    let inner = &value[4..value.len() - 1];
    let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
    if parts.len() != 3 { return None; }
    let r = parts[0].parse::<f32>().ok()? / 255.0;
    let g = parts[1].parse::<f32>().ok()? / 255.0;
    let b = parts[2].parse::<f32>().ok()? / 255.0;
    Some(Color::new(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0), 1.0))
}

pub fn build_display_list(layout_root: &LayoutBox) -> Vec<DisplayCommand> {
    let mut list = Vec::new();
    let viewport_clip = Rect { x: 0.0, y: 0.0, width: f32::MAX, height: f32::MAX };
    build_display_list_inner(layout_root, &mut list, &viewport_clip);
    list
}

fn build_display_list_inner(
    layout_box: &LayoutBox,
    list: &mut Vec<DisplayCommand>,
    clip: &Rect,
) {
    let styled = match &layout_box.box_type {
        BoxType::BlockNode(s) | BoxType::InlineNode(s) => Some(s),
        BoxType::AnonymousBlock => None,
    };

    let dim = &layout_box.dimensions;

    // 1. Background
    if let Some(bg) = styled.and_then(|s| {
        s.specified_values.get("background-color")
            .or_else(|| s.specified_values.get("background"))
    }) {
        if let Some(color) = parse_color(bg) {
            let rect = Rect {
                x: dim.content.x - dim.padding.left,
                y: dim.content.y - dim.padding.top,
                width: dim.content.width + dim.padding.left + dim.padding.right,
                height: dim.content.height + dim.padding.top + dim.padding.bottom,
            };
            if let Some(clipped) = rect_intersect(&rect, clip) {
                list.push(DisplayCommand::SolidColor(clipped, color));
            }
        }
    }

    // 2. Border
    if let Some(styled) = styled {
        let border_width = parse_value(
            styled.specified_values.get("border-width")
                .or_else(|| styled.specified_values.get("border-top-width"))
        );
        if border_width > 0.0 {
            if let Some(color_val) = styled.specified_values.get("border-color")
                .or_else(|| styled.specified_values.get("border-top-color"))
            {
                if let Some(color) = parse_color(color_val) {
                    let rect = Rect {
                        x: dim.content.x - dim.padding.left - dim.border.left,
                        y: dim.content.y - dim.padding.top - dim.border.top,
                        width: dim.content.width + dim.padding.left + dim.padding.right
                             + dim.border.left + dim.border.right,
                        height: dim.content.height + dim.padding.top + dim.padding.bottom
                              + dim.border.top + dim.border.bottom,
                    };
                    if let Some(clipped) = rect_intersect(&rect, clip) {
                        list.push(DisplayCommand::Border(clipped, border_width, color));
                    }
                }
            }
        }
    }

    // 3. Children
    let child_clip = Rect {
        x: dim.content.x,
        y: dim.content.y,
        width: dim.content.width,
        height: dim.content.height,
    };
    for child in &layout_box.children {
        build_display_list_inner(child, list, &child_clip);
    }

    // 4. Text (with line wrapping based on container width)
    if let Some(styled) = styled {
        let color = styled.specified_values.get("color")
            .and_then(|v| parse_color(v))
            .unwrap_or_else(Color::black);

        let font_size = parse_value(styled.specified_values.get("font-size")).max(16.0);
        let line_height = font_size * 1.2;
        let char_width = font_size * 0.5;

        let mut text_y = dim.content.y;
        let container_width = dim.content.width;

        // Word-wrap: split text into lines that fit within container_width
        for child in &styled.children {
            if let Node::Text(text) = &child.node {
                let trimmed = text.trim();
                if trimmed.is_empty() { continue; }

                let chars_per_line = if container_width > 0.0 && char_width > 0.0 {
                    (container_width / char_width).floor().max(1.0) as usize
                } else {
                    trimmed.chars().count()
                };

                let mut line_start = 0usize;
                let text_len = trimmed.chars().count();

                while line_start < text_len {
                    let line_end = (line_start + chars_per_line).min(text_len);
                    // Try to break at word boundary (space) if possible
                    let mut actual_end = line_end;
                    if actual_end < text_len {
                        // Look backward for a space to break at
                        let slice: String = trimmed.chars().skip(line_start).take(line_end - line_start).collect();
                        if let Some(last_space) = slice.rfind(' ') {
                            actual_end = line_start + last_space;
                        }
                    }

                    let line_text: String = trimmed.chars().skip(line_start).take(actual_end - line_start).collect();
                    let trimmed_line = line_text.trim();
                    if !trimmed_line.is_empty() {
                        let line_width = trimmed_line.chars().count() as f32 * char_width;
                        let text_rect = Rect {
                            x: dim.content.x,
                            y: text_y,
                            width: line_width,
                            height: line_height,
                        };
                        if let Some(clipped) = rect_intersect(&text_rect, clip) {
                            list.push(DisplayCommand::Text(trimmed_line.to_string(), clipped, color.clone()));
                        }
                        text_y += line_height;
                    }

                    line_start = actual_end;
                    // Skip leading space on next line
                    while line_start < text_len {
                        let ch = trimmed.chars().nth(line_start);
                        if ch == Some(' ') { line_start += 1; } else { break; }
                    }
                }
            }
        }

        // 5. Link underline (drawn under the last line of text)
        if let Node::Element(el) = &styled.node {
            if el.tag == "a" && text_y > dim.content.y {
                let underline_rect = Rect {
                    x: dim.content.x,
                    y: text_y - 1.0,
                    width: dim.content.width,
                    height: 1.0,
                };
                if let Some(clipped) = rect_intersect(&underline_rect, clip) {
                    list.push(DisplayCommand::Border(clipped, 1.0, color));
                }
            }
            // 6. Input placeholder
            if el.tag == "input" {
                let value = el.attrs.get("value")
                    .or_else(|| el.attrs.get("placeholder"))
                    .cloned()
                    .unwrap_or_default();
                let input_rect = Rect {
                    x: dim.content.x,
                    y: dim.content.y,
                    width: dim.content.width,
                    height: dim.content.height,
                };
                if let Some(clipped) = rect_intersect(&input_rect, clip) {
                    // White background
                    list.push(DisplayCommand::SolidColor(clipped.clone(), Color::new(1.0, 1.0, 1.0, 1.0)));
                    // Gray border
                    list.push(DisplayCommand::Border(clipped.clone(), 1.0, Color::new(0.7, 0.7, 0.7, 1.0)));
                    // Text inside
                    if !value.is_empty() {
                        let text_rect = Rect {
                            x: dim.content.x + 4.0,
                            y: dim.content.y + 2.0,
                            width: dim.content.width - 8.0,
                            height: dim.content.height - 4.0,
                        };
                        if let Some(clipped_text) = rect_intersect(&text_rect, clip) {
                            list.push(DisplayCommand::Text(value, clipped_text, Color::black()));
                        }
                    }
                }
            }
            // 7. Image placeholder
            if el.tag == "img" {
                let alt = el.attrs.get("alt").cloned();
                let img_rect = Rect {
                    x: dim.content.x,
                    y: dim.content.y,
                    width: dim.content.width,
                    height: dim.content.height,
                };
                if let Some(clipped) = rect_intersect(&img_rect, clip) {
                    let src = el.attrs.get("src").cloned().unwrap_or_default();
                    list.push(DisplayCommand::Image(src, clipped, alt));
                }
            }
        }
    }
}

fn rect_intersect(a: &Rect, b: &Rect) -> Option<Rect> {
    let x1 = a.x.max(b.x);
    let y1 = a.y.max(b.y);
    let x2 = (a.x + a.width).min(b.x + b.width);
    let y2 = (a.y + a.height).min(b.y + b.height);
    let width = x2 - x1;
    let height = y2 - y1;
    if width > 0.0 && height > 0.0 {
        Some(Rect { x: x1, y: y1, width, height })
    } else {
        None
    }
}
