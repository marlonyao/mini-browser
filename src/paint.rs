use crate::dom::Node;
use crate::layout::{parse_value, BoxType, LayoutBox, Rect};
use fontdue::layout::{Layout, TextStyle, CoordinateSystem};
use fontdue::Font;

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

    pub fn black() -> Self {
        Color::new(0.0, 0.0, 0.0, 1.0)
    }

    pub fn white() -> Self {
        Color::new(1.0, 1.0, 1.0, 1.0)
    }

    pub fn red() -> Self {
        Color::new(1.0, 0.0, 0.0, 1.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DisplayCommand {
    SolidColor(Rect, Color),
    Text(String, Rect, Color),
    Border(Rect, f32, Color),
}

pub fn parse_color(value: &str) -> Option<Color> {
    let value = value.trim().to_lowercase();
    match value.as_str() {
        "black" => Some(Color::new(0.0, 0.0, 0.0, 1.0)),
        "white" => Some(Color::new(1.0, 1.0, 1.0, 1.0)),
        "red" => Some(Color::red()),
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
        4 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()? as f32 / 255.0;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()? as f32 / 255.0;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()? as f32 / 255.0;
            let a = u8::from_str_radix(&hex[3..4].repeat(2), 16).ok()? as f32 / 255.0;
            Some(Color::new(r, g, b, a))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
            Some(Color::new(r, g, b, 1.0))
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()? as f32 / 255.0;
            Some(Color::new(r, g, b, a))
        }
        _ => None,
    }
}

fn parse_rgb(value: &str) -> Option<Color> {
    let inner = &value[4..value.len() - 1];
    let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
    if parts.len() != 3 {
        return None;
    }
    let r = parts[0].parse::<f32>().ok()? / 255.0;
    let g = parts[1].parse::<f32>().ok()? / 255.0;
    let b = parts[2].parse::<f32>().ok()? / 255.0;
    Some(Color::new(
        r.clamp(0.0, 1.0),
        g.clamp(0.0, 1.0),
        b.clamp(0.0, 1.0),
        1.0,
    ))
}

pub fn build_display_list(layout_root: &LayoutBox) -> Vec<DisplayCommand> {
    let mut list = Vec::new();
    let viewport_clip = Rect {
        x: 0.0,
        y: 0.0,
        width: f32::MAX,
        height: f32::MAX,
    };
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
        s.specified_values
            .get("background-color")
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

    // 2. Border (simplified: uniform border)
    if let Some(styled) = styled {
        let border_width = parse_value(
            styled
                .specified_values
                .get("border-width")
                .or_else(|| styled.specified_values.get("border-top-width")),
        );
        if border_width > 0.0 {
            if let Some(color_val) = styled
                .specified_values
                .get("border-color")
                .or_else(|| styled.specified_values.get("border-top-color"))
            {
                if let Some(color) = parse_color(color_val) {
                    let rect = Rect {
                        x: dim.content.x - dim.padding.left - dim.border.left,
                        y: dim.content.y - dim.padding.top - dim.border.top,
                        width: dim.content.width
                            + dim.padding.left
                            + dim.padding.right
                            + dim.border.left
                            + dim.border.right,
                        height: dim.content.height
                            + dim.padding.top
                            + dim.padding.bottom
                            + dim.border.top
                            + dim.border.bottom,
                    };
                    if let Some(clipped) = rect_intersect(&rect, clip) {
                        list.push(DisplayCommand::Border(clipped, border_width, color));
                    }
                }
            }
        }
    }

    // 3. Children — clipped to parent's content area
    let child_clip = Rect {
        x: dim.content.x,
        y: dim.content.y,
        width: dim.content.width,
        height: dim.content.height,
    };
    for child in &layout_box.children {
        build_display_list_inner(child, list, &child_clip);
    }

    // 4. Text
    if let Some(styled) = styled {
        let color = styled
            .specified_values
            .get("color")
            .and_then(|v| parse_color(v))
            .unwrap_or_else(Color::black);

        let font_size = parse_value(styled.specified_values.get("font-size")).max(16.0);
        let line_height = font_size * 1.2;
        let char_width = font_size * 0.6; // rough estimate

        let mut text_y = dim.content.y;
        for child in &styled.children {
            if let Node::Text(text) = &child.node {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    // Calculate how many lines this text needs
                    let text_width = trimmed.chars().count() as f32 * char_width;
                    let num_lines = ((text_width / dim.content.width).ceil() as usize).max(1);
                    let text_height = line_height * num_lines as f32;

                    let text_rect = Rect {
                        x: dim.content.x,
                        y: text_y,
                        width: dim.content.width,
                        height: text_height,
                    };
                    if let Some(clipped) = rect_intersect(&text_rect, clip) {
                        list.push(DisplayCommand::Text(
                            trimmed.to_string(),
                            clipped,
                            color.clone(),
                        ));
                    }
                    text_y += text_height;
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
        Some(Rect {
            x: x1,
            y: y1,
            width,
            height,
        })
    } else {
        None
    }
}

pub fn render_to_terminal(commands: &[DisplayCommand], width: usize, height: usize) -> String {
    #[derive(Clone)]
    struct Cell {
        ch: char,
        fg: Color,
        bg: Color,
    }

    let white = Color::white();
    let black = Color::black();
    let mut grid = vec![vec![Cell { ch: ' ', fg: black.clone(), bg: white.clone() }; width]; height];

    for cmd in commands {
        match cmd {
            DisplayCommand::SolidColor(rect, color) => {
                let x0 = rect.x.max(0.0) as usize;
                let y0 = rect.y.max(0.0) as usize;
                let x1 = ((rect.x + rect.width).ceil() as usize).min(width);
                let y1 = ((rect.y + rect.height).ceil() as usize).min(height);
                for y in y0..y1 {
                    for x in x0..x1 {
                        grid[y][x].bg = color.clone();
                        grid[y][x].ch = ' ';
                    }
                }
            }
            DisplayCommand::Border(rect, bw, color) => {
                let x0 = rect.x.max(0.0) as usize;
                let y0 = rect.y.max(0.0) as usize;
                let x1 = ((rect.x + rect.width).ceil() as usize).min(width);
                let y1 = ((rect.y + rect.height).ceil() as usize).min(height);
                let w = *bw as usize;
                for y in y0..y1 {
                    for x in x0..x1 {
                        if y < y0 + w || y + w >= y1 || x < x0 + w || x + w >= x1 {
                            grid[y][x].bg = color.clone();
                            grid[y][x].ch = ' ';
                        }
                    }
                }
            }
            DisplayCommand::Text(text, rect, color) => {
                let x0 = rect.x.max(0.0) as usize;
                let y0 = rect.y.max(0.0) as usize;
                let x1 = ((rect.x + rect.width).ceil() as usize).min(width);
                let y1 = ((rect.y + rect.height).ceil() as usize).min(height);
                let mut x = x0;
                let mut y = y0;
                for ch in text.chars() {
                    if x < x1 && y < y1 {
                        grid[y][x].ch = ch;
                        grid[y][x].fg = color.clone();
                    }
                    x += 1;
                    if x >= x1 {
                        x = x0;
                        y += 1;
                    }
                }
            }
        }
    }

    let mut result = String::new();
    for row in &grid {
        let mut current_fg = black.clone();
        let mut current_bg = white.clone();
        for cell in row {
            if cell.fg != current_fg {
                result.push_str(&format!(
                    "\x1b[38;2;{};{};{}m",
                    (cell.fg.r * 255.0) as u8,
                    (cell.fg.g * 255.0) as u8,
                    (cell.fg.b * 255.0) as u8
                ));
                current_fg = cell.fg.clone();
            }
            if cell.bg != current_bg {
                result.push_str(&format!(
                    "\x1b[48;2;{};{};{}m",
                    (cell.bg.r * 255.0) as u8,
                    (cell.bg.g * 255.0) as u8,
                    (cell.bg.b * 255.0) as u8
                ));
                current_bg = cell.bg.clone();
            }
            result.push(cell.ch);
        }
        result.push_str("\x1b[0m\n");
    }
    result
}

/// Render display commands to a pixel buffer (shared by PPM and GUI)
pub fn render_to_pixels(commands: &[DisplayCommand], width: usize, height: usize) -> Vec<Vec<Color>> {
    let mut pixels = vec![vec![Color::white(); width]; height];

    // Load default font
    let font_data = include_bytes!("/usr/share/fonts/liberation-sans/LiberationSans-Regular.ttf");
    let font = Font::from_bytes(font_data as &[u8], fontdue::FontSettings::default()).unwrap();
    let font_bold_data = include_bytes!("/usr/share/fonts/liberation-sans/LiberationSans-Bold.ttf");
    let font_bold = Font::from_bytes(font_bold_data as &[u8], fontdue::FontSettings::default()).unwrap();
    let fonts = &[&font, &font_bold];

    for cmd in commands {
        match cmd {
            DisplayCommand::SolidColor(rect, color) => {
                fill_rect(&mut pixels, rect, color, width, height);
            }
            DisplayCommand::Border(rect, bw, color) => {
                draw_border(&mut pixels, rect, *bw, color, width, height);
            }
            DisplayCommand::Text(text, rect, color) => {
                let font_size = 16.0f32; // default font size
                render_text(&mut pixels, text, rect, color, width, height, fonts, font_size);
            }
        }
    }

    pixels
}

/// Render text using fontdue for glyph rasterization
fn render_text(
    pixels: &mut [Vec<Color>],
    text: &str,
    rect: &Rect,
    color: &Color,
    width: usize,
    height: usize,
    fonts: &[&Font],
    font_size: f32,
) {
    use fontdue::layout::{LayoutSettings, WrapStyle, HorizontalAlign, VerticalAlign};

    let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
    let settings = LayoutSettings {
        x: 0.0,
        y: 0.0,
        max_width: Some(rect.width),
        max_height: None,
        wrap_style: WrapStyle::Word,
        wrap_hard_breaks: true,
        horizontal_align: HorizontalAlign::Left,
        vertical_align: VerticalAlign::Top,
        line_height: 0.0,
    };
    layout.append(fonts, &TextStyle::new(text, font_size, 0));

    // If layout didn't wrap (no max_width set in append), we need to use settings
    layout.reset(&settings);
    layout.append(fonts, &TextStyle::new(text, font_size, 0));

    let x0 = rect.x.max(0.0) as i32;
    let y0 = rect.y.max(0.0) as i32;

    for glyph in layout.glyphs() {
        let (metrics, bitmap) = fonts[glyph.font_index].rasterize_config(glyph.key);
        let gw = metrics.width;
        let gh = metrics.height;

        let gx = x0 + glyph.x as i32;
        let gy = y0 + glyph.y as i32;

        for row_idx in 0..gh {
            let py = gy + row_idx as i32;
            if py < 0 || py >= height as i32 {
                continue;
            }
            for col_idx in 0..gw {
                let px = gx + col_idx as i32;
                if px < 0 || px >= width as i32 {
                    continue;
                }
                let alpha = bitmap[row_idx * gw + col_idx];
                if alpha > 0 {
                    let a = alpha as f32 / 255.0;
                    let bg = &pixels[py as usize][px as usize];
                    // Alpha blend
                    let r = color.r * a + bg.r * (1.0 - a);
                    let g = color.g * a + bg.g * (1.0 - a);
                    let b = color.b * a + bg.b * (1.0 - a);
                    pixels[py as usize][px as usize] = Color::new(r, g, b, 1.0);
                }
            }
        }
    }
}

/// Convert pixel buffer to PPM string
pub fn render_to_ppm(commands: &[DisplayCommand], width: usize, height: usize) -> String {
    let pixels = render_to_pixels(commands, width, height);
    let mut result = format!("P3\n{} {}\n255\n", width, height);
    for row in &pixels {
        for pixel in row {
            result.push_str(&format!(
                "{} {} {} ",
                (pixel.r * 255.0) as u8,
                (pixel.g * 255.0) as u8,
                (pixel.b * 255.0) as u8
            ));
        }
        result.push('\n');
    }
    result
}

/// Render display commands to a minifb u32 buffer (RGBA packed)
pub fn render_to_buffer(commands: &[DisplayCommand], width: usize, height: usize) -> Vec<u32> {
    let pixels = render_to_pixels(commands, width, height);
    let mut buffer = vec![0u32; width * height];
    for y in 0..height {
        for x in 0..width {
            let c = &pixels[y][x];
            let r = (c.r * 255.0) as u32;
            let g = (c.g * 255.0) as u32;
            let b = (c.b * 255.0) as u32;
            buffer[y * width + x] = (r << 16) | (g << 8) | b;
        }
    }
    buffer
}

fn fill_rect(
    pixels: &mut [Vec<Color>],
    rect: &Rect,
    color: &Color,
    width: usize,
    height: usize,
) {
    let x0 = rect.x.max(0.0) as usize;
    let y0 = rect.y.max(0.0) as usize;
    let x1 = ((rect.x + rect.width).ceil() as usize).min(width);
    let y1 = ((rect.y + rect.height).ceil() as usize).min(height);
    for y in y0..y1 {
        for x in x0..x1 {
            pixels[y][x] = color.clone();
        }
    }
}

fn draw_border(
    pixels: &mut [Vec<Color>],
    rect: &Rect,
    bw: f32,
    color: &Color,
    width: usize,
    height: usize,
) {
    if bw <= 0.0 {
        return;
    }
    let x0 = rect.x.max(0.0) as usize;
    let y0 = rect.y.max(0.0) as usize;
    let x1 = ((rect.x + rect.width).ceil() as usize).min(width);
    let y1 = ((rect.y + rect.height).ceil() as usize).min(height);
    let w = bw as usize;
    for y in y0..y1 {
        for x in x0..x1 {
            if y < y0 + w || y + w >= y1 || x < x0 + w || x + w >= x1 {
                pixels[y][x] = color.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::Node;
    use crate::layout::{BoxType, Dimensions, EdgeSizes, LayoutBox, Rect};
    use crate::style::StyledNode;
    use std::collections::HashMap;

    fn styled_text(text: &str) -> StyledNode {
        StyledNode {
            node: Node::text(text),
            specified_values: HashMap::new(),
            children: vec![],
        }
    }

    fn styled_element(
        tag: &str,
        styles: HashMap<String, String>,
        children: Vec<StyledNode>,
    ) -> StyledNode {
        StyledNode {
            node: Node::element(tag, HashMap::new(), vec![]),
            specified_values: styles,
            children,
        }
    }

    fn layout_box(styled: StyledNode, children: Vec<LayoutBox>) -> LayoutBox {
        let mut box_ = LayoutBox::new(BoxType::BlockNode(styled));
        box_.children = children;
        box_
    }

    fn dim(x: f32, y: f32, w: f32, h: f32) -> Dimensions {
        Dimensions {
            content: Rect {
                x,
                y,
                width: w,
                height: h,
            },
            ..Dimensions::default()
        }
    }

    #[test]
    fn test_color_named() {
        assert_eq!(parse_color("red"), Some(Color::red()));
        assert_eq!(parse_color("green"), Some(Color::new(0.0, 128.0 / 255.0, 0.0, 1.0)));
        assert_eq!(parse_color("blue"), Some(Color::new(0.0, 0.0, 1.0, 1.0)));
        assert_eq!(parse_color("black"), Some(Color::black()));
        assert_eq!(parse_color("white"), Some(Color::white()));
        assert_eq!(parse_color("yellow"), Some(Color::new(1.0, 1.0, 0.0, 1.0)));
        assert_eq!(parse_color("orange"), Some(Color::new(1.0, 0.64706, 0.0, 1.0)));
        assert_eq!(parse_color("gray"), Some(Color::new(0.50196, 0.50196, 0.50196, 1.0)));
        assert_eq!(parse_color("purple"), Some(Color::new(0.50196, 0.0, 0.50196, 1.0)));
        assert_eq!(parse_color("cyan"), Some(Color::new(0.0, 1.0, 1.0, 1.0)));
    }

    #[test]
    fn test_color_hex() {
        assert_eq!(parse_color("#ffffff"), Some(Color::white()));
        assert_eq!(parse_color("#ff0000"), Some(Color::red()));
        assert_eq!(parse_color("#00ff00"), Some(Color::new(0.0, 1.0, 0.0, 1.0)));
        assert_eq!(parse_color("#0000ff"), Some(Color::new(0.0, 0.0, 1.0, 1.0)));
        assert_eq!(parse_color("#123abc"), Some(Color::new(
            0x12 as f32 / 255.0,
            0x3a as f32 / 255.0,
            0xbc as f32 / 255.0,
            1.0,
        )));
    }

    #[test]
    fn test_color_hex_short() {
        assert_eq!(parse_color("#fff"), Some(Color::white()));
        assert_eq!(parse_color("#000"), Some(Color::black()));
        assert_eq!(parse_color("#f00"), Some(Color::red()));
    }

    #[test]
    fn test_color_black_default() {
        // Unknown color strings return None; the caller (display list builder) falls back to black
        assert_eq!(parse_color("notacolor"), None);
        assert_eq!(parse_color(""), None);
        assert_eq!(parse_color("#gggggg"), None);
    }

    #[test]
    fn test_display_list_background() {
        let mut styles = HashMap::new();
        styles.insert("background-color".to_string(), "red".to_string());
        let styled = styled_element("div", styles, vec![]);
        let mut root = layout_box(styled, vec![]);
        root.dimensions = dim(10.0, 10.0, 100.0, 50.0);

        let list = build_display_list(&root);
        assert_eq!(list.len(), 1);
        match &list[0] {
            DisplayCommand::SolidColor(rect, color) => {
                assert_eq!(
                    *rect,
                    Rect {
                        x: 10.0,
                        y: 10.0,
                        width: 100.0,
                        height: 50.0
                    }
                );
                assert_eq!(*color, Color::red());
            }
            _ => panic!("Expected SolidColor"),
        }
    }

    #[test]
    fn test_display_list_text() {
        let text = styled_text("Hello World");
        let styled = styled_element("p", HashMap::new(), vec![text]);
        let mut root = layout_box(styled, vec![]);
        root.dimensions = dim(10.0, 10.0, 100.0, 50.0);

        let list = build_display_list(&root);
        assert!(list.iter().any(|cmd| matches!(
            cmd,
            DisplayCommand::Text(t, _, _) if t == "Hello World"
        )));
    }

    #[test]
    fn test_display_list_nested() {
        let mut child_styles = HashMap::new();
        child_styles.insert("background-color".to_string(), "blue".to_string());
        let child_styled = styled_element("div", child_styles, vec![]);
        let mut child = layout_box(child_styled, vec![]);
        child.dimensions = dim(10.0, 10.0, 50.0, 20.0);

        let mut parent_styles = HashMap::new();
        parent_styles.insert("background-color".to_string(), "red".to_string());
        let parent_styled = styled_element("div", parent_styles, vec![]);
        let mut parent = layout_box(parent_styled, vec![child]);
        parent.dimensions = dim(0.0, 0.0, 100.0, 100.0);

        let list = build_display_list(&parent);
        assert_eq!(list.len(), 2);
        // Parent background first, then child background
        match &list[0] {
            DisplayCommand::SolidColor(_, color) => {
                assert_eq!(*color, Color::red());
            }
            _ => panic!("Expected parent SolidColor"),
        }
        match &list[1] {
            DisplayCommand::SolidColor(_, color) => {
                assert_eq!(*color, Color::new(0.0, 0.0, 1.0, 1.0));
            }
            _ => panic!("Expected child SolidColor"),
        }
    }

    #[test]
    fn test_render_ppm() {
        let list = vec![DisplayCommand::SolidColor(
            Rect {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
            },
            Color::red(),
        )];
        let ppm = render_to_ppm(&list, 20, 20);
        assert!(ppm.starts_with("P3\n"));
        assert!(ppm.contains("20 20\n"));
        assert!(ppm.contains("255\n"));
    }

    #[test]
    fn test_parse_rgb_colors() {
        assert_eq!(
            parse_color("rgb(255, 0, 0)"),
            Some(Color::new(1.0, 0.0, 0.0, 1.0))
        );
        assert_eq!(
            parse_color("rgb(0,255,0)"),
            Some(Color::new(0.0, 1.0, 0.0, 1.0))
        );
        assert_eq!(
            parse_color("rgb(0, 0, 255)"),
            Some(Color::new(0.0, 0.0, 1.0, 1.0))
        );
    }

    #[test]
    fn test_rect_intersect() {
        let a = Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        let b = Rect {
            x: 50.0,
            y: 50.0,
            width: 100.0,
            height: 100.0,
        };
        let r = rect_intersect(&a, &b).unwrap();
        assert_eq!(r.x, 50.0);
        assert_eq!(r.y, 50.0);
        assert_eq!(r.width, 50.0);
        assert_eq!(r.height, 50.0);

        let c = Rect {
            x: 200.0,
            y: 200.0,
            width: 10.0,
            height: 10.0,
        };
        assert!(rect_intersect(&a, &c).is_none());
    }

    #[test]
    fn test_build_display_list_with_padding() {
        let mut styles = HashMap::new();
        styles.insert("background-color".to_string(), "blue".to_string());
        let styled = styled_element("div", styles, vec![]);
        let mut root = layout_box(styled, vec![]);
        root.dimensions = Dimensions {
            content: Rect {
                x: 20.0,
                y: 20.0,
                width: 100.0,
                height: 50.0,
            },
            padding: EdgeSizes {
                top: 10.0,
                right: 10.0,
                bottom: 10.0,
                left: 10.0,
            },
            ..Dimensions::default()
        };

        let list = build_display_list(&root);
        assert_eq!(list.len(), 1);
        match &list[0] {
            DisplayCommand::SolidColor(rect, _) => {
                assert_eq!(rect.x, 10.0);
                assert_eq!(rect.y, 10.0);
                assert_eq!(rect.width, 120.0);
                assert_eq!(rect.height, 70.0);
            }
            _ => panic!("Expected SolidColor"),
        }
    }

    #[test]
    fn test_build_display_list_border() {
        let mut styles = HashMap::new();
        styles.insert("border-width".to_string(), "5px".to_string());
        styles.insert("border-color".to_string(), "black".to_string());
        let styled = styled_element("div", styles, vec![]);
        let mut root = layout_box(styled, vec![]);
        root.dimensions = Dimensions {
            content: Rect {
                x: 10.0,
                y: 10.0,
                width: 100.0,
                height: 50.0,
            },
            padding: EdgeSizes {
                top: 5.0,
                left: 5.0,
                bottom: 5.0,
                right: 5.0,
            },
            border: EdgeSizes {
                top: 5.0,
                left: 5.0,
                bottom: 5.0,
                right: 5.0,
            },
            ..Dimensions::default()
        };

        let list = build_display_list(&root);
        assert_eq!(list.len(), 1);
        match &list[0] {
            DisplayCommand::Border(rect, width, _) => {
                assert_eq!(*width, 5.0);
                assert_eq!(rect.x, 0.0);
                assert_eq!(rect.y, 0.0);
                assert_eq!(rect.width, 120.0);
                assert_eq!(rect.height, 70.0);
            }
            _ => panic!("Expected Border"),
        }
    }

    #[test]
    fn test_build_display_list_clipping() {
        let mut styles = HashMap::new();
        styles.insert("background-color".to_string(), "red".to_string());
        let child_styled = styled_element("div", styles, vec![]);
        let mut child = layout_box(child_styled, vec![]);
        child.dimensions = dim(90.0, 10.0, 100.0, 50.0);

        let parent_styled = styled_element("div", HashMap::new(), vec![]);
        let mut parent = layout_box(parent_styled, vec![child]);
        parent.dimensions = dim(0.0, 0.0, 100.0, 100.0);

        let list = build_display_list(&parent);
        assert_eq!(list.len(), 1);
        match &list[0] {
            DisplayCommand::SolidColor(rect, _) => {
                assert_eq!(rect.x, 90.0);
                assert_eq!(rect.y, 10.0);
                assert_eq!(rect.width, 10.0);
                assert_eq!(rect.height, 50.0);
            }
            _ => panic!("Expected SolidColor"),
        }
    }

    #[test]
    fn test_terminal_render_basic() {
        let list = vec![DisplayCommand::SolidColor(
            Rect {
                x: 0.0,
                y: 0.0,
                width: 2.0,
                height: 2.0,
            },
            Color::red(),
        )];
        let output = render_to_terminal(&list, 4, 4);
        assert!(output.contains("\x1b["));
        assert!(output.contains('\n'));
    }
}
