use std::fs;
use std::io::Read;

/// TrueType / OpenType / TTC font parser — reads cmap + hmtx for precise glyph widths.
use std::sync::OnceLock;

static FONT_CACHE: OnceLock<FontMetrics> = OnceLock::new();

/// Load and cache the system font. Call once at startup.
pub fn init_font_cache() {
    let paths: Vec<&str> = vec![
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/opentype/noto/NotoSerifCJK-Regular.ttc",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/opentype/tlwg/Loma.otf",
        "/usr/share/fonts/opentype/unifont/unifont.otf",
    ];
    for path in &paths {
        if let Some(metrics) = FontMetrics::from_file(path) {
            println!("[font] Loaded {}: {} glyphs, UPEM={}", path, metrics.glyph_widths.len(), metrics.units_per_em);
            let _ = FONT_CACHE.set(metrics);
            return;
        }
    }
    // Fallback: create empty metrics (all chars = em width)
    println!("[font] WARNING: No system font found, using fallback widths");
    let fallback = FontMetrics {
        glyph_widths: std::collections::HashMap::new(),
        units_per_em: 1000,
        font_name: "fallback".to_string(),
    };
    let _ = FONT_CACHE.set(fallback);
}

/// Get the cached font metrics.
pub fn get_font_metrics() -> &'static FontMetrics {
    if FONT_CACHE.get().is_none() {
        init_font_cache();
    }
    FONT_CACHE.get().unwrap()
}

impl Clone for FontMetrics {
    fn clone(&self) -> Self {
        FontMetrics {
            glyph_widths: self.glyph_widths.clone(),
            units_per_em: self.units_per_em,
            font_name: self.font_name.clone(),
        }
    }
}

pub struct FontMetrics {
    /// Maps Unicode code point → advanceWidth in font units (design units)
    pub glyph_widths: std::collections::HashMap<u32, u16>,
    pub units_per_em: u16,
    pub font_name: String,
}

impl FontMetrics {
    pub fn from_file(path: &str) -> Option<Self> {
        let data = fs::read(path).ok()?;
        Self::parse(&data, path)
    }

    pub fn parse(data: &[u8], name: &str) -> Option<Self> {
        // Check for TTC (TrueType Collection)
        let ttc_tag = read_tag(data, 0)?;
        if ttc_tag == "ttcf" {
            // TTC: read first font for now
            let num_fonts = read_u32_be(data, 8)? as usize;
            if num_fonts == 0 { return None; }
            let offset = read_u32_be(data, 12)? as usize;
            return Self::parse_ttf_at(data, offset, name);
        }
        // Regular TTF/OTF
        Self::parse_ttf_at(data, 0, name)
    }

    fn parse_ttf_at(data: &[u8], file_offset: usize, name: &str) -> Option<Self> {
        // Read Offset Table
        let sfnt_version = read_tag(data, file_offset)?;
        if sfnt_version != "OTTO" && sfnt_version != "\x00\x01\x00\x00" {
            return None; // not a valid TTF/OTF
        }
        let num_tables = read_u16_be(data, file_offset + 4)? as usize;

        // Search table directory for cmap and hmtx
        let mut cmap_offset: Option<usize> = None;
        let mut cmap_len: usize = 0;
        let mut hmtx_offset: Option<usize> = None;
        let mut hhea_offset: Option<usize> = None;
        let mut head_offset: Option<usize> = None;

        let table_dir_start = file_offset + 12;
        for i in 0..num_tables {
            let entry = table_dir_start + i * 16;
            let tag = read_tag(data, entry)?;
            let offset = read_u32_be(data, entry + 8)? as usize;
            let length = read_u32_be(data, entry + 12)? as usize;
            match tag {
                "cmap" => { cmap_offset = Some(offset); cmap_len = length; }
                "hmtx" => { hmtx_offset = Some(offset); }
                "hhea" => { hhea_offset = Some(offset); }
                "head" => { head_offset = Some(offset); }
                _ => {}
            }
        }

        let head_offset = head_offset?;
        let units_per_em = read_u16_be(data, head_offset + 18)?;

        let hhea_offset = hhea_offset?;
        let num_hmetrics = read_u16_be(data, hhea_offset + 34)? as usize;

        let hmtx_offset = hmtx_offset?;
        let mut glyph_widths = std::collections::HashMap::new();

        // Read cmap → build char → glyph index mapping, then read hmtx
        if let Some(cmap_off) = cmap_offset {
            let char_to_glyph = parse_cmap(data, cmap_off, cmap_len)?;
            for (&ch, &glyph_id) in &char_to_glyph {
                let width = read_hmtx(data, hmtx_offset, glyph_id as usize, num_hmetrics)?;
                glyph_widths.insert(ch, width);
            }
        }

        Some(FontMetrics {
            glyph_widths,
            units_per_em,
            font_name: name.to_string(),
        })
    }

    /// Get width in pixels for a character at the given font size.
    pub fn char_width_px(&self, ch: char, font_size_px: f32) -> f32 {
        let code = ch as u32;
        let units = self.glyph_widths.get(&code).copied().unwrap_or(self.units_per_em);
        (units as f32 / self.units_per_em as f32) * font_size_px
    }

    /// Get width in pixels for a string.
    pub fn text_width_px(&self, text: &str, font_size_px: f32) -> f32 {
        text.chars().map(|ch| self.char_width_px(ch, font_size_px)).sum()
    }

    /// Fallback: check if this font covers a character.
    pub fn has_char(&self, ch: char) -> bool {
        self.glyph_widths.contains_key(&(ch as u32))
    }
}

fn read_tag(data: &[u8], offset: usize) -> Option<&str> {
    if offset + 4 > data.len() { return None; }
    std::str::from_utf8(&data[offset..offset + 4]).ok()
}

fn read_u16_be(data: &[u8], offset: usize) -> Option<u16> {
    if offset + 2 > data.len() { return None; }
    Some(((data[offset] as u16) << 8) | (data[offset + 1] as u16))
}

fn read_u32_be(data: &[u8], offset: usize) -> Option<u32> {
    if offset + 4 > data.len() { return None; }
    Some(
        ((data[offset] as u32) << 24)
        | ((data[offset + 1] as u32) << 16)
        | ((data[offset + 2] as u32) << 8)
        | (data[offset + 3] as u32)
    )
}

fn read_i16_be(data: &[u8], offset: usize) -> Option<i16> {
    read_u16_be(data, offset).map(|v| v as i16)
}

fn parse_cmap(data: &[u8], cmap_offset: usize, _cmap_len: usize) -> Option<std::collections::HashMap<u32, u16>> {
    let version = read_u16_be(data, cmap_offset)?;
    let num_subtables = read_u16_be(data, cmap_offset + 2)? as usize;

    // Find the best subtable (prefer format 12, then 4, then 6)
    let mut best_offset: Option<usize> = None;
    let mut best_format: u16 = 0;
    let mut best_platform = 3; // lower is better (prefer Windows/Unicode)

    for i in 0..num_subtables {
        let entry = cmap_offset + 4 + i * 8;
        let platform = read_u16_be(data, entry)?;
        let encoding = read_u16_be(data, entry + 2)?;
        let offset = read_u32_be(data, entry + 4)? as usize + cmap_offset;
        let format = read_u16_be(data, offset)?;

        let score = match format {
            12 => 100,
            4 => 50,
            6 => 30,
            0 => 10,
            _ => 0,
        };
        if score > best_format {
            best_format = format;
            best_offset = Some(offset);
            best_platform = platform;
        }
    }

    let offset = best_offset?;
    let format = read_u16_be(data, offset)?;

    let mut map = std::collections::HashMap::new();
    match format {
        4 => parse_cmap_format4(data, offset, &mut map),
        6 => parse_cmap_format6(data, offset, &mut map),
        12 => parse_cmap_format12(data, offset, &mut map),
        _ => return None,
    }?;

    Some(map)
}

fn parse_cmap_format4(
    data: &[u8],
    offset: usize,
    map: &mut std::collections::HashMap<u32, u16>,
) -> Option<()> {
    let seg_count_x2 = read_u16_be(data, offset + 6)? as usize;
    let seg_count = seg_count_x2 / 2;
    let search_range = read_u16_be(data, offset + 8)? as usize;
    let entry_selector = read_u16_be(data, offset + 10)? as usize;
    let range_shift = read_u16_be(data, offset + 12)? as usize;

    let end_codes_start = offset + 14;
    let start_codes_start = end_codes_start + seg_count_x2 + 2; // +2 for reservedPad
    let id_delta_start = start_codes_start + seg_count_x2;
    let id_range_offset_start = id_delta_start + seg_count_x2;

    for i in 0..seg_count {
        let end_code = read_u16_be(data, end_codes_start + i * 2)?;
        let start_code = read_u16_be(data, start_codes_start + i * 2)?;
        let id_delta = read_i16_be(data, id_delta_start + i * 2)?;
        let id_range_offset = read_u16_be(data, id_range_offset_start + i * 2)?;

        if id_range_offset == 0 {
            for ch in start_code..=end_code {
                let glyph = ((ch as i32 + id_delta as i32) & 0xFFFF) as u16;
                map.insert(ch as u32, glyph);
            }
        } else {
            for ch in start_code..=end_code {
                let ro = id_range_offset_start + i * 2 + id_range_offset as usize + (ch - start_code) as usize * 2;
                let glyph = read_u16_be(data, ro).unwrap_or(0);
                if glyph != 0 {
                    map.insert(ch as u32, glyph);
                }
            }
        }
    }

    Some(())
}

fn parse_cmap_format6(
    data: &[u8],
    offset: usize,
    map: &mut std::collections::HashMap<u32, u16>,
) -> Option<()> {
    let first_code = read_u16_be(data, offset + 6)? as u32;
    let entry_count = read_u16_be(data, offset + 8)? as usize;
    for i in 0..entry_count {
        let glyph = read_u16_be(data, offset + 10 + i * 2)?;
        map.insert(first_code + i as u32, glyph);
    }
    Some(())
}

fn parse_cmap_format12(
    data: &[u8],
    offset: usize,
    map: &mut std::collections::HashMap<u32, u16>,
) -> Option<()> {
    let num_groups = read_u32_be(data, offset + 12)? as usize;
    let groups_start = offset + 16;
    for i in 0..num_groups {
        let start_char = read_u32_be(data, groups_start + i * 12)?;
        let end_char = read_u32_be(data, groups_start + i * 12 + 4)?;
        let start_glyph = read_u32_be(data, groups_start + i * 12 + 8)?;
        for (idx, ch) in (start_char..=end_char).enumerate() {
            map.insert(ch, (start_glyph + idx as u32) as u16);
        }
    }
    Some(())
}

fn read_hmtx(data: &[u8], hmtx_offset: usize, glyph_id: usize, num_hmetrics: usize) -> Option<u16> {
    if glyph_id < num_hmetrics {
        let off = hmtx_offset + glyph_id * 4;
        read_u16_be(data, off)
    } else {
        // Mono-spaced fallback: use last long metric advance, lsb from extras
        let last = hmtx_offset + (num_hmetrics - 1) * 4;
        let advance = read_u16_be(data, last)?;
        let extra_index = glyph_id - num_hmetrics;
        let _lsb_off = last + 2 + extra_index * 2;
        // We ignore lsb for width; just return advance
        Some(advance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_system_font() {
        // Try Noto Sans CJK SC
        let paths = [
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/opentype/tlwg/Loma.otf",
        ];
        for path in &paths {
            if let Some(metrics) = FontMetrics::from_file(path) {
                println!("Loaded {}: {} chars, UPEM={}", path, metrics.glyph_widths.len(), metrics.units_per_em);
                // Check some known widths
                let w_a = metrics.char_width_px('a', 16.0);
                let w_space = metrics.char_width_px(' ', 16.0);
                let w_cjk = metrics.char_width_px('中', 16.0);
                println!("  'a'={:.2}px, space={:.2}px, '中'={:.2}px", w_a, w_space, w_cjk);
                assert!(w_a > 0.0);
                assert!(w_cjk > 0.0);
                return;
            }
        }
    }

    #[test]
    fn test_i_narrower_than_m() {
        let path = "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc";
        if let Some(metrics) = FontMetrics::from_file(path) {
            let w_i = metrics.char_width_px('i', 16.0);
            let w_m = metrics.char_width_px('m', 16.0);
            println!("i={:.2}, m={:.2}", w_i, w_m);
            // 'i' should be significantly narrower than 'm' in any real font
            assert!(w_i < w_m * 0.5, "'i' ({:.2}) should be much narrower than 'm' ({:.2})", w_i, w_m);
        }
    }
}
