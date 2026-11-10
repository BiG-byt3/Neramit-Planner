//! Office-like colours, fonts and the vector icon set used by the ribbon.

use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, Painter, Pos2, Rect, Shape, Stroke, Vec2};
use std::sync::Arc;

pub const GREEN: Color32 = Color32::from_rgb(0x14, 0x7A, 0x42);
pub const GREEN_LIGHT: Color32 = Color32::from_rgb(0xD7, 0xEB, 0xDF);
pub const RIBBON_BG: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
pub const RIBBON_BORDER: Color32 = Color32::from_rgb(0xD1, 0xD1, 0xD1);
pub const HOVER_BG: Color32 = Color32::from_rgb(0xEB, 0xEB, 0xEB);
pub const PRESSED_BG: Color32 = Color32::from_rgb(0xDD, 0xDD, 0xDD);
pub const TEXT: Color32 = Color32::from_rgb(0x26, 0x26, 0x26);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x66, 0x66, 0x66);
pub const DISABLED: Color32 = Color32::from_rgb(0xA6, 0xA6, 0xA6);
pub const GRID_LINE: Color32 = Color32::from_rgb(0xDA, 0xDA, 0xDA);
pub const HEADER_BG: Color32 = Color32::from_rgb(0xE6, 0xE6, 0xE6);
pub const CELL_SEL_BORDER: Color32 = Color32::from_rgb(0x2E, 0x77, 0x32);
pub const SUMMARY: Color32 = Color32::from_rgb(0x40, 0x40, 0x40);
pub const LINK: Color32 = Color32::from_rgb(0x2F, 0x5F, 0x9E);
pub const TIMELINE_BG: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
pub const TIMELINE_BAR: Color32 = Color32::from_rgb(0xDE, 0xEA, 0xF6);
/// Modern chrome
pub const TAB_ROW: Color32 = Color32::from_rgb(0xF0, 0xF0, 0xF0);
pub const ACCENT: Color32 = Color32::from_rgb(0x0F, 0x70, 0x3B);
pub const STATUS_BG: Color32 = Color32::from_rgb(0xF5, 0xF5, 0xF5);
pub const NAV_BG: Color32 = Color32::from_rgb(0xF0, 0xF0, 0xF0);
pub const BACKSTAGE_NAV: Color32 = Color32::from_rgb(0xE4, 0xE4, 0xEA);
pub const CONTENT_BG: Color32 = Color32::from_rgb(0xF5, 0xF5, 0xF5);
/// Classic ("Colorful") chrome:
/// green title bar and tab row, white selected tab, flat light-gray ribbon body.
pub const CLASSIC_GREEN: Color32 = Color32::from_rgb(0x2E, 0x77, 0x32);
/// Header (title bar + tab row): a continuous green gradient in the application's brand palette.
pub const HEADER_TOP: Color32 = Color32::from_rgb(0x19, 0x5A, 0x28);
pub const HEADER_MID: Color32 = Color32::from_rgb(0x20, 0x69, 0x2F);
pub const HEADER_BOTTOM: Color32 = Color32::from_rgb(0x27, 0x77, 0x36);
/// Text of the active tab (on the light ribbon sheet).
pub const HEADER_INK: Color32 = Color32::from_rgb(0x1A, 0x5E, 0x2A);
pub const GOLD: Color32 = Color32::from_rgb(0xF5, 0xB8, 0x3D);
/// Rounded top corners of a tab sheet.
pub const TAB_TOP: egui::CornerRadius = egui::CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 };

/// Translucent white for overlays on the green header (hover, pills, badges).
pub fn white(alpha: u8) -> Color32 {
    Color32::from_white_alpha(alpha)
}

/// Vertical two-colour gradient.
pub fn vgradient(p: &Painter, r: Rect, top: Color32, bottom: Color32) {
    gradient(p, r, [top, top, bottom, bottom]);
}

/// Horizontal two-colour gradient.
pub fn hgradient(p: &Painter, r: Rect, left: Color32, right: Color32) {
    gradient(p, r, [left, right, left, right]);
}

fn gradient(p: &Painter, r: Rect, [lt, rt, lb, rb]: [Color32; 4]) {
    let mut m = egui::Mesh::default();
    m.colored_vertex(r.left_top(), lt);
    m.colored_vertex(r.right_top(), rt);
    m.colored_vertex(r.left_bottom(), lb);
    m.colored_vertex(r.right_bottom(), rb);
    m.add_triangle(0, 1, 2);
    m.add_triangle(1, 3, 2);
    p.add(Shape::mesh(m));
}
/// Ribbon: light sheet (also the active tab) fading slightly towards its bottom edge.
pub const CLASSIC_RIBBON_BG: Color32 = Color32::from_rgb(0xF7, 0xF8, 0xF7);
pub const CLASSIC_RIBBON_BOTTOM: Color32 = Color32::from_rgb(0xEF, 0xF1, 0xEF);
/// Ribbon controls: brand-green hover, pressed and checked states.
pub const RIBBON_HOVER: Color32 = Color32::from_rgb(0xE2, 0xF0, 0xE5);
pub const RIBBON_CHECKED: Color32 = Color32::from_rgb(0xCF, 0xE7, 0xD5);
pub const RIBBON_CHECKED_EDGE: Color32 = Color32::from_rgb(0x86, 0xC2, 0x93);
pub const BRAND_GREEN: Color32 = Color32::from_rgb(0x2B, 0x82, 0x38);
/// The sheet header: light grey titles,
/// the selected column darker with a green rule, grey separators, grey title text.
pub const SHEET_HEAD_BG: Color32 = Color32::from_rgb(0xE6, 0xE6, 0xE6);
pub const SHEET_HEAD_SEL: Color32 = Color32::from_rgb(0xD2, 0xD2, 0xD2);
pub const SHEET_HEAD_SEP: Color32 = Color32::from_rgb(0xD4, 0xD4, 0xD4);
pub const SHEET_HEAD_SEP_SEL: Color32 = Color32::from_rgb(0xB8, 0xB8, 0xB8);
pub const SHEET_HEAD_BOTTOM: Color32 = Color32::from_rgb(0xAB, 0xAB, 0xAB);
/// selected row with one selected cell: its ID cell and row edges (MPP)
pub const SHEET_ID_SEL: Color32 = Color32::from_rgb(0xD1, 0xD1, 0xD1);
pub const SHEET_ID_SEL_LINE: Color32 = Color32::from_rgb(0xC1, 0xC1, 0xC1);
pub const SHEET_SEL_ROW_LINE: Color32 = Color32::from_rgb(0x78, 0x77, 0x78);
/// the sheet / chart divider (5 px)
pub const SHEET_SPLIT_EDGE: Color32 = Color32::from_rgb(0xBF, 0xBF, 0xBF);
pub const SHEET_HEAD_TEXT: Color32 = Color32::from_rgb(0x5E, 0x63, 0x6A);
pub const SHEET_ID_EDGE: Color32 = Color32::from_rgb(0xAB, 0xAB, 0xAB);
/// The standard link lines: critical red, others blue.
pub const SHEET_LINK: Color32 = Color32::from_rgb(0x26, 0x7C, 0xD3);
pub const SHEET_LINK_CRIT: Color32 = Color32::from_rgb(0xDD, 0x24, 0x21);
pub const CLASSIC_RIBBON_EDGE: Color32 = Color32::from_rgb(0xD2, 0xD2, 0xD2);
pub const CLASSIC_STATUS_BG: Color32 = Color32::from_rgb(0xF3, 0xF3, 0xF3);
/// column header row of the sheet in the classic look (timescale tiers stay #E6E6E6)
pub const SHEET_HEADER_CLASSIC: Color32 = Color32::from_rgb(0xD2, 0xD2, 0xD2);
/// Selected row in the sheet (gray band) and the green lines above / below it that cross the chart.
pub const ROW_SEL_CLASSIC: Color32 = Color32::from_rgb(0xC6, 0xC6, 0xC6);
pub const ROW_SEL_ID: Color32 = Color32::from_rgb(0xD2, 0xD2, 0xD2);
pub const ROW_SEL_LINE: Color32 = Color32::from_rgb(0x2E, 0x77, 0x32);

pub fn rgb(c: crate::format::Rgb) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

/// Font for a text style: Noto Sans (or a loaded system font) in regular / bold weight.
pub fn style_font(ts: &crate::format::TextStyle) -> egui::FontId {
    egui::FontId::new(ts.px(), family(&ts.font, ts.bold))
}

/// Font for column titles, timescale tiers and bar text. Thai legacy faces (Cordia New, Angsana
/// New, Browallia New, TH Sarabun) draw about a quarter smaller than other fonts at the same
/// point size, which made 8-9 pt titles hard to read: they are enlarged to a normal font's size.
pub fn chrome_font(ts: &crate::format::TextStyle) -> egui::FontId {
    let mut f = style_font(ts);
    if let Some(scale) = thai_scale_of(&ts.font) {
        if scale < 0.8 {
            f.size /= scale;
        }
    }
    f
}

/// Size of Thai letters in a font against the bundled Noto Sans Thai (see `SysFont::thai_scale`),
/// for the hand-listed fonts and for any other font once it is loaded.
fn thai_scale_of(name: &str) -> Option<f32> {
    if let Some(sf) = SYSTEM_FONTS.iter().find(|s| s.name.eq_ignore_ascii_case(name)) {
        return Some(sf.thai_scale);
    }
    LOADED_FITS.lock().ok()?.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, (_, s))| *s)
}

/// (has Thai letters, Thai scale) worked out for loaded fonts that are not in `SYSTEM_FONTS`.
static LOADED_FITS: std::sync::Mutex<Vec<(String, (bool, f32))>> = std::sync::Mutex::new(Vec::new());

/// Height of a letter in a font, per em (from its outline's bounds).
fn letter_height(data: &[u8], index: u32, c: char) -> Option<f32> {
    let face = ttf_parser::Face::parse(data, index).ok()?;
    let r = face.glyph_bounding_box(face.glyph_index(c)?)?;
    Some((r.y_max - r.y_min) as f32 / face.units_per_em() as f32)
}

/// How the bundled Noto Sans Thai fits a font the program has no measured values for:
/// (the font has Thai letters of its own, size of Noto Sans Thai inside its family).
/// A Thai font: Noto's Thai letters take the height of the font's own (ก against ก). A Latin font:
/// Thai is balanced against the font's x-height as it is against Noto Sans (`THAI_SCALE`).
/// These rules give the hand-measured values of `SYSTEM_FONTS` within a few per cent.
pub(crate) fn thai_fit(data: &[u8], index: u32) -> (bool, f32) {
    if let (Some(h), Some(noto)) = (letter_height(data, index, 'ก'), letter_height(NOTO_THAI, 0, 'ก')) {
        return (true, (h / noto).clamp(0.5, 1.5));
    }
    let xh = letter_height(data, index, 'x').or_else(|| {
        let face = ttf_parser::Face::parse(data, index).ok()?;
        Some(face.x_height()? as f32 / face.units_per_em() as f32)
    });
    match (xh, letter_height(NOTO_SANS, 0, 'x')) {
        (Some(xh), Some(noto)) if xh > 0.0 => (false, (THAI_SCALE * xh / noto).clamp(0.6, 1.5)),
        _ => (false, THAI_SCALE),
    }
}

/// Font family key used by `install_fonts` / `ensure_font` for a face name and weight.
pub fn family(name: &str, bold: bool) -> FontFamily {
    if name.is_empty() || name == crate::format::DEFAULT_FONT {
        if bold {
            FontFamily::Name(BOLD.into())
        } else {
            FontFamily::Proportional
        }
    } else if bold {
        FontFamily::Name(format!("{name} Bold").into())
    } else {
        FontFamily::Name(name.into())
    }
}

/// A Windows font offered in the Font box (loaded on demand from C:\Windows\Fonts).
pub struct SysFont {
    pub name: &'static str,
    file: &'static str,
    bold_file: &'static str,
    /// face of the bold style in `bold_file` (the Thai collections hold every style)
    bold_index: u32,
    /// the font has Thai letters of its own
    thai: bool,
    /// size of the bundled Noto Sans Thai inside this family so Thai matches the font's own
    /// letters (measured glyph heights; Cordia New draws small for its point size)
    thai_scale: f32,
}

const fn sys(name: &'static str, file: &'static str, bold_file: &'static str, bold_index: u32, thai: bool, thai_scale: f32) -> SysFont {
    SysFont { name, file, bold_file, bold_index, thai, thai_scale }
}

pub const SYSTEM_FONTS: [SysFont; 12] = [
    sys("Arial", "arial.ttf", "arialbd.ttf", 0, false, 1.07),
    sys("Calibri", "calibri.ttf", "calibrib.ttf", 0, false, 0.96),
    sys("Leelawadee UI", "LeelawUI.ttf", "LeelaUIb.ttf", 0, true, 0.91),
    sys("Segoe UI", "segoeui.ttf", "segoeuib.ttf", 0, false, 1.03),
    sys("Tahoma", "tahoma.ttf", "tahomabd.ttf", 0, true, 1.0),
    sys("Times New Roman", "times.ttf", "timesbd.ttf", 0, false, 0.92),
    sys("Verdana", "verdana.ttf", "verdanab.ttf", 0, false, 1.12),
    // Thai fonts common in MPP files (TrueType collections: 0 regular, 1 bold)
    sys("Angsana New", "angsana.ttc", "angsana.ttc", 1, true, 0.72),
    sys("Browallia New", "browalia.ttc", "browalia.ttc", 1, true, 0.72),
    sys("Cordia New", "cordia.ttc", "cordia.ttc", 1, true, 0.73),
    // Thai government fonts, usually installed for the user only
    sys("TH SarabunPSK", "THSarabun.ttf", "THSarabun Bold.ttf", 0, true, 0.72),
    sys("TH Sarabun New", "THSarabunNew.ttf", "THSarabunNew Bold.ttf", 0, true, 0.72),
];

/// A font file in the Windows Fonts folder or the user's own (fonts installed "for me only").
fn font_file(file: &str) -> Option<std::path::PathBuf> {
    let windows = std::path::PathBuf::from(std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into())).join("Fonts");
    let user = std::env::var_os("LOCALAPPDATA").map(|d| std::path::PathBuf::from(d).join("Microsoft").join("Windows").join("Fonts"));
    [Some(windows), user].into_iter().flatten().map(|d| d.join(file)).find(|p| p.exists())
}

/// Font names for the Font box: the bundled Noto Sans, then every font family installed on this PC.
pub fn available_fonts() -> &'static [String] {
    static LIST: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    LIST.get_or_init(|| {
        let default = crate::format::DEFAULT_FONT;
        std::iter::once(default.to_string())
            .chain(crate::sysfonts::families().iter().map(|f| f.name.clone()).filter(|n| !n.eq_ignore_ascii_case(default)))
            .collect()
    })
}

/// The Font box's names matching what was typed: names that start with it first, then names with
/// a word that starts with it ("sarabun" finds "TH Sarabun New"), then names that contain it.
pub fn find_fonts(query: &str) -> Vec<&'static String> {
    let q = query.trim().to_lowercase();
    let all = available_fonts();
    if q.is_empty() {
        return all.iter().collect();
    }
    let rank = |name: &str| {
        let n = name.to_lowercase();
        if n.starts_with(&q) {
            Some(0)
        } else if n.split([' ', '-']).any(|w| w.starts_with(&q)) {
            Some(1)
        } else if n.contains(&q) {
            Some(2)
        } else {
            None
        }
    };
    let mut hits: Vec<(u8, &'static String)> = all.iter().filter_map(|f| rank(f).map(|r| (r, f))).collect();
    hits.sort_by_key(|(r, _)| *r); // stable: alphabetical inside each rank
    hits.into_iter().map(|(_, f)| f).collect()
}

/// A face to load: a file and its index in a .ttc collection.
type FaceFile = (std::path::PathBuf, u32);

/// The files of a family (regular, bold): the hand-listed files for the fonts in `SYSTEM_FONTS`,
/// else the installed family the font scan found under that name.
fn family_files(name: &str) -> Option<(FaceFile, Option<FaceFile>)> {
    if let Some(sf) = SYSTEM_FONTS.iter().find(|f| f.name.eq_ignore_ascii_case(name)) {
        if let Some(reg) = font_file(sf.file) {
            return Some(((reg, 0), font_file(sf.bold_file).map(|p| (p, sf.bold_index))));
        }
    }
    let f = crate::sysfonts::find(name)?;
    Some(((f.regular.path.clone(), f.regular.index), f.bold.as_ref().map(|b| (b.path.clone(), b.index))))
}

/// Make `name` usable as a font family (regular + " Bold"), reading it from the PC's fonts.
/// Thai text in those families falls back to the bundled Noto Sans Thai, like Windows font linking.
pub fn ensure_font(ctx: &egui::Context, name: &str) -> bool {
    use egui::epaint::text::{FontInsert, FontPriority, InsertFontFamily};
    if name.is_empty() || name == crate::format::DEFAULT_FONT {
        return true;
    }
    let loaded_id = egui::Id::unique(("font_loaded", name.to_string()));
    if ctx.data(|d| d.get_temp::<bool>(loaded_id)).unwrap_or(false) {
        return true;
    }
    let Some(((reg_path, reg_index), bold)) = family_files(name) else { return false };
    let Ok(reg_bytes) = std::fs::read(&reg_path) else { return false };
    if ttf_parser::Face::parse(&reg_bytes, reg_index).is_err() {
        return false;
    }
    let (bold_bytes, bold_index) = match bold.and_then(|(p, i)| Some((std::fs::read(p).ok()?, i))) {
        Some((b, i)) if ttf_parser::Face::parse(&b, i).is_ok() => (b, i),
        _ => (reg_bytes.clone(), reg_index),
    };
    let (has_thai, thai_scale) = match SYSTEM_FONTS.iter().find(|f| f.name.eq_ignore_ascii_case(name)) {
        Some(sf) => (sf.thai, sf.thai_scale),
        None => {
            let fit = thai_fit(&reg_bytes, reg_index);
            if let Ok(mut v) = LOADED_FITS.lock() {
                v.push((name.to_string(), fit));
            }
            fit
        }
    };
    for (fam, bytes, index, thai) in [(name.to_string(), reg_bytes, reg_index, "noto_thai"), (format!("{name} Bold"), bold_bytes, bold_index, "noto_thai_bold")] {
        let thai_data: &'static [u8] = if thai == "noto_thai" { NOTO_THAI } else { NOTO_THAI_BOLD };
        // both faces of the family get the same line metrics (those of the face that leads it),
        // so Thai and Latin letters share one baseline
        let (sys_bytes, thai_font) = if has_thai {
            // Noto Sans Thai leads (all Thai letters in one look): the system font follows its metrics
            let thai_m = line_metrics(thai_data, 0).map(|(a, d, g)| (a * thai_scale, d * thai_scale, g * thai_scale));
            let sys = match thai_m {
                Some(m) => with_line_metrics(&bytes, index, m, 1.0),
                None => bytes.clone(),
            };
            (sys, FontData::from_static(thai_data).tweak(egui::epaint::text::FontTweak { scale: thai_scale, ..Default::default() }))
        } else {
            let thai_font = thai_fallback(&bytes, index, thai_data, thai_scale);
            (bytes, thai_font)
        };
        let mut data = FontData::from_owned(sys_bytes);
        data.index = index;
        ctx.add_font(FontInsert::new(&format!("sys:{fam}"), data, vec![InsertFontFamily { family: FontFamily::Name(fam.clone().into()), priority: FontPriority::Highest }]));
        // the bundled Thai font inside the new family, under its own key (add_font ignores a key
        // that is already registered, which left Thai text as boxes). Our Thai is pre-shaped for
        // that font, so it goes first in a Thai system font (one look for every Thai letter) and
        // last in a Latin one.
        ctx.add_font(FontInsert::new(
            &format!("{thai}@{fam}"),
            thai_font,
            vec![InsertFontFamily { family: FontFamily::Name(fam.into()), priority: if has_thai { FontPriority::Highest } else { FontPriority::Lowest } }],
        ));
    }
    ctx.data_mut(|d| d.insert_temp(loaded_id, true));
    true
}

/// Draw a line in one of the Gridlines dialog styles.
/// A dotted horizontal / vertical line of one-pixel dots, `step` device pixels apart.
fn pixel_dots(p: &Painter, a: Pos2, b: Pos2, step: usize, color: Color32) {
    let ppp = p.pixels_per_point();
    let clip = p.clip_rect();
    let px = |v: f32| (v * ppp).floor();
    let mut mesh = egui::Mesh::default();
    let mut dot = |x: f32, y: f32| {
        let r = Rect::from_min_size(Pos2::new(x / ppp, y / ppp), Vec2::splat(1.0 / ppp));
        if clip.contains_rect(r) {
            mesh.add_colored_rect(r, color);
        }
    };
    if (a.x - b.x).abs() < 0.01 {
        let x = px(a.x);
        let (y0, y1) = (px(a.y.min(b.y)), px(a.y.max(b.y)));
        let mut y = (y0 / step as f32).ceil() * step as f32;
        while y < y1 {
            dot(x, y);
            y += step as f32;
        }
    } else if (a.y - b.y).abs() < 0.01 {
        let y = px(a.y);
        let (x0, x1) = (px(a.x.min(b.x)), px(a.x.max(b.x)));
        let mut x = (x0 / step as f32).ceil() * step as f32;
        while x < x1 {
            dot(x, y);
            x += step as f32;
        }
    } else {
        p.add(Shape::dashed_line(&[a, b], Stroke::new(1.0_f32, color), 1.0, step as f32 - 1.0));
        return;
    }
    p.add(Shape::mesh(mesh));
}

/// Put a horizontal / vertical line on pixel centres (crisp 1 px lines).
#[allow(dead_code)]
fn snap_line(p: &Painter, a: Pos2, b: Pos2) -> (Pos2, Pos2) {
    let ppp = p.pixels_per_point();
    let c = |v: f32| ((v * ppp).floor() + 0.5) / ppp;
    if (a.x - b.x).abs() < 0.01 {
        (Pos2::new(c(a.x), a.y), Pos2::new(c(a.x), b.y))
    } else if (a.y - b.y).abs() < 0.01 {
        (Pos2::new(a.x, c(a.y)), Pos2::new(b.x, c(a.y)))
    } else {
        (a, b)
    }
}

pub fn styled_line(p: &Painter, a: Pos2, b: Pos2, kind: crate::format::LineType, color: Color32) {
    use crate::format::LineType;
    let stroke = Stroke::new(1.0_f32, color);
    match kind {
        LineType::None => {}
        LineType::Solid => { p.line_segment([a, b], stroke); }
        // dots: snapped to the pixel grid and 2 px long, otherwise anti-aliasing blurs 1 px dots
        // at fractional positions into near-invisible grey
        // the standard dots: single device pixels (every 4th / every 2nd), drawn as an unsmoothed mesh
        // so they stay crisp (anti-aliased 1 px dashes fade to near-invisible grey)
        LineType::Dotted => pixel_dots(p, a, b, 4, color),
        LineType::DenseDots => pixel_dots(p, a, b, 2, color),
        LineType::Dashed => { p.add(Shape::dashed_line(&[a, b], stroke, 4.0, 3.0)); }
        LineType::DashDot => { p.add(Shape::dashed_line_with_offset(&[a, b], stroke, &[5.0, 1.0], &[2.0, 2.0], 0.0)); }
    }
}

/// Fill a rectangle with one of the bar patterns.
pub fn pattern_fill(p: &Painter, r: Rect, pattern: crate::format::Pattern, color: Color32) {
    use crate::format::Pattern;
    let thin = Stroke::new(1.0_f32, color);
    match pattern {
        Pattern::Hollow => {}
        Pattern::Solid => { p.rect_filled(r, 0.0, color); }
        Pattern::LightDots | Pattern::MediumDots | Pattern::DenseDots => {
            let step = match pattern {
                Pattern::LightDots => 4.0,
                Pattern::MediumDots => 3.0,
                _ => 2.0,
            };
            let mut y = r.top() + 1.0;
            let mut odd = false;
            while y < r.bottom() {
                let mut x = r.left() + if odd { step / 2.0 } else { 1.0 };
                while x < r.right() {
                    p.rect_filled(Rect::from_min_size(Pos2::new(x, y), Vec2::splat(1.0)), 0.0, color);
                    x += step;
                }
                y += step / 2.0;
                odd = !odd;
            }
        }
        Pattern::DiagonalUp | Pattern::DiagonalDown | Pattern::DiagonalCross => {
            let clip = p.with_clip_rect(r);
            let h = r.height();
            let mut x = r.left() - h;
            while x < r.right() {
                if pattern != Pattern::DiagonalDown {
                    clip.line_segment([Pos2::new(x, r.bottom()), Pos2::new(x + h, r.top())], thin);
                }
                if pattern != Pattern::DiagonalUp {
                    clip.line_segment([Pos2::new(x, r.top()), Pos2::new(x + h, r.bottom())], thin);
                }
                x += 4.0;
            }
        }
        Pattern::Horizontal | Pattern::Vertical | Pattern::Cross => {
            if pattern != Pattern::Vertical {
                let mut y = r.top() + 1.0;
                while y < r.bottom() {
                    p.line_segment([Pos2::new(r.left(), y), Pos2::new(r.right(), y)], thin);
                    y += 3.0;
                }
            }
            if pattern != Pattern::Horizontal {
                let mut x = r.left() + 1.0;
                while x < r.right() {
                    p.line_segment([Pos2::new(x, r.top()), Pos2::new(x, r.bottom())], thin);
                    x += 3.0;
                }
            }
        }
    }
}

/// Draw one bar end marker centred at `c` with the given height.
pub fn end_marker(p: &Painter, c: Pos2, h: f32, st: crate::format::EndStyle) {
    use crate::format::{EndShape, EndType};
    let col = rgb(st.color);
    let r = h / 2.0;
    let stroke = Stroke::new(1.2_f32, col);
    let fill = if st.kind == EndType::Solid { col } else { Color32::TRANSPARENT };
    let poly = |pts: Vec<Pos2>| match st.kind {
        EndType::Solid => Shape::convex_polygon(pts, fill, Stroke::NONE),
        EndType::Framed => Shape::convex_polygon(pts, Color32::TRANSPARENT, stroke),
        EndType::Dashed => Shape::dashed_line(&{
            let mut v = pts.clone();
            v.push(pts[0]);
            v
        }, stroke, 2.0, 2.0)
        .into_iter()
        .next()
        .unwrap_or(Shape::Noop),
    };
    match st.shape {
        EndShape::None => {}
        EndShape::Diamond => { p.add(poly(vec![c + Vec2::new(0.0, -r), c + Vec2::new(r, 0.0), c + Vec2::new(0.0, r), c + Vec2::new(-r, 0.0)])); }
        EndShape::Circle => {
            if st.kind == EndType::Solid {
                p.circle_filled(c, r, col);
            } else {
                p.circle_stroke(c, r, stroke);
            }
        }
        EndShape::Dot => { p.circle_filled(c, r * 0.4, col); }
        EndShape::Square => { p.add(poly(vec![c + Vec2::new(-r, -r), c + Vec2::new(r, -r), c + Vec2::new(r, r), c + Vec2::new(-r, r)])); }
        EndShape::TriangleUp | EndShape::HouseUp => { p.add(poly(vec![c + Vec2::new(0.0, -r), c + Vec2::new(r, r), c + Vec2::new(-r, r)])); }
        EndShape::TriangleDown | EndShape::HouseDown => { p.add(poly(vec![c + Vec2::new(-r, -r), c + Vec2::new(r, -r), c + Vec2::new(0.0, r)])); }
        EndShape::ArrowUp | EndShape::CaretUp => {
            p.add(poly(vec![c + Vec2::new(0.0, -r), c + Vec2::new(r, 0.0), c + Vec2::new(-r, 0.0)]));
            if st.shape == EndShape::ArrowUp {
                p.line_segment([c, c + Vec2::new(0.0, r)], Stroke::new(2.0_f32, col));
            }
        }
        EndShape::ArrowDown | EndShape::CaretDown => {
            p.add(poly(vec![c + Vec2::new(-r, 0.0), c + Vec2::new(r, 0.0), c + Vec2::new(0.0, r)]));
            if st.shape == EndShape::ArrowDown {
                p.line_segment([c + Vec2::new(0.0, -r), c], Stroke::new(2.0_f32, col));
            }
        }
        EndShape::ArrowLeft => { p.add(poly(vec![c + Vec2::new(-r, 0.0), c + Vec2::new(r * 0.4, -r), c + Vec2::new(r * 0.4, r)])); }
        EndShape::ArrowRight => { p.add(poly(vec![c + Vec2::new(r, 0.0), c + Vec2::new(-r * 0.4, -r), c + Vec2::new(-r * 0.4, r)])); }
        EndShape::Star => {
            let mut pts = Vec::new();
            for k in 0..10 {
                let a = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::PI / 5.0;
                let rr = if k % 2 == 0 { r } else { r * 0.45 };
                pts.push(c + Vec2::new(a.cos() * rr, a.sin() * rr));
            }
            p.add(Shape::closed_line(pts, Stroke::new(1.0_f32, col)));
        }
        EndShape::Line => { p.line_segment([c + Vec2::new(0.0, -r), c + Vec2::new(0.0, r)], Stroke::new(2.0_f32, col)); }
        // summary bar caps: a short vertical tick with a triangular foot pointing down
        EndShape::BracketLeft => { p.add(Shape::convex_polygon(vec![c + Vec2::new(0.0, -r * 0.4), c + Vec2::new(r, -r * 0.4), c + Vec2::new(0.0, r)], col, Stroke::NONE)); }
        EndShape::BracketRight => { p.add(Shape::convex_polygon(vec![c + Vec2::new(-r, -r * 0.4), c + Vec2::new(0.0, -r * 0.4), c + Vec2::new(0.0, r)], col, Stroke::NONE)); }
    }
}

pub const BOLD: &str = "bold";

/// Embedded UI fonts: Noto Sans (Latin) with Noto Sans Thai as the Thai fallback.
/// Bundled in the executable so the portable build looks the same on every PC (SIL OFL 1.1).
pub(crate) const NOTO_SANS: &[u8] = include_bytes!("../assets/fonts/NotoSans-Regular.ttf");
// "bold" uses SemiBold / Medium: full Bold Thai is too heavy at UI sizes
pub(crate) const NOTO_SANS_BOLD: &[u8] = include_bytes!("../assets/fonts/NotoSans-SemiBold.ttf");
// Thai fonts carry the pre-shaped glyphs added by tools/thai_preshape.py (see src/thai.rs)
pub(crate) const NOTO_THAI: &[u8] = include_bytes!("../assets/fonts/NotoSansThai-Regular-RD.ttf");
pub(crate) const NOTO_THAI_BOLD: &[u8] = include_bytes!("../assets/fonts/NotoSansThai-Medium-RD.ttf");


/// Where a font's tables are: (table offset by tag, units per em). `index` picks a face in a .ttc.
fn font_tables(data: &[u8], index: u32) -> Option<(impl Fn(&[u8; 4]) -> Option<usize> + '_, f32)> {
    let u16_at = |o: usize| data.get(o..o + 2).map(|b| u16::from_be_bytes([b[0], b[1]]));
    let u32_at = |o: usize| data.get(o..o + 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize);
    let base = if data.get(0..4) == Some(b"ttcf") { u32_at(12 + 4 * index as usize)? } else { 0 };
    let n = u16_at(base + 4)? as usize;
    let table = move |tag: &[u8; 4]| {
        (0..n).map(|k| base + 12 + 16 * k).find(|&r| data.get(r..r + 4) == Some(&tag[..])).and_then(|r| data.get(r + 8..r + 12)).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize)
    };
    let upm = u16_at(table(b"head")? + 18)? as f32;
    (upm > 0.0).then_some((table, upm))
}

/// A font's line metrics per em (ascent, descent (negative), line gap), chosen as the text engine
/// chooses them: OS/2 typo metrics when USE_TYPO_METRICS is set, else hhea, else typo / win.
fn line_metrics(data: &[u8], index: u32) -> Option<(f32, f32, f32)> {
    let (table, upm) = font_tables(data, index)?;
    let u16_at = |o: usize| data.get(o..o + 2).map(|b| u16::from_be_bytes([b[0], b[1]]));
    let i16_at = |o: usize| u16_at(o).map(|v| v as i16 as f32);
    let os2 = table(b"OS/2");
    let typo = os2.and_then(|o| Some((i16_at(o + 68)?, i16_at(o + 70)?, i16_at(o + 72)?)));
    let use_typo = os2.and_then(|o| u16_at(o + 62)).map(|f| f & 0x80 != 0).unwrap_or(false);
    let hhea = table(b"hhea").and_then(|h| Some((i16_at(h + 4)?, i16_at(h + 6)?, i16_at(h + 8)?)));
    let (a, d, g) = match (use_typo, typo, hhea) {
        (true, Some(t), _) => t,
        (_, _, Some(h)) if h.0 != 0.0 || h.1 != 0.0 => h,
        (_, Some(t), _) if t.0 != 0.0 || t.1 != 0.0 => t,
        _ => {
            let o = os2?;
            (u16_at(o + 74)? as f32, -(u16_at(o + 76)? as f32), 0.0)
        }
    };
    Some((a / upm, d / upm, g / upm))
}

/// A copy of a font whose line metrics are set so that, drawn at `scale`, they equal `target`
/// (per em). Used for the fallback face of a family: in a row that mixes faces the text engine
/// starts each face's line box at the row top and centres the difference of the line heights, so a
/// face with other ascent / descent proportions (Noto Sans Thai against Noto Sans) sat up to
/// 1.3 pt higher or lower than the letters next to it, by a different amount at each size. With
/// equal metrics both faces get the same baseline. Only the copy in memory changes.
fn with_line_metrics(data: &[u8], index: u32, target: (f32, f32, f32), scale: f32) -> Vec<u8> {
    let mut out = data.to_vec();
    let Some((table, upm)) = font_tables(data, index) else { return out };
    let units = |v: f32| (v / scale * upm).round().clamp(i16::MIN as f32, i16::MAX as f32) as i16;
    let (a, d, g) = (units(target.0), units(target.1), units(target.2));
    let mut put = |o: usize, v: i16| {
        if let Some(b) = out.get_mut(o..o + 2) {
            b.copy_from_slice(&v.to_be_bytes());
        }
    };
    if let Some(h) = table(b"hhea") {
        put(h + 4, a);
        put(h + 6, d);
        put(h + 8, g);
    }
    if let Some(o) = table(b"OS/2") {
        put(o + 68, a);
        put(o + 70, d);
        put(o + 72, g);
        put(o + 74, a.max(0));
        put(o + 76, d.saturating_neg().max(0));
    }
    out
}

/// Size balance of the Thai fallback fonts against Noto Sans (see `install_fonts`).
const THAI_SCALE: f32 = 1.1;

/// A Thai fallback face for a Latin font: Noto Sans Thai enlarged by `scale` and given the Latin
/// font's line metrics, so Thai letters sit on the Latin baseline.
fn thai_fallback(latin: &[u8], latin_index: u32, thai: &[u8], scale: f32) -> FontData {
    let data = match line_metrics(latin, latin_index) {
        Some(m) => with_line_metrics(thai, 0, m, scale),
        None => thai.to_vec(),
    };
    FontData::from_owned(data).tweak(egui::epaint::text::FontTweak { scale, ..Default::default() })
}

/// Body text size used across the sheets and ribbon.
pub const UI_SIZE: f32 = 12.0;

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    for (key, data) in [("noto", NOTO_SANS), ("noto_bold", NOTO_SANS_BOLD), ("noto_thai", NOTO_THAI), ("noto_thai_bold", NOTO_THAI_BOLD)] {
        let fd = if key.starts_with("noto_thai") {
            // Noto Sans Thai's letter body matches the Latin x-height, so Thai reads smaller than
            // English at the same size. Enlarge it slightly (as Office's Thai UI fonts are) so
            // mixed Thai / English labels look balanced, and sit it on the Latin baseline.
            thai_fallback(if key == "noto_thai" { NOTO_SANS } else { NOTO_SANS_BOLD }, 0, data, THAI_SCALE)
        } else {
            FontData::from_static(data)
        };
        fonts.font_data.insert(key.to_string(), Arc::new(fd));
    }
    // egui's bundled fonts stay at the end of the list as fallbacks (symbols, emoji)
    let defaults = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let mut regular = vec!["noto".to_string(), "noto_thai".to_string()];
    regular.extend(defaults.iter().cloned());
    let mut bold = vec!["noto_bold".to_string(), "noto_thai_bold".to_string()];
    bold.extend(defaults.iter().cloned());
    fonts.families.insert(FontFamily::Proportional, regular);
    fonts.families.insert(FontFamily::Name(BOLD.into()), bold);
    ctx.set_fonts(fonts);

    ctx.all_styles_mut(|s| {
        use egui::{FontId, TextStyle};
        s.text_styles.insert(TextStyle::Body, FontId::proportional(UI_SIZE));
        s.text_styles.insert(TextStyle::Button, FontId::proportional(UI_SIZE));
        s.text_styles.insert(TextStyle::Small, FontId::proportional(10.5));
        s.text_styles.insert(TextStyle::Heading, FontId::proportional(16.0));
        s.text_styles.insert(TextStyle::Monospace, FontId::monospace(12.0));
        s.visuals = egui::Visuals::light();
        s.visuals.panel_fill = Color32::WHITE;
        s.visuals.window_fill = Color32::WHITE;
        // Windows controls have a border: text boxes, buttons, combo boxes and check boxes would
        // otherwise vanish on the white dialogs (menus clear it again for their items)
        s.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(0xB4, 0xB8, 0xB4));
        s.visuals.selection.bg_fill = GREEN_LIGHT;
        s.visuals.selection.stroke = Stroke::new(1.0_f32, GREEN);
        s.visuals.widgets.hovered.weak_bg_fill = HOVER_BG;
        s.visuals.widgets.active.weak_bg_fill = PRESSED_BG;
        s.visuals.window_corner_radius = egui::CornerRadius::same(2);
        s.visuals.menu_corner_radius = egui::CornerRadius::same(2);
        s.spacing.button_padding = Vec2::new(8.0, 3.0);
        s.spacing.item_spacing = Vec2::new(6.0, 4.0);
    });
}

/// A dialog (popup card): roomy margins, rounded corners and a soft shadow, so its content does
/// not sit on the edges (egui's own modal frame is the 6 px menu frame).
pub fn modal(id: egui::Id) -> egui::Modal {
    egui::Modal::new(id).frame(dialog_frame())
}

pub fn dialog_frame() -> egui::Frame {
    egui::Frame::new()
        .inner_margin(egui::Margin { left: 22, right: 22, top: 18, bottom: 16 })
        .corner_radius(8)
        .fill(Color32::WHITE)
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0xD2, 0xD6, 0xD2)))
        .shadow(egui::epaint::Shadow { offset: [0, 6], blur: 24, spread: 0, color: Color32::from_black_alpha(46) })
}

/// Spacing inside dialogs: a little more air between rows and around button text.
pub fn dialog_spacing(ui: &mut egui::Ui) {
    let s = ui.spacing_mut();
    s.item_spacing = Vec2::new(8.0, 6.0);
    s.button_padding = Vec2::new(10.0, 4.0);
}

/// Application logo as a cached texture (64 px for chrome, 256 px for the About page).
pub fn logo_texture(ctx: &egui::Context, big: bool) -> egui::TextureHandle {
    let id = egui::Id::unique(("neramit_logo", big));
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return t;
    }
    let (bytes, size): (&[u8], usize) = if big { (include_bytes!("../assets/logo-256.rgba"), 256) } else { (include_bytes!("../assets/icon-64.rgba"), 64) };
    let img = egui::ColorImage::from_rgba_unmultiplied([size, size], bytes);
    let tex = ctx.load_texture(if big { "neramit_logo_256" } else { "neramit_logo_64" }, img, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, tex.clone()));
    tex
}

pub fn bold(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name(BOLD.into()))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    Gantt,
    Sheet,
    ResourceSheet,
    Paste,
    Cut,
    Copy,
    Painter,
    Link,
    Unlink,
    Indent,
    Outdent,
    Manual,
    Auto,
    Task,
    Summary,
    Milestone,
    Info,
    Notes,
    Timeline,
    Scroll,
    /// Task > Move: a bar over a two-way arrow
    Move,
    Find,
    Calendar,
    ProjectInfo,
    Calculate,
    AssignRes,
    AddRes,
    ZoomIn,
    ZoomOut,
    Entire,
    Selected,
    Report,
    Undo,
    Redo,
    Percent(u8),
    Inspect,
    Delete,
    Export,
    /// Help > Check for Updates: a download arrow in a circle
    Update,
    /// Help > AI Assistants: a sparkle in a circle
    Ai,
}

/// Paint a small vector icon into `r` (the icon set lives in `icons.rs`).
pub fn paint_icon(p: &Painter, r: Rect, icon: Icon, enabled: bool) {
    crate::icons::paint_icon(p, r, icon, enabled);
}

/// Paint text; a trailing " ▾" is drawn as a small vector caret (Segoe UI lacks the glyph).
pub fn text(p: &Painter, pos: Pos2, align: egui::Align2, text: &str, font: egui::FontId, color: Color32) -> Rect {
    let shaped = crate::thai::shape(text);
    let text: &str = &shaped;
    let Some(base) = text.strip_suffix(" ▾") else {
        return p.text(pos, align, text, font, color);
    };
    let galley = p.layout_no_wrap(base.to_string(), font, color);
    let caret_w = 9.0;
    let size = galley.size() + Vec2::new(caret_w, 0.0);
    let rect = align.anchor_size(pos, size);
    let text_rect = Rect::from_min_size(rect.min, galley.size());
    p.galley(text_rect.min, galley, color);
    let c = Pos2::new(text_rect.right() + 5.5, text_rect.center().y + 1.0);
    p.add(Shape::convex_polygon(vec![c + Vec2::new(-3.0, -1.5), c + Vec2::new(3.0, -1.5), c + Vec2::new(0.0, 1.8)], color, Stroke::NONE));
    rect
}

/// Icons of ribbon commands without an `Icon`, looked up by control name (see `icons.rs`).
pub fn paint_named(p: &Painter, r: Rect, name: &str, enabled: bool) {
    crate::icons::paint_named(p, r, name, enabled);
}

/// Selected application icon, shared with the window chrome and About page.
pub fn paint_title_mark(ctx: &egui::Context, p: &Painter, r: Rect) {
    let texture = logo_texture(ctx, false);
    p.image(texture.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
}

#[cfg(test)]
mod baseline_tests {
    use eframe::egui::{Color32, Context, FontFamily, FontId};

    /// Visual bottom of "H" and of Thai "ก" (both sit on the baseline) in one mixed row.
    fn bottoms(ctx: &Context, family: FontFamily, size: f32) -> (f32, f32) {
        let text = crate::thai::shape("Hก").to_string();
        let g = ctx.fonts_mut(|f| f.layout_no_wrap(text, FontId::new(size, family), Color32::BLACK));
        let bottom = |i: usize| {
            let gl = &g.rows[0].glyphs[i];
            gl.pos.y + gl.uv_rect.offset.y + gl.uv_rect.size.y
        };
        (bottom(0), bottom(1))
    }

    /// Thai and Latin letters in one row share a baseline (within the one anti-aliasing pixel a
    /// rounded letter may add), at every UI size and display scale, in the bundled fonts and in
    /// the Windows fonts offered in the Font box. Before the fix Thai sat up to 1.3 pt high.
    #[test]
    fn thai_and_latin_share_a_baseline() {
        for ppp in [1.0f32, 1.25, 1.5, 2.0] {
            let ctx = Context::default();
            super::install_fonts(&ctx);
            ctx.set_pixels_per_point(ppp);
            let _ = ctx.run_pass(Default::default(), |_| {});
            let mut families = vec![FontFamily::Proportional, FontFamily::Name(super::BOLD.into())];
            // the hand-listed fonts, and installed fonts the program knows nothing about (a .ttc
            // face, a Thai font, a Semibold family, an OpenType one ...)
            let others = ["Microsoft Sans Serif", "Georgia", "Consolas", "Cambria", "Segoe UI Semibold", "Leelawadee", "Bahnschrift", "Sitka Text", "Gabriola"];
            for name in super::SYSTEM_FONTS.iter().map(|sf| sf.name).chain(others) {
                if super::ensure_font(&ctx, name) {
                    families.push(FontFamily::Name(name.into()));
                }
            }
            let _ = ctx.run_pass(Default::default(), |_| {});
            for family in &families {
                for size in [9.0f32, 10.5, 11.0, 12.0, 12.5, 13.0, 16.0] {
                    let (h, th) = bottoms(&ctx, family.clone(), size);
                    assert!((th - h).abs() <= 1.0 / ppp + 0.01, "{family:?} {size} pt at {ppp}x: H ends at {h}, Thai at {th}");
                }
            }
        }
    }

    /// Fonts with no measured values get their Thai size from `thai_fit`: on the measured fonts
    /// it lands near the hand-measured values.
    #[test]
    fn computed_thai_fit_matches_the_measured_fonts() {
        for sf in &super::SYSTEM_FONTS {
            let Some(path) = super::font_file(sf.file) else { continue };
            let data = std::fs::read(path).unwrap();
            let (thai, scale) = super::thai_fit(&data, 0);
            println!("{:<18} thai {} / {}  scale {:.3} / {:.2}", sf.name, thai, sf.thai, scale, sf.thai_scale);
            assert_eq!(thai, sf.thai, "{}", sf.name);
            assert!((scale - sf.thai_scale).abs() <= 0.02, "{}: {scale} against {}", sf.name, sf.thai_scale);
        }
    }

    /// Every installed font loads (or is turned down) without trouble. Slow: run by hand.
    #[test]
    #[ignore]
    fn every_installed_font_loads() {
        let ctx = Context::default();
        super::install_fonts(&ctx);
        let _ = ctx.run_pass(Default::default(), |_| {});
        let mut bad = Vec::new();
        for name in super::available_fonts().iter().skip(1) {
            if !super::ensure_font(&ctx, name) {
                bad.push(name.clone());
                continue;
            }
            let _ = ctx.run_pass(Default::default(), |_| {});
            // the baseline each face is drawn on (letter bottoms mislead in script fonts whose H
            // has a swash below the line)
            let text = crate::thai::shape("Hก").to_string();
            let g = ctx.fonts_mut(|f| f.layout_no_wrap(text, FontId::new(12.0, FontFamily::Name(name.as_str().into())), Color32::BLACK));
            let (h, th) = (g.rows[0].glyphs[0].pos.y, g.rows[0].glyphs[1].pos.y);
            if (th - h).abs() > 0.51 {
                bad.push(format!("{name}: H baseline {h} Thai {th}"));
            }
        }
        println!("{} fonts, not usable: {bad:#?}", super::available_fonts().len());
    }
}
