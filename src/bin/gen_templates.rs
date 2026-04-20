//! Printable SeedQR template generator.
//!
//! Produces one letter-size PDF per supported QR size (12-word V1 at 21×21,
//! 24-word V2 at 25×25). Each PDF is laid out as a 3×3 grid of cards; each
//! card has pre-printed finder patterns, section dividers, and — for V2+ —
//! alignment patterns. Users transcribe their seed backup by filling in the
//! remaining blank modules.
//!
//! Run with: `cargo run --features templates --bin gen-templates`

use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref, Str};

const PAGE_W: f32 = 612.0;
const PAGE_H: f32 = 792.0;

// Sheet layout: 3 cards wide × 3 tall preserves the original ~0.77 aspect
// ratio (PAGE_H/PAGE_W ≈ cols/rows when rows ≈ cols for letter).
const SHEET_COLS: u32 = 3;
const SHEET_ROWS: u32 = 3;

const CYAN_R: f32 = 26.0 / 255.0;
const CYAN_G: f32 = 248.0 / 255.0;
const CYAN_B: f32 = 255.0 / 255.0;

const LOGO_SVG: &str = include_str!("../../assets/brand/faraday-logo.svg");
const LOGO_VIEWBOX_H: f32 = 37.0;
const LOGO_ASPECT: f32 = 172.0 / 37.0;

#[derive(Clone, Copy)]
struct Variant {
    qr_size: u32,
    version: u32,
    word_count: u32,
    /// Distance between heavier section dividers. Chosen so the grid breaks
    /// into equal-sized square sections (qr_size / section_step per side).
    section_step: u32,
    /// Centres of 5×5 alignment patterns, in module coords. V1 has none.
    alignment_centers: &'static [(u32, u32)],
}

const VARIANTS: &[Variant] = &[
    Variant {
        qr_size: 21,
        version: 1,
        word_count: 12,
        section_step: 7, // 3×3 blocks of 7 modules, aligns with finder geometry
        alignment_centers: &[],
    },
    Variant {
        qr_size: 25,
        version: 2,
        word_count: 24,
        section_step: 5, // 5×5 blocks of 5 modules
        alignment_centers: &[(18, 18)],
    },
];

fn main() -> std::io::Result<()> {
    std::fs::create_dir_all("assets/templates")?;
    for v in VARIANTS {
        let bytes = build_pdf(v);
        let out = format!(
            "assets/templates/seedqr_{}w_{}x{}_9up.pdf",
            v.word_count, v.qr_size, v.qr_size
        );
        std::fs::write(&out, &bytes)?;
        println!("wrote {} ({} bytes)", out, bytes.len());
    }
    Ok(())
}

fn build_pdf(v: &Variant) -> Vec<u8> {
    let catalog_id = Ref::new(1);
    let page_tree_id = Ref::new(2);
    let page_id = Ref::new(3);
    let content_id = Ref::new(4);
    let font_helv_id = Ref::new(5);
    let font_helv_bold_id = Ref::new(6);
    let font_courier_id = Ref::new(7);

    let mut pdf = Pdf::new();
    pdf.catalog(catalog_id).pages(page_tree_id);
    pdf.pages(page_tree_id).kids([page_id]).count(1);

    pdf.type1_font(font_helv_id).base_font(Name(b"Helvetica"));
    pdf.type1_font(font_helv_bold_id).base_font(Name(b"Helvetica-Bold"));
    pdf.type1_font(font_courier_id).base_font(Name(b"Courier"));

    let mut page = pdf.page(page_id);
    page.parent(page_tree_id)
        .media_box(Rect::new(0.0, 0.0, PAGE_W, PAGE_H))
        .contents(content_id);
    page.resources()
        .fonts()
        .pair(Name(b"Helv"), font_helv_id)
        .pair(Name(b"HelvB"), font_helv_bold_id)
        .pair(Name(b"Cour"), font_courier_id);
    page.finish();

    let mut content = Content::new();

    let cw = PAGE_W / SHEET_COLS as f32;
    let ch = PAGE_H / SHEET_ROWS as f32;
    for row in 0..SHEET_ROWS {
        for col in 0..SHEET_COLS {
            let cx = col as f32 * cw;
            let cy = (SHEET_ROWS - 1 - row) as f32 * ch;
            draw_template(&mut content, cx, cy, cw, ch, v);
        }
    }

    pdf.stream(content_id, &content.finish());
    pdf.finish()
}

/// Render one card at (cx, cy) with size (cw, ch) for the given `Variant`.
/// All internal dimensions derive from `cw` so the card scales cleanly.
fn draw_template(c: &mut Content, cx: f32, cy: f32, cw: f32, ch: f32, v: &Variant) {
    let s = cw / 306.0;

    let frame_inset = 10.0 * s;
    let inside_pad = 8.0 * s;
    let content_pad = 14.0 * s;

    let logo_h = 18.0 * s;
    let subtitle_size = (8.0 * s).max(6.0);
    let subtitle_gap = 5.0 * s;

    let footer_title_size = (8.0 * s).max(6.5);
    let footer_body_size = (7.0 * s).max(6.0);
    let footer_line_gap = 2.5 * s;

    let label_pad = 14.0 * s;

    set_stroke_gray(c, 0.6);
    c.set_line_width(0.4);
    c.rect(
        cx + frame_inset,
        cy + frame_inset,
        cw - 2.0 * frame_inset,
        ch - 2.0 * frame_inset,
    );
    c.stroke();

    let border_top = cy + ch - frame_inset;
    let border_bot = cy + frame_inset;
    let text_x = cx + frame_inset + content_pad;

    // Header
    let logo_y = border_top - inside_pad - logo_h;
    draw_logo(c, text_x, logo_y, logo_h);

    let subtitle_baseline = logo_y - subtitle_gap - subtitle_size * 0.2;
    set_fill_gray(c, 0.0);
    let subtitle = format!(
        "CompactSeedQR  |  V{}  |  {sz}x{sz}  |  {}w",
        v.version,
        v.word_count,
        sz = v.qr_size
    );
    text(c, Name(b"Cour"), subtitle_size, text_x, subtitle_baseline, &subtitle);
    let header_bottom = subtitle_baseline - subtitle_size;

    // Footer text setup (wrap body)
    let bar_x = text_x;
    let body_x = text_x + 7.0 * s;
    let body_max_w = cw - frame_inset - content_pad - (body_x - (cx + frame_inset));
    let body_lines = wrap_text(
        "Anyone with this QR can sign transactions. Store offline. Never photograph.",
        footer_body_size * 0.55,
        body_max_w,
        2,
    );
    let footer_block_h = footer_title_size
        + footer_line_gap
        + body_lines.len() as f32 * (footer_body_size + footer_line_gap);
    let footer_top = border_bot + inside_pad + footer_block_h;

    // Grid sizing from remaining space
    let free_top = header_bottom;
    let free_bot = footer_top;
    let free_h = free_top - free_bot;

    let avail_w = cw - 2.0 * (frame_inset + content_pad) - label_pad;
    let avail_h = free_h - label_pad;
    let cell = (avail_w / v.qr_size as f32).min(avail_h / v.qr_size as f32);
    let grid = cell * v.qr_size as f32;

    let grid_block_h = grid + label_pad;
    let grid_y = free_bot + (free_h - grid_block_h) / 2.0;
    let grid_top = grid_y + grid;

    let grid_block_w = grid + label_pad;
    let grid_x = cx + (cw - grid_block_w) / 2.0 + label_pad;

    // Fine cell gridlines
    set_stroke_gray(c, 0.78);
    c.set_line_width(0.22);
    for i in 0..=v.qr_size {
        let t = i as f32 * cell;
        c.move_to(grid_x, grid_y + t).line_to(grid_x + grid, grid_y + t).stroke();
        c.move_to(grid_x + t, grid_y).line_to(grid_x + t, grid_y + grid).stroke();
    }

    // Heavier dividers every `section_step` modules, creating equal square
    // sections within the grid.
    set_stroke_gray(c, 0.35);
    c.set_line_width(0.7_f32.max(0.7 * s));
    let mut i = v.section_step;
    while i < v.qr_size {
        let t = i as f32 * cell;
        c.move_to(grid_x, grid_y + t).line_to(grid_x + grid, grid_y + t).stroke();
        c.move_to(grid_x + t, grid_y).line_to(grid_x + t, grid_y + grid).stroke();
        i += v.section_step;
    }

    // Outer border, heaviest
    set_stroke_gray(c, 0.0);
    c.set_line_width(0.9_f32.max(0.7 * s));
    c.rect(grid_x, grid_y, grid, grid);
    c.stroke();

    // Axis labels at each section boundary plus the final edge
    let axis_font_size = (6.5 * s).max(5.0);
    set_fill_gray(c, 0.0);
    let mut mark = v.section_step;
    while mark <= v.qr_size {
        let label = format!("{}", mark);
        let tx = grid_x + (mark as f32 - 0.5) * cell;
        text_centered(c, Name(b"Cour"), axis_font_size, tx, grid_top + 4.5 * s, &label);

        let ty = grid_y + grid - (mark as f32 - 0.5) * cell - axis_font_size * 0.32;
        text_right(c, Name(b"Cour"), axis_font_size, grid_x - 4.0 * s, ty, &label);
        mark += v.section_step;
    }

    // Finder patterns
    draw_finder(c, grid_x, grid_top, cell, 0, 0);
    draw_finder(c, grid_x, grid_top, cell, 0, v.qr_size - 7);
    draw_finder(c, grid_x, grid_top, cell, v.qr_size - 7, 0);

    // Alignment patterns (V2+)
    for &(arow, acol) in v.alignment_centers {
        draw_alignment(c, grid_x, grid_top, cell, arow, acol);
    }

    // Footer: cyan bar + title + wrapped body
    let footer_title_y = border_bot + inside_pad
        + body_lines.len() as f32 * (footer_body_size + footer_line_gap);
    let bar_bot = border_bot + inside_pad - 1.0;
    set_fill_rgb(c, CYAN_R, CYAN_G, CYAN_B);
    c.rect(bar_x, bar_bot, 1.5, footer_block_h + 1.0);
    c.fill_nonzero();

    set_fill_gray(c, 0.0);
    text(
        c,
        Name(b"HelvB"),
        footer_title_size,
        body_x,
        footer_title_y,
        "Solana wallet seed.",
    );
    for (i, line) in body_lines.iter().enumerate() {
        let y = border_bot + inside_pad
            + (body_lines.len() - 1 - i) as f32 * (footer_body_size + footer_line_gap);
        text(c, Name(b"Helv"), footer_body_size, body_x, y, line);
    }
}

fn wrap_text(text: &str, glyph_w: f32, max_w: f32, max_lines: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        let candidate = if cur.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", cur, word)
        };
        if (candidate.len() as f32) * glyph_w <= max_w {
            cur = candidate;
        } else {
            if !cur.is_empty() {
                lines.push(std::mem::take(&mut cur));
                if lines.len() == max_lines {
                    break;
                }
            }
            cur = word.to_string();
        }
    }
    if !cur.is_empty() && lines.len() < max_lines {
        lines.push(cur);
    }
    lines
}

/// 7×7 finder pattern: black outer, white inset, black 3×3 core.
fn draw_finder(c: &mut Content, grid_x: f32, grid_top: f32, cell: f32, row: u32, col: u32) {
    let x = grid_x + col as f32 * cell;
    let y = grid_top - (row as f32 + 7.0) * cell;

    set_fill_gray(c, 0.0);
    c.rect(x, y, 7.0 * cell, 7.0 * cell).fill_nonzero();

    set_fill_gray(c, 1.0);
    c.rect(x + cell, y + cell, 5.0 * cell, 5.0 * cell).fill_nonzero();

    set_fill_gray(c, 0.0);
    c.rect(x + 2.0 * cell, y + 2.0 * cell, 3.0 * cell, 3.0 * cell).fill_nonzero();
}

/// 5×5 alignment pattern centred on (crow, ccol): black outer, white inset,
/// black 1×1 core. Required for QR versions ≥ 2.
fn draw_alignment(
    c: &mut Content,
    grid_x: f32,
    grid_top: f32,
    cell: f32,
    crow: u32,
    ccol: u32,
) {
    let x = grid_x + (ccol as f32 - 2.0) * cell;
    let y = grid_top - (crow as f32 + 3.0) * cell;

    set_fill_gray(c, 0.0);
    c.rect(x, y, 5.0 * cell, 5.0 * cell).fill_nonzero();

    set_fill_gray(c, 1.0);
    c.rect(x + cell, y + cell, 3.0 * cell, 3.0 * cell).fill_nonzero();

    set_fill_gray(c, 0.0);
    c.rect(x + 2.0 * cell, y + 2.0 * cell, cell, cell).fill_nonzero();
}

fn draw_logo(c: &mut Content, x: f32, y: f32, target_h: f32) {
    let scale = target_h / LOGO_VIEWBOX_H;
    set_fill_rgb(c, CYAN_R, CYAN_G, CYAN_B);

    for d in extract_path_data(LOGO_SVG) {
        let mut started = false;
        for subpath in svg_subpaths(&d) {
            if subpath.is_empty() {
                continue;
            }
            let (sx, sy) = subpath[0];
            c.move_to(x + sx * scale, y + (LOGO_VIEWBOX_H - sy) * scale);
            for &(px, py) in subpath.iter().skip(1) {
                c.line_to(x + px * scale, y + (LOGO_VIEWBOX_H - py) * scale);
            }
            c.close_path();
            started = true;
        }
        if started {
            c.fill_nonzero();
        }
    }

    let _ = LOGO_ASPECT;
}

fn extract_path_data(svg: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = svg;
    while let Some(idx) = rest.find(" d=\"") {
        rest = &rest[idx + 4..];
        if let Some(end) = rest.find('"') {
            out.push(rest[..end].to_string());
            rest = &rest[end + 1..];
        } else {
            break;
        }
    }
    out
}

fn svg_subpaths(d: &str) -> Vec<Vec<(f32, f32)>> {
    let mut subpaths: Vec<Vec<(f32, f32)>> = Vec::new();
    let mut cur: Vec<(f32, f32)> = Vec::new();
    let mut cx = 0.0_f32;
    let mut cy = 0.0_f32;
    let bytes = d.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'M' | b'm' => {
                if !cur.is_empty() {
                    subpaths.push(std::mem::take(&mut cur));
                }
                i += 1;
                i += skip_sep(&d[i..]);
                let (x, n) = read_num(&d[i..]);
                i += n;
                i += skip_sep(&d[i..]);
                let (y, n) = read_num(&d[i..]);
                i += n;
                cx = x;
                cy = y;
                cur.push((cx, cy));
            }
            b'H' | b'h' => {
                i += 1;
                i += skip_sep(&d[i..]);
                let (x, n) = read_num(&d[i..]);
                i += n;
                cx = x;
                cur.push((cx, cy));
            }
            b'V' | b'v' => {
                i += 1;
                i += skip_sep(&d[i..]);
                let (y, n) = read_num(&d[i..]);
                i += n;
                cy = y;
                cur.push((cx, cy));
            }
            b'Z' | b'z' => {
                i += 1;
                if !cur.is_empty() {
                    subpaths.push(std::mem::take(&mut cur));
                }
            }
            _ => i += 1,
        }
    }
    if !cur.is_empty() {
        subpaths.push(cur);
    }
    subpaths
}

fn read_num(s: &str) -> (f32, usize) {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'-' | b'+' | b'.' | b'0'..=b'9' | b'e' | b'E' => i += 1,
            _ => break,
        }
    }
    let n: f32 = s[..i].parse().unwrap_or(0.0);
    (n, i)
}

fn skip_sep(s: &str) -> usize {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && matches!(bytes[i], b' ' | b',' | b'\t' | b'\n' | b'\r') {
        i += 1;
    }
    i
}

fn set_fill_gray(c: &mut Content, g: f32) {
    c.set_fill_gray(g);
}

fn set_stroke_gray(c: &mut Content, g: f32) {
    c.set_stroke_gray(g);
}

fn set_fill_rgb(c: &mut Content, r: f32, g: f32, b: f32) {
    c.set_fill_rgb(r, g, b);
}

fn text(c: &mut Content, font: Name, size: f32, x: f32, y: f32, s: &str) {
    c.begin_text();
    c.set_font(font, size);
    c.next_line(x, y);
    c.show(Str(s.as_bytes()));
    c.end_text();
}

fn text_centered(c: &mut Content, font: Name, size: f32, cx: f32, y: f32, s: &str) {
    let em = if font.0 == b"Cour" { 0.6 } else { 0.55 };
    let w = s.len() as f32 * size * em;
    text(c, font, size, cx - w / 2.0, y, s);
}

fn text_right(c: &mut Content, font: Name, size: f32, x_right: f32, y: f32, s: &str) {
    let em = if font.0 == b"Cour" { 0.6 } else { 0.55 };
    let w = s.len() as f32 * size * em;
    text(c, font, size, x_right - w, y, s);
}
