use crate::dom::Node;
use crate::style::StyledNode;
use serde::Serialize;

#[derive(Debug, Default, PartialEq, Clone, Serialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FloatSide {
    Left,
    Right,
}

/// Tracks floating boxes inside a block container so that inline
/// content can flow around them.
#[derive(Debug, Default, Clone)]
pub struct FloatContext {
    pub floats: Vec<(Rect, FloatSide)>,
}

impl FloatContext {
    /// Return the horizontal interval that is not covered by any float
    /// overlapping the vertical band `[y, y + height)`.
    pub fn available_rect(&self, y: f32, height: f32, container: &Rect) -> Rect {
        let mut left = container.x;
        let mut right = container.x + container.width;
        let bottom = y + height;

        for (rect, side) in &self.floats {
            let float_bottom = rect.y + rect.height;
            let float_top = rect.y;
            if bottom > float_top && y < float_bottom {
                match side {
                    FloatSide::Left => left = left.max(rect.x + rect.width),
                    FloatSide::Right => right = right.min(rect.x),
                }
            }
        }

        Rect {
            x: left,
            y,
            width: (right - left).max(0.0),
            height,
        }
    }

    /// Lowest y where all current floats have ended (used for `clear: both`).
    pub fn clear_both_y(&self) -> f32 {
        self.floats.iter().map(|(r, _)| r.y + r.height).fold(0.0f32, f32::max)
    }
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
    InlineBlockNode(StyledNode),  // 新增
    AnonymousBlock,
    FloatLeftNode(StyledNode),   // 新增
    FloatRightNode(StyledNode),  // 新增
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
            let float = styled.specified_values.get("float").map(|s| s.as_str());
            match float {
                Some("left") => BoxType::FloatLeftNode(styled.clone()),
                Some("right") => BoxType::FloatRightNode(styled.clone()),
                _ => match display {
                    Some("inline") => BoxType::InlineNode(styled.clone()),
                    Some("inline-block") => BoxType::InlineBlockNode(styled.clone()),
                    _ => BoxType::BlockNode(styled.clone()),
                },
            }
        }
    };

    let mut root = LayoutBox::new(box_type);
    let mut inline_boxes: Vec<LayoutBox> = Vec::new();

    for child in &styled.children {
        if matches!(child.node, Node::Text(_)) {
            continue;
        }
        if let Node::Element(el) = &child.node {
            if matches!(el.tag.as_str(), "head" | "style" | "script" | "meta" | "link" | "title") {
                continue;
            }
            // Hardcode: skip <input type="hidden"> since attribute selectors aren't supported
            if el.tag == "input" && el.attrs.get("type").map(|s| s.as_str()) == Some("hidden") {
                continue;
            }
        }

        let child_display = child.specified_values.get("display").map(|s| s.as_str());
        if child_display == Some("none") {
            continue;
        }
        let is_inline = child_display == Some("inline") || child_display == Some("inline-block");

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
        BoxType::BlockNode(_) | BoxType::InlineBlockNode(_) => {
            layout_block(layout_box, &containing_block);
        }
        BoxType::AnonymousBlock => {
            layout_inline_block(layout_box, &containing_block, &mut FloatContext::default());
        }
        BoxType::InlineNode(_) => {
            layout_inline_node(layout_box, &containing_block);
        }
        BoxType::FloatLeftNode(_) | BoxType::FloatRightNode(_) => {
            // Float boxes are laid out by their parent block container.
            // When layout() is called directly on a float (e.g. top-level),
            // treat it like a normal block for sizing.
            layout_float(layout_box, &containing_block.content, &mut FloatContext::default());
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

fn layout_inline_block(layout_box: &mut LayoutBox, containing_block: &Dimensions, float_context: &mut FloatContext) {
    layout_box.dimensions.content.x = containing_block.content.x;
    layout_box.dimensions.content.y = containing_block.content.y;
    layout_box.dimensions.content.width = containing_block.content.width;

    let mut current_x = layout_box.dimensions.content.x;
    let mut current_y = layout_box.dimensions.content.y;
    let mut line_height = 0.0f32;
    let container_rect = layout_box.dimensions.content.clone();

    for child in &mut layout_box.children {
        match &child.box_type {
            BoxType::InlineNode(_) => {
                let mut child_containing = Dimensions::default();
                child_containing.content.width = container_rect.width;
                layout_inline_node(child, &child_containing);

                let child_total_width = child.dimensions.content.width
                    + child.dimensions.padding.left + child.dimensions.padding.right
                    + child.dimensions.border.left + child.dimensions.border.right
                    + child.dimensions.margin.left + child.dimensions.margin.right;

                let child_total_height = child.dimensions.content.height
                    + child.dimensions.padding.top + child.dimensions.padding.bottom
                    + child.dimensions.border.top + child.dimensions.border.bottom
                    + child.dimensions.margin.top + child.dimensions.margin.bottom;

                // Query float context for available space at current line
                let available = float_context.available_rect(current_y, child_total_height, &container_rect);

                // If current x is before available start, move right
                if current_x < available.x {
                    current_x = available.x;
                }

                // Line wrap check
                if current_x + child_total_width > available.x + available.width
                    && current_x > available.x
                {
                    current_y += line_height;
                    let new_available = float_context.available_rect(current_y, child_total_height, &container_rect);
                    current_x = new_available.x;
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
            BoxType::InlineBlockNode(_) => {
                let styled = match &child.box_type {
                    BoxType::InlineBlockNode(s) => Some(s.clone()),
                    _ => None,
                };

                // Parse margin / border / padding
                let margin_left = get_length(styled.as_ref(), "margin-left", containing_block.content.width);
                let margin_right = get_length(styled.as_ref(), "margin-right", containing_block.content.width);
                let padding_left = get_length(styled.as_ref(), "padding-left", containing_block.content.width);
                let padding_right = get_length(styled.as_ref(), "padding-right", containing_block.content.width);
                let border_left = get_length(styled.as_ref(), "border-left", containing_block.content.width);
                let border_right = get_length(styled.as_ref(), "border-right", containing_block.content.width);

                let margin_top = get_length(styled.as_ref(), "margin-top", containing_block.content.width);
                let margin_bottom = get_length(styled.as_ref(), "margin-bottom", containing_block.content.width);
                let padding_top = get_length(styled.as_ref(), "padding-top", containing_block.content.width);
                let padding_bottom = get_length(styled.as_ref(), "padding-bottom", containing_block.content.width);
                let border_top = get_length(styled.as_ref(), "border-top", containing_block.content.width);
                let border_bottom = get_length(styled.as_ref(), "border-bottom", containing_block.content.width);

                child.dimensions.margin = EdgeSizes { top: margin_top, right: margin_right, bottom: margin_bottom, left: margin_left };
                child.dimensions.padding = EdgeSizes { top: padding_top, right: padding_right, bottom: padding_bottom, left: padding_left };
                child.dimensions.border = EdgeSizes { top: border_top, right: border_right, bottom: border_bottom, left: border_left };

                // Calculate width
                let total_horizontal = margin_left + border_left + padding_left + padding_right + border_right + margin_right;
                let explicit_width = styled.as_ref()
                    .and_then(|s| s.specified_values.get("width"))
                    .map(|v| parse_length_percent(v, containing_block.content.width));

                let box_sizing = get_box_sizing(styled.as_ref());
                let content_width = match (explicit_width, box_sizing) {
                    (Some(w), "border-box") => (w - border_left - padding_left - padding_right - border_right).max(0.0),
                    (Some(w), _) => w,
                    (None, _) => (container_rect.width - total_horizontal).max(0.0),
                };

                // Min/max width
                let min_width = styled.as_ref()
                    .and_then(|s| s.specified_values.get("min-width"))
                    .map(|v| parse_length_percent(v, containing_block.content.width))
                    .unwrap_or(0.0);
                let max_width = styled.as_ref()
                    .and_then(|s| s.specified_values.get("max-width"))
                    .map(|v| parse_length_percent(v, containing_block.content.width))
                    .unwrap_or(f32::INFINITY);
                let content_width = content_width.clamp(min_width, max_width);

                // Internal block layout (relative coordinates, offset later)
                let mut ib_containing = Dimensions::default();
                ib_containing.content.x = 0.0;
                ib_containing.content.y = 0.0;
                ib_containing.content.width = content_width;
                layout_block(child, &ib_containing);

                // Total outer size for line layout
                let child_total_width = content_width
                    + padding_left + padding_right
                    + border_left + border_right
                    + margin_left + margin_right;
                let child_total_height = child.dimensions.content.height
                    + padding_top + padding_bottom
                    + border_top + border_bottom
                    + margin_top + margin_bottom;

                // Query float context for available space
                let available = float_context.available_rect(current_y, child_total_height, &container_rect);

                if current_x < available.x {
                    current_x = available.x;
                }

                // Line wrap check
                if current_x + child_total_width > available.x + available.width
                    && current_x > available.x
                {
                    current_y += line_height;
                    let new_available = float_context.available_rect(current_y, child_total_height, &container_rect);
                    current_x = new_available.x;
                    line_height = 0.0;
                }

                // Place inline-block at correct absolute position
                let target_x = current_x + margin_left + border_left + padding_left;
                let target_y = current_y + margin_top + border_top + padding_top;
                set_absolute_positions(child, target_x, target_y);

                current_x += child_total_width;
                line_height = line_height.max(child_total_height);
            }
            _ => {}
        }
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

fn get_box_sizing(styled: Option<&StyledNode>) -> &str {
    styled
        .and_then(|s| s.specified_values.get("box-sizing"))
        .map(|s| s.as_str())
        .unwrap_or("content-box")
}

fn collapse_margins(m1: f32, m2: f32) -> f32 {
    if m1 > 0.0 && m2 > 0.0 {
        m1.max(m2)
    } else if m1 < 0.0 && m2 < 0.0 {
        m1.min(m2)
    } else {
        m1 + m2
    }
}

fn compute_vertical_margins(layout_box: &LayoutBox, container_width: f32) -> (f32, f32) {
    match &layout_box.box_type {
        BoxType::BlockNode(styled)
        | BoxType::InlineNode(styled)
        | BoxType::InlineBlockNode(styled)
        | BoxType::FloatLeftNode(styled)
        | BoxType::FloatRightNode(styled) => {
            let top = get_length(Some(styled), "margin-top", container_width);
            let bottom = get_length(Some(styled), "margin-bottom", container_width);
            (top, bottom)
        }
        BoxType::AnonymousBlock => (0.0, 0.0),
    }
}

fn layout_block(layout_box: &mut LayoutBox, containing_block: &Dimensions) {
    let styled = match &layout_box.box_type {
        BoxType::BlockNode(s) | BoxType::InlineBlockNode(s) | BoxType::InlineNode(s)
        | BoxType::FloatLeftNode(s) | BoxType::FloatRightNode(s) => Some(s.clone()),
        BoxType::AnonymousBlock => None,
    };

    let box_sizing = get_box_sizing(styled.as_ref());

    // ── Horizontal metrics ─────────────────────────────
    let margin_left = get_length(styled.as_ref(), "margin-left", containing_block.content.width);
    let margin_right = get_length(styled.as_ref(), "margin-right", containing_block.content.width);
    let padding_left = get_length(styled.as_ref(), "padding-left", containing_block.content.width);
    let padding_right = get_length(styled.as_ref(), "padding-right", containing_block.content.width);
    let border_left = get_length(styled.as_ref(), "border-left", containing_block.content.width);
    let border_right = get_length(styled.as_ref(), "border-right", containing_block.content.width);

    // ── Width calculation ──────────────────────────────
    let explicit_width = styled.as_ref()
        .and_then(|s| s.specified_values.get("width"))
        .map(|v| parse_length_percent(v, containing_block.content.width));

    let total_horizontal = margin_left + border_left + padding_left + padding_right + border_right + margin_right;

    let content_width = match (explicit_width, box_sizing) {
        (Some(w), "border-box") => {
            (w - border_left - padding_left - padding_right - border_right).max(0.0)
        }
        (Some(w), _) => w,
        (None, _) => (containing_block.content.width - total_horizontal).max(0.0),
    };

    // min/max-width constraints
    let min_width = styled.as_ref()
        .and_then(|s| s.specified_values.get("min-width"))
        .map(|v| parse_length_percent(v, containing_block.content.width))
        .unwrap_or(0.0);
    let max_width = styled.as_ref()
        .and_then(|s| s.specified_values.get("max-width"))
        .map(|v| parse_length_percent(v, containing_block.content.width))
        .unwrap_or(f32::INFINITY);
    let content_width = content_width.clamp(min_width, max_width);

    layout_box.dimensions.content.width = content_width;
    layout_box.dimensions.margin.left = margin_left;
    layout_box.dimensions.margin.right = margin_right;
    layout_box.dimensions.padding.left = padding_left;
    layout_box.dimensions.padding.right = padding_right;
    layout_box.dimensions.border.left = border_left;
    layout_box.dimensions.border.right = border_right;

    // ── Vertical metrics ─────────────────────────────────
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

    // ── Children layout with margin collapsing + floats ─────────
    let mut float_context = FloatContext::default();

    // Round 1: layout all float children first
    for child in &mut layout_box.children {
        if matches!(child.box_type, BoxType::FloatLeftNode(_) | BoxType::FloatRightNode(_)) {
            layout_float(child, &layout_box.dimensions.content, &mut float_context);
        }
    }

    // Round 2: layout non-float children, passing float context to inline blocks
    let mut last_border_bottom = layout_box.dimensions.content.y;
    let mut last_margin_bottom = 0.0f32;

    for child in &mut layout_box.children {
        if matches!(child.box_type, BoxType::FloatLeftNode(_) | BoxType::FloatRightNode(_)) {
            continue;
        }

        let (child_margin_top, child_margin_bottom) = compute_vertical_margins(child, layout_box.dimensions.content.width);

        let mut child_containing = Dimensions::default();
        child_containing.content.x = layout_box.dimensions.content.x;
        child_containing.content.width = layout_box.dimensions.content.width;

        // Margin collapsing between siblings
        let collapsed = collapse_margins(last_margin_bottom, child_margin_top);
        let mut child_y = last_border_bottom + collapsed - child_margin_top;

        // Handle `clear: both` — push below all active floats
        let clear = match &child.box_type {
            BoxType::BlockNode(s) | BoxType::InlineNode(s) | BoxType::InlineBlockNode(s) => {
                s.specified_values.get("clear").map(|v| v.as_str())
            }
            BoxType::AnonymousBlock => None,
            _ => None,
        };
        if clear == Some("both") || clear == Some("left") || clear == Some("right") {
            child_y = child_y.max(float_context.clear_both_y());
        }
        child_containing.content.y = child_y;

        match &child.box_type {
            BoxType::BlockNode(_) | BoxType::InlineBlockNode(_) => {
                layout_block(child, &child_containing);
            }
            BoxType::AnonymousBlock => {
                layout_inline_block(child, &child_containing, &mut float_context);
            }
            BoxType::InlineNode(_) => {
                layout_inline_node(child, &child_containing);
            }
            _ => {}
        }

        last_border_bottom = child.dimensions.content.y
            + child.dimensions.content.height
            + child.dimensions.padding.bottom
            + child.dimensions.border.bottom;
        last_margin_bottom = child_margin_bottom;
    }

    // ── Height calculation ─────────────────────────────
    let explicit_height = styled.as_ref()
        .and_then(|s| s.specified_values.get("height"))
        .map(|v| parse_length_percent(v, containing_block.content.height));

    let min_height = styled.as_ref()
        .and_then(|s| s.specified_values.get("min-height"))
        .map(|v| parse_length_percent(v, containing_block.content.height))
        .unwrap_or(0.0);
    let max_height = styled.as_ref()
        .and_then(|s| s.specified_values.get("max-height"))
        .map(|v| parse_length_percent(v, containing_block.content.height))
        .unwrap_or(f32::INFINITY);

    let computed_content_height = if layout_box.children.is_empty() {
        0.0
    } else {
        let last = layout_box.children.last().unwrap();
        let last_child_bottom = last.dimensions.content.y
            + last.dimensions.content.height
            + last.dimensions.padding.bottom
            + last.dimensions.border.bottom;
        last_child_bottom - layout_box.dimensions.content.y
    };

    let final_height = match explicit_height {
        Some(h) => {
            let content_h = if box_sizing == "border-box" {
                (h - border_top - padding_top - padding_bottom - border_bottom).max(0.0)
            } else {
                h
            };
            content_h.clamp(min_height, max_height)
        }
        None => {
            let mut h = computed_content_height;
            if h <= 0.0 {
                if let Some(styled) = styled.as_ref() {
                    let text_height = measure_text_height(styled, layout_box.dimensions.content.width);
                    if text_height > 0.0 {
                        h = text_height;
                    }
                }
            }
            h.clamp(min_height, max_height)
        }
    };

    layout_box.dimensions.content.height = final_height;
}

/// Layout a floated box.  Computes size, finds a free horizontal band
/// inside `container`, and registers the occupied rectangle in
/// `float_context` so that later inline content can flow around it.
fn layout_float(layout_box: &mut LayoutBox, container: &Rect, float_context: &mut FloatContext) {
    let styled = match &layout_box.box_type {
        BoxType::FloatLeftNode(s) | BoxType::FloatRightNode(s) => Some(s.clone()),
        _ => return,
    };

    let side = match &layout_box.box_type {
        BoxType::FloatLeftNode(_) => FloatSide::Left,
        BoxType::FloatRightNode(_) => FloatSide::Right,
        _ => FloatSide::Left,
    };

    let box_sizing = get_box_sizing(styled.as_ref());

    // ── Horizontal metrics ──
    let margin_left = get_length(styled.as_ref(), "margin-left", container.width);
    let margin_right = get_length(styled.as_ref(), "margin-right", container.width);
    let padding_left = get_length(styled.as_ref(), "padding-left", container.width);
    let padding_right = get_length(styled.as_ref(), "padding-right", container.width);
    let border_left = get_length(styled.as_ref(), "border-left", container.width);
    let border_right = get_length(styled.as_ref(), "border-right", container.width);

    // ── Width ──
    let total_horizontal = margin_left + border_left + padding_left + padding_right + border_right + margin_right;
    let explicit_width = styled.as_ref()
        .and_then(|s| s.specified_values.get("width"))
        .map(|v| parse_length_percent(v, container.width));

    let mut content_width = match (explicit_width, box_sizing) {
        (Some(w), "border-box") => (w - border_left - padding_left - padding_right - border_right).max(0.0),
        (Some(w), _) => w,
        (None, _) => (container.width - total_horizontal).max(0.0),
    };

    let min_width = styled.as_ref()
        .and_then(|s| s.specified_values.get("min-width"))
        .map(|v| parse_length_percent(v, container.width))
        .unwrap_or(0.0);
    let max_width = styled.as_ref()
        .and_then(|s| s.specified_values.get("max-width"))
        .map(|v| parse_length_percent(v, container.width))
        .unwrap_or(f32::INFINITY);
    content_width = content_width.clamp(min_width, max_width);

    // ── Vertical metrics ──
    let margin_top = get_length(styled.as_ref(), "margin-top", container.width);
    let margin_bottom = get_length(styled.as_ref(), "margin-bottom", container.width);
    let padding_top = get_length(styled.as_ref(), "padding-top", container.width);
    let padding_bottom = get_length(styled.as_ref(), "padding-bottom", container.width);
    let border_top = get_length(styled.as_ref(), "border-top", container.width);
    let border_bottom = get_length(styled.as_ref(), "border-bottom", container.width);

    // ── Find a vertical band tall enough ──
    let outer_width = content_width
        + padding_left + padding_right
        + border_left + border_right
        + margin_left + margin_right;

    // Use 1px as a probe height; we will re-measure after internal layout.
    let mut y = container.y;
    loop {
        let avail = float_context.available_rect(y, 1.0, container);
        if avail.width >= outer_width {
            break;
        }
        y += 1.0;
    }

    // ── Internal block layout (relative to content area) ──
    let mut inner = Dimensions::default();
    let content_x = match side {
        FloatSide::Left => {
            let avail = float_context.available_rect(y, 1.0, container);
            avail.x + margin_left + border_left + padding_left
        }
        FloatSide::Right => {
            let avail = float_context.available_rect(y, 1.0, container);
            avail.x + avail.width - margin_right - border_right - padding_right - content_width
        }
    };
    inner.content.x = content_x;
    inner.content.y = y + margin_top + border_top + padding_top;
    inner.content.width = content_width;
    layout_block(layout_box, &inner);

    let content_height = layout_box.dimensions.content.height;

    // ── Register in float context ──
    let outer_x = content_x - margin_left - border_left - padding_left;
    let outer_y = y;
    let outer_height = content_height
        + padding_top + padding_bottom
        + border_top + border_bottom
        + margin_top + margin_bottom;

    float_context.floats.push((Rect {
        x: outer_x,
        y: outer_y,
        width: outer_width,
        height: outer_height,
    }, side));

    // ── Store final dimensions ──
    layout_box.dimensions.content.x = content_x;
    layout_box.dimensions.content.y = y + margin_top + border_top + padding_top;
    layout_box.dimensions.content.width = content_width;
    layout_box.dimensions.content.height = content_height;
    layout_box.dimensions.margin = EdgeSizes {
        top: margin_top, right: margin_right, bottom: margin_bottom, left: margin_left,
    };
    layout_box.dimensions.padding = EdgeSizes {
        top: padding_top, right: padding_right, bottom: padding_bottom, left: padding_left,
    };
    layout_box.dimensions.border = EdgeSizes {
        top: border_top, right: border_right, bottom: border_bottom, left: border_left,
    };
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
        BoxType::BlockNode(styled) | BoxType::InlineNode(styled) | BoxType::InlineBlockNode(styled)
        | BoxType::FloatLeftNode(styled) | BoxType::FloatRightNode(styled) => {
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
    use crate::dom::Element;
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
    fn test_layout_margin_collapsing() {
        // 20px bottom + 10px top => collapsed to 20px (max of same-sign margins)
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

        // child1 content starts at parent content top (0)
        assert_eq!(child1.dimensions.content.y, 0.0);
        // child2 content starts after 20px collapsed margin + its own border/padding (0)
        assert_eq!(child2.dimensions.content.y, 20.0);
    }

    #[test]
    fn test_layout_margin_collapsing_equal() {
        // 15px bottom + 15px top => collapsed to 15px
        let mut styles1 = HashMap::new();
        styles1.insert("margin-bottom".to_string(), "15px".to_string());
        let div1 = styled_element("div", styles1, vec![]);

        let mut styles2 = HashMap::new();
        styles2.insert("margin-top".to_string(), "15px".to_string());
        let div2 = styled_element("div", styles2, vec![]);

        let parent = styled_element("div", HashMap::new(), vec![div1, div2]);
        let mut root = build_layout_tree(&parent);
        layout(&mut root, viewport());

        let child2 = &root.children[1];
        assert_eq!(child2.dimensions.content.y, 15.0);
    }

    #[test]
    fn test_layout_margin_no_collapse_with_padding() {
        // Parent has padding, so margin collapsing with parent is prevented.
        // But sibling collapsing still happens.
        let mut parent_styles = HashMap::new();
        parent_styles.insert("padding-top".to_string(), "5px".to_string());

        let mut styles1 = HashMap::new();
        styles1.insert("margin-bottom".to_string(), "20px".to_string());
        let div1 = styled_element("div", styles1, vec![]);

        let mut styles2 = HashMap::new();
        styles2.insert("margin-top".to_string(), "10px".to_string());
        let div2 = styled_element("div", styles2, vec![]);

        let parent = styled_element("div", parent_styles, vec![div1, div2]);
        let mut root = build_layout_tree(&parent);
        layout(&mut root, viewport());

        let child1 = &root.children[0];
        let child2 = &root.children[1];

        // Parent content top = 0 + 5(parent padding) + 0(parent border) + 0(parent margin) = 5
        // Wait, parent padding is applied inside layout_block, so parent.content.y = containing.y(0) + margin(0) + border(0) + padding(5) = 5
        // child1 starts at parent.content.y = 5
        assert_eq!(child1.dimensions.content.y, 5.0);
        // child2 starts after child1 + collapsed margin 20
        assert_eq!(child2.dimensions.content.y, 5.0 + 20.0);
    }

    #[test]
    fn test_layout_box_sizing_border_box() {
        let mut styles = HashMap::new();
        styles.insert("width".to_string(), "200px".to_string());
        styles.insert("padding-left".to_string(), "20px".to_string());
        styles.insert("padding-right".to_string(), "20px".to_string());
        styles.insert("border-left".to_string(), "5px".to_string());
        styles.insert("border-right".to_string(), "5px".to_string());
        styles.insert("box-sizing".to_string(), "border-box".to_string());

        let styled = styled_element("div", styles, vec![]);
        let mut root = build_layout_tree(&styled);
        layout(&mut root, viewport());

        // border-box: content width = 200 - 5 - 20 - 20 - 5 = 150
        assert_eq!(root.dimensions.content.width, 150.0);
        // Total border-box width still 200
        assert_eq!(
            root.dimensions.content.width
                + root.dimensions.padding.left + root.dimensions.padding.right
                + root.dimensions.border.left + root.dimensions.border.right,
            200.0
        );
    }

    #[test]
    fn test_layout_min_max_width() {
        let mut styles = HashMap::new();
        styles.insert("width".to_string(), "50%".to_string());   // 400px in 800px viewport
        styles.insert("min-width".to_string(), "500px".to_string());

        let styled = styled_element("div", styles, vec![]);
        let mut root = build_layout_tree(&styled);
        layout(&mut root, viewport());

        assert_eq!(root.dimensions.content.width, 500.0);
    }

    #[test]
    fn test_layout_inline_block() {
        let mut ib_styles = HashMap::new();
        ib_styles.insert("display".to_string(), "inline-block".to_string());
        ib_styles.insert("width".to_string(), "100px".to_string());
        ib_styles.insert("height".to_string(), "50px".to_string());
        let ib = styled_element("span", ib_styles, vec![]);

        let parent = styled_element("div", HashMap::new(), vec![ib]);
        let mut root = build_layout_tree(&parent);
        layout(&mut root, viewport());

        // Parent should have one AnonymousBlock containing the inline-block
        assert_eq!(root.children.len(), 1);
        let anon = &root.children[0];
        assert!(matches!(anon.box_type, BoxType::AnonymousBlock));
        assert_eq!(anon.children.len(), 1);

        let ib_box = &anon.children[0];
        assert!(matches!(ib_box.box_type, BoxType::InlineBlockNode(_)));
        assert_eq!(ib_box.dimensions.content.width, 100.0);
        assert_eq!(ib_box.dimensions.content.height, 50.0);
    }

    #[test]
    fn test_skip_hidden_input() {
        let mut dom_attrs = HashMap::new();
        dom_attrs.insert("type".to_string(), "hidden".to_string());
        dom_attrs.insert("value".to_string(), "secret".to_string());
        let hidden_input = StyledNode {
            node: Node::Element(Element::new("input", dom_attrs, vec![])),
            specified_values: HashMap::new(),
            children: vec![],
        };

        let visible_text = StyledNode {
            node: Node::text("Hello"),
            specified_values: HashMap::new(),
            children: vec![],
        };
        let parent = styled_element("div", HashMap::new(), vec![hidden_input, visible_text]);

        let mut root = build_layout_tree(&parent);
        layout(&mut root, viewport());

        // Text node and hidden input should both be skipped => empty div
        assert_eq!(root.children.len(), 0);
    }

    #[test]
    fn test_layout_inline_block_auto_width() {
        let mut ib_styles = HashMap::new();
        ib_styles.insert("display".to_string(), "inline-block".to_string());
        // No explicit width; should use available width (800 - margins - borders - padding)
        let ib = styled_element("span", ib_styles, vec![]);

        let parent = styled_element("div", HashMap::new(), vec![ib]);
        let mut root = build_layout_tree(&parent);
        layout(&mut root, viewport());

        let anon = &root.children[0];
        let ib_box = &anon.children[0];
        assert!(matches!(ib_box.box_type, BoxType::InlineBlockNode(_)));
        // Auto width in 800px viewport => 800.0 (no margin/padding/border on inline-block)
        assert_eq!(ib_box.dimensions.content.width, 800.0);
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
    fn test_float_left_basic() {
        let mut float_styles = HashMap::new();
        float_styles.insert("float".to_string(), "left".to_string());
        float_styles.insert("width".to_string(), "200px".to_string());
        float_styles.insert("height".to_string(), "100px".to_string());
        let float_box = styled_element("div", float_styles, vec![]);

        let parent = styled_element("div", HashMap::new(), vec![float_box]);
        let mut root = build_layout_tree(&parent);
        layout(&mut root, viewport());

        let float_child = &root.children[0];
        assert!(matches!(float_child.box_type, BoxType::FloatLeftNode(_)));
        assert_eq!(float_child.dimensions.content.x, 0.0);
        assert_eq!(float_child.dimensions.content.y, 0.0);
        assert_eq!(float_child.dimensions.content.width, 200.0);
        assert_eq!(float_child.dimensions.content.height, 100.0);
    }

    #[test]
    fn test_float_right_basic() {
        let mut float_styles = HashMap::new();
        float_styles.insert("float".to_string(), "right".to_string());
        float_styles.insert("width".to_string(), "200px".to_string());
        float_styles.insert("height".to_string(), "100px".to_string());
        let float_box = styled_element("div", float_styles, vec![]);

        let parent = styled_element("div", HashMap::new(), vec![float_box]);
        let mut root = build_layout_tree(&parent);
        layout(&mut root, viewport());

        let float_child = &root.children[0];
        assert!(matches!(float_child.box_type, BoxType::FloatRightNode(_)));
        assert_eq!(float_child.dimensions.content.x, 800.0 - 200.0);
        assert_eq!(float_child.dimensions.content.y, 0.0);
        assert_eq!(float_child.dimensions.content.width, 200.0);
        assert_eq!(float_child.dimensions.content.height, 100.0);
    }

    #[test]
    fn test_two_floats_left() {
        let mut f1 = HashMap::new();
        f1.insert("float".to_string(), "left".to_string());
        f1.insert("width".to_string(), "200px".to_string());
        f1.insert("height".to_string(), "100px".to_string());

        let mut f2 = HashMap::new();
        f2.insert("float".to_string(), "left".to_string());
        f2.insert("width".to_string(), "200px".to_string());
        f2.insert("height".to_string(), "100px".to_string());

        let parent = styled_element("div", HashMap::new(), vec![
            styled_element("div", f1, vec![]),
            styled_element("div", f2, vec![]),
        ]);
        let mut root = build_layout_tree(&parent);
        layout(&mut root, viewport());

        let first = &root.children[0];
        let second = &root.children[1];

        // First float at left edge
        assert_eq!(first.dimensions.content.x, 0.0);
        // Second float should be placed to the right of the first
        assert_eq!(second.dimensions.content.x, 200.0);
    }

    #[test]
    fn test_clear_both() {
        let mut float_styles = HashMap::new();
        float_styles.insert("float".to_string(), "left".to_string());
        float_styles.insert("width".to_string(), "200px".to_string());
        float_styles.insert("height".to_string(), "100px".to_string());
        let float_box = styled_element("div", float_styles, vec![]);

        let mut clear_styles = HashMap::new();
        clear_styles.insert("clear".to_string(), "both".to_string());
        clear_styles.insert("height".to_string(), "50px".to_string());
        let clear_box = styled_element("div", clear_styles, vec![]);

        let parent = styled_element("div", HashMap::new(), vec![float_box, clear_box]);
        let mut root = build_layout_tree(&parent);
        layout(&mut root, viewport());

        let clear_child = &root.children[1];
        // Clear child should be placed below the float (y >= 100)
        assert_eq!(clear_child.dimensions.content.y, 100.0);
    }
}
