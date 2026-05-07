# Mini Browser 🌐

A toy web browser engine built from scratch in Rust, for learning how browsers work.

## Architecture

```
HTML Source
    │
    ▼
┌──────────┐    ┌──────────┐
│   HTML    │    │   CSS    │
│  Parser   │    │  Parser  │
└────┬─────┘    └────┬─────┘
     │               │
     ▼               ▼
  DOM Tree      Stylesheet
     │               │
     └───────┬───────┘
             │
             ▼
      ┌────────────┐
      │   Style    │
      │   Tree     │  (CSS selector matching + cascade)
      └─────┬──────┘
            │
            ▼
      ┌────────────┐
      │   Layout   │
      │   Engine   │  (Block layout + box model)
      └─────┬──────┘
            │
            ▼
      ┌────────────┐
      │   Paint    │
      │  Display   │  (Display list: rects, text, borders)
      │   List     │
      └─────┬──────┘
            │
            ▼
      ┌────────────┐
      │   egui     │  (Cross-platform GUI renderer)
      │  / eframe  │
      └────────────┘
```

## Current Status

### ✅ Completed

| Phase | Module | Description | Lines | Status |
|-------|--------|-------------|-------|--------|
| 1 | `network/` | URL parsing + `file://` protocol | 125 | ✅ Done |
| 1 | `html/` | HTML tokenizer + parser → DOM tree | 455 | ✅ Done |
| 2 | `css/` | CSS tokenizer + parser + selector matching | 563 | ✅ Done |
| 2 | `style/` | Style tree (cascade + specificity) | 166 | ✅ Done |
| 3 | `layout/` | Block layout engine (box model) | 450 | ✅ Done |
| 4 | `paint/` | Display list builder (color/text/border) | 207 | ✅ Done |
| 6 | `main.rs` | GUI window (egui + eframe) | 194 | ✅ Done |

**Total: ~2,631 lines of Rust**

### Supported CSS Features

- ✅ Type selectors (`div`, `p`, `body`)
- ✅ Class selectors (`.header`, `.content`)
- ✅ Descendant selectors (`div p`)
- ✅ ID selectors (`#main`)
- ✅ Specificity-based cascade
- ✅ `background-color`, `color` (named + #hex + rgb())
- ✅ `padding`, `margin`, `border-width`, `border-color` (shorthand expansion)
- ✅ `width`, `height` (px / %)
- ✅ `display: block | inline | none`
- ✅ `font-size`

### Supported HTML Features

- ✅ Basic tag parsing (`<div>`, `<p>`, `<span>`, etc.)
- ✅ Nested elements
- ✅ Text nodes
- ✅ `<style>` inline CSS
- ✅ `style=""` attribute
- ✅ `class=""` attribute
- ✅ `id` attribute
- ✅ `file://` protocol

## TODO

### Phase 5: JavaScript Engine (Next Up)

- [ ] Integrate `boa_engine` crate for JS execution
- [ ] Expose basic DOM API (`document.getElementById`, `element.innerHTML`)
- [ ] Handle `<script>` tags
- [ ] Simple event binding (`onclick`, etc.)

> **Note:** boa_engine compilation requires >2GB RAM. Build on local machine.

### Rendering Improvements

- [ ] Inline layout (text flowing around floating elements)
- [ ] Flexbox layout
- [ ] Text wrapping & line height (currently single-line per text node)
- [ ] Font family / font weight support
- [ ] Images (`<img>` tag)
- [ ] Links (`<a>` tag with clickable regions)
- [ ] Scroll support for long pages

### CSS Improvements

- [ ] Pseudo-classes (`:hover`, `:first-child`)
- [ ] CSS properties: `display: flex`, `position`, `overflow`
- [ ] `box-sizing: border-box`
- [ ] Media queries
- [ ] CSS values: `em`, `rem`, `vh`, `vw`
- [ ] `background` shorthand (`background: #color url(...)`)
- [ ] `border-radius`

### Browser Features

- [ ] HTTP/HTTPS support (network layer currently only does `file://`)
- [ ] URL bar navigation
- [ ] Back/forward history
- [ ] Bookmarking
- [ ] DevTools-like inspector (view DOM tree, styles, layout)

### Form Controls

- [ ] `<input type="text">` rendering + keyboard input
- [ ] `<button>` rendering + click events
- [ ] `<select>` dropdown
- [ ] `<textarea>` multi-line input
- [ ] `<form>` submission

## Getting Started

```bash
git clone https://github.com/marlonyao/mini-browser.git
cd mini-browser
cargo run
```

In the URL bar, enter a `file://` path to an HTML file:
```
file:///path/to/your/page.html
```

### CLI Test (Headless)

Verify the engine produces a display list without a GUI:

```bash
cargo run --bin test_engine -- /path/to/page.html
```

## Tech Stack

| Component | Choice | Why |
|-----------|--------|-----|
| HTML Parser | Hand-written | Learn parsing algorithms |
| CSS Parser | Hand-written | Learn parsing algorithms |
| Layout Engine | Hand-written | Understand box model |
| JS Engine | boa_engine | Production-ready JS in Rust |
| GUI | egui + eframe | Cross-platform, built-in text rendering |
| Language | Rust | Safety + performance + fun |

## License

MIT
