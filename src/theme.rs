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
pub const BAR_FILL: Color32 = Color32::from_rgb(0x8C, 0xB4, 0xE6);
pub const SUMMARY: Color32 = Color32::from_rgb(0x40, 0x40, 0x40);
pub const LINK: Color32 = Color32::from_rgb(0x2F, 0x5F, 0x9E);
pub const TODAY: Color32 = Color32::from_rgb(0x8B, 0xBF, 0x8A);
pub const TIMELINE_BG: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
pub const TIMELINE_BAR: Color32 = Color32::from_rgb(0xDE, 0xEA, 0xF6);
/// Modern chrome
pub const TAB_ROW: Color32 = Color32::from_rgb(0xF0, 0xF0, 0xF0);
pub const ACCENT: Color32 = Color32::from_rgb(0x0F, 0x70, 0x3B);
pub const STATUS_BG: Color32 = Color32::from_rgb(0xF5, 0xF5, 0xF5);
pub const NAV_BG: Color32 = Color32::from_rgb(0xF0, 0xF0, 0xF0);
pub const BACKSTAGE_NAV: Color32 = Color32::from_rgb(0xE4, 0xE4, 0xEA);
pub const CONTENT_BG: Color32 = Color32::from_rgb(0xF5, 0xF5, 0xF5);
pub const TIMELINE_TASK: Color32 = Color32::from_rgb(0x9D, 0xC3, 0xE6);
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
/// the vertical view-name bar: the standard horizontal gradient (25 px) and green label
pub const SHEET_VIEW_BAR: [u8; 25] = [
    0xD1, 0xD3, 0xD5, 0xD7, 0xD8, 0xDA, 0xDB, 0xDC, 0xDF, 0xE0, 0xE0, 0xE1, 0xE1, 0xE1, 0xE2, 0xE2, 0xE3, 0xE3, 0xE3, 0xE3, 0xE3, 0xE4, 0xE4, 0xE4, 0xE4,
];
pub const SHEET_VIEW_LABEL: Color32 = Color32::from_rgb(0x2F, 0x6F, 0x2D);
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

/// Paint a small vector icon into `r`.
pub fn paint_icon(p: &Painter, r: Rect, icon: Icon, enabled: bool) {
    let col = |c: Color32| if enabled { c } else { DISABLED };
    let blue = col(Color32::from_rgb(0x2B, 0x6C, 0xB5));
    let lblue = col(BAR_FILL);
    let green = col(GREEN);
    let dark = col(Color32::from_rgb(0x44, 0x44, 0x44));
    let red = col(Color32::from_rgb(0xC0, 0x39, 0x2B));
    let yellow = col(Color32::from_rgb(0xF2, 0xC2, 0x3A));
    let w = r.width();
    let h = r.height();
    let at = |fx: f32, fy: f32| Pos2::new(r.left() + fx * w, r.top() + fy * h);
    let rr = |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_min_max(at(x0, y0), at(x1, y1));
    let s1 = Stroke::new((w / 16.0).max(1.0), dark);
    match icon {
        Icon::Gantt => {
            p.rect_stroke(rr(0.05, 0.1, 0.95, 0.9), 1.0, Stroke::new(1.0_f32, dark), egui::StrokeKind::Inside);
            p.rect_filled(rr(0.15, 0.22, 0.55, 0.38), 0.0, blue);
            p.rect_filled(rr(0.35, 0.45, 0.75, 0.6), 0.0, blue);
            p.rect_filled(rr(0.5, 0.68, 0.88, 0.82), 0.0, blue);
        }
        Icon::Sheet | Icon::ResourceSheet => {
            p.rect_filled(rr(0.08, 0.1, 0.92, 0.9), 0.0, Color32::WHITE);
            p.rect_stroke(rr(0.08, 0.1, 0.92, 0.9), 0.0, Stroke::new(1.0_f32, dark), egui::StrokeKind::Inside);
            p.rect_filled(rr(0.08, 0.1, 0.92, 0.28), 0.0, if icon == Icon::Sheet { lblue } else { col(GREEN_LIGHT) });
            for fy in [0.46, 0.64] {
                p.line_segment([at(0.08, fy), at(0.92, fy)], Stroke::new(1.0_f32, dark));
            }
            p.line_segment([at(0.35, 0.1), at(0.35, 0.9)], Stroke::new(1.0_f32, dark));
            if icon == Icon::ResourceSheet {
                p.circle_filled(at(0.72, 0.62), w * 0.14, green);
            }
        }
        Icon::Paste => {
            p.rect_filled(rr(0.12, 0.15, 0.72, 0.9), 2.0, col(Color32::from_rgb(0xC8, 0x9B, 0x5E)));
            p.rect_filled(rr(0.3, 0.08, 0.55, 0.22), 1.0, dark);
            p.rect_filled(rr(0.42, 0.4, 0.92, 0.95), 0.0, Color32::WHITE);
            p.rect_stroke(rr(0.42, 0.4, 0.92, 0.95), 0.0, Stroke::new(1.0_f32, dark), egui::StrokeKind::Inside);
        }
        Icon::Cut => {
            p.circle_stroke(at(0.3, 0.75), w * 0.14, Stroke::new(1.5_f32, blue));
            p.circle_stroke(at(0.7, 0.75), w * 0.14, Stroke::new(1.5_f32, blue));
            p.line_segment([at(0.38, 0.65), at(0.75, 0.1)], s1);
            p.line_segment([at(0.62, 0.65), at(0.25, 0.1)], s1);
        }
        Icon::Copy => {
            p.rect_stroke(rr(0.1, 0.1, 0.6, 0.7), 0.0, Stroke::new(1.0_f32, dark), egui::StrokeKind::Inside);
            p.rect_filled(rr(0.38, 0.3, 0.9, 0.92), 0.0, Color32::WHITE);
            p.rect_stroke(rr(0.38, 0.3, 0.9, 0.92), 0.0, Stroke::new(1.0_f32, blue), egui::StrokeKind::Inside);
        }
        Icon::Painter => {
            p.rect_filled(rr(0.1, 0.1, 0.8, 0.38), 1.0, yellow);
            p.line_segment([at(0.45, 0.38), at(0.45, 0.9)], Stroke::new(w * 0.14, dark));
        }
        Icon::Link | Icon::Unlink => {
            p.rect_filled(rr(0.05, 0.12, 0.45, 0.32), 0.0, blue);
            p.rect_filled(rr(0.55, 0.68, 0.95, 0.88), 0.0, blue);
            let pts = vec![at(0.45, 0.22), at(0.72, 0.22), at(0.72, 0.62)];
            p.add(Shape::line(pts, Stroke::new(1.5_f32, dark)));
            p.add(Shape::convex_polygon(vec![at(0.62, 0.55), at(0.82, 0.55), at(0.72, 0.68)], dark, Stroke::NONE));
            if icon == Icon::Unlink {
                p.line_segment([at(0.55, 0.3), at(0.9, 0.5)], Stroke::new(2.0_f32, red));
                p.line_segment([at(0.9, 0.3), at(0.55, 0.5)], Stroke::new(2.0_f32, red));
            }
        }
        Icon::Indent | Icon::Outdent => {
            for (i, fy) in [0.15, 0.4, 0.65, 0.9].iter().enumerate() {
                let x0 = if i == 1 || i == 2 { 0.45 } else { 0.1 };
                p.line_segment([at(x0, *fy), at(0.95, *fy)], Stroke::new(1.5_f32, dark));
            }
            let (a, b) = if icon == Icon::Indent { (0.08, 0.32) } else { (0.32, 0.08) };
            p.add(Shape::convex_polygon(vec![at(b, 0.52), at(a, 0.38), at(a, 0.66)], green, Stroke::NONE));
        }
        Icon::Manual => {
            // dates set by hand: a bar with a padlock
            p.rect_filled(rr(0.05, 0.5, 0.72, 0.8), 1.0, lblue);
            p.rect_stroke(rr(0.05, 0.5, 0.72, 0.8), 1.0, Stroke::new(1.0_f32, blue), egui::StrokeKind::Inside);
            p.circle_stroke(at(0.77, 0.27), w * 0.11, Stroke::new((w / 14.0).max(1.2), dark));
            p.rect_filled(rr(0.58, 0.27, 0.96, 0.55), 1.5, red);
        }
        Icon::Auto => {
            // dates worked out from the links: a bar with a turning arrow
            p.rect_filled(rr(0.05, 0.5, 0.72, 0.8), 1.0, blue);
            let c = at(0.74, 0.3);
            let rad = w * 0.18;
            let arc: Vec<Pos2> = (0..=16).map(|k| {
                let a = std::f32::consts::PI * (0.15 + 1.45 * k as f32 / 16.0);
                c + Vec2::new(a.cos() * rad, -a.sin() * rad)
            }).collect();
            let end = arc[arc.len() - 1];
            p.add(Shape::line(arc, Stroke::new((w / 12.0).max(1.4), green)));
            let t = (w * 0.13).max(3.0);
            p.add(Shape::convex_polygon(vec![end + Vec2::new(-t, -t * 0.2), end + Vec2::new(t, -t * 0.2), end + Vec2::new(0.0, t * 0.9)], green, Stroke::NONE));
        }
        Icon::Task => {
            p.rect_filled(rr(0.05, 0.3, 0.95, 0.7), 1.0, lblue);
            p.rect_stroke(rr(0.05, 0.3, 0.95, 0.7), 1.0, Stroke::new(1.0_f32, blue), egui::StrokeKind::Inside);
        }
        Icon::Summary => {
            p.rect_filled(rr(0.05, 0.25, 0.95, 0.4), 0.0, dark);
            p.add(Shape::convex_polygon(vec![at(0.05, 0.4), at(0.2, 0.4), at(0.05, 0.6)], dark, Stroke::NONE));
            p.add(Shape::convex_polygon(vec![at(0.8, 0.4), at(0.95, 0.4), at(0.95, 0.6)], dark, Stroke::NONE));
            p.rect_filled(rr(0.25, 0.68, 0.75, 0.85), 0.0, lblue);
        }
        Icon::Milestone => {
            let c = r.center();
            let d = w * 0.35;
            p.add(Shape::convex_polygon(
                vec![c + Vec2::new(0.0, -d), c + Vec2::new(d, 0.0), c + Vec2::new(0.0, d), c + Vec2::new(-d, 0.0)],
                dark,
                Stroke::NONE,
            ));
        }
        Icon::Info => {
            p.circle_filled(r.center(), w * 0.42, blue);
            p.text(r.center(), egui::Align2::CENTER_CENTER, "i", bold(h * 0.6), Color32::WHITE);
        }
        Icon::Notes => {
            p.rect_filled(rr(0.15, 0.08, 0.85, 0.92), 0.0, yellow);
            for fy in [0.3, 0.5, 0.7] {
                p.line_segment([at(0.25, fy), at(0.75, fy)], Stroke::new(1.0_f32, dark));
            }
        }
        Icon::Timeline => {
            // a ruler with ticks, a bar and a diamond above it
            p.line_segment([at(0.05, 0.78), at(0.95, 0.78)], Stroke::new(1.2_f32, dark));
            for fx in [0.05, 0.35, 0.65, 0.95] {
                p.line_segment([at(fx, 0.68), at(fx, 0.78)], Stroke::new(1.0_f32, dark));
            }
            p.rect_filled(rr(0.08, 0.3, 0.62, 0.52), 2.0, blue);
            let c = at(0.8, 0.41);
            let d = w * 0.12;
            p.add(Shape::convex_polygon(vec![c + Vec2::new(0.0, -d), c + Vec2::new(d, 0.0), c + Vec2::new(0.0, d), c + Vec2::new(-d, 0.0)], green, Stroke::NONE));
        }
        Icon::Scroll => {
            // a target on the bar's start
            p.rect_filled(rr(0.3, 0.38, 0.95, 0.62), 1.0, lblue);
            p.circle_stroke(at(0.27, 0.5), w * 0.2, Stroke::new((w / 14.0).max(1.2), dark));
            p.circle_filled(at(0.27, 0.5), w * 0.07, green);
        }
        Icon::Move => {
            p.rect_filled(rr(0.2, 0.18, 0.8, 0.44), 1.0, lblue);
            p.rect_stroke(rr(0.2, 0.18, 0.8, 0.44), 1.0, Stroke::new(1.0_f32, blue), egui::StrokeKind::Inside);
            let s = Stroke::new((w / 12.0).max(1.4), green);
            p.line_segment([at(0.14, 0.72), at(0.86, 0.72)], s);
            let t = (w * 0.12).max(3.0);
            for (tip, dir) in [(at(0.05, 0.72), 1.0f32), (at(0.95, 0.72), -1.0)] {
                p.add(Shape::convex_polygon(vec![tip, tip + Vec2::new(dir * t * 1.2, -t), tip + Vec2::new(dir * t * 1.2, t)], green, Stroke::NONE));
            }
        }
        Icon::Find => {
            p.circle_stroke(at(0.4, 0.4), w * 0.27, Stroke::new(2.0_f32, dark));
            p.line_segment([at(0.6, 0.6), at(0.92, 0.92)], Stroke::new(2.5_f32, dark));
        }
        Icon::Calendar | Icon::ProjectInfo => {
            p.rect_filled(rr(0.08, 0.12, 0.92, 0.92), 1.0, Color32::WHITE);
            p.rect_stroke(rr(0.08, 0.12, 0.92, 0.92), 1.0, Stroke::new(1.0_f32, dark), egui::StrokeKind::Inside);
            p.rect_filled(rr(0.08, 0.12, 0.92, 0.32), 0.0, if icon == Icon::Calendar { red } else { blue });
            for ix in 0..3 {
                for iy in 0..2 {
                    let x = 0.2 + ix as f32 * 0.22;
                    let y = 0.45 + iy as f32 * 0.22;
                    p.rect_filled(rr(x, y, x + 0.14, y + 0.12), 0.0, dark);
                }
            }
        }
        Icon::Calculate => {
            p.rect_filled(rr(0.15, 0.05, 0.85, 0.95), 2.0, dark);
            p.rect_filled(rr(0.25, 0.12, 0.75, 0.32), 0.0, col(GREEN_LIGHT));
            for ix in 0..3 {
                for iy in 0..3 {
                    let x = 0.25 + ix as f32 * 0.18;
                    let y = 0.42 + iy as f32 * 0.17;
                    p.rect_filled(rr(x, y, x + 0.12, y + 0.11), 0.0, Color32::WHITE);
                }
            }
        }
        Icon::AssignRes | Icon::AddRes => {
            p.circle_filled(at(0.4, 0.3), w * 0.17, blue);
            p.add(Shape::convex_polygon(vec![at(0.12, 0.9), at(0.2, 0.55), at(0.6, 0.55), at(0.68, 0.9)], blue, Stroke::NONE));
            if icon == Icon::AddRes {
                p.line_segment([at(0.82, 0.45), at(0.82, 0.85)], Stroke::new(2.5_f32, green));
                p.line_segment([at(0.62, 0.65), at(1.0, 0.65)], Stroke::new(2.5_f32, green));
            } else {
                p.rect_filled(rr(0.65, 0.6, 1.0, 0.75), 0.0, lblue);
            }
        }
        Icon::ZoomIn | Icon::ZoomOut => {
            p.circle_stroke(at(0.4, 0.4), w * 0.28, Stroke::new(2.0_f32, dark));
            p.line_segment([at(0.6, 0.6), at(0.92, 0.92)], Stroke::new(2.5_f32, dark));
            p.line_segment([at(0.27, 0.4), at(0.53, 0.4)], Stroke::new(1.8_f32, blue));
            if icon == Icon::ZoomIn {
                p.line_segment([at(0.4, 0.27), at(0.4, 0.53)], Stroke::new(1.8_f32, blue));
            }
        }
        Icon::Entire | Icon::Selected => {
            p.rect_stroke(rr(0.05, 0.15, 0.95, 0.85), 0.0, Stroke::new(1.0_f32, dark), egui::StrokeKind::Inside);
            p.rect_filled(rr(0.15, 0.3, 0.5, 0.45), 0.0, blue);
            p.rect_filled(rr(0.45, 0.55, 0.85, 0.7), 0.0, if icon == Icon::Selected { green } else { blue });
        }
        Icon::Report => {
            p.rect_filled(rr(0.12, 0.05, 0.88, 0.95), 0.0, Color32::WHITE);
            p.rect_stroke(rr(0.12, 0.05, 0.88, 0.95), 0.0, Stroke::new(1.0_f32, dark), egui::StrokeKind::Inside);
            for (i, hh) in [0.3, 0.5, 0.4].iter().enumerate() {
                let x = 0.25 + i as f32 * 0.2;
                p.rect_filled(rr(x, 0.85 - hh, x + 0.13, 0.85), 0.0, [blue, green, yellow][i]);
            }
        }
        Icon::Undo | Icon::Redo => {
            let (a, b) = if icon == Icon::Undo { (0.2, 0.8) } else { (0.8, 0.2) };
            p.add(Shape::line(vec![at(a, 0.4), at(b, 0.4), at(b, 0.8), at(0.5, 0.8)], Stroke::new(1.6_f32, col(Color32::WHITE))));
            let tip = if icon == Icon::Undo { 0.05 } else { 0.95 };
            p.add(Shape::convex_polygon(vec![at(tip, 0.4), at(a, 0.2), at(a, 0.6)], col(Color32::WHITE), Stroke::NONE));
        }
        Icon::Percent(v) => {
            p.circle_stroke(r.center(), w * 0.42, Stroke::new(1.0_f32, dark));
            let frac = v as f32 / 100.0;
            if frac > 0.0 {
                let c = r.center();
                let rad = w * 0.42;
                let mut pts = vec![c];
                let steps = 24;
                for i in 0..=steps {
                    let a = -std::f32::consts::FRAC_PI_2 + frac * std::f32::consts::TAU * i as f32 / steps as f32;
                    pts.push(c + Vec2::new(a.cos() * rad, a.sin() * rad));
                }
                p.add(Shape::Path(egui::epaint::PathShape { points: pts, closed: true, fill: blue, stroke: egui::epaint::PathStroke::NONE }));
            }
        }
        Icon::Inspect => {
            // Trace: a trail of dots leading into the bar
            for (k, fx) in [0.08f32, 0.22, 0.37].iter().enumerate() {
                p.circle_filled(at(*fx, 0.5), w * (0.04 + 0.018 * k as f32), green);
            }
            p.rect_filled(rr(0.5, 0.36, 0.95, 0.64), 1.0, lblue);
            p.rect_filled(rr(0.5, 0.36, 0.56, 0.64), 0.0, green);
        }
        Icon::Delete => {
            p.line_segment([at(0.2, 0.2), at(0.8, 0.8)], Stroke::new(2.5_f32, red));
            p.line_segment([at(0.8, 0.2), at(0.2, 0.8)], Stroke::new(2.5_f32, red));
        }
        Icon::Export => {
            p.rect_filled(rr(0.1, 0.1, 0.65, 0.9), 0.0, Color32::WHITE);
            p.rect_stroke(rr(0.1, 0.1, 0.65, 0.9), 0.0, Stroke::new(1.0_f32, dark), egui::StrokeKind::Inside);
            p.line_segment([at(0.45, 0.5), at(0.95, 0.5)], Stroke::new(2.0_f32, green));
            p.add(Shape::convex_polygon(vec![at(1.0, 0.5), at(0.8, 0.35), at(0.8, 0.65)], green, Stroke::NONE));
        }
        Icon::Update => {
            p.circle_filled(at(0.5, 0.5), w * 0.44, green);
            p.line_segment([at(0.5, 0.2), at(0.5, 0.62)], Stroke::new((w / 9.0).max(1.5), Color32::WHITE));
            p.add(Shape::convex_polygon(vec![at(0.5, 0.8), at(0.3, 0.56), at(0.7, 0.56)], Color32::WHITE, Stroke::NONE));
        }
        Icon::Ai => {
            p.circle_filled(at(0.5, 0.5), w * 0.44, blue);
            // a four-pointed sparkle, and a small one at its top right
            let star = |cx: f32, cy: f32, r: f32| {
                let k = r * 0.28;
                let pts = vec![at(cx, cy - r), at(cx + k, cy - k), at(cx + r, cy), at(cx + k, cy + k), at(cx, cy + r), at(cx - k, cy + k), at(cx - r, cy), at(cx - k, cy - k)];
                for i in 0..4 {
                    p.add(Shape::convex_polygon(vec![at(cx, cy), pts[i * 2], pts[(i * 2 + 1) % 8], pts[(i * 2 + 2) % 8]], Color32::WHITE, Stroke::NONE));
                }
            };
            star(0.45, 0.55, 0.26);
            star(0.7, 0.3, 0.11);
        }
    }
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

/// Fluent-style outline icons for the modern ribbon, looked up by control name.
/// Thin dark outlines with a single blue / green accent.
pub fn paint_named(p: &Painter, r: Rect, name: &str, enabled: bool) {
    let col = |c: Color32| if enabled { c } else { DISABLED };
    let ink = col(Color32::from_rgb(0x42, 0x42, 0x42));
    let blue = col(Color32::from_rgb(0x2B, 0x6C, 0xB5));
    let lblue = col(Color32::from_rgb(0x9C, 0xC3, 0xEA));
    let green = col(GREEN);
    let red = col(Color32::from_rgb(0xC4, 0x31, 0x4B));
    let orange = col(Color32::from_rgb(0xE8, 0x9A, 0x3C));
    let yellow = col(Color32::from_rgb(0xF2, 0xC2, 0x3A));
    let w = r.width();
    let h = r.height();
    let at = |fx: f32, fy: f32| Pos2::new(r.left() + fx * w, r.top() + fy * h);
    let rr = |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_min_max(at(x0, y0), at(x1, y1));
    let s = Stroke::new((w / 22.0).max(1.0), ink);
    let page = |p: &Painter| {
        p.rect_filled(rr(0.18, 0.06, 0.82, 0.94), 1.0, Color32::WHITE);
        p.rect_stroke(rr(0.18, 0.06, 0.82, 0.94), 1.0, s, egui::StrokeKind::Inside);
    };
    let grid = |p: &Painter, accent: Color32| {
        p.rect_filled(rr(0.08, 0.14, 0.92, 0.86), 1.0, Color32::WHITE);
        p.rect_stroke(rr(0.08, 0.14, 0.92, 0.86), 1.0, s, egui::StrokeKind::Inside);
        p.rect_filled(rr(0.08, 0.14, 0.92, 0.32), 0.0, accent);
        for fy in [0.5, 0.68] {
            p.line_segment([at(0.08, fy), at(0.92, fy)], s);
        }
        p.line_segment([at(0.36, 0.14), at(0.36, 0.86)], s);
    };
    match name {
        "Task Usage" | "Resource Usage" => {
            grid(p, lblue);
            p.rect_filled(rr(0.45, 0.38, 0.85, 0.46), 0.0, blue);
        }
        "Task Board" | "Task Boards" => {
            for (i, x) in [0.08f32, 0.38, 0.68].iter().enumerate() {
                p.rect_stroke(rr(*x, 0.12, x + 0.26, 0.88), 1.0, s, egui::StrokeKind::Inside);
                p.rect_filled(rr(x + 0.04, 0.2 + i as f32 * 0.1, x + 0.22, 0.34 + i as f32 * 0.1), 0.0, [blue, green, lblue][i]);
            }
        }
        "Network Diagram" => {
            for (x, y) in [(0.06f32, 0.12f32), (0.06, 0.62), (0.58, 0.37)] {
                p.rect_filled(rr(x, y, x + 0.36, y + 0.26), 1.0, lblue);
                p.rect_stroke(rr(x, y, x + 0.36, y + 0.26), 1.0, Stroke::new(1.0_f32, blue), egui::StrokeKind::Inside);
            }
            p.line_segment([at(0.42, 0.25), at(0.58, 0.5)], s);
            p.line_segment([at(0.42, 0.75), at(0.58, 0.5)], s);
        }
        "Calendar" => {
            p.rect_stroke(rr(0.08, 0.14, 0.92, 0.9), 1.0, s, egui::StrokeKind::Inside);
            p.rect_filled(rr(0.08, 0.14, 0.92, 0.32), 0.0, green);
            for ix in 0..3 {
                for iy in 0..2 {
                    let x = 0.2 + ix as f32 * 0.22;
                    let y = 0.44 + iy as f32 * 0.22;
                    p.rect_filled(rr(x, y, x + 0.12, y + 0.12), 0.0, ink);
                }
            }
        }
        "Other Views" | "Tables" | "Column Settings" => grid(p, lblue),
        "Custom Fields" => grid(p, col(GREEN_LIGHT)),
        "Sort" => {
            p.text(at(0.3, 0.3), egui::Align2::CENTER_CENTER, "A", egui::FontId::proportional(h * 0.36), blue);
            p.text(at(0.3, 0.72), egui::Align2::CENTER_CENTER, "Z", egui::FontId::proportional(h * 0.36), blue);
            p.line_segment([at(0.7, 0.12), at(0.7, 0.88)], s);
            p.line_segment([at(0.7, 0.88), at(0.55, 0.72)], s);
            p.line_segment([at(0.7, 0.88), at(0.85, 0.72)], s);
        }
        "Outline" => {
            for (i, fy) in [0.2f32, 0.42, 0.64, 0.86].iter().enumerate() {
                let x0 = if i == 1 || i == 2 { 0.36 } else { 0.1 };
                p.rect_filled(rr(x0, fy - 0.06, x0 + 0.08, fy + 0.02), 0.0, blue);
                p.line_segment([at(x0 + 0.14, fy - 0.02), at(0.92, fy - 0.02)], s);
            }
        }
        "Highlight" => {
            p.add(Shape::convex_polygon(vec![at(0.2, 0.7), at(0.62, 0.12), at(0.86, 0.3), at(0.44, 0.88)], yellow, s));
        }
        "Filter" => {
            p.add(Shape::line(vec![at(0.1, 0.15), at(0.9, 0.15), at(0.58, 0.52), at(0.58, 0.88), at(0.42, 0.78), at(0.42, 0.52), at(0.1, 0.15)], s));
        }
        "Group by" => {
            for y in [0.12f32, 0.42, 0.72] {
                p.rect_filled(rr(0.08, y, 0.28, y + 0.18), 0.0, blue);
                p.line_segment([at(0.36, y + 0.09), at(0.92, y + 0.09)], s);
            }
        }
        "Zoom" | "Spelling..." => {
            p.circle_stroke(at(0.42, 0.42), w * 0.28, s);
            p.line_segment([at(0.62, 0.62), at(0.92, 0.92)], Stroke::new(s.width * 1.8, ink));
            if name == "Spelling..." {
                p.add(Shape::line(vec![at(0.28, 0.42), at(0.38, 0.54), at(0.58, 0.3)], Stroke::new(s.width * 1.4, green)));
            }
        }
        "New Window" | "Arrange All" | "Switch Windows" | "Hide" => {
            p.rect_stroke(rr(0.08, 0.2, 0.92, 0.86), 1.0, s, egui::StrokeKind::Inside);
            p.rect_filled(rr(0.08, 0.2, 0.92, 0.34), 0.0, lblue);
            if name == "New Window" {
                p.line_segment([at(0.78, 0.0), at(0.78, 0.3)], Stroke::new(2.0_f32, green));
                p.line_segment([at(0.63, 0.15), at(0.93, 0.15)], Stroke::new(2.0_f32, green));
            }
        }
        "Macros" | "View Macros" => {
            page(p);
            for fy in [0.3f32, 0.5, 0.7] {
                p.line_segment([at(0.3, fy), at(0.7, fy)], s);
            }
            p.circle_filled(at(0.74, 0.76), w * 0.12, red);
        }
        "Language" => {
            let c = r.center();
            let rad = w * 0.4;
            p.circle_stroke(c, rad, Stroke::new(1.4_f32, blue));
            p.add(Shape::ellipse_stroke(c, Vec2::new(rad * 0.45, rad), Stroke::new(1.2_f32, blue)));
            p.line_segment([c + Vec2::new(-rad, 0.0), c + Vec2::new(rad, 0.0)], Stroke::new(1.2_f32, blue));
            p.line_segment([c + Vec2::new(-rad * 0.85, -rad * 0.5), c + Vec2::new(rad * 0.85, -rad * 0.5)], Stroke::new(1.0_f32, blue));
            p.line_segment([c + Vec2::new(-rad * 0.85, rad * 0.5), c + Vec2::new(rad * 0.85, rad * 0.5)], Stroke::new(1.0_f32, blue));
        }
        "Help" | "Feedback" | "Show Training" => {
            p.circle_stroke(r.center(), w * 0.4, s);
            let mark = match name {
                "Help" => "?",
                "Feedback" => "!",
                _ => "i",
            };
            p.text(r.center(), egui::Align2::CENTER_CENTER, mark, bold(h * 0.5), blue);
        }
        "Text Styles..." | "Font..." => {
            p.text(at(0.38, 0.55), egui::Align2::CENTER_CENTER, "A", bold(h * 0.62), ink);
            p.rect_filled(rr(0.62, 0.62, 0.92, 0.9), 0.0, blue);
        }
        "Gridlines" | "Layout" => {
            p.rect_stroke(rr(0.08, 0.1, 0.92, 0.9), 1.0, s, egui::StrokeKind::Inside);
            for fy in [0.36f32, 0.63] {
                p.line_segment([at(0.08, fy), at(0.92, fy)], Stroke::new(1.0_f32, blue));
            }
            if name == "Layout" {
                p.rect_filled(rr(0.25, 0.18, 0.7, 0.3), 0.0, blue);
                p.rect_filled(rr(0.4, 0.71, 0.85, 0.83), 0.0, blue);
            }
        }
        "Align Left" | "Center" | "Align Right" | "Wrap Text" => {
            for (i, fy) in [0.2f32, 0.4, 0.6, 0.8].iter().enumerate() {
                let len = if i % 2 == 0 { 0.8 } else { 0.5 };
                let x0 = match name {
                    "Align Left" | "Wrap Text" => 0.1,
                    "Center" => 0.5 - len / 2.0,
                    _ => 0.9 - len,
                };
                p.line_segment([at(x0, *fy), at(x0 + len, *fy)], s);
            }
        }
        "Format" | "Bar Styles" | "Gantt Chart Gallery" => {
            p.rect_filled(rr(0.06, 0.2, 0.62, 0.4), 0.0, lblue);
            p.rect_stroke(rr(0.06, 0.2, 0.62, 0.4), 0.0, Stroke::new(1.0_f32, blue), egui::StrokeKind::Inside);
            p.rect_filled(rr(0.36, 0.6, 0.94, 0.8), 0.0, blue);
        }
        "Task Path" | "Baseline" | "Slippage" => {
            p.rect_filled(rr(0.06, 0.18, 0.7, 0.36), 0.0, if name == "Task Path" { orange } else { blue });
            p.rect_filled(rr(0.3, 0.52, 0.94, 0.7), 0.0, if name == "Baseline" { col(Color32::from_rgb(0x9E, 0x9E, 0x9E)) } else { lblue });
            if name == "Slippage" {
                p.line_segment([at(0.7, 0.36), at(0.94, 0.52)], Stroke::new(1.5_f32, red));
            }
        }
        "Drawing" => {
            p.add(Shape::convex_polygon(vec![at(0.15, 0.85), at(0.25, 0.6), at(0.72, 0.12), at(0.88, 0.28), at(0.4, 0.76)], yellow, s));
        }
        "Subproject" | "Links Between Projects..." | "Compare Projects" | "Move Project" => {
            page(p);
            p.rect_filled(rr(0.3, 0.3, 0.7, 0.42), 0.0, blue);
            p.rect_filled(rr(0.4, 0.56, 0.72, 0.68), 0.0, if name == "Compare Projects" { green } else { blue });
        }
        "Get Add-ins" | "My Add-ins" => {
            p.rect_stroke(rr(0.12, 0.3, 0.88, 0.9), 2.0, s, egui::StrokeKind::Inside);
            p.add(Shape::line(vec![at(0.32, 0.3), at(0.32, 0.12), at(0.68, 0.12), at(0.68, 0.3)], s));
            p.circle_filled(at(0.5, 0.6), w * 0.1, green);
        }
        "Manage Sprints" => {
            p.add(Shape::line(vec![at(0.82, 0.5), at(0.7, 0.2), at(0.35, 0.15), at(0.15, 0.45), at(0.3, 0.8), at(0.7, 0.85)], Stroke::new(s.width * 1.5, blue)));
            p.add(Shape::convex_polygon(vec![at(0.7, 0.72), at(0.86, 0.85), at(0.68, 0.98)], blue, Stroke::NONE));
        }
        "WBS" => {
            p.rect_filled(rr(0.35, 0.08, 0.65, 0.28), 0.0, blue);
            for x in [0.06f32, 0.38, 0.7] {
                p.rect_filled(rr(x, 0.66, x + 0.24, 0.86), 0.0, lblue);
                p.line_segment([at(x + 0.12, 0.66), at(0.5, 0.28)], s);
            }
        }
        "Set Baseline" => {
            p.rect_filled(rr(0.06, 0.2, 0.7, 0.38), 0.0, blue);
            p.rect_filled(rr(0.06, 0.48, 0.7, 0.6), 0.0, col(Color32::from_rgb(0x9E, 0x9E, 0x9E)));
            p.line_segment([at(0.06, 0.8), at(0.94, 0.8)], s);
        }
        "Update Project..." => {
            page(p);
            p.add(Shape::line(vec![at(0.3, 0.55), at(0.45, 0.7), at(0.72, 0.36)], Stroke::new(s.width * 1.6, green)));
        }
        "New Report" | "Dashboards" | "Resources" | "Costs" | "In Progress" | "Getting Started" | "Custom" | "Recent" | "Export Data" => {
            page(p);
            let accent = match name {
                "Costs" => orange,
                "Resources" => green,
                "In Progress" => red,
                _ => blue,
            };
            for (i, hh) in [0.22f32, 0.38, 0.3].iter().enumerate() {
                let x = 0.28 + i as f32 * 0.16;
                p.rect_filled(rr(x, 0.82 - hh, x + 0.11, 0.82), 0.0, accent);
            }
        }
        "Resource Pool" => {
            p.circle_filled(at(0.32, 0.32), w * 0.13, blue);
            p.circle_filled(at(0.68, 0.32), w * 0.13, green);
            p.rect_filled(rr(0.1, 0.55, 0.9, 0.85), 4.0, lblue);
        }
        "Level Selection" | "Level Resource" | "Level All" | "Leveling Options" | "Clear Leveling" | "Next Overallocation" => {
            for (i, hh) in [0.45f32, 0.75, 0.35, 0.6].iter().enumerate() {
                let x = 0.08 + i as f32 * 0.22;
                p.rect_filled(rr(x, 0.92 - hh, x + 0.16, 0.92), 0.0, if *hh > 0.7 { red } else { blue });
            }
            p.line_segment([at(0.04, 0.42), at(0.96, 0.42)], Stroke::new(1.5_f32, green));
        }
        "Split Task" => {
            p.rect_filled(rr(0.04, 0.38, 0.4, 0.62), 0.0, blue);
            p.rect_filled(rr(0.6, 0.38, 0.96, 0.62), 0.0, blue);
            for x in [0.44f32, 0.5, 0.56] {
                p.circle_filled(at(x, 0.5), 1.0, ink);
            }
        }
        "Background Color" | "Font Color" => {
            let label = if name == "Font Color" { "A" } else { "ab" };
            p.text(at(0.5, 0.42), egui::Align2::CENTER_CENTER, label, bold(h * 0.48), ink);
            let bar = if name == "Font Color" { col(Color32::from_rgb(0xED, 0x1C, 0x24)) } else { col(Color32::from_rgb(0xFF, 0xF2, 0x00)) };
            p.rect_filled(rr(0.12, 0.78, 0.88, 0.95), 0.0, bar);
        }
        "Details" | "Notes..." => {
            page(p);
            for fy in [0.3f32, 0.48, 0.66] {
                p.line_segment([at(0.3, fy), at(0.7, fy)], s);
            }
        }
        "Import BOQ" => {
            // a green spreadsheet with an arrow into the plan's bars
            p.rect_filled(rr(0.06, 0.08, 0.6, 0.78), 1.0, Color32::WHITE);
            p.rect_filled(rr(0.06, 0.08, 0.6, 0.24), 0.0, green);
            for fy in [0.4f32, 0.56] {
                p.line_segment([at(0.06, fy), at(0.6, fy)], s);
            }
            p.line_segment([at(0.26, 0.24), at(0.26, 0.78)], s);
            p.rect_stroke(rr(0.06, 0.08, 0.6, 0.78), 1.0, s, egui::StrokeKind::Inside);
            p.rect_filled(rr(0.66, 0.5, 0.94, 0.6), 0.0, blue);
            p.rect_filled(rr(0.74, 0.7, 0.96, 0.8), 0.0, blue);
            let a = Stroke::new((w / 12.0).max(1.5), blue);
            p.line_segment([at(0.44, 0.92), at(0.7, 0.92)], a);
            p.line_segment([at(0.6, 0.84), at(0.7, 0.92)], a);
        }
        "Payment Schedule" => {
            // a banknote with the baht sign and a calendar edge
            p.rect_filled(rr(0.06, 0.24, 0.94, 0.76), 2.0, col(Color32::from_rgb(0xE3, 0xF1, 0xE6)));
            p.rect_stroke(rr(0.06, 0.24, 0.94, 0.76), 2.0, Stroke::new((w / 22.0).max(1.0), green), egui::StrokeKind::Inside);
            p.circle_stroke(at(0.5, 0.5), w * 0.17, Stroke::new((w / 22.0).max(1.0), green));
            p.text(at(0.5, 0.5), egui::Align2::CENTER_CENTER, "฿", bold(h * 0.3), green);
            for fx in [0.16f32, 0.84] {
                p.circle_filled(at(fx, 0.5), w * 0.035, green);
            }
        }
        "Copy Picture" | "Save Picture" => {
            // a picture: frame, sun and hills; a second sheet behind it (copy) or a down arrow (save)
            let save = name == "Save Picture";
            let (x0, y0, x1, y1) = if save { (0.06, 0.08, 0.74, 0.64) } else { (0.2, 0.22, 0.94, 0.86) };
            if !save {
                p.rect_filled(rr(0.06, 0.08, 0.8, 0.72), 1.0, Color32::WHITE);
                p.rect_stroke(rr(0.06, 0.08, 0.8, 0.72), 1.0, s, egui::StrokeKind::Inside);
            }
            p.rect_filled(rr(x0, y0, x1, y1), 1.0, Color32::WHITE);
            let fx = |f: f32| x0 + (x1 - x0) * f;
            let fy = |f: f32| y0 + (y1 - y0) * f;
            p.circle_filled(at(fx(0.72), fy(0.3)), w * 0.06, yellow);
            p.add(Shape::convex_polygon(vec![at(fx(0.06), fy(0.92)), at(fx(0.38), fy(0.42)), at(fx(0.62), fy(0.92))], green, Stroke::NONE));
            p.add(Shape::convex_polygon(vec![at(fx(0.4), fy(0.92)), at(fx(0.66), fy(0.58)), at(fx(0.94), fy(0.92))], lblue, Stroke::NONE));
            p.rect_stroke(rr(x0, y0, x1, y1), 1.0, s, egui::StrokeKind::Inside);
            if save {
                let a = Stroke::new((w / 12.0).max(1.5), blue);
                p.line_segment([at(0.78, 0.5), at(0.78, 0.92)], a);
                p.line_segment([at(0.64, 0.78), at(0.78, 0.93)], a);
                p.line_segment([at(0.92, 0.78), at(0.78, 0.93)], a);
            }
        }
        _ => {
            page(p);
            p.rect_filled(rr(0.3, 0.42, 0.7, 0.56), 0.0, blue);
        }
    }
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
