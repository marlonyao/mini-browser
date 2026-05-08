use crate::dom::Node;
use crate::style::StyledNode;

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct EdgeSizes {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Dimensions {
    pub content: Rect,
    pub padding: EdgeSizes,
    pub border: EdgeSizes,
    pub margin: EdgeSizes,
}

#[derive(Debug, PartialEq, Clone)]
pub enum BoxType {
    BlockNode(StyledNode),
    InlineNode(StyledNode),
    AnonymousBlock,
}

#[derive(Debug, PartialEq, Clone)]
pub struct LayoutBox {
    pub box_type: BoxType,
    pub dimensions: Dimensions,
    pub children: Vec<LayoutBox>,
}

impl LayoutBox {
    pub fn new(box_type: BoxType) -> Self {
        LayoutBox {
            box_type,
            dimensions: Dimensions::default(),
            children: Vec::new(),
        }
    }
}

pub fn build_layout_tree(styled: &StyledNode) -> LayoutBox {
    let box_type = match &styled.node {
        Node::Text(_) => panic!("Text nodes should not build layout tree directly"),
        Node::Element(_) => {
            let display = styled.specified_values.get("display").map(|s| s.as_str());
            match display {
                Some("inline") => BoxType::InlineNode(styled.clone()),
                _ => BoxType::BlockNode(styled.clone()),
            }
        }
    };

    let mut root = LayoutBox::new(box_type);
    let mut inline_boxes: Vec<LayoutBox> = Vec::new();

    for child in &styled.children {
        if let Node::Text(_) = &child.node {
            continue;
        }
        if let Node::Element(el) = &child.node {
            if matches!(el.tag.as_str(), "head" | "style" | "script" | "meta" | "link" | "title") {
                continue;
            }
        }

        let child_display = child.specified_values.get("display").map(|s| s.as_str());
        if child_display == Some("none") {
            continue;
        }
        let is_inline = child_display == Some("inline");

        if is_inline {
            let child_box = build_layout_tree(child);
            inline_boxes.push(child_box);
        } else {
            if !inline_boxes.is_empty() {
                let mut anonymous = LayoutBox::new(BoxType::AnonymousBlock);
                anonymous.children = std::mem::take(&mut inline_boxes);
                root.children.push(anonymous);
            }
            let child_box = build_layout_tree(child);
            root.children.push(child_box);
        }
    }

    if !inline_boxes.is_empty() {
        let mut anonymous = LayoutBox::new(BoxType::AnonymousBlock);
        anonymous.children = std::mem::take(&mut inline_boxes);
        root.children.push(anonymous);
    }

    // Pre-calculate minimum content height from text (rough estimate with max width)
    // Will be refined during actual layout with correct width
    root
}

pub fn layout(layout_box: &mut LayoutBox, containing_block: Dimensions) {
    match layout_box.box_type {
        BoxType::BlockNode(_) => {
            layout_block(layout_box, &containing_block);
        }
        BoxType::AnonymousBlock => {
            layout_inline_block(layout_box, &containing_block);
        }
        BoxType::InlineNode(_) => {
            layout_inline_node(layout_box, &containing_block);
        }
    }
}

fn set_absolute_positions(layout_box: &mut LayoutBox, abs_x: f32, abs_y: f32) {
    let dim = &mut layout_box.dimensions;
    let offset_x = abs_x - dim.content.x;
    let offset_y = abs_y - dim.content.y;
    dim.content.x = abs_x;
    dim.content.y = abs_y;
    for child in &mut layout_box.children {
        let child_abs_x = child.dimensions.content.x + offset_x;
        let child_abs_y = child.dimensions.content.y + offset_y;
        set_absolute_positions(child, child_abs_x, child_abs_y);
    }
}

fn layout_inline_block(layout_box: &mut LayoutBox, containing_block: &Dimensions) {
    layout_box.dimensions.content.x = containing_block.content.x;
    layout_box.dimensions.content.y = containing_block.content.y;
    layout_box.dimensions.content.width = containing_block.content.width;

    let mut current_x = layout_box.dimensions.content.x;
    let mut current_y = layout_box.dimensions.content.y;
    let mut line_height = 0.0f32;
    let available_width = layout_box.dimensions.content.width;

    for child in &mut layout_box.children {
        let mut child_containing = Dimensions::default();
        child_containing.content.width = available_width;
        layout(child, child_containing);

        let child_total_width = child.dimensions.content.width
            + child.dimensions.padding.left + child.dimensions.padding.right
            + child.dimensions.border.left + child.dimensions.border.right
            + child.dimensions.margin.left + child.dimensions.margin.right;

        let child_total_height = child.dimensions.content.height
            + child.dimensions.padding.top + child.dimensions.padding.bottom
            + child.dimensions.border.top + child.dimensions.border.bottom
            + child.dimensions.margin.top + child.dimensions.margin.bottom;

        // Line wrap check
        if current_x + child_total_width > layout_box.dimensions.content.x + available_width
            && current_x > layout_box.dimensions.content.x
        {
            current_y += line_height;
            current_x = layout_box.dimensions.content.x;
            line_height = 0.0;
        }

        let target_x = current_x
            + child.dimensions.margin.left
            + child.dimensions.border.left
            + child.dimensions.padding.left;
        let target_y = current_y
            + child.dimensions.margin.top
            + child.dimensions.border.top
            + child.dimensions.padding.top;
        set_absolute_positions(child, target_x, target_y);

        current_x += child_total_width;
        line_height = line_height.max(child_total_height);
    }

    layout_box.dimensions.content.height = current_y + line_height - layout_box.dimensions.content.y;
}

fn layout_inline_node(layout_box: &mut LayoutBox, containing_block: &Dimensions) {
    let styled = match &layout_box.box_type {
        BoxType::InlineNode(s) => Some(s.clone()),
        _ => None,
    };

    // Compute inline box metrics
    let padding_left = get_length(styled.as_ref(), "padding-left", containing_block.content.width);
    let padding_right = get_length(styled.as_ref(), "padding-right", containing_block.content.width);
    let border_left = get_length(styled.as_ref(), "border-left", containing_block.content.width);
    let border_right = get_length(styled.as_ref(), "border-right", containing_block.content.width);
    let margin_left = get_length(styled.as_ref(), "margin-left", containing_block.content.width);
    let margin_right = get_length(styled.as_ref(), "margin-right", containing_block.content.width);
    let padding_top = get_length(styled.as_ref(), "padding-top", containing_block.content.width);
    let padding_bottom = get_length(styled.as_ref(), "padding-bottom", containing_block.content.width);
    let border_top = get_length(styled.as_ref(), "border-top-width", containing_block.content.width);
    let border_bottom = get_length(styled.as_ref(), "border-bottom-width", containing_block.content.width);
    let margin_top = get_length(styled.as_ref(), "margin-top", containing_block.content.width);
    let margin_bottom = get_length(styled.as_ref(), "margin-bottom", containing_block.content.width);

    layout_box.dimensions.padding.left = padding_left;
    layout_box.dimensions.padding.right = padding_right;
    layout_box.dimensions.border.left = border_left;
    layout_box.dimensions.border.right = border_right;
    layout_box.dimensions.margin.left = margin_left;
    layout_box.dimensions.margin.right = margin_right;
    layout_box.dimensions.padding.top = padding_top;
    layout_box.dimensions.padding.bottom = padding_bottom;
    layout_box.dimensions.border.top = border_top;
    layout_box.dimensions.border.bottom = border_bottom;
    layout_box.dimensions.margin.top = margin_top;
    layout_box.dimensions.margin.bottom = margin_bottom;

    let font_size = styled.as_ref()
        .and_then(|s| s.specified_values.get("font-size"))
        .map(|v| parse_value(Some(v)))
        .unwrap_or(16.0);
    let line_height = font_size * 1.2;
    let char_width = font_size * 0.5;

    // Measure text content from styled node children
    let mut text_width = 0.0f32;
    let mut text_height = 0.0f32;
    if let Some(s) = styled.as_ref() {
        for child in &s.children {
            if let Node::Text(text) = &child.node {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    let chars = trimmed.chars().count() as f32;
                    text_width += chars * char_width;
                    text_height = line_height;
                }
            }
        }
    }

    // Recursively layout inline children (e.g. <span><a>...</a></span>)
    let mut children_width = 0.0f32;
    let mut children_height = 0.0f32;
    for child in &mut layout_box.children {
        let mut child_containing = Dimensions::default();
        child_containing.content.width = containing_block.content.width;
        layout_inline_node(child, &child_containing);
        children_width += child.dimensions.content.width
            + child.dimensions.padding.left + child.dimensions.padding.right
            + child.dimensions.border.left + child.dimensions.border.right
            + child.dimensions.margin.left + child.dimensions.margin.right;
        children_height = children_height.max(
            child.dimensions.content.height
                + child.dimensions.padding.top + child.dimensions.padding.bottom
                + child.dimensions.border.top + child.dimensions.border.bottom
                + child.dimensions.margin.top + child.dimensions.margin.bottom,
        );
    }

    layout_box.dimensions.content.width = text_width.max(children_width);
    layout_box.dimensions.content.height = text_height.max(children_height);

    // Special: <img> placeholder sizing
    if let Some(s) = styled.as_ref() {
        if let Node::Element(el) = &s.node {
            if el.tag == "img" {
                let mut img_w = 100.0f32;
                let mut img_h = 100.0f32;
                if let Some(w) = el.attrs.get("width") {
                    if let Ok(v) = w.parse::<f32>() { img_w = v; }
                }
                if let Some(h) = el.attrs.get("height") {
                    if let Ok(v) = h.parse::<f32>() { img_h = v; }
                }
                layout_box.dimensions.content.width = img_w;
                layout_box.dimensions.content.height = img_h;
            } else if el.tag == "input" {
                let mut input_w = 200.0f32;
                let mut input_h = 30.0f32;
                if let Some(w) = el.attrs.get("width") {
                    if let Ok(v) = w.parse::<f32>() { input_w = v; }
                }
                if let Some(h) = el.attrs.get("height") {
                    if let Ok(v) = h.parse::<f32>() { input_h = v; }
                }
                if let Some(size) = el.attrs.get("size") {
                    if let Ok(v) = size.parse::<f32>() { input_w = v * 10.0; }
                }
                layout_box.dimensions.content.width = input_w;
                layout_box.dimensions.content.height = input_h;
            }
        }
    }
}

fn layout_block(layout_box: &mut LayoutBox, containing_block: &Dimensions) {
    let styled = match &layout_box.box_type {
        BoxType::BlockNode(s) | BoxType::InlineNode(s) => Some(s.clone()),
        BoxType::AnonymousBlock => None,
    };

    let margin_left = get_length(styled.as_ref(), "margin-left", containing_block.content.width);
    let margin_right = get_length(styled.as_ref(), "margin-right", containing_block.content.width);
    let padding_left = get_length(styled.as_ref(), "padding-left", containing_block.content.width);
    let padding_right = get_length(styled.as_ref(), "padding-right", containing_block.content.width);
    let border_left = get_length(styled.as_ref(), "border-left", containing_block.content.width);
    let border_right = get_length(styled.as_ref(), "border-right", containing_block.content.width);

    let explicit_width = styled.as_ref()
        .and_then(|s| s.specified_values.get("width"))
        .map(|v| parse_length_percent(v, containing_block.content.width));

    let total_horizontal = margin_left + border_left + padding_left + padding_right + border_right + margin_right;
    let width = explicit_width.unwrap_or((containing_block.content.width - total_horizontal).max(0.0));

    layout_box.dimensions.content.width = width;
    layout_box.dimensions.margin.left = margin_left;
    layout_box.dimensions.margin.right = margin_right;
    layout_box.dimensions.padding.left = padding_left;
    layout_box.dimensions.padding.right = padding_right;
    layout_box.dimensions.border.left = border_left;
    layout_box.dimensions.border.right = border_right;

    let margin_top = get_length(styled.as_ref(), "margin-top", containing_block.content.width);
    let margin_bottom = get_length(styled.as_ref(), "margin-bottom", containing_block.content.width);
    let padding_top = get_length(styled.as_ref(), "padding-top", containing_block.content.width);
    let padding_bottom = get_length(styled.as_ref(), "padding-bottom", containing_block.content.width);
    let border_top = get_length(styled.as_ref(), "border-top", containing_block.content.width);
    let border_bottom = get_length(styled.as_ref(), "border-bottom", containing_block.content.width);

    layout_box.dimensions.margin.top = margin_top;
    layout_box.dimensions.margin.bottom = margin_bottom;
    layout_box.dimensions.padding.top = padding_top;
    layout_box.dimensions.padding.bottom = padding_bottom;
    layout_box.dimensions.border.top = border_top;
    layout_box.dimensions.border.bottom = border_bottom;

    layout_box.dimensions.content.x = containing_block.content.x + margin_left + border_left + padding_left;
    layout_box.dimensions.content.y = containing_block.content.y + margin_top + border_top + padding_top;

    let mut current_y = layout_box.dimensions.content.y;
    for child in &mut layout_box.children {
        let mut child_containing = Dimensions::default();
        child_containing.content.x = layout_box.dimensions.content.x;
        child_containing.content.y = current_y;
        child_containing.content.width = layout_box.dimensions.content.width;

        layout(child, child_containing);

        current_y = child.dimensions.content.y
            + child.dimensions.content.height
            + child.dimensions.padding.bottom
            + child.dimensions.border.bottom
            + child.dimensions.margin.bottom;
    }

    if let Some(h) = styled.as_ref().and_then(|s| s.specified_values.get("height")) {
        layout_box.dimensions.content.height = parse_length_percent(h, containing_block.content.height);
    } else {
        let content_bottom = if layout_box.children.is_empty() {
            layout_box.dimensions.content.y
        } else {
            let last = layout_box.children.last().unwrap();
            last.dimensions.content.y
                + last.dimensions.content.height
                + last.dimensions.padding.bottom
                + last.dimensions.border.bottom
                + last.dimensions.margin.bottom
        };
        let computed = content_bottom - layout_box.dimensions.content.y;

        // If height is 0 but we have text content, calculate minimum text height
        if computed <= 0.0 {
            if let Some(styled) = styled.as_ref() {
                let text_height = measure_text_height(styled, layout_box.dimensions.content.width);
                if text_height > 0.0 {
                    layout_box.dimensions.content.height = text_height;
                } else {
                    layout_box.dimensions.content.height = computed;
                }
            } else {
                layout_box.dimensions.content.height = computed;
            }
        } else {
            layout_box.dimensions.content.height = computed;
        }
    }
}

/// Estimate text height based on character count and container width
fn measure_text_height(styled: &StyledNode, container_width: f32) -> f32 {
    let font_size = 16.0f32;
    let line_height = font_size * 1.2;
    let char_width = font_size * 0.5; // approximate

    let mut total_height = 0.0f32;
    for child in &styled.children {
        if let Node::Text(text) = &child.node {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                let text_width = trimmed.len() as f32 * char_width;
                let lines = if container_width > 0.0 {
                    (text_width / container_width).ceil().max(1.0)
                } else {
                    1.0
                };
                total_height += line_height * lines;
            }
        }
    }
    total_height
}

fn get_length(styled: Option<&StyledNode>, property: &str, container_size: f32) -> f32 {
    styled
        .and_then(|s| s.specified_values.get(property))
        .map(|v| parse_length_percent(v, container_size))
        .unwrap_or(0.0)
}

fn parse_length_percent(value: &str, container_size: f32) -> f32 {
    let value = value.trim();
    if value.ends_with("px") {
        value[..value.len() - 2].trim().parse::<f32>().unwrap_or(0.0)
    } else if value.ends_with('%') {
        value[..value.len() - 1].trim().parse::<f32>().unwrap_or(0.0) / 100.0 * container_size
    } else if value == "auto" {
        0.0
    } else {
        value.parse::<f32>().unwrap_or(0.0)
    }
}

pub fn parse_value(value: Option<&String>) -> f32 {
    match value {
        None => 0.0,
        Some(v) => {
            let v = v.trim();
            if v.ends_with("px") {
                v[..v.len() - 2].trim().parse::<f32>().unwrap_or(0.0)
            } else if v.ends_with('%') {
                v[..v.len() - 1].trim().parse::<f32>().unwrap_or(0.0)
            } else if v == "auto" {
                0.0
            } else {
                v.parse::<f32>().unwrap_or(0.0)
            }
        }
    }
}

pub fn print_layout_box(layout_box: &LayoutBox, indent: usize) {
    let spaces = "  ".repeat(indent);

    match &layout_box.box_type {
        BoxType::BlockNode(styled) | BoxType::InlineNode(styled) => {
            if let Node::Element(el) = &styled.node {
                println!(
                    "{}<{}> x={:.0} y={:.0} w={:.0} h={:.0}",
                    spaces,
                    el.tag,
                    layout_box.dimensions.content.x,
                    layout_box.dimensions.content.y,
                    layout_box.dimensions.content.width,
                    layout_box.dimensions.content.height
                );

                for child in &styled.children {
                    if let Node::Text(text) = &child.node {
                        let trimmed = text.trim();
                        if !trimmed.is_empty() {
                            println!("{}\"{}\"", "  ".repeat(indent + 1), trimmed);
                        }
                    }
                }
            }
        }
        BoxType::AnonymousBlock => {}
    }

    for child in &layout_box.children {
        print_layout_box(child, indent + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn styled_element(tag: &str, styles: HashMap<String, String>, children: Vec<StyledNode>) -> StyledNode {
        StyledNode {
            node: Node::element(tag, HashMap::new(), vec![]),
            specified_values: styles,
            children,
        }
    }

    fn viewport() -> Dimensions {
        Dimensions {
            content: Rect {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 600.0,
            },
            ..Dimensions::default()
        }
    }

    #[test]
    fn test_dimensions_defaults() {
        let d = Dimensions::default();
        assert_eq!(d.content, Rect::default());
        assert_eq!(d.padding, EdgeSizes::default());
        assert_eq!(d.border, EdgeSizes::default());
        assert_eq!(d.margin, EdgeSizes::default());
        assert_eq!(d.content.x, 0.0);
        assert_eq!(d.content.y, 0.0);
        assert_eq!(d.content.width, 0.0);
        assert_eq!(d.content.height, 0.0);
    }

    #[test]
    fn test_layout_simple_block() {
        let styled = styled_element("div", HashMap::new(), vec![]);
        let mut root = build_layout_tree(&styled);
        layout(&mut root, viewport());

        assert_eq!(root.dimensions.content.width, 800.0);
        assert_eq!(root.dimensions.content.height, 0.0);
        assert_eq!(root.dimensions.content.x, 0.0);
        assert_eq!(root.dimensions.content.y, 0.0);
    }

    #[test]
    fn test_layout_nested_blocks() {
        let inner = styled_element("span", HashMap::new(), vec![]);
        let middle = styled_element("p", HashMap::new(), vec![inner]);
        let outer = styled_element("div", HashMap::new(), vec![middle]);

        let mut root = build_layout_tree(&outer);
        layout(&mut root, viewport());

        assert_eq!(root.dimensions.content.width, 800.0);
        assert_eq!(root.children.len(), 1);
        let middle_box = &root.children[0];
        assert_eq!(middle_box.dimensions.content.width, 800.0);
        assert_eq!(middle_box.children.len(), 1);
        let inner_box = &middle_box.children[0];
        assert_eq!(inner_box.dimensions.content.width, 800.0);
    }

    #[test]
    fn test_layout_with_padding() {
        let mut styles = HashMap::new();
        styles.insert("padding-top".to_string(), "10px".to_string());
        styles.insert("padding-right".to_string(), "10px".to_string());
        styles.insert("padding-bottom".to_string(), "10px".to_string());
        styles.insert("padding-left".to_string(), "10px".to_string());

        let styled = styled_element("div", styles, vec![]);
        let mut root = build_layout_tree(&styled);
        layout(&mut root, viewport());

        assert_eq!(root.dimensions.padding.top, 10.0);
        assert_eq!(root.dimensions.padding.left, 10.0);
        assert_eq!(root.dimensions.content.x, 10.0);
        assert_eq!(root.dimensions.content.y, 10.0);
        assert_eq!(root.dimensions.content.width, 780.0);
    }

    #[test]
    fn test_layout_with_margin() {
        let mut styles1 = HashMap::new();
        styles1.insert("margin-bottom".to_string(), "20px".to_string());
        let div1 = styled_element("div", styles1, vec![]);

        let mut styles2 = HashMap::new();
        styles2.insert("margin-top".to_string(), "10px".to_string());
        let div2 = styled_element("div", styles2, vec![]);

        let parent = styled_element("div", HashMap::new(), vec![div1, div2]);
        let mut root = build_layout_tree(&parent);
        layout(&mut root, viewport());

        let child1 = &root.children[0];
        let child2 = &root.children[1];

        assert_eq!(child1.dimensions.content.y, 0.0);
        assert_eq!(child2.dimensions.content.y, 30.0);
    }

    #[test]
    fn test_layout_explicit_width() {
        let mut styles = HashMap::new();
        styles.insert("width".to_string(), "400px".to_string());

        let styled = styled_element("div", styles, vec![]);
        let mut root = build_layout_tree(&styled);
        layout(&mut root, viewport());

        assert_eq!(root.dimensions.content.width, 400.0);
    }

    #[test]
    fn test_layout_auto_height() {
        let mut child_styles = HashMap::new();
        child_styles.insert("height".to_string(), "100px".to_string());
        let child = styled_element("div", child_styles, vec![]);

        let parent = styled_element("div", HashMap::new(), vec![child]);
        let mut root = build_layout_tree(&parent);
        layout(&mut root, viewport());

        assert_eq!(root.dimensions.content.height, 100.0);
    }

    #[test]
    fn test_layout_percentage_width() {
        let mut styles = HashMap::new();
        styles.insert("width".to_string(), "50%".to_string());

        let styled = styled_element("div", styles, vec![]);
        let mut root = build_layout_tree(&styled);
        layout(&mut root, viewport());

        assert_eq!(root.dimensions.content.width, 400.0);
    }
}
