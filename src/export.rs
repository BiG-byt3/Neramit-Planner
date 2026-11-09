//! Save As PDF and Excel.
//!
//! * PDF: the Gantt Chart view as printed — table (ID, Task Name, Duration, Start,
//!   Finish) on the left, timescale and bars on the right, fitted to the page width, paginated by rows,
//!   with header and "Page n of m" footer. Written directly (PDF 1.4) with the bundled Noto fonts
//!   embedded as subsetted CID TrueType fonts, so Thai prints exactly as on screen and copies as text.
//! * Excel: an .xlsx workbook with the task list (real dates, numbers, outline indent) and the
//!   resource sheet, written with rust_xlsxwriter.

use crate::app::{App, TCol};
use crate::gantt::{tiers, unit_label, unit_next, unit_start, Unit};
use crate::i18n::{tr, tr_raw, trf};
use crate::locale::Locale;
use crate::model::{LinkType, Project};
use chrono::{Datelike, Duration, NaiveDate};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;
use std::path::Path;

// ====================================================================== data

pub struct ExportRow {
    pub id: usize,
    pub level: u8,
    pub name: String,
    pub summary: bool,
    pub milestone: bool,
    pub critical: bool,
    pub manual: bool,
    pub active: bool,
    pub percent: u8,
    pub duration: String,
    pub start: NaiveDate,
    pub finish: NaiveDate,
    pub start_text: String,
    pub finish_text: String,
    pub bar_date: String,
    pub predecessors: String,
    pub resources: String,
    pub notes: String,
    pub wbs: String,
    pub mode: String,
    pub constraint: String,
    pub deadline: Option<NaiveDate>,
    pub priority: u16,
    pub total_slack: String,
    /// (row index of the predecessor in `rows`, link type)
    pub links: Vec<(usize, LinkType)>,
}

pub struct ExportResource {
    pub id: usize,
    pub name: String,
    pub rtype: String,
    pub initials: String,
    pub group: String,
    pub max_units: f64,
    pub std_rate: f64,
    pub ovt_rate: f64,
    pub cost_per_use: f64,
}

pub struct ExportData {
    pub title: String,
    pub start: NaiveDate,
    pub finish: NaiveDate,
    pub rows: Vec<ExportRow>,
    pub resources: Vec<ExportResource>,
    pub loc: Locale,
    pub show_critical: bool,
    pub calendar: crate::model::Calendar,
    pub printed: String,
    pub start_text: String,
    pub finish_text: String,
    /// page size in points (width, height)
    pub page_w: f32,
    pub page_h: f32,
    /// second header line: company, project manager, status date (empty: none)
    pub subtitle: String,
    /// signature boxes (Prepared by / Checked by / Approved by) at the end of the last page
    pub signoff: bool,
}

impl ExportData {
    pub fn build(doc: &Project, rows: &[usize], app: &App) -> ExportData {
        let pos: HashMap<u32, usize> = rows.iter().enumerate().map(|(k, &i)| (doc.tasks[i].uid, k)).collect();
        let numbers = doc.outline_numbers();
        let out_rows = rows
            .iter()
            .map(|&i| {
                let t = &doc.tasks[i];
                ExportRow {
                    id: i + 1,
                    level: t.outline_level,
                    name: t.name.clone(),
                    summary: t.is_summary,
                    milestone: t.is_milestone(),
                    critical: t.critical,
                    manual: t.manual,
                    active: t.active,
                    percent: t.percent_complete,
                    duration: app.task_duration_text(t),
                    start: t.start,
                    finish: t.finish,
                    start_text: app.fmt_date(t.start),
                    finish_text: app.fmt_finish(t.finish),
                    bar_date: app.fmt_bar_date(t.start),
                    predecessors: app.cell_text(i, TCol::Predecessors),
                    resources: doc.resource_names(t),
                    notes: t.notes.clone(),
                    wbs: if t.wbs.is_empty() { numbers[i].clone() } else { t.wbs.clone() },
                    mode: tr_raw(if t.manual { "Manually Scheduled" } else { "Auto Scheduled" }).to_string(),
                    constraint: tr_raw(t.constraint_type.label()).to_string(),
                    deadline: t.deadline,
                    priority: t.priority,
                    total_slack: crate::fileio::format_duration(t.total_slack),
                    links: t.predecessors.iter().filter_map(|l| pos.get(&l.pred_uid).map(|&k| (k, l.kind))).collect(),
                }
            })
            .collect();
        let resources = doc
            .resources
            .iter()
            .enumerate()
            .map(|(k, r)| ExportResource {
                id: k + 1,
                name: r.name.clone(),
                rtype: tr_raw(r.rtype.label()).to_string(),
                initials: r.initials.clone(),
                group: r.group.clone(),
                max_units: r.max_units,
                std_rate: r.std_rate,
                ovt_rate: r.ovt_rate,
                cost_per_use: r.cost_per_use,
            })
            .collect();
        let title = if !doc.title.is_empty() {
            doc.title.clone()
        } else {
            app.path.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Project1".into())
        };
        let start = doc.tasks.iter().map(|t| t.start).min().unwrap_or(doc.start).min(doc.start);
        let finish = doc.finish_date().max(start);
        let today = chrono::Local::now().date_naive();
        let mut sub: Vec<String> = Vec::new();
        if !doc.company.trim().is_empty() {
            sub.push(doc.company.trim().to_string());
        }
        if !doc.manager.trim().is_empty() {
            sub.push(format!("{} {}", tr("Manager:"), doc.manager.trim()));
        }
        if let Some(sd) = doc.status_date {
            sub.push(crate::i18n::trf("Status as of {}", &[&app.loc.long_date(sd)]));
        }
        ExportData {
            title,
            start,
            finish,
            rows: out_rows,
            resources,
            loc: app.loc.clone(),
            show_critical: doc.formatting.show_critical,
            calendar: doc.calendar.clone(),
            printed: app.loc.long_date(today),
            start_text: app.fmt_date(start),
            finish_text: app.fmt_finish(finish),
            page_w: A4.0,
            page_h: A4.1,
            subtitle: sub.join("   ·   "),
            signoff: app.print.signoff,
        }
    }
}

// ====================================================================== Excel

pub fn write_xlsx(path: &Path, d: &ExportData) -> Result<(), String> {
    xlsx(path, d).map_err(|e| format!("Cannot write the Excel workbook: {e}"))
}

fn xlsx(path: &Path, d: &ExportData) -> Result<(), rust_xlsxwriter::XlsxError> {
    use rust_xlsxwriter::{Color, DocProperties, ExcelDateTime, Format, FormatAlign, FormatBorder, Workbook};
    let mut wb = Workbook::new();
    wb.set_properties(&DocProperties::new().set_title(d.title.clone()).set_comment("Exported by Neramit Planner"));
    // Calibri has no Thai glyphs (Excel falls back to a mismatched face); Leelawadee UI ships
    // with Windows and Office and suits both scripts at 11 pt
    let is_thai = |s: &str| s.chars().any(|c| ('\u{0E00}'..='\u{0E7F}').contains(&c));
    let thai = is_thai(&d.title) || d.rows.iter().any(|r| is_thai(&r.name)) || d.resources.iter().any(|r| is_thai(&r.name)) || crate::i18n::is_thai();
    let font = if thai { "Leelawadee UI" } else { "Calibri" };
    let base = Format::new().set_font_name(font).set_font_size(11);
    let header = base.clone().set_bold().set_font_color(Color::White).set_background_color(Color::RGB(0x31752F)).set_border(FormatBorder::Thin).set_border_color(Color::RGB(0xBFBFBF)).set_text_wrap().set_align(FormatAlign::VerticalCenter);
    let date_fmt = if d.loc.buddhist() {
        // Thai Buddhist calendar in Excel's number-format language: [$-calendar 07, locale th-TH]
        "[$-107041E]d/m/yyyy".to_string()
    } else {
        match d.loc.order {
            crate::locale::Order::Mdy => "m/d/yyyy".to_string(),
            crate::locale::Order::Ymd => "yyyy-mm-dd".to_string(),
            crate::locale::Order::Dmy => "d/m/yyyy".to_string(),
        }
    };
    let to_xl = |dt: NaiveDate| ExcelDateTime::from_ymd(dt.year() as u16, dt.month() as u8, dt.day() as u8);

    // ---------------- Tasks
    {
        let ws = wb.add_worksheet();
        ws.set_name(tr_raw("Tasks ").trim())?;
        let cols: [(&str, f64); 17] = [
            ("ID", 6.0),
            ("WBS", 9.0),
            ("Outline Level", 8.0),
            ("Task Mode", 16.0),
            ("Task Name", 48.0),
            ("Duration", 12.0),
            ("Start", 12.0),
            ("Finish", 12.0),
            ("% Complete", 11.0),
            ("Predecessors", 16.0),
            ("Resource Names", 24.0),
            ("Constraint Type", 22.0),
            ("Deadline", 12.0),
            ("Priority", 9.0),
            ("Critical", 9.0),
            ("Total Slack", 11.0),
            ("Notes", 40.0),
        ];
        for (c, (h, w)) in cols.iter().enumerate() {
            ws.write_string_with_format(0, c as u16, tr_raw(h), &header)?;
            ws.set_column_width(c as u16, *w)?;
        }
        ws.set_row_height(0, 30)?;
        for (k, r) in d.rows.iter().enumerate() {
            let row = (k + 1) as u32;
            let mut f = base.clone();
            if r.summary {
                f = f.set_bold();
            }
            if !r.active {
                f = f.set_font_color(Color::RGB(0x808080)).set_font_strikethrough();
            }
            let datef = f.clone().set_num_format(&date_fmt).set_align(FormatAlign::Right);
            ws.write_number_with_format(row, 0, r.id as f64, &f)?;
            ws.write_string_with_format(row, 1, &r.wbs, &f)?;
            ws.write_number_with_format(row, 2, r.level as f64, &f)?;
            ws.write_string_with_format(row, 3, &r.mode, &f)?;
            ws.write_string_with_format(row, 4, &r.name, &f.clone().set_indent(r.level.saturating_sub(1).min(15)))?;
            ws.write_string_with_format(row, 5, &r.duration, &f.clone().set_align(FormatAlign::Right))?;
            ws.write_datetime_with_format(row, 6, to_xl(r.start)?, &datef)?;
            ws.write_datetime_with_format(row, 7, to_xl(r.finish)?, &datef)?;
            ws.write_number_with_format(row, 8, r.percent as f64 / 100.0, &f.clone().set_num_format("0%"))?;
            ws.write_string_with_format(row, 9, &r.predecessors, &f)?;
            ws.write_string_with_format(row, 10, &r.resources, &f)?;
            ws.write_string_with_format(row, 11, &r.constraint, &f)?;
            match r.deadline {
                Some(dl) => {
                    ws.write_datetime_with_format(row, 12, to_xl(dl)?, &datef)?;
                }
                None => {
                    ws.write_string_with_format(row, 12, "NA", &f)?;
                }
            }
            ws.write_number_with_format(row, 13, r.priority as f64, &f)?;
            ws.write_string_with_format(row, 14, tr_raw(if r.critical { "Yes" } else { "No" }), &f)?;
            ws.write_string_with_format(row, 15, &r.total_slack, &f)?;
            ws.write_string_with_format(row, 16, &r.notes.replace('\r', ""), &f)?;
        }
        ws.set_freeze_panes(1, 5)?;
        ws.autofilter(0, 0, d.rows.len().max(1) as u32, (cols.len() - 1) as u16)?;
        ws.set_landscape();
    }

    // ---------------- Resources
    {
        let ws = wb.add_worksheet();
        ws.set_name(tr_raw("Resources"))?;
        let cols: [(&str, f64); 9] = [
            ("ID", 6.0),
            ("Resource Name", 32.0),
            ("Type", 12.0),
            ("Initials", 10.0),
            ("Group", 16.0),
            ("Max. Units", 12.0),
            ("Standard Rate", 14.0),
            ("Overtime Rate", 14.0),
            ("Cost/Use", 12.0),
        ];
        for (c, (h, w)) in cols.iter().enumerate() {
            ws.write_string_with_format(0, c as u16, tr_raw(h), &header)?;
            ws.set_column_width(c as u16, *w)?;
        }
        let money = base.clone().set_num_format("#,##0.00");
        for (k, r) in d.resources.iter().enumerate() {
            let row = (k + 1) as u32;
            ws.write_number_with_format(row, 0, r.id as f64, &base)?;
            ws.write_string_with_format(row, 1, &r.name, &base)?;
            ws.write_string_with_format(row, 2, &r.rtype, &base)?;
            ws.write_string_with_format(row, 3, &r.initials, &base)?;
            ws.write_string_with_format(row, 4, &r.group, &base)?;
            ws.write_number_with_format(row, 5, r.max_units / 100.0, &base.clone().set_num_format("0%"))?;
            ws.write_number_with_format(row, 6, r.std_rate, &money)?;
            ws.write_number_with_format(row, 7, r.ovt_rate, &money)?;
            ws.write_number_with_format(row, 8, r.cost_per_use, &money)?;
        }
        ws.set_freeze_panes(1, 0)?;
    }

    // ---------------- Project summary
    {
        let ws = wb.add_worksheet();
        ws.set_name(tr_raw("Project"))?;
        let bold = base.clone().set_bold();
        let datef = base.clone().set_num_format(&date_fmt).set_align(FormatAlign::Left);
        ws.set_column_width(0, 24)?;
        ws.set_column_width(1, 48)?;
        ws.write_string_with_format(0, 0, tr_raw("Title"), &bold)?;
        ws.write_string_with_format(0, 1, &d.title, &base)?;
        ws.write_string_with_format(1, 0, tr_raw("Start Date"), &bold)?;
        ws.write_datetime_with_format(1, 1, to_xl(d.start)?, &datef)?;
        ws.write_string_with_format(2, 0, tr_raw("Finish Date"), &bold)?;
        ws.write_datetime_with_format(2, 1, to_xl(d.finish)?, &datef)?;
        ws.write_string_with_format(3, 0, tr_raw("Tasks ").trim(), &bold)?;
        ws.write_number_with_format(3, 1, d.rows.len() as f64, &base.clone().set_align(FormatAlign::Left))?;
        ws.write_string_with_format(4, 0, tr_raw("Calendar"), &bold)?;
        ws.write_string_with_format(4, 1, &d.calendar.name, &base)?;
    }
    wb.save(path)
}

// ====================================================================== PDF: fonts

const LATIN: usize = 0;
const LATIN_BOLD: usize = 1;
const THAI: usize = 2;
const THAI_BOLD: usize = 3;

struct PdfFont {
    data: &'static [u8],
    name: &'static str,
    face: ttf_parser::Face<'static>,
    upem: f32,
    /// glyph id -> text it represents (ToUnicode)
    used: BTreeMap<u16, String>,
}

impl PdfFont {
    fn new(data: &'static [u8], name: &'static str) -> PdfFont {
        let face = ttf_parser::Face::parse(data, 0).expect("bundled font");
        let upem = face.units_per_em() as f32;
        PdfFont { data, name, face, upem, used: BTreeMap::new() }
    }

    fn glyph(&self, c: char) -> u16 {
        self.face.glyph_index(c).map(|g| g.0).unwrap_or(0)
    }

    fn advance(&self, gid: u16) -> f32 {
        self.face.glyph_hor_advance(ttf_parser::GlyphId(gid)).unwrap_or(0) as f32 / self.upem
    }

    /// Unicode character drawn by a pre-shaped glyph: the RD fonts build each Private Use glyph as a
    /// composite of one original Thai glyph, so follow the component back through the cmap.
    fn origin_of(&self, gid: u16) -> Option<char> {
        let comp = composite_component(self.data, gid)?;
        let by_cmap = |g: u16| ('\u{0E00}'..='\u{0E7F}').find(|c| self.face.glyph_index(*c).map(|x| x.0) == Some(g));
        by_cmap(comp).or_else(|| {
            // alternate forms ("uni0E49.small", "maiTho-thai.narrow") are not in the cmap: use the base glyph
            let name = self.face.glyph_name(ttf_parser::GlyphId(comp))?;
            let base = name.split('.').next()?;
            if let Some(hex) = base.strip_prefix("uni").filter(|h| h.len() == 4) {
                return u32::from_str_radix(hex, 16).ok().and_then(char::from_u32);
            }
            by_cmap(self.face.glyph_index_by_name(base)?.0)
        })
    }
}

/// First component glyph of a composite glyph (None for simple glyphs).
fn composite_component(data: &[u8], gid: u16) -> Option<u16> {
    let rd16 = |o: usize| data.get(o..o + 2).map(|b| u16::from_be_bytes([b[0], b[1]]));
    let rd32 = |o: usize| data.get(o..o + 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]));
    let n = rd16(4)? as usize;
    let find = |tag: &[u8; 4]| (0..n).find_map(|i| {
        let r = 12 + i * 16;
        (data.get(r..r + 4)? == tag).then(|| rd32(r + 8).map(|o| o as usize)).flatten()
    });
    let (head, loca, glyf) = (find(b"head")?, find(b"loca")?, find(b"glyf")?);
    let long = rd16(head + 50)? == 1;
    let at = |g: usize| if long { rd32(loca + g * 4).map(|v| v as usize) } else { rd16(loca + g * 2).map(|v| v as usize * 2) };
    let (a, b) = (at(gid as usize)?, at(gid as usize + 1)?);
    if b <= a || (rd16(glyf + a)? as i16) >= 0 {
        return None;
    }
    rd16(glyf + a + 12)
}

struct Fonts {
    f: Vec<PdfFont>,
}

fn is_thai_char(c: char) -> bool {
    ('\u{0E00}'..='\u{0E7F}').contains(&c) || ('\u{E100}'..='\u{E8FF}').contains(&c)
}

impl Fonts {
    fn new() -> Fonts {
        Fonts {
            f: vec![
                PdfFont::new(crate::theme::NOTO_SANS, "NotoSans"),
                PdfFont::new(crate::theme::NOTO_SANS_BOLD, "NotoSans-SemiBold"),
                PdfFont::new(crate::theme::NOTO_THAI, "NotoSansThai"),
                PdfFont::new(crate::theme::NOTO_THAI_BOLD, "NotoSansThai-Medium"),
            ],
        }
    }

    /// Split shaped display text into (font, glyphs) runs.
    fn runs(&self, s: &str, bold: bool) -> Vec<(usize, Vec<(u16, char)>)> {
        let mut out: Vec<(usize, Vec<(u16, char)>)> = Vec::new();
        let mut last_thai = false;
        for c in s.chars() {
            // spaces and punctuation stay with the surrounding script
            let thai = if is_thai_char(c) { true } else if c.is_alphanumeric() { false } else { last_thai };
            let mut fi = match (thai, bold) {
                (false, false) => LATIN,
                (false, true) => LATIN_BOLD,
                (true, false) => THAI,
                (true, true) => THAI_BOLD,
            };
            let mut gid = self.f[fi].glyph(c);
            if gid == 0 {
                // fall back to the other script's font (e.g. digits in a Thai run)
                let alt = match fi {
                    LATIN => THAI,
                    LATIN_BOLD => THAI_BOLD,
                    THAI => LATIN,
                    _ => LATIN_BOLD,
                };
                let g2 = self.f[alt].glyph(c);
                if g2 != 0 {
                    fi = alt;
                    gid = g2;
                }
            }
            last_thai = fi >= THAI;
            match out.last_mut() {
                Some((f, v)) if *f == fi => v.push((gid, c)),
                _ => out.push((fi, vec![(gid, c)])),
            }
        }
        out
    }

    fn width(&self, s: &str, size: f32, bold: bool) -> f32 {
        self.runs(s, bold).iter().map(|(f, g)| g.iter().map(|(gid, _)| self.f[*f].advance(*gid)).sum::<f32>()).sum::<f32>() * size
    }

    /// Text shortened with "…" to fit `max_w`.
    fn fit(&self, s: &str, size: f32, bold: bool, max_w: f32) -> String {
        if self.width(s, size, bold) <= max_w {
            return s.to_string();
        }
        let chars: Vec<char> = s.chars().collect();
        let mut n = chars.len();
        while n > 0 {
            n -= 1;
            // never cut between a Thai base and its marks
            while n > 0 && chars.get(n).map(|c| is_mark(*c)).unwrap_or(false) {
                n -= 1;
            }
            let t: String = chars[..n].iter().collect::<String>() + "…";
            if self.width(&t, size, bold) <= max_w {
                return t;
            }
        }
        String::new()
    }
}

fn is_mark(c: char) -> bool {
    matches!(c, '\u{0E31}' | '\u{0E34}'..='\u{0E3A}' | '\u{0E47}'..='\u{0E4E}')
}

/// TrueType subset: every glyph id keeps its number (so advances and the CID mapping stay valid) but
/// glyphs that are not used get empty outlines. Composite glyphs pull in their components.
fn subset_ttf(data: &[u8], keep: &BTreeSet<u16>) -> Vec<u8> {
    let rd16 = |o: usize| u16::from_be_bytes([data[o], data[o + 1]]);
    let rd32 = |o: usize| u32::from_be_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]]);
    let num_tables = rd16(4) as usize;
    let mut tables: BTreeMap<[u8; 4], (usize, usize)> = BTreeMap::new();
    for i in 0..num_tables {
        let r = 12 + i * 16;
        let tag = [data[r], data[r + 1], data[r + 2], data[r + 3]];
        tables.insert(tag, (rd32(r + 8) as usize, rd32(r + 12) as usize));
    }
    let (head_o, _) = tables[b"head"];
    let long_loca = rd16(head_o + 50) == 1;
    let (maxp_o, _) = tables[b"maxp"];
    let num_glyphs = rd16(maxp_o + 4) as usize;
    let (loca_o, _) = tables[b"loca"];
    let (glyf_o, _) = tables[b"glyf"];
    let loca = |g: usize| -> usize {
        if long_loca {
            rd32(loca_o + g * 4) as usize
        } else {
            rd16(loca_o + g * 2) as usize * 2
        }
    };
    // closure over composite components
    let mut all: BTreeSet<u16> = keep.clone();
    all.insert(0);
    let mut stack: Vec<u16> = all.iter().copied().collect();
    while let Some(g) = stack.pop() {
        let g = g as usize;
        if g >= num_glyphs {
            continue;
        }
        let (a, b) = (loca(g), loca(g + 1));
        if b <= a {
            continue;
        }
        let o = glyf_o + a;
        let contours = rd16(o) as i16;
        if contours >= 0 {
            continue;
        }
        let mut p = o + 10;
        loop {
            let flags = rd16(p);
            let comp = rd16(p + 2);
            if all.insert(comp) {
                stack.push(comp);
            }
            p += 4 + if flags & 0x1 != 0 { 4 } else { 2 };
            if flags & 0x8 != 0 {
                p += 2;
            } else if flags & 0x40 != 0 {
                p += 4;
            } else if flags & 0x80 != 0 {
                p += 8;
            }
            if flags & 0x20 == 0 {
                break;
            }
        }
    }
    // new glyf + long loca
    let mut glyf = Vec::new();
    let mut loca_new = Vec::with_capacity((num_glyphs + 1) * 4);
    for g in 0..num_glyphs {
        loca_new.extend_from_slice(&(glyf.len() as u32).to_be_bytes());
        if all.contains(&(g as u16)) {
            let (a, b) = (loca(g), loca(g + 1));
            if b > a {
                glyf.extend_from_slice(&data[glyf_o + a..glyf_o + b]);
                while glyf.len() % 4 != 0 {
                    glyf.push(0);
                }
            }
        }
    }
    loca_new.extend_from_slice(&(glyf.len() as u32).to_be_bytes());
    let mut head = data[head_o..head_o + 54].to_vec();
    head[50] = 0;
    head[51] = 1;
    head[8..12].copy_from_slice(&[0, 0, 0, 0]);
    let wanted: [&[u8; 4]; 10] = [b"OS/2", b"cmap", b"cvt ", b"fpgm", b"gasp", b"hhea", b"hmtx", b"maxp", b"name", b"prep"];
    let mut out_tables: Vec<([u8; 4], Vec<u8>)> = Vec::new();
    for t in wanted {
        if let Some(&(o, l)) = tables.get(t) {
            out_tables.push((*t, data[o..o + l].to_vec()));
        }
    }
    out_tables.push((*b"glyf", glyf));
    out_tables.push((*b"head", head));
    out_tables.push((*b"loca", loca_new));
    out_tables.sort_by(|a, b| a.0.cmp(&b.0));
    let n = out_tables.len();
    let mut es = 0;
    while (1 << (es + 1)) <= n {
        es += 1;
    }
    let sr = (1u16 << es) * 16;
    let mut out = Vec::new();
    out.extend_from_slice(&0x0001_0000u32.to_be_bytes());
    out.extend_from_slice(&(n as u16).to_be_bytes());
    out.extend_from_slice(&sr.to_be_bytes());
    out.extend_from_slice(&(es as u16).to_be_bytes());
    out.extend_from_slice(&((n as u16) * 16 - sr).to_be_bytes());
    let mut offset = 12 + n * 16;
    let mut body = Vec::new();
    for (tag, d) in &out_tables {
        let mut sum: u32 = 0;
        for ch in d.chunks(4) {
            let mut w = [0u8; 4];
            w[..ch.len()].copy_from_slice(ch);
            sum = sum.wrapping_add(u32::from_be_bytes(w));
        }
        out.extend_from_slice(tag);
        out.extend_from_slice(&sum.to_be_bytes());
        out.extend_from_slice(&(offset as u32).to_be_bytes());
        out.extend_from_slice(&(d.len() as u32).to_be_bytes());
        body.extend_from_slice(d);
        while body.len() % 4 != 0 {
            body.push(0);
        }
        offset = 12 + n * 16 + body.len();
    }
    out.extend_from_slice(&body);
    out
}

// ====================================================================== PDF: page canvas

pub type Rgb = (f32, f32, f32);

fn rgb(r: u8, g: u8, b: u8) -> Rgb {
    (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}

/// A drawing primitive of a printed page (page points, origin top left).
#[derive(Clone, Debug)]
pub enum PrevOp {
    Fill(f32, f32, f32, f32, Rgb),
    Stroke(f32, f32, f32, f32, Rgb, f32),
    Line(Vec<(f32, f32)>, Rgb, f32),
    Poly(Vec<(f32, f32)>, Rgb),
    /// x, baseline, size, bold, text (shaped), colour
    Text(f32, f32, f32, bool, String, Rgb),
}

struct Canvas<'a> {
    ops: String,
    page_h: f32,
    fonts: &'a mut Fonts,
    /// also keep the primitives (print preview)
    rec: Option<Vec<PrevOp>>,
}

impl<'a> Canvas<'a> {
    fn y(&self, y: f32) -> f32 {
        self.page_h - y
    }

    fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: Rgb) {
        if let Some(r) = self.rec.as_mut() {
            r.push(PrevOp::Fill(x, y, w, h, c));
        }
        let _ = writeln!(self.ops, "{:.3} {:.3} {:.3} rg {:.2} {:.2} {:.2} {:.2} re f", c.0, c.1, c.2, x, self.y(y + h), w, h);
    }

    fn stroke_rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: Rgb, lw: f32) {
        if let Some(r) = self.rec.as_mut() {
            r.push(PrevOp::Stroke(x, y, w, h, c, lw));
        }
        let _ = writeln!(self.ops, "{:.3} {:.3} {:.3} RG {lw:.2} w [] 0 d {:.2} {:.2} {:.2} {:.2} re S", c.0, c.1, c.2, x, self.y(y + h), w, h);
    }

    fn line(&mut self, pts: &[(f32, f32)], c: Rgb, lw: f32, dash: Option<(f32, f32)>) {
        if pts.len() < 2 {
            return;
        }
        if let Some(r) = self.rec.as_mut() {
            r.push(PrevOp::Line(pts.to_vec(), c, lw));
        }
        let d = match dash {
            Some((a, b)) => format!("[{a:.1} {b:.1}] 0 d"),
            None => "[] 0 d".into(),
        };
        let _ = write!(self.ops, "{:.3} {:.3} {:.3} RG {lw:.2} w {d} ", c.0, c.1, c.2);
        for (k, (x, y)) in pts.iter().enumerate() {
            let _ = write!(self.ops, "{:.2} {:.2} {} ", x, self.y(*y), if k == 0 { "m" } else { "l" });
        }
        self.ops.push_str("S\n");
    }

    fn poly(&mut self, pts: &[(f32, f32)], c: Rgb) {
        if let Some(r) = self.rec.as_mut() {
            r.push(PrevOp::Poly(pts.to_vec(), c));
        }
        let _ = write!(self.ops, "{:.3} {:.3} {:.3} rg ", c.0, c.1, c.2);
        for (k, (x, y)) in pts.iter().enumerate() {
            let _ = write!(self.ops, "{:.2} {:.2} {} ", x, self.y(*y), if k == 0 { "m" } else { "l" });
        }
        self.ops.push_str("h f\n");
    }

    /// Draw display text (already shaped) with its left baseline at (x, baseline).
    fn text(&mut self, x: f32, baseline: f32, size: f32, bold: bool, s: &str, c: Rgb) {
        if s.is_empty() {
            return;
        }
        if let Some(r) = self.rec.as_mut() {
            r.push(PrevOp::Text(x, baseline, size, bold, s.to_string(), c));
        }
        let runs = self.fonts.runs(s, bold);
        let mut cx = x;
        let by = self.y(baseline);
        for (fi, glyphs) in runs {
            let mut hex = String::new();
            let mut adv = 0.0;
            for (gid, ch) in &glyphs {
                let _ = write!(hex, "{gid:04X}");
                let f = &mut self.fonts.f[fi];
                adv += f.advance(*gid);
                let orig = if ('\u{E100}'..='\u{E8FF}').contains(ch) { f.origin_of(*gid).unwrap_or(*ch) } else { *ch };
                f.used.entry(*gid).or_insert_with(|| orig.to_string());
            }
            let _ = writeln!(self.ops, "BT {:.3} {:.3} {:.3} rg /F{fi} {size:.2} Tf {cx:.2} {by:.2} Td <{hex}> Tj ET", c.0, c.1, c.2);
            cx += adv * size;
        }
    }

    /// Text aligned in a box: 0 left, 1 center, 2 right; vertically centred.
    fn text_in(&mut self, x: f32, y: f32, w: f32, h: f32, size: f32, bold: bool, s: &str, align: u8, c: Rgb) {
        let s = self.fonts.fit(s, size, bold, w - 4.0);
        let tw = self.fonts.width(&s, size, bold);
        let tx = match align {
            1 => x + (w - tw) / 2.0,
            2 => x + w - tw - 2.0,
            _ => x + 2.0,
        };
        let baseline = y + h / 2.0 + size * 0.35;
        self.text(tx, baseline, size, bold, &s, c);
    }
}

// ====================================================================== PDF: layout

/// A4 landscape (points)
pub const A4: (f32, f32) = (841.89, 595.28);
const MARGIN: f32 = 28.0;
const ROW_H: f32 = 13.0;
const HEAD_H: f32 = 26.0;
const FONT: f32 = 7.2;
/// height of the signature boxes with the space above them
const SIGN_H: f32 = 84.0;

/// Rows on each page: full pages, and a last page that keeps `reserve` points free (the
/// signature boxes) — when it cannot, its rows are shared with one more page.
fn paginate(n: usize, per_page: usize, reserve: f32) -> Vec<usize> {
    let mut pages = Vec::new();
    let mut left = n;
    while left > 0 {
        let take = left.min(per_page);
        pages.push(take);
        left -= take;
    }
    if pages.is_empty() {
        pages.push(0);
    }
    let cap = per_page.saturating_sub((reserve / ROW_H).ceil() as usize);
    let last = *pages.last().unwrap();
    if reserve > 0.0 && last > cap {
        let moved = (last / 2).clamp(1, cap.max(1)).min(last);
        *pages.last_mut().unwrap() = last - moved;
        pages.push(moved);
    }
    pages
}

/// Prepared by / Checked by / Approved by: signature line, name in brackets, date.
fn signature_boxes(cv: &mut Canvas, d: &ExportData, y: f32, ink: Rgb, grid: Rgb) {
    let labels = ["Prepared by", "Checked by", "Approved by"];
    let gap = 14.0;
    let w = ((d.page_w - 2.0 * MARGIN - gap * 2.0) / 3.0).min(220.0);
    let total = w * 3.0 + gap * 2.0;
    let x0 = d.page_w - MARGIN - total;
    let h = SIGN_H - 14.0;
    let dim = rgb(0x55, 0x55, 0x55);
    for (k, l) in labels.iter().enumerate() {
        let x = x0 + k as f32 * (w + gap);
        cv.stroke_rect(x, y, w, h, grid, 0.6);
        cv.text_in(x, y + 2.0, w, 12.0, 8.0, true, &shape(tr(l)), 1, ink);
        let sig = shape(&format!("{} ........................................", tr("Signature")));
        cv.text_in(x, y + 30.0, w, 12.0, 7.5, false, &sig, 1, dim);
        cv.text_in(x, y + 43.0, w, 12.0, 7.5, false, "( ........................................ )", 1, dim);
        let date = shape(&format!("{} ........ / ........ / ........", tr("Date")));
        cv.text_in(x, y + 56.0, w, 12.0, 7.5, false, &date, 1, dim);
    }
}
const BLUE: (u8, u8, u8) = (35, 118, 176);

pub fn write_pdf(path: &Path, d: &ExportData) -> Result<(), String> {
    let bytes = pdf_bytes(d);
    std::fs::write(path, bytes).map_err(|e| format!("Cannot write the PDF file: {e}"))
}

fn shape(s: &str) -> String {
    crate::thai::shape(s).into_owned()
}

pub fn pdf_bytes(d: &ExportData) -> Vec<u8> {
    let (fonts, contents, _) = layout_pages(d, false);
    assemble(&fonts, &contents, &d.title, d.page_w, d.page_h)
}

/// The pages as drawing primitives (page points, origin top left) for the print preview.
pub fn preview(d: &ExportData) -> Vec<Vec<PrevOp>> {
    layout_pages(d, true).2
}

fn layout_pages(d: &ExportData, record: bool) -> (Fonts, Vec<String>, Vec<Vec<PrevOp>>) {
    let mut fonts = Fonts::new();
    // table columns: ID, Task Name, Duration, Start, Finish
    let cols: [(&str, f32, u8); 5] = [("ID", 22.0, 1), ("Task Name", 178.0, 0), ("Duration", 50.0, 2), ("Start", 58.0, 2), ("Finish", 58.0, 2)];
    let table_w: f32 = cols.iter().map(|c| c.1).sum();
    let top = MARGIN + if d.subtitle.is_empty() { 34.0 } else { 44.0 };
    let footer_h = 22.0;
    let chart_x = MARGIN + table_w + 2.0;
    let chart_w = d.page_w - MARGIN - chart_x;
    let rows_per_page = (((d.page_h - MARGIN - footer_h) - (top + HEAD_H)) / ROW_H).floor().max(1.0) as usize;
    let page_rows = paginate(d.rows.len(), rows_per_page, if d.signoff { SIGN_H } else { 0.0 });
    let pages = page_rows.len();

    // timescale fitted to the page width
    let span_start = d.start - Duration::days(2);
    let span_end = d.finish + Duration::days(4);
    let days = ((span_end - span_start).num_days() + 1).max(7) as f32;
    let ppd = chart_w / days;
    let xd = |dt: NaiveDate| chart_x + (dt - span_start).num_days() as f32 * ppd;
    // tier units chosen as on screen (screen px are ~1.5x larger than these 6-7 pt labels)
    let (top_u, bot_u) = tiers(ppd * 1.6);

    let grid = rgb(0xD0, 0xD0, 0xD0);
    let ink = rgb(0x26, 0x26, 0x26);
    let header_bg = rgb(0xE6, 0xE6, 0xE6);
    let title_bg = rgb(0xD2, 0xD2, 0xD2);
    let link_c = rgb(0x2F, 0x5F, 0x9E);

    let mut contents: Vec<String> = Vec::new();
    let mut previews: Vec<Vec<PrevOp>> = Vec::new();
    for page in 0..pages {
        let mut cv = Canvas { ops: String::new(), page_h: d.page_h, fonts: &mut fonts, rec: record.then(Vec::new) };
        let first: usize = page_rows[..page].iter().sum();
        let last = first + page_rows[page];

        // ---- page header
        cv.text(MARGIN, MARGIN + 12.0, 13.0, true, &shape(&d.title), ink);
        let range = format!("{} - {}", d.start_text, d.finish_text);
        let rw = cv.fonts.width(&shape(&range), 8.0, false);
        cv.text(d.page_w - MARGIN - rw, MARGIN + 12.0, 8.0, false, &shape(&range), rgb(0x55, 0x55, 0x55));
        cv.line(&[(MARGIN, MARGIN + 20.0), (d.page_w - MARGIN, MARGIN + 20.0)], rgb(0x2E, 0x77, 0x32), 1.2, None);
        if !d.subtitle.is_empty() {
            cv.text(MARGIN, MARGIN + 31.0, 8.0, false, &shape(&d.subtitle), rgb(0x40, 0x40, 0x40));
        }

        // ---- table header
        let mut x = MARGIN;
        for (h, w, _) in cols.iter() {
            cv.fill_rect(x, top, *w, HEAD_H, title_bg);
            cv.stroke_rect(x, top, *w, HEAD_H, grid, 0.4);
            cv.text_in(x, top, *w, HEAD_H, FONT, true, tr(h), 1, ink);
            x += w;
        }

        // ---- timescale (two tiers)
        let tier_h = HEAD_H / 2.0;
        cv.fill_rect(chart_x, top, chart_w, HEAD_H, header_bg);
        for (k, u) in [top_u, bot_u].iter().enumerate() {
            let ty = top + k as f32 * tier_h;
            let mut dt = unit_start(span_start, *u);
            while dt <= span_end {
                let nx = unit_next(dt, *u);
                let (x0, x1) = (xd(dt).max(chart_x), xd(nx).min(chart_x + chart_w));
                if x1 > x0 {
                    cv.line(&[(x0, ty), (x0, ty + tier_h)], grid, 0.4, None);
                    let lbl = shape(&unit_label(dt, *u, k == 0, (x1 - x0) * 1.6, &d.loc));
                    if !matches!(u, Unit::Hour(_)) {
                        cv.text_in(x0, ty, x1 - x0, tier_h, 6.2, false, &lbl, if k == 0 { 0 } else { 1 }, ink);
                    }
                }
                dt = nx;
            }
            cv.line(&[(chart_x, ty + tier_h), (chart_x + chart_w, ty + tier_h)], grid, 0.4, None);
        }
        cv.stroke_rect(chart_x, top, chart_w, HEAD_H, grid, 0.4);

        let body_top = top + HEAD_H;
        let body_h = (last - first) as f32 * ROW_H;
        // ---- chart background: non-working days and top-tier lines
        if ppd >= 2.5 {
            let mut dt = span_start;
            while dt <= span_end {
                if !d.calendar.is_working(dt) {
                    cv.fill_rect(xd(dt), body_top, ppd, body_h, rgb(0xF2, 0xF2, 0xF2));
                }
                dt += Duration::days(1);
            }
        }
        let mut dt = unit_start(span_start, top_u);
        while dt <= span_end {
            let x = xd(dt);
            if x > chart_x && x < chart_x + chart_w {
                cv.line(&[(x, body_top), (x, body_top + body_h)], rgb(0xE0, 0xE0, 0xE0), 0.4, None);
            }
            dt = unit_next(dt, top_u);
        }

        // ---- rows
        let row_y = |k: usize| body_top + (k - first) as f32 * ROW_H;
        for k in first..last {
            let r = &d.rows[k];
            let y = row_y(k);
            let color = if !r.active { rgb(0x99, 0x99, 0x99) } else { ink };
            let bold = r.summary;
            let mut x = MARGIN;
            for (c, (_, w, align)) in cols.iter().enumerate() {
                let text = match c {
                    0 => r.id.to_string(),
                    1 => shape(&r.name),
                    2 => shape(&r.duration),
                    3 => shape(&r.start_text),
                    _ => shape(&r.finish_text),
                };
                if c == 0 {
                    cv.fill_rect(x, y, *w, ROW_H, header_bg);
                }
                if c == 1 {
                    let indent = (r.level.saturating_sub(1) as f32 * 8.0).min(80.0);
                    cv.text_in(x + indent, y, *w - indent, ROW_H, FONT, bold, &text, 0, color);
                } else {
                    cv.text_in(x, y, *w, ROW_H, FONT, bold, &text, *align, color);
                }
                cv.line(&[(x + w, y), (x + w, y + ROW_H)], grid, 0.3, None);
                x += w;
            }
            cv.line(&[(MARGIN, y + ROW_H), (MARGIN + table_w, y + ROW_H)], grid, 0.3, None);

            // ---- bar
            let cy = y + ROW_H / 2.0;
            let x0 = xd(r.start);
            let x1 = xd(r.finish) + ppd;
            if r.summary {
                let c = rgb(0x10, 0x10, 0x10);
                cv.fill_rect(x0, cy - 3.0, (x1 - x0).max(1.0), 2.6, c);
                cv.poly(&[(x0, cy - 0.5), (x0 + 3.0, cy - 0.5), (x0, cy + 3.0)], c);
                cv.poly(&[(x1 - 3.0, cy - 0.5), (x1, cy - 0.5), (x1, cy + 3.0)], c);
            } else if r.milestone {
                let c = rgb(0x10, 0x10, 0x10);
                let m = 3.6;
                cv.poly(&[(x0, cy - m), (x0 + m, cy), (x0, cy + m), (x0 - m, cy)], c);
                cv.text(x0 + m + 2.0, cy + 2.2, 6.0, false, &shape(&r.bar_date), ink);
            } else {
                let (fill, edge) = if !r.active {
                    (rgb(0xFF, 0xFF, 0xFF), rgb(0x99, 0x99, 0x99))
                } else if r.critical && d.show_critical {
                    (rgb(0xE0, 0x30, 0x30), rgb(0xB0, 0x20, 0x20))
                } else if r.manual {
                    (rgb(0x78, 0xBE, 0xD2), rgb(0x00, 0x80, 0x80))
                } else {
                    (rgb(BLUE.0, BLUE.1, BLUE.2), rgb(BLUE.0, BLUE.1, BLUE.2))
                };
                cv.fill_rect(x0, cy - 3.0, (x1 - x0).max(1.0), 6.0, fill);
                cv.stroke_rect(x0, cy - 3.0, (x1 - x0).max(1.0), 6.0, edge, 0.4);
                if r.percent > 0 {
                    let w = (x1 - x0) * r.percent as f32 / 100.0;
                    cv.fill_rect(x0, cy - 0.8, w, 1.6, rgb(0x10, 0x10, 0x10));
                }
                if !r.resources.is_empty() {
                    cv.text(x1 + 3.0, cy + 2.2, 6.0, false, &shape(&r.resources), ink);
                }
            }
        }

        // ---- links between rows on this page
        for k in first..last {
            let r = &d.rows[k];
            for (p, kind) in &r.links {
                if *p < first || *p >= last {
                    continue;
                }
                let pr = &d.rows[*p];
                let py = row_y(*p) + ROW_H / 2.0;
                let sy = row_y(k) + ROW_H / 2.0;
                let p_end = if pr.milestone { xd(pr.start) + 3.6 } else { xd(pr.finish) + ppd };
                let p_start = if pr.milestone { xd(pr.start) - 3.6 } else { xd(pr.start) };
                let s_start = if r.milestone { xd(r.start) - 3.6 } else { xd(r.start) };
                let s_end = if r.milestone { xd(r.start) + 3.6 } else { xd(r.finish) + ppd };
                let down = sy > py;
                let edge = if down { sy - 3.2 } else { sy + 3.2 };
                let (pts, tip, dir): (Vec<(f32, f32)>, (f32, f32), (f32, f32)) = match kind {
                    LinkType::FS if s_start >= p_end + 2.0 => {
                        let ax = s_start + 2.0;
                        (vec![(p_end, py), (ax, py), (ax, edge)], (ax, edge), (0.0, if down { 1.0 } else { -1.0 }))
                    }
                    LinkType::FS => {
                        let mid = if down { row_y(k) } else { row_y(k) + ROW_H };
                        (vec![(p_end, py), (p_end + 3.0, py), (p_end + 3.0, mid), (s_start - 4.0, mid), (s_start - 4.0, sy), (s_start, sy)], (s_start, sy), (1.0, 0.0))
                    }
                    LinkType::SS => {
                        let lx = p_start.min(s_start) - 4.0;
                        (vec![(p_start, py), (lx, py), (lx, sy), (s_start, sy)], (s_start, sy), (1.0, 0.0))
                    }
                    LinkType::FF => {
                        let rx = p_end.max(s_end) + 4.0;
                        (vec![(p_end, py), (rx, py), (rx, sy), (s_end, sy)], (s_end, sy), (-1.0, 0.0))
                    }
                    LinkType::SF => {
                        let rx = s_end + 4.0;
                        (vec![(p_start, py), (p_start - 4.0, py), (p_start - 4.0, (py + sy) / 2.0), (rx, (py + sy) / 2.0), (rx, sy), (s_end, sy)], (s_end, sy), (-1.0, 0.0))
                    }
                };
                cv.line(&pts, link_c, 0.5, None);
                let (tx, ty) = tip;
                let (dx, dy) = dir;
                let (nx, ny) = (-dy, dx);
                cv.poly(&[(tx, ty), (tx - dx * 2.6 + nx * 1.6, ty - dy * 2.6 + ny * 1.6), (tx - dx * 2.6 - nx * 1.6, ty - dy * 2.6 - ny * 1.6)], link_c);
            }
        }
        cv.stroke_rect(MARGIN, top, table_w, HEAD_H + body_h, grid, 0.5);
        cv.stroke_rect(chart_x, top, chart_w, HEAD_H + body_h, grid, 0.5);
        if d.signoff && page + 1 == pages {
            signature_boxes(&mut cv, d, top + HEAD_H + body_h + 10.0, ink, grid);
        }

        // ---- footer
        let fy = d.page_h - MARGIN - 6.0;
        cv.line(&[(MARGIN, fy - 12.0), (d.page_w - MARGIN, fy - 12.0)], grid, 0.5, None);
        cv.text(MARGIN, fy, 7.0, false, &shape(&d.title), rgb(0x55, 0x55, 0x55));
        let pg = trf("Page {} of {}", &[&(page + 1).to_string(), &pages.to_string()]);
        let pw = cv.fonts.width(&pg, 7.0, false);
        cv.text((d.page_w - pw) / 2.0, fy, 7.0, false, &pg, rgb(0x55, 0x55, 0x55));
        let printed = shape(&trf("Printed {}", &[&d.printed]));
        let prw = cv.fonts.width(&printed, 7.0, false);
        cv.text(d.page_w - MARGIN - prw, fy, 7.0, false, &printed, rgb(0x55, 0x55, 0x55));

        contents.push(std::mem::take(&mut cv.ops));
        if let Some(r) = cv.rec.take() {
            previews.push(r);
        }
    }
    (fonts, contents, previews)
}

// ====================================================================== PDF: file structure

fn deflate(data: &[u8]) -> Vec<u8> {
    miniz_oxide::deflate::compress_to_vec_zlib(data, 6)
}

fn pdf_text_string(s: &str) -> String {
    let mut hex = String::from("<FEFF");
    for u in s.encode_utf16() {
        let _ = write!(hex, "{u:04X}");
    }
    hex.push('>');
    hex
}

fn assemble(fonts: &Fonts, contents: &[String], title: &str, page_w: f32, page_h: f32) -> Vec<u8> {
    let mut objs: Vec<Vec<u8>> = Vec::new();
    // object numbers are 1-based; reserve catalog (1) and pages (2)
    objs.push(Vec::new());
    objs.push(Vec::new());
    let add = |o: Vec<u8>, objs: &mut Vec<Vec<u8>>| -> usize {
        objs.push(o);
        objs.len()
    };
    let stream = |dict: &str, data: &[u8]| -> Vec<u8> {
        let mut v = format!("<< {dict} /Length {} >>\nstream\n", data.len()).into_bytes();
        v.extend_from_slice(data);
        v.extend_from_slice(b"\nendstream");
        v
    };

    // fonts actually used
    let mut font_refs: Vec<(usize, usize)> = Vec::new();
    for (fi, f) in fonts.f.iter().enumerate() {
        if f.used.is_empty() {
            continue;
        }
        let keep: BTreeSet<u16> = f.used.keys().copied().collect();
        let sub = subset_ttf(f.data, &keep);
        let comp = deflate(&sub);
        let file = add(stream(&format!("/Filter /FlateDecode /Length1 {}", sub.len()), &comp), &mut objs);
        let s = f.upem;
        let sc = |v: i16| (v as f32 * 1000.0 / s).round() as i32;
        let bb = f.face.global_bounding_box();
        // subset tag: six capital letters, unique per font
        let tag = format!("BRXP{}A+{}", (b'A' + fi as u8) as char, f.name);
        let desc = add(
            format!(
                "<< /Type /FontDescriptor /FontName /{tag} /Flags 32 /FontBBox [{} {} {} {}] /ItalicAngle 0 /Ascent {} /Descent {} /CapHeight {} /StemV 80 /FontFile2 {file} 0 R >>",
                sc(bb.x_min),
                sc(bb.y_min),
                sc(bb.x_max),
                sc(bb.y_max),
                sc(f.face.ascender()),
                sc(f.face.descender()),
                sc(f.face.capital_height().unwrap_or(f.face.ascender()))
            )
            .into_bytes(),
            &mut objs,
        );
        let mut w = String::from("[");
        for gid in f.used.keys() {
            let _ = write!(w, " {gid} [{}]", (f.advance(*gid) * 1000.0).round() as i32);
        }
        w.push_str(" ]");
        let cid = add(
            format!("<< /Type /Font /Subtype /CIDFontType2 /BaseFont /{tag} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor {desc} 0 R /W {w} /DW 1000 /CIDToGIDMap /Identity >>").into_bytes(),
            &mut objs,
        );
        // ToUnicode
        let mut cmap = String::from("/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n");
        let entries: Vec<(&u16, &String)> = f.used.iter().collect();
        for chunk in entries.chunks(100) {
            let _ = writeln!(cmap, "{} beginbfchar", chunk.len());
            for (gid, text) in chunk {
                let mut u = String::new();
                for cu in text.encode_utf16() {
                    let _ = write!(u, "{cu:04X}");
                }
                let _ = writeln!(cmap, "<{:04X}> <{u}>", gid);
            }
            cmap.push_str("endbfchar\n");
        }
        cmap.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
        let tu = add(stream("/Filter /FlateDecode", &deflate(cmap.as_bytes())), &mut objs);
        let font = add(format!("<< /Type /Font /Subtype /Type0 /BaseFont /{tag} /Encoding /Identity-H /DescendantFonts [{cid} 0 R] /ToUnicode {tu} 0 R >>").into_bytes(), &mut objs);
        font_refs.push((fi, font));
    }
    let mut font_dict = String::from("<<");
    for (fi, o) in &font_refs {
        let _ = write!(font_dict, " /F{fi} {o} 0 R");
    }
    font_dict.push_str(" >>");

    let mut kids = Vec::new();
    for c in contents {
        let cs = add(stream("/Filter /FlateDecode", &deflate(c.as_bytes())), &mut objs);
        let page = add(
            format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {page_w:.2} {page_h:.2}] /Resources << /Font {font_dict} >> /Contents {cs} 0 R >>").into_bytes(),
            &mut objs,
        );
        kids.push(page);
    }
    objs[0] = b"<< /Type /Catalog /Pages 2 0 R >>".to_vec();
    objs[1] = format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids.iter().map(|k| format!("{k} 0 R")).collect::<Vec<_>>().join(" "), kids.len()).into_bytes();
    let now = chrono::Local::now();
    let info = add(
        format!("<< /Title {} /Producer (Neramit Planner {}) /Creator (Neramit Planner) /CreationDate (D:{}) >>", pdf_text_string(title), env!("CARGO_PKG_VERSION"), now.format("%Y%m%d%H%M%S")).into_bytes(),
        &mut objs,
    );

    let mut out: Vec<u8> = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::with_capacity(objs.len());
    for (k, o) in objs.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", k + 1).as_bytes());
        out.extend_from_slice(o);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for o in offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R /Info {info} 0 R >>\nstartxref\n{xref}\n%%EOF\n", objs.len() + 1).as_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_page_keeps_room_for_signatures() {
        assert_eq!(paginate(0, 30, SIGN_H), vec![0]);
        assert_eq!(paginate(10, 30, SIGN_H), vec![10]);
        // 7 rows of 13 pt are reserved: a last page of 28 rows shares them with one more page
        assert_eq!(paginate(58, 30, SIGN_H), vec![30, 14, 14]);
        assert_eq!(paginate(58, 30, 0.0), vec![30, 28]);
        for n in 0..200 {
            let p = paginate(n, 30, SIGN_H);
            assert_eq!(p.iter().sum::<usize>(), n);
            assert!(*p.last().unwrap() <= 30 - 7, "{n}: {p:?}");
        }
    }

    #[test]
    fn subset_keeps_glyph_numbering() {
        let f = PdfFont::new(crate::theme::NOTO_THAI, "t");
        let g = f.glyph('ก');
        let keep: BTreeSet<u16> = [g].into_iter().collect();
        let sub = subset_ttf(f.data, &keep);
        let face = ttf_parser::Face::parse(&sub, 0).expect("subset parses");
        assert_eq!(face.number_of_glyphs(), f.face.number_of_glyphs());
        assert!(face.outline_glyph(ttf_parser::GlyphId(g), &mut NullOutline).is_some());
        assert!(sub.len() < f.data.len());
    }

    struct NullOutline;
    impl ttf_parser::OutlineBuilder for NullOutline {
        fn move_to(&mut self, _: f32, _: f32) {}
        fn line_to(&mut self, _: f32, _: f32) {}
        fn quad_to(&mut self, _: f32, _: f32, _: f32, _: f32) {}
        fn curve_to(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: f32) {}
        fn close(&mut self) {}
    }
}
