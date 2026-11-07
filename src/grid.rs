//! Spreadsheet-style task table and resource sheet.
//!
//! Rows follow the standard sheet: text styles per task category, cell formats from the Font group,
//! "Wrap Text" columns that grow a row by whole lines, gridlines from the Gridlines dialog and the
//! gray selected-row band with green lines above and below (the lines continue into the chart).

use crate::i18n::{tr, trs};
use crate::app::{App, EditState, RCol, TCol, View};
use crate::dialogs;
use crate::format::{GridKind, LineType, TextItem, TextStyle};
use crate::model::Project;
use crate::ribbon::Cmd;
use crate::theme::{self, paint_icon, Icon};
use eframe::egui::{self, text::LayoutJob, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, TextFormat, Ui, Vec2};

/// One sheet row unit (MPP with Calibri 11 at 100 %: 23 px).
pub const ROW_H: f32 = 23.0;
pub const HEADER_H: f32 = 40.0;
pub const ID_W: f32 = 34.0;
/// width of the vertical view-name bar left of the sheet (MPP: 25 px)
pub const VIEW_BAR_W: f32 = 25.0;
pub const SCROLLBAR: f32 = 14.0;
const MAX_WRAP_LINES: usize = 12;

/// Width that shows the column title and every cell of task column `k` on one line (Best Fit).
pub fn best_fit_width(app: &App, ctx: &egui::Context, k: usize) -> f32 {
    let Some(&(col, cur)) = app.task_cols.get(k) else { return 0.0 };
    if col == TCol::Indicators {
        return cur;
    }
    let measure = |s: &str, font: FontId| ctx.fonts_mut(|f| f.layout_no_wrap(crate::thai::shape(s).to_string(), font, theme::TEXT).size().x);
    let hst = app.doc.formatting.text_style(TextItem::RowColumnTitles);
    let mut hfont = theme::chrome_font(&hst);
    hfont.size = hfont.size.max(12.0);
    // title on one line + the filter arrow
    let mut w = measure(&app.col_title(col), hfont) + 4.0 + 18.0;
    if col != TCol::Mode {
        let numbers = if app.show_outline_number && col == TCol::Name { app.doc.outline_numbers() } else { Vec::new() };
        for i in 0..app.doc.tasks.len() {
            let t = &app.doc.tasks[i];
            let st = app.doc.formatting.text_style(text_item_of(&app.doc, i)).merged(&t.cell_format(col.title()));
            let font = theme::style_font(&st);
            let (s, extra) = if col == TCol::Name {
                let indent = if app.doc.options.indent_name { t.outline_level.saturating_sub(1) as f32 * 14.0 } else { 0.0 };
                let label = if numbers.is_empty() { t.name.clone() } else { format!("{} {}", numbers[i], t.name) };
                (label, 4.0 + indent + 14.0 + 8.0)
            } else {
                (app.cell_text(i, col), 12.0)
            };
            w = w.max(measure(&s, font) + extra);
        }
    }
    w.clamp(30.0, 900.0).ceil()
}

/// Height of the column header / timescale header (three timescale tiers need a taller header).
pub fn header_h(app: &App) -> f32 {
    let ts = &app.doc.formatting.timescale;
    // MPP: 40 px of titles plus its grey bottom line
    let (base, extra) = (HEADER_H + 1.0, 16.0);
    if !ts.auto && ts.show == 3 {
        base + extra
    } else {
        base
    }
}

/// Number of rows (tasks + trailing blank rows) the sheet scrolls over.
pub fn total_rows(app: &App) -> usize {
    match app.view {
        View::ResourceSheet => app.doc.resources.len() + 40,
        _ => app.visible_rows().len() + 40,
    }
}

/// Total scrollable height of the sheet body.
pub fn content_height(app: &App) -> f32 {
    match app.view {
        View::ResourceSheet => total_rows(app) as f32 * ROW_H,
        _ => app.row_top(total_rows(app)),
    }
}

pub fn clamp_scroll(app: &mut App, body_h: f32) {
    let max = (content_height(app) - body_h).max(0.0);
    app.scroll_y = app.scroll_y.clamp(0.0, max);
    if let Some(r) = app.scroll_to_row.take() {
        let (top, h) = match app.view {
            View::ResourceSheet => (r as f32 * ROW_H, ROW_H),
            _ => (app.row_top(r), app.row_height(r)),
        };
        if top < app.scroll_y {
            app.scroll_y = top;
        } else if top + h > app.scroll_y + body_h {
            app.scroll_y = top + h - body_h;
        }
    }
}

/// Text style item of a task row (Text Styles dialog categories).
pub fn text_item_of(doc: &Project, i: usize) -> TextItem {
    let t = &doc.tasks[i];
    if !t.active {
        TextItem::Inactive
    } else if t.external.is_some() {
        TextItem::External
    } else if t.marked {
        TextItem::Marked
    } else if t.is_summary {
        TextItem::Summary
    } else if t.is_milestone() {
        TextItem::Milestone
    } else if t.critical && doc.formatting.show_critical {
        TextItem::Critical
    } else {
        TextItem::Noncritical
    }
}

/// Effective text style of a cell: the row's text-style item layered with the cell's own format.
pub fn cell_style(app: &App, i: usize, col: TCol) -> TextStyle {
    let base = app.doc.formatting.text_style(text_item_of(&app.doc, i));
    base.merged(&app.doc.tasks[i].cell_format(col.title()))
}

/// Insert Column > Custom Fields: a submenu per type (Text, Number, ...) with each field's alias.
fn custom_field_menu(app: &App, ui: &mut Ui, present: &[TCol]) -> Option<TCol> {
    let mut picked = None;
    ui.separator();
    ui.menu_button(tr("Custom Fields"), |ui| {
        for k in crate::custom::CfKind::ALL {
            ui.menu_button(tr(k.label()), |ui| {
                egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                    for n in 1..=k.count() {
                        let f = crate::custom::CustomField::new(k, n);
                        let c = TCol::Custom(f);
                        let title = crate::custom::title(&app.doc, f, false);
                        let label = if title == f.name() { title } else { format!("{} ({})", f.name(), title) };
                        if !present.contains(&c) && ui.button(crate::thai::shape(&label).to_string()).clicked() {
                            picked = Some(c);
                            ui.close();
                        }
                    }
                });
            });
        }
    });
    picked
}

/// The project summary task's row (ID 0): the project rolled up, in the Project Summary Tasks
/// text style; it is read-only (Project Information changes the project).
fn project_summary_row(app: &App, ui: &Ui, p: &egui::Painter, r: Rect, xs: &[f32], id_w: f32) {
    let t = app.project_summary();
    let y = r.top();
    p.rect_filled(r, 0.0, Color32::WHITE);
    let idr = Rect::from_min_size(r.min, Vec2::new(id_w, ROW_H));
    p.rect_filled(idr, 0.0, theme::HEADER_BG);
    let head = app.doc.formatting.text_style(TextItem::RowColumnTitles);
    p.text(idr.center(), Align2::CENTER_CENTER, "0", theme::chrome_font(&head), head.color.map(theme::rgb).unwrap_or(theme::SHEET_HEAD_TEXT));
    let st = app.doc.formatting.text_style(TextItem::ProjectSummary);
    let color = st.color.map(theme::rgb).unwrap_or(theme::TEXT);
    for (c, (col, w)) in app.task_cols.iter().enumerate() {
        let cr = Rect::from_min_size(Pos2::new(xs[c], y), Vec2::new(*w, ROW_H));
        let cp = p.with_clip_rect(cr.intersect(r));
        match col {
            TCol::Indicators => {}
            TCol::Name => {
                let mut tx = cr.left() + 4.0;
                if app.doc.options.show_outline_symbol {
                    let c = Pos2::new(tx + 5.0, cr.center().y);
                    let tri = vec![c + Vec2::new(3.5, -3.5), c + Vec2::new(3.5, 3.5), c + Vec2::new(-3.5, 3.5)];
                    cp.add(egui::Shape::convex_polygon(tri, theme::TEXT, Stroke::NONE));
                }
                tx += 14.0;
                let label = if app.show_outline_number { format!("0 {}", t.name) } else { t.name.clone() };
                let g = galley(ui, &label, &st, color, None);
                cp.galley(Pos2::new(tx, y + ROW_H / 2.0 - g.size().y / 2.0), g, color);
            }
            _ => {
                let s = app.task_cell_text(&t, None, *col);
                let s = if matches!(col, TCol::ConstraintType | TCol::Critical | TCol::Status | TCol::FixedCostAccrual) { trs(s) } else { s };
                let g = galley(ui, &s, &st, color, None);
                let gx = match app.col_alignment(*col) {
                    1 => cr.center().x - g.size().x / 2.0,
                    2 => cr.right() - 4.0 - g.size().x,
                    _ => cr.left() + 4.0,
                };
                cp.galley(Pos2::new(gx, y + ROW_H / 2.0 - g.size().y / 2.0), g, color);
            }
        }
    }
    let g_rows = app.doc.formatting.gridline(GridKind::SheetRows);
    let g_cols = app.doc.formatting.gridline(GridKind::SheetColumns);
    let vcol = if g_cols.normal == LineType::None { LineType::Solid } else { g_cols.normal };
    for (k, &gx) in xs.iter().enumerate() {
        if k == 0 {
            p.line_segment([Pos2::new(gx + 0.5, y), Pos2::new(gx + 0.5, y + ROW_H)], Stroke::new(1.0_f32, theme::SHEET_ID_EDGE));
        } else {
            theme::styled_line(p, Pos2::new(gx, y), Pos2::new(gx, y + ROW_H), vcol, theme::rgb(g_cols.normal_color));
        }
    }
    let ht = if g_rows.normal == LineType::None { LineType::Solid } else { g_rows.normal };
    theme::styled_line(p, Pos2::new(r.left(), y + ROW_H), Pos2::new(xs[xs.len() - 1], y + ROW_H), ht, theme::rgb(g_rows.normal_color));
}

fn galley(ui: &Ui, text: &str, st: &TextStyle, color: Color32, wrap: Option<f32>) -> std::sync::Arc<egui::Galley> {
    let shaped = crate::thai::shape(text);
    let font = theme::style_font(st);
    // own line breaking for wrapped cells: egui would break Thai (no spaces) inside a syllable
    let shaped = match wrap {
        Some(w) => std::borrow::Cow::Owned(crate::thai::wrap(&shaped, w.max(10.0), |s| ui.painter().layout_no_wrap(s.to_string(), font.clone(), color).size().x)),
        None => shaped,
    };
    let mut job = LayoutJob::default();
    job.append(
        &shaped,
        0.0,
        TextFormat {
            font_id: theme::style_font(st),
            color,
            italics: st.italic,
            underline: if st.underline { Stroke::new(1.0_f32, color) } else { Stroke::NONE },
            strikethrough: if st.strike { Stroke::new(1.0_f32, color) } else { Stroke::NONE },
            ..Default::default()
        },
    );
    match wrap {
        Some(_) => job.wrap.max_rows = MAX_WRAP_LINES,
        None => job.wrap.max_rows = 1,
    }
    ui.painter().layout_job(job)
}

fn header_cell(app: &App, ui: &Ui, p: &egui::Painter, r: Rect, title: &str, selected: bool, align: u8) {
    p.rect_filled(r, 0.0, if selected { theme::SHEET_HEAD_SEL } else { theme::SHEET_HEAD_BG });
    // MPP: a grey bottom line (the separators are drawn on the column lines)
    p.line_segment([Pos2::new(r.left(), r.bottom() - 0.5), Pos2::new(r.right(), r.bottom() - 0.5)], Stroke::new(1.0_f32, theme::SHEET_HEAD_BOTTOM));
    if selected && !title.is_empty() {
        // MPP marks the selected column with a green rule under its title
        p.rect_filled(Rect::from_min_max(Pos2::new(r.left(), r.bottom() - 2.0), Pos2::new(r.right(), r.bottom())), 0.0, theme::ROW_SEL_LINE);
    }
    if title.is_empty() {
        return;
    }
    // MPP headers: title (wrapped to two lines when the column is narrow) plus a filter arrow
    let arrow = r.width() > 40.0;
    let text_r = if arrow { Rect::from_min_max(r.min + Vec2::new(4.0, 0.0), egui::pos2(r.right() - 14.0, r.bottom())) } else { r.shrink2(Vec2::new(3.0, 0.0)) };
    let st = app.doc.formatting.text_style(TextItem::RowColumnTitles);
    let mut job = LayoutJob::default();
    let hfont = theme::chrome_font(&st);
    // column titles wrap only between words ("Task / Mode"); a single word that does not fit is clipped
    // like in MPP instead of being split ("Pre / s")
    let measure = |s: &str| ui.painter().layout_no_wrap(s.to_string(), hfont.clone(), theme::TEXT).size().x;
    let shaped_title = crate::thai::shape(title);
    let wrapped = if shaped_title.contains(' ') { crate::thai::wrap_words(&shaped_title, text_r.width().max(10.0), measure) } else { shaped_title.to_string() };
    job.append(&wrapped, 0.0, TextFormat { font_id: hfont.clone(), color: st.color.map(theme::rgb).unwrap_or(theme::SHEET_HEAD_TEXT), ..Default::default() });
    job.wrap.max_rows = 2;
    let g = ui.painter().layout_job(job);
    let x = match align {
        1 => text_r.center().x - g.size().x / 2.0,
        2 => text_r.right() - g.size().x,
        _ => text_r.left(),
    };
    let clip = p.with_clip_rect(text_r);
    clip.galley(Pos2::new(x.max(text_r.left()), r.center().y - g.size().y / 2.0 + 1.0), g, theme::TEXT);
    if arrow {
        let c = egui::pos2(r.right() - 8.0, r.center().y + 1.0);
        p.add(egui::Shape::convex_polygon(vec![c + Vec2::new(-3.5, -2.0), c + Vec2::new(3.5, -2.0), c + Vec2::new(0.0, 2.5)], theme::TEXT, Stroke::NONE));
    }
}

/// Vertical scrollbar.
pub fn vscrollbar(app: &mut App, ui: &mut Ui, r: Rect, body_h: f32) {
    let p = ui.painter();
    p.rect_filled(r, 0.0, Color32::from_rgb(0xF0, 0xF0, 0xF0));
    let content = content_height(app).max(body_h);
    let frac = (body_h / content).clamp(0.05, 1.0);
    let thumb_h = (r.height() * frac).max(20.0);
    let max_scroll = (content - body_h).max(1.0);
    let t = app.scroll_y / max_scroll;
    let thumb = Rect::from_min_size(Pos2::new(r.left() + 2.0, r.top() + t * (r.height() - thumb_h)), Vec2::new(r.width() - 4.0, thumb_h));
    let resp = ui.interact(r, ui.scope_id().with("vscroll"), Sense::click_and_drag());
    p.rect_filled(thumb, 3.0, if resp.dragged() || resp.hovered() { Color32::from_rgb(0xA0, 0xA0, 0xA0) } else { Color32::from_rgb(0xC2, 0xC2, 0xC2) });
    if resp.dragged() {
        let dy = resp.drag_delta().y;
        app.scroll_y += dy / (r.height() - thumb_h).max(1.0) * max_scroll;
    } else if resp.clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            if pos.y < thumb.top() {
                app.scroll_y -= body_h;
            } else if pos.y > thumb.bottom() {
                app.scroll_y += body_h;
            }
        }
    }
    app.scroll_y = app.scroll_y.clamp(0.0, (content - body_h).max(0.0));
}

/// Rebuild `app.row_tops` from the wrapped text of the visible rows.
fn layout_rows(app: &mut App, ui: &Ui) {
    let vis = app.visible_rows();
    let mut tops = Vec::with_capacity(vis.len() + 1);
    let mut y = 0.0;
    // View > Group by: a header band before the first row of every group
    let mut bands = Vec::new();
    if app.group != crate::app::TaskGroup::None {
        let mut last: Option<i64> = None;
        for (r, &i) in vis.iter().enumerate() {
            let (k, label) = app.group_of(i);
            if last != Some(k) {
                bands.push((r, label));
                last = Some(k);
            }
        }
    }
    let mut band_iter = bands.iter().peekable();
    // the style and font of each kind of row, worked out once (rows without cell formats)
    let mut item_styles: Vec<(TextItem, TextStyle, FontId)> = Vec::new();
    for (r, &i) in vis.iter().enumerate() {
        if band_iter.peek().map(|b| b.0 == r).unwrap_or(false) {
            band_iter.next();
            y += crate::app::GROUP_H;
        }
        let t = &app.doc.tasks[i];
        let mut lines = 1usize;
        for (col, w) in app.task_cols.iter() {
            if !app.wrap_cols.contains(col) {
                continue;
            }
            let text: std::borrow::Cow<str> = if *col == TCol::Name { std::borrow::Cow::Borrowed(t.name.as_str()) } else { std::borrow::Cow::Owned(app.cell_text(i, *col)) };
            if text.is_empty() {
                continue;
            }
            let indent = if *col == TCol::Name && app.doc.options.indent_name { (t.outline_level.saturating_sub(1)) as f32 * 14.0 + 14.0 } else { 0.0 };
            let avail = *w - indent - 8.0;
            if avail < 12.0 {
                continue;
            }
            let own;
            let (st, font) = if t.cell_formats.is_empty() {
                let item = text_item_of(&app.doc, i);
                let k = match item_styles.iter().position(|e| e.0 == item) {
                    Some(k) => k,
                    None => {
                        let st = app.doc.formatting.text_style(item);
                        let font = theme::style_font(&st);
                        item_styles.push((item, st, font));
                        item_styles.len() - 1
                    }
                };
                (&item_styles[k].1, &item_styles[k].2)
            } else {
                let st = cell_style(app, i, *col);
                let font = theme::style_font(&st);
                own = (st, font);
                (&own.0, &own.1)
            };
            let key = {
                use std::hash::{Hash, Hasher};
                let mut h = std::collections::hash_map::DefaultHasher::new();
                (text.as_ref(), avail.to_bits(), font.size.to_bits(), &font.family).hash(&mut h);
                h.finish()
            };
            let n = match app.wrap_cache.get(&key) {
                Some(n) => *n,
                None => {
                    let n = galley(ui, &text, st, theme::TEXT, Some(avail)).rows.len().clamp(1, MAX_WRAP_LINES);
                    if app.wrap_cache.len() > 50_000 {
                        app.wrap_cache.clear();
                    }
                    app.wrap_cache.insert(key, n);
                    n
                }
            };
            lines = lines.max(n);
        }
        tops.push(y);
        y += lines as f32 * ROW_H;
    }
    tops.push(y);
    app.row_tops = tops;
    app.group_bands = bands;
}

/// Group header bands of the sheet: the standard yellow group rows with the group's rolled-up
/// duration, dates, work and cost.
fn draw_group_bands(app: &App, bp: &egui::Painter, body: Rect, xs: &[f32]) {
    let fill = Color32::from_rgb(0xFF, 0xF4, 0xB8);
    let edge = Color32::from_rgb(0xD8, 0xC8, 0x70);
    let st = app.doc.formatting.text_style(TextItem::All);
    let mut bst = st.clone();
    bst.bold = true;
    let bold = theme::style_font(&bst);
    let right = xs.last().copied().unwrap_or(body.right());
    for (k, (r, label)) in app.group_bands.iter().enumerate() {
        let y1 = body.top() + app.row_top(*r) - app.scroll_y;
        let y0 = y1 - crate::app::GROUP_H;
        if y1 < body.top() || y0 > body.bottom() {
            continue;
        }
        let br = Rect::from_min_max(Pos2::new(body.left(), y0), Pos2::new(right, y1));
        bp.rect_filled(br, 0.0, fill);
        bp.line_segment([Pos2::new(br.left(), y1 - 0.5), Pos2::new(br.right(), y1 - 0.5)], Stroke::new(1.0_f32, edge));
        let totals = app.group_totals(k);
        for (c, (col, w)) in app.task_cols.iter().enumerate() {
            let cr = Rect::from_min_size(Pos2::new(xs[c], y0), Vec2::new(*w, crate::app::GROUP_H));
            let cp = bp.with_clip_rect(cr.intersect(body));
            let text = match (col, totals) {
                (TCol::Name, _) => label.clone(),
                (TCol::Start, Some((s, _, _, _))) => app.fmt_date(s.date()),
                (TCol::Finish, Some((_, f, _, _))) => app.fmt_finish(f.date()),
                (TCol::Duration, Some((s, f, _, _))) => util_days(app, app.doc.calendar.work_minutes(s, f)),
                (TCol::Work, Some((_, _, w, _))) => crate::util::fmt_dur(w, crate::model::DurUnit::Hours, false, app.dur_units(), &app.doc.options, crate::i18n::is_thai()),
                (TCol::Cost, Some((_, _, _, c))) => crate::util::fmt_money(c),
                _ => String::new(),
            };
            if text.is_empty() {
                continue;
            }
            if *col == TCol::Name {
                cp.text(Pos2::new(cr.left() + 4.0, cr.center().y), Align2::LEFT_CENTER, crate::thai::shape(&text), bold.clone(), theme::TEXT);
            } else {
                cp.text(Pos2::new(cr.left() + 4.0, cr.center().y), Align2::LEFT_CENTER, text, bold.clone(), theme::TEXT);
            }
        }
    }
}

fn util_days(app: &App, m: i64) -> String {
    crate::util::fmt_dur(m.max(0), crate::model::DurUnit::Days, false, app.dur_units(), &app.doc.options, crate::i18n::is_thai())
}

// ======================================================================= task grid

pub fn task_grid(app: &mut App, ui: &mut Ui, rect: Rect, cmds: &mut Vec<Cmd>) {
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, Color32::WHITE);
    let hh = header_h(app);
    let header = Rect::from_min_size(rect.min, Vec2::new(rect.width(), hh));
    let body = Rect::from_min_max(Pos2::new(rect.left(), rect.top() + hh), rect.max);
    // Format > Project Summary Task: row 0 above the tasks
    let ps_band = app.shows_project_summary().then(|| Rect::from_min_size(body.min, Vec2::new(body.width(), ROW_H)));
    let body = match ps_band {
        Some(b) => Rect::from_min_max(Pos2::new(body.left(), b.bottom()), body.max),
        None => body,
    };
    let vis = app.visible_rows();
    let numbers = if app.show_outline_number { app.doc.outline_numbers() } else { Vec::new() };

    // column x positions
    let mut xs = Vec::with_capacity(app.task_cols.len() + 1);
    let id_w = app.id_w;
    // MPP: the ID column's edge is the first column's left line
    let id_overlap = 1.0;
    let mut x = rect.left() + id_w - id_overlap;
    for (_, w) in &app.task_cols {
        xs.push(x);
        x += *w;
    }
    xs.push(x);
    let add_col_rect = Rect::from_min_max(Pos2::new(x, rect.top()), Pos2::new(x + 110.0, rect.top() + hh));
    layout_rows(app, ui);

    // ---------------- header
    header_cell(app, ui, &p, Rect::from_min_size(header.min, Vec2::new(id_w - id_overlap, hh)), "", false, 1);
    for (k, (col, w)) in app.task_cols.iter().enumerate() {
        let r = Rect::from_min_size(Pos2::new(xs[k], rect.top()), Vec2::new(*w, hh));
        let talign = app.col_title_align.iter().find(|(c, _)| c == col).map(|(_, a)| *a).unwrap_or(if *col == TCol::Name { 0 } else { 1 });
        if *col == TCol::Indicators {
            header_cell(app, ui, &p, r, "", k == app.sel_col, 1);
            paint_icon(&p, Rect::from_center_size(r.center() + Vec2::new(0.0, 1.0), Vec2::splat(13.0)), Icon::Info, true);
        } else {
            let sel_head = {
                let (ca, cb) = app.sel_cols();
                k >= ca && k <= cb
            };
            header_cell(app, ui, &p, r, &app.col_title(*col), sel_head, talign);
        }
    }
    header_cell(app, ui, &p, add_col_rect, tr("Add New Column"), false, 0);
    if add_col_rect.right() < rect.right() {
        p.rect_filled(Rect::from_min_max(Pos2::new(add_col_rect.right(), rect.top()), Pos2::new(rect.right(), rect.top() + hh)), 0.0, theme::SHEET_HEAD_BG);
    }
    {
        // the standard title separators on the column lines: darker on both sides of the selected columns
        let (ca, cb) = app.sel_cols();
        let mut lines: Vec<f32> = xs.clone();
        lines.push(add_col_rect.right());
        for (k, lx) in lines.iter().enumerate() {
            let beside = (k >= ca && k <= cb + 1) && k < xs.len();
            let c = if beside { theme::SHEET_HEAD_SEP_SEL } else { theme::SHEET_HEAD_SEP };
            p.line_segment([Pos2::new(lx + 0.5, rect.top()), Pos2::new(lx + 0.5, rect.top() + hh - 1.0)], Stroke::new(1.0_f32, c));
        }
    }

    // header interactions: resize, context menu, add column
    let mut resize: Option<(usize, f32)> = None;
    for k in 0..app.task_cols.len() {
        let bx = xs[k + 1];
        let hr = Rect::from_center_size(Pos2::new(bx, header.center().y), Vec2::new(8.0, hh));
        let resp = ui.interact(hr, ui.scope_id().with(("colresize", k)), Sense::click_and_drag());
        if resp.hovered() || resp.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        if resp.dragged() {
            resize = Some((k, resp.drag_delta().x));
        }
        if resp.double_clicked() {
            resize = Some((k, f32::NAN));
        }
    }
    if let Some((k, dx)) = resize {
        if dx.is_nan() {
            // MPP: double-clicking a column's right border fits it to its contents
            app.task_cols[k].1 = best_fit_width(app, ui.ctx(), k);
        } else {
            app.task_cols[k].1 = (app.task_cols[k].1 + dx).max(16.0);
        }
    }
    for k in 0..app.task_cols.len() {
        let r = Rect::from_min_max(Pos2::new(xs[k] + 4.0, rect.top()), Pos2::new(xs[k + 1] - 4.0, rect.top() + hh));
        let resp = ui.interact(r, ui.scope_id().with(("colhdr", k)), Sense::click());
        if resp.clicked() || resp.secondary_clicked() {
            app.commit_edit();
            app.sel_col = k;
            app.sel_col_anchor = None;
            app.sel_whole_rows = false;
        }
        // MPP: double-clicking a column title opens Field Settings
        if resp.double_clicked() {
            cmds.push(Cmd::FieldSettings);
        }
        let col = app.task_cols[k].0;
        resp.context_menu(|ui| {
            ui.set_min_width(190.0);
            if ui.button(tr("Field Settings...")).clicked() {
                cmds.push(Cmd::FieldSettings);
                ui.close();
            }
            if ui.button(tr("Custom Fields...")).clicked() {
                cmds.push(Cmd::CustomFields);
                ui.close();
            }
            ui.separator();
            ui.menu_button("Insert Column", |ui| {
                let present: Vec<TCol> = app.task_cols.iter().map(|c| c.0).collect();
                for c in TCol::ALL {
                    if !present.contains(&c) && ui.button(tr(c.title())).clicked() {
                        app.task_cols.insert(k, (c, c.default_width()));
                        ui.close();
                    }
                }
                if let Some(c) = custom_field_menu(app, ui, &present) {
                    app.task_cols.insert(k, (c, c.default_width()));
                    ui.close();
                }
            });
            if ui.add_enabled(col != TCol::Name, egui::Button::new(tr("Hide Column"))).clicked() {
                app.task_cols.remove(k);
                app.sel_col = app.sel_col.min(app.task_cols.len().saturating_sub(1));
                ui.close();
                return;
            }
            ui.separator();
            let mut wrapped = app.wrap_cols.contains(&col);
            if ui.checkbox(&mut wrapped, tr("Wrap Text")).clicked() {
                cmds.push(Cmd::WrapText);
                ui.close();
            }
            for (a, l) in [(0u8, "Align Left"), (1, "Center"), (2, "Align Right")] {
                if ui.radio(app.col_alignment(col) == a, tr(l)).clicked() {
                    cmds.push(Cmd::AlignCol(a));
                    ui.close();
                }
            }
            ui.separator();
            if ui.button(tr("Font...")).clicked() {
                cmds.push(Cmd::FontDialog);
                ui.close();
            }
            if ui.button(tr("Text Styles...")).clicked() {
                cmds.push(Cmd::TextStyles);
                ui.close();
            }
        });
    }
    let add_resp = ui.interact(add_col_rect, ui.scope_id().with("addcol"), Sense::click());
    egui::Popup::menu(&add_resp).close_behavior(egui::PopupCloseBehavior::CloseOnClick).show(|ui| {
        ui.set_min_width(160.0);
        let present: Vec<TCol> = app.task_cols.iter().map(|c| c.0).collect();
        for c in TCol::ALL {
            if !present.contains(&c) && ui.button(tr(c.title())).clicked() {
                app.task_cols.push((c, c.default_width()));
            }
        }
        if let Some(c) = custom_field_menu(app, ui, &present) {
            app.task_cols.push((c, c.default_width()));
        }
    });

    if let Some(band) = ps_band {
        project_summary_row(app, ui, &p.with_clip_rect(band), band, &xs, id_w - id_overlap);
    }

    // ---------------- body
    clamp_scroll(app, body.height());
    let first = app.row_at(app.scroll_y);
    let (sa, sb) = (app.sel_anchor.min(app.sel_row), app.sel_anchor.max(app.sel_row));
    let bp = p.with_clip_rect(body);
    let sheet_right = xs[xs.len() - 1];
    let line_col = theme::rgb;
    let g_rows = app.doc.formatting.gridline(GridKind::SheetRows);
    let g_cols = app.doc.formatting.gridline(GridKind::SheetColumns);
    let single_sel = app.single_cell_selected();

    let mut row = first;
    loop {
        let y = body.top() + app.row_top(row) - app.scroll_y;
        if y > body.bottom() {
            break;
        }
        let rh = app.row_height(row);
        let rr = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(rect.width(), rh));
        let selected = row >= sa && row <= sb;
        // View > Highlight: MPP paints the matching rows' cells yellow
        if let Some(&ti) = vis.get(row) {
            if app.is_highlighted(ti) && xs.len() > 1 {
                bp.rect_filled(Rect::from_min_max(Pos2::new(xs[0], y), Pos2::new(xs[xs.len() - 1], y + rh)), 0.0, Color32::from_rgb(0xFF, 0xFF, 0x00));
            }
        }
        // ID cell
        let idr = Rect::from_min_size(rr.min, Vec2::new(id_w - id_overlap, rh));
        bp.rect_filled(idr, 0.0, if selected { if single_sel { theme::SHEET_ID_SEL } else { theme::ROW_SEL_ID } } else { theme::HEADER_BG });
        // MPP draws the row lines across the ID column too (the next row's fill covers ours)
        if y > body.top() + 0.5 && g_rows.normal != LineType::None {
            bp.line_segment([Pos2::new(idr.left(), y + 0.5), Pos2::new(idr.right() - 1.0, y + 0.5)], Stroke::new(1.0_f32, theme::rgb(g_rows.normal_color)));
        }
        if selected {
            // MPP: the selected cells are grey, the active cell stays white with its border
            let (ca, cb) = app.sel_cols();
            let shade = theme::ROW_SEL_CLASSIC;
            let (x0, x1) = (xs[ca], xs[(cb + 1).min(xs.len() - 1)]);
            if row == app.sel_row && app.sel_col >= ca && app.sel_col <= cb {
                let (a0, a1) = (xs[app.sel_col], xs[(app.sel_col + 1).min(xs.len() - 1)]);
                bp.rect_filled(Rect::from_min_max(Pos2::new(x0, y), Pos2::new(a0, y + rh)), 0.0, shade);
                bp.rect_filled(Rect::from_min_max(Pos2::new(a1, y), Pos2::new(x1, y + rh)), 0.0, shade);
            } else {
                bp.rect_filled(Rect::from_min_max(Pos2::new(x0, y), Pos2::new(x1, y + rh)), 0.0, shade);
            }
        }
        let task_idx = vis.get(row).copied();
        if let Some(i) = task_idx {
            let t = app.doc.tasks[i].clone();
            let item_style = app.doc.formatting.text_style(text_item_of(&app.doc, i));
            {
                // MPP draws the row numbers in the Row & Column Titles style, like the headers
                let st = app.doc.formatting.text_style(TextItem::RowColumnTitles);
                let color = st.color.map(theme::rgb).unwrap_or(theme::SHEET_HEAD_TEXT);
                bp.text(Pos2::new(idr.center().x, y + ROW_H / 2.0), Align2::CENTER_CENTER, (i + 1).to_string(), theme::chrome_font(&st), color);
            }
            for (c, (col, w)) in app.task_cols.iter().enumerate() {
                let cr = Rect::from_min_size(Pos2::new(xs[c], y), Vec2::new(*w, rh));
                let cp = bp.with_clip_rect(cr.intersect(body));
                let st = item_style.merged(&t.cell_format(col.title()));
                if let Some(bg) = st.background {
                    cp.rect_filled(cr, 0.0, theme::rgb(bg));
                }
                // (an inactive task and a task of another project are grey, as in MPP)
                let text_color = st.color.map(theme::rgb).unwrap_or(if !t.active || t.external.is_some() { theme::DISABLED } else { theme::TEXT });
                let align = app.col_alignment(*col);
                let wrap = app.wrap_cols.contains(col);
                match col {
                    TCol::Indicators => {
                        let mut ix = cr.left() + 3.0;
                        let iy = cr.top() + (ROW_H - 12.0) / 2.0;
                        if t.percent_complete == 100 && !t.is_summary {
                            cp.add(egui::Shape::line(
                                vec![Pos2::new(ix + 1.0, iy + 6.0), Pos2::new(ix + 4.0, iy + 10.0), Pos2::new(ix + 10.0, iy + 2.0)],
                                Stroke::new(1.8_f32, theme::GREEN),
                            ));
                            ix += 12.0;
                        }
                        if t.constraint_type.needs_date() && t.constraint_date.is_some() && !t.manual {
                            paint_icon(&cp, Rect::from_min_size(Pos2::new(ix, iy), Vec2::splat(12.0)), Icon::Calendar, true);
                            ix += 13.0;
                        }
                        if let Some(dl) = t.deadline {
                            if t.finish > dl && t.percent_complete < 100 {
                                // missed deadline: red diamond with an exclamation mark
                                let c = Pos2::new(ix + 6.0, iy + 6.0);
                                cp.add(egui::Shape::convex_polygon(vec![c + Vec2::new(0.0, -6.0), c + Vec2::new(6.0, 0.0), c + Vec2::new(0.0, 6.0), c + Vec2::new(-6.0, 0.0)], Color32::from_rgb(0xD1, 0x3B, 0x2E), Stroke::NONE));
                                cp.text(c, Align2::CENTER_CENTER, tr("!"), theme::bold(9.0), Color32::WHITE);
                                ix += 13.0;
                            }
                        }
                        if t.overallocated && !t.is_summary {
                            // the standard red person: an assigned resource is overallocated
                            let red = Color32::from_rgb(0xD1, 0x3B, 0x2E);
                            cp.circle_filled(Pos2::new(ix + 6.0, iy + 3.0), 2.6, red);
                            cp.rect_filled(Rect::from_min_max(Pos2::new(ix + 2.5, iy + 6.5), Pos2::new(ix + 9.5, iy + 12.0)), 2.5, red);
                            ix += 13.0;
                        }
                        if !t.notes.is_empty() {
                            paint_icon(&cp, Rect::from_min_size(Pos2::new(ix, iy), Vec2::splat(12.0)), Icon::Notes, true);
                        }
                    }
                    TCol::Mode => {
                        let ir = Rect::from_center_size(Pos2::new(cr.center().x, cr.top() + ROW_H / 2.0), Vec2::new(16.0, 14.0));
                        if t.manual {
                            cp.circle_filled(ir.center() - Vec2::new(0.0, 2.0), 3.5, Color32::from_rgb(0xC0, 0x39, 0x2B));
                            cp.line_segment([ir.center() + Vec2::new(0.0, 1.0), ir.center() + Vec2::new(-3.0, 7.0)], Stroke::new(1.3_f32, theme::TEXT));
                        } else {
                            paint_icon(&cp, ir, Icon::Auto, true);
                        }
                    }
                    TCol::Name => {
                        let indent = if app.doc.options.indent_name { (t.outline_level.saturating_sub(1)) as f32 * 14.0 } else { 0.0 };
                        let mut tx = cr.left() + 4.0 + indent;
                        if t.is_summary && app.doc.options.show_outline_symbol {
                            let c = Pos2::new(tx + 5.0, cr.top() + ROW_H / 2.0);
                            let tri = if t.collapsed {
                                vec![c + Vec2::new(-2.5, -4.5), c + Vec2::new(2.5, 0.0), c + Vec2::new(-2.5, 4.5)]
                            } else {
                                vec![c + Vec2::new(3.5, -3.5), c + Vec2::new(3.5, 3.5), c + Vec2::new(-3.5, 3.5)]
                            };
                            if t.collapsed {
                                // MPP: a collapsed group shows an outlined triangle
                                cp.add(egui::Shape::closed_line(tri, Stroke::new(1.0_f32, theme::SHEET_HEAD_TEXT)));
                            } else {
                                cp.add(egui::Shape::convex_polygon(tri, theme::TEXT, Stroke::NONE));
                            }
                        }
                        tx += 14.0;
                        let label = if app.show_outline_number { format!("{} {}", numbers[i], t.name) } else { t.name.clone() };
                        let g = galley(ui, &label, &st, text_color, if wrap { Some(cr.right() - tx - 4.0) } else { None });
                        let gx = match align {
                            1 => (tx + (cr.right() - tx) / 2.0 - g.size().x / 2.0).max(tx),
                            2 => (cr.right() - 4.0 - g.size().x).max(tx),
                            _ => tx,
                        };
                        let gy = if wrap { y + (ROW_H - g.rows.first().map(|r| r.rect().height()).unwrap_or(14.0)) / 2.0 } else { y + ROW_H / 2.0 - g.size().y / 2.0 };
                        cp.galley(Pos2::new(gx, gy), g, text_color);
                    }
                    _ => {
                        let s = app.cell_text(i, *col);
                        let s = if matches!(col, TCol::ConstraintType | TCol::Critical | TCol::Status | TCol::FixedCostAccrual) { trs(s) } else { s };
                        let g = galley(ui, &s, &st, text_color, if wrap { Some(cr.width() - 8.0) } else { None });
                        let gx = match align {
                            1 => cr.center().x - g.size().x / 2.0,
                            2 => cr.right() - 4.0 - g.size().x,
                            _ => cr.left() + 4.0,
                        };
                        let gy = if wrap { y + (ROW_H - g.rows.first().map(|r| r.rect().height()).unwrap_or(14.0)) / 2.0 } else { y + ROW_H / 2.0 - g.size().y / 2.0 };
                        cp.galley(Pos2::new(gx, gy), g, text_color);
                    }
                }
            }
        }
        // grid lines
        let vcol = if g_cols.normal == LineType::None { LineType::Solid } else { g_cols.normal };
        for (k, &gx) in xs.iter().enumerate() {
            if k == 0 {
                // the ID column's edge
                bp.line_segment([Pos2::new(gx + 0.5, y), Pos2::new(gx + 0.5, y + rh)], Stroke::new(1.0_f32, theme::SHEET_ID_EDGE));
            } else {
                theme::styled_line(&bp, Pos2::new(gx, y), Pos2::new(gx, y + rh), vcol, line_col(g_cols.normal_color));
            }
        }
        let (ht, hc) = if g_rows.interval > 1 && (row + 1) % g_rows.interval as usize == 0 && g_rows.interval_type != LineType::None {
            (g_rows.interval_type, g_rows.interval_color)
        } else {
            (if g_rows.normal == LineType::None { LineType::Solid } else { g_rows.normal }, g_rows.normal_color)
        };
        theme::styled_line(&bp, Pos2::new(rect.left(), y + rh), Pos2::new(sheet_right, y + rh), ht, line_col(hc));
        // one selected cell (MPP): grey row edges, the ID cell darker, a green frame over the
        // cell's own gridlines
        if single_sel && row == app.sel_row {
            for ly in [y + 0.5, y + rh + 0.5] {
                bp.line_segment([Pos2::new(idr.left(), ly), Pos2::new(idr.right() - 1.0, ly)], Stroke::new(1.0_f32, theme::SHEET_ID_SEL_LINE));
                bp.line_segment([Pos2::new(idr.right(), ly), Pos2::new(sheet_right, ly)], Stroke::new(1.0_f32, theme::SHEET_SEL_ROW_LINE));
            }
        } else if selected {
            if row == sa {
                bp.line_segment([Pos2::new(rect.left(), y + 0.5), Pos2::new(rect.right(), y + 0.5)], Stroke::new(2.0_f32, theme::ROW_SEL_LINE));
            }
            if row == sb {
                bp.line_segment([Pos2::new(rect.left(), y + rh - 0.5), Pos2::new(rect.right(), y + rh - 0.5)], Stroke::new(2.0_f32, theme::ROW_SEL_LINE));
            }
        }
        // active cell border
        if row == app.sel_row && app.editing.is_none() && app.sel_col < app.task_cols.len() && app.grid_has_focus {
            // MPP: 2 px over the cell's gridlines and one pixel outside them
            let cr = Rect::from_min_max(Pos2::new(xs[app.sel_col] - 1.0, y), Pos2::new(xs[app.sel_col + 1] + 1.0, y + rh + 1.0));
            bp.rect_stroke(cr, 0.0, Stroke::new(2.0_f32, theme::CELL_SEL_BORDER), egui::StrokeKind::Middle);
        }
        row += 1;
        if row > first + 400 {
            break;
        }
    }
    if !app.group_bands.is_empty() {
        draw_group_bands(app, &bp, body, &xs);
    }

    // ---------------- body interaction
    let resp = ui.interact(body, ui.scope_id().with("grid_body"), Sense::click_and_drag());
    let ncols = app.task_cols.len();
    let xs_hit = xs.clone();
    let hit = |app: &App, pos: Pos2| -> (usize, Option<usize>) {
        let row = app.row_at(pos.y - body.top() + app.scroll_y);
        let col = (0..ncols).find(|&c| pos.x >= xs_hit[c] && pos.x < xs_hit[c + 1]);
        (row, col)
    };
    if resp.clicked() || resp.drag_started() || resp.secondary_clicked() {
        app.grid_has_focus = true;
        app.drawing_sel = None;
        ui.memory_mut(|m| m.surrender_focus(egui::Id::unique("cell_editor")));
        if let Some(pos) = resp.interact_pointer_pos() {
            let (row, col) = hit(app, pos);
            let shift = ui.input(|i| i.modifiers.shift);
            let was_same = row == app.sel_row && col == Some(app.sel_col) && app.editing.is_none() && !shift;
            app.commit_edit();
            let vis_now = app.visible_rows();
            // collapse triangle
            let mut toggled = false;
            if let (Some(c), Some(&i)) = (col, vis_now.get(row)) {
                if app.task_cols[c].0 == TCol::Name && app.doc.tasks[i].is_summary {
                    let indent = if app.doc.options.indent_name { (app.doc.tasks[i].outline_level.saturating_sub(1)) as f32 * 14.0 } else { 0.0 };
                    let tri_x = xs[c] + 4.0 + indent;
                    if pos.x >= tri_x - 2.0 && pos.x <= tri_x + 13.0 && resp.clicked() {
                        app.toggle_collapse(i);
                        toggled = true;
                    }
                }
            }
            if !toggled {
                if resp.secondary_clicked() && row >= sa && row <= sb {
                    // keep multi selection for context menu
                } else {
                    app.sel_row = row;
                    if !shift {
                        app.sel_anchor = row;
                    }
                    // a press on the row numbers selects whole rows; on a cell, a cell range starts
                    app.sel_whole_rows = col.is_none();
                    if shift && app.sel_col_anchor.is_none() {
                        app.sel_col_anchor = Some(app.sel_col);
                    }
                    if let (Some(c), false) = (col, shift) {
                        app.sel_col_anchor = Some(c);
                    }
                }
                if let Some(c) = col {
                    app.sel_col = c;
                    if app.format_painter.is_some() {
                        // Format Painter: the clicked cell (or row) takes the formatting
                        if resp.clicked() {
                            app.sel_col_anchor = None;
                            app.paint_format();
                        }
                    } else if was_same && resp.clicked() && app.task_cols[c].0.editable() {
                        start_edit(app, row, c, None);
                    }
                } else if app.format_painter.is_some() && resp.clicked() {
                    app.paint_format();
                }
            }
        }
    }
    if resp.dragged() && !resp.drag_started() {
        if let Some(pos) = ui.ctx().pointer_interact_pos() {
            let (row, col) = hit(app, pos);
            app.sel_row = row;
            if let (Some(c), false) = (col, app.sel_whole_rows) {
                app.sel_col = c;
            }
            if pos.y > body.bottom() - 4.0 {
                app.scroll_y += ROW_H / 2.0;
            } else if pos.y < body.top() + 4.0 {
                app.scroll_y -= ROW_H / 2.0;
            }
        }
    }
    if resp.double_clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let (row, col) = hit(app, pos);
            app.editing = None;
            if row < vis.len() {
                let on_name_tri = col.map(|c| app.task_cols[c].0 == TCol::Name).unwrap_or(false) && app.doc.tasks[vis[row]].is_summary && {
                    let c = col.unwrap();
                    let indent = if app.doc.options.indent_name { (app.doc.tasks[vis[row]].outline_level.saturating_sub(1)) as f32 * 14.0 } else { 0.0 };
                    pos.x <= xs[c] + 4.0 + indent + 13.0
                };
                if !on_name_tri {
                    dialogs::open_task_info(app, 0);
                }
            } else if let Some(c) = col {
                start_edit(app, row, c, None);
            }
        }
    }
    resp.context_menu(|ui| {
        ui.set_min_width(200.0);
        let has = app.current_task().is_some();
        let b = |ui: &mut Ui, enabled: bool, label: &str, cmds: &mut Vec<Cmd>, cmd: Cmd| {
            if ui.add_enabled(enabled, egui::Button::new(tr(label))).clicked() {
                cmds.push(cmd);
                ui.close();
            }
        };
        b(ui, has, "Cut Cell", cmds, Cmd::Cut);
        b(ui, has, "Copy Cell", cmds, Cmd::Copy);
        b(ui, !app.clipboard.is_empty(), "Paste", cmds, Cmd::Paste);
        ui.separator();
        b(ui, has, "Scroll to Task", cmds, Cmd::ScrollToTask);
        b(ui, true, "Insert Task", cmds, Cmd::InsertTask);
        if ui.add_enabled(has, egui::Button::new(tr("Delete Task"))).clicked() {
            app.delete_selected();
            ui.close();
        }
        b(ui, has, "Inactivate Task", cmds, Cmd::Inactivate);
        ui.separator();
        b(ui, has, "Manually Schedule", cmds, Cmd::Manual);
        b(ui, has, "Auto Schedule", cmds, Cmd::Auto);
        ui.separator();
        b(ui, has, "Assign Resources...", cmds, Cmd::AssignResources);
        b(ui, has, "Fill Down", cmds, Cmd::FillDown);
        b(ui, has, "Clear Contents", cmds, Cmd::Clear);
        ui.separator();
        b(ui, has, "Information...", cmds, Cmd::TaskInfo);
        b(ui, has, "Notes...", cmds, Cmd::Notes);
        b(ui, has, "Add to Timeline", cmds, Cmd::AddToTimeline);
        ui.separator();
        b(ui, has, "Font...", cmds, Cmd::FontDialog);
        b(ui, true, "Text Styles...", cmds, Cmd::TextStyles);
    });

    // ---------------- editor overlay
    draw_editor(app, ui, body, |c| (xs[c], xs[c + 1]), EditorKind::Task);

    // right border
    p.line_segment([rect.right_top(), rect.right_bottom()], Stroke::new(1.0_f32, theme::RIBBON_BORDER));
}

pub fn start_edit(app: &mut App, row: usize, col: usize, initial: Option<String>) {
    let text = match app.view {
        View::ResourceSheet => {
            if row < app.doc.resources.len() {
                app.res_edit_text(row, app.res_cols[col].0)
            } else {
                String::new()
            }
        }
        _ => {
            let vis = app.visible_rows();
            let tcol = app.task_cols[col].0;
            if !tcol.editable() {
                return;
            }
            match vis.get(row) {
                Some(&i) => {
                    if app.doc.tasks[i].is_summary && matches!(tcol, TCol::Duration | TCol::Start | TCol::Finish) {
                        return;
                    }
                    app.edit_text(i, tcol)
                }
                None => String::new(),
            }
        }
    };
    let select_all = initial.is_none();
    app.editing = Some(EditState { row, col, text: initial.unwrap_or(text), just_opened: true, select_all });
}

enum EditorKind {
    Task,
    Resource,
}

fn draw_editor(app: &mut App, ui: &mut Ui, body: Rect, col_x: impl Fn(usize) -> (f32, f32), kind: EditorKind) {
    let Some(ed) = app.editing.clone() else { return };
    let (top, h) = match kind {
        EditorKind::Task => (app.row_top(ed.row), app.row_height(ed.row)),
        EditorKind::Resource => (ed.row as f32 * ROW_H, ROW_H),
    };
    let y = body.top() + top - app.scroll_y;
    if y + h < body.top() || y > body.bottom() {
        return;
    }
    let (x0, x1) = col_x(ed.col);
    let cell = Rect::from_min_max(Pos2::new(x0, y), Pos2::new(x1.max(x0 + 60.0), y + h));
    let id = egui::Id::unique("cell_editor");

    // drop-down choice columns
    let choices: Option<Vec<String>> = match kind {
        EditorKind::Task if app.task_cols[ed.col].0 == TCol::Mode => Some(vec!["Manually Scheduled".into(), "Auto Scheduled".into()]),
        EditorKind::Task if app.task_cols[ed.col].0 == TCol::ConstraintType => Some(crate::model::ConstraintType::ALL.iter().map(|c| c.label().to_string()).collect()),
        EditorKind::Resource if app.res_cols[ed.col].0 == RCol::Type => Some(vec!["Work".into(), "Material".into(), "Cost".into()]),
        _ => None,
    };
    if let Some(ch) = choices {
        let p = ui.painter();
        p.rect_filled(cell, 0.0, Color32::WHITE);
        p.rect_stroke(cell, 0.0, Stroke::new(1.5_f32, theme::CELL_SEL_BORDER), egui::StrokeKind::Inside);
        p.text(cell.left_center() + Vec2::new(4.0, 0.0), Align2::LEFT_CENTER, &ed.text, FontId::proportional(theme::UI_SIZE), theme::TEXT);
        let mut chosen = None;
        let area = egui::Area::new(egui::Id::unique("choice_popup")).order(egui::Order::Foreground).fixed_pos(cell.left_bottom()).show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(cell.width().max(150.0));
                for c in &ch {
                    if ui.selectable_label(ed.text == *c, c.as_str()).clicked() {
                        chosen = Some(c.to_string());
                    }
                }
            });
        });
        let esc = ui.input(|i| i.key_pressed(egui::Key::Escape));
        if let Some(c) = chosen {
            if let Some(e) = app.editing.as_mut() {
                e.text = c;
            }
            app.commit_edit();
        } else if esc {
            app.editing = None;
        } else if ui.input(|i| i.pointer.any_pressed()) {
            let pos = ui.input(|i| i.pointer.interact_pos());
            if let Some(pos) = pos {
                if !area.response.rect.contains(pos) && !cell.contains(pos) {
                    app.editing = None;
                }
            }
        }
        return;
    }

    let mut text = ed.text.clone();
    let resp = ui.put(
        cell,
        egui::TextEdit::singleline(&mut text).id(id).font(FontId::proportional(theme::UI_SIZE)).margin(Vec2::new(3.0, 2.0)).desired_width(cell.width()),
    );
    ui.painter().rect_stroke(cell, 0.0, Stroke::new(1.5_f32, theme::CELL_SEL_BORDER), egui::StrokeKind::Inside);
    if ed.just_opened {
        resp.request_focus();
        if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), id) {
            let n = text.chars().count();
            let from = if ed.select_all { 0 } else { n };
            state.cursor.set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(from), egui::text::CCursor::new(n))));
            state.store(ui.ctx(), id);
        }
    }
    let (enter, tab, esc) = ui.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Tab), i.key_pressed(egui::Key::Escape)));
    if let Some(e) = app.editing.as_mut() {
        e.text = text;
        e.just_opened = false;
    }
    if esc {
        app.editing = None;
        return;
    }
    if resp.lost_focus() || (!ed.just_opened && !resp.has_focus()) {
        let row = ed.row;
        app.commit_edit();
        // the key that ended editing must not be handled again by the sheet navigation
        ui.input_mut(|i| {
            i.consume_key(egui::Modifiers::NONE, egui::Key::Enter);
            i.consume_key(egui::Modifiers::NONE, egui::Key::Tab);
        });
        if enter {
            match kind {
                EditorKind::Task => {
                    app.sel_row = row + 1;
                    app.sel_anchor = app.sel_row;
                    app.scroll_to_row = Some(app.sel_row);
                }
                EditorKind::Resource => app.res_sel_row = row + 1,
            }
        } else if tab {
            match kind {
                EditorKind::Task => app.sel_col = (app.sel_col + 1).min(app.task_cols.len() - 1),
                EditorKind::Resource => app.res_sel_col = (app.res_sel_col + 1).min(app.res_cols.len() - 1),
            }
        }
    }
}

// ======================================================================= resource sheet

pub fn resource_grid(app: &mut App, ui: &mut Ui, rect: Rect, cmds: &mut Vec<Cmd>) {
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, Color32::WHITE);
    let body = Rect::from_min_max(Pos2::new(rect.left(), rect.top() + HEADER_H), rect.max);
    let mut xs = Vec::new();
    let mut x = rect.left() + ID_W;
    for (_, w) in &app.res_cols {
        xs.push(x);
        x += *w;
    }
    xs.push(x);
    header_cell(app, ui, &p, Rect::from_min_size(rect.min, Vec2::new(ID_W, HEADER_H)), "", false, 1);
    for (k, (col, w)) in app.res_cols.iter().enumerate() {
        header_cell(app, ui, &p, Rect::from_min_size(Pos2::new(xs[k], rect.top()), Vec2::new(*w, HEADER_H)), tr(col.title()), k == app.res_sel_col, 1);
    }
    if x < rect.right() {
        p.rect_filled(Rect::from_min_max(Pos2::new(x, rect.top()), Pos2::new(rect.right(), rect.top() + HEADER_H)), 0.0, if app.settings.classic_ui { theme::SHEET_HEADER_CLASSIC } else { theme::HEADER_BG });
    }
    // header menu: Insert Column / Hide Column (the standard column menu)
    let hresp = ui.interact(Rect::from_min_size(rect.min, Vec2::new(rect.width(), HEADER_H)), ui.scope_id().with("res_header"), Sense::click());
    let hover_col = ui.ctx().pointer_hover_pos().and_then(|pos| (0..app.res_cols.len()).find(|&k| pos.x >= xs[k] && pos.x < xs[k + 1]));
    hresp.context_menu(|ui| {
        ui.set_min_width(180.0);
        ui.menu_button(tr("Insert Column"), |ui| {
            let present: Vec<RCol> = app.res_cols.iter().map(|c| c.0).collect();
            for c in RCol::ALL {
                if !present.contains(&c) && ui.button(tr(c.title())).clicked() {
                    let at = hover_col.map(|k| k + 1).unwrap_or(app.res_cols.len()).min(app.res_cols.len());
                    app.res_cols.insert(at, (c, c.default_width()));
                    ui.close();
                }
            }
        });
        if let Some(k) = hover_col {
            if ui.add_enabled(app.res_cols[k].0 != RCol::Name, egui::Button::new(tr("Hide Column"))).clicked() {
                app.res_cols.remove(k);
                ui.close();
            }
        }
    });
    for k in 0..app.res_cols.len() {
        let hr = Rect::from_center_size(Pos2::new(xs[k + 1], rect.top() + HEADER_H / 2.0), Vec2::new(8.0, HEADER_H));
        let resp = ui.interact(hr, ui.scope_id().with(("rcolresize", k)), Sense::drag());
        if resp.hovered() || resp.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        if resp.dragged() {
            app.res_cols[k].1 = (app.res_cols[k].1 + resp.drag_delta().x).max(20.0);
        }
    }

    clamp_scroll(app, body.height());
    let first = (app.scroll_y / ROW_H).floor() as usize;
    let offset = app.scroll_y - first as f32 * ROW_H;
    let rows_fit = (body.height() / ROW_H).ceil() as usize + 1;
    let bp = p.with_clip_rect(body);
    let st = app.doc.formatting.text_style(TextItem::All);
    let font = theme::style_font(&st);
    for k in 0..rows_fit {
        let row = first + k;
        let y = body.top() + k as f32 * ROW_H - offset;
        let selected = row == app.res_sel_row;
        let idr = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(ID_W, ROW_H));
        bp.rect_filled(idr, 0.0, if selected { theme::ROW_SEL_ID } else { theme::HEADER_BG });
        if selected {
            bp.rect_filled(Rect::from_min_max(Pos2::new(rect.left() + ID_W, y), Pos2::new(xs[xs.len() - 1], y + ROW_H)), 0.0, theme::ROW_SEL_CLASSIC);
        }
        if row < app.doc.resources.len() {
            bp.text(Pos2::new(idr.center().x, idr.center().y), Align2::CENTER_CENTER, (row + 1).to_string(), FontId::proportional(12.0), theme::TEXT);
            for (c, (col, w)) in app.res_cols.iter().enumerate() {
                let cr = Rect::from_min_size(Pos2::new(xs[c], y), Vec2::new(*w, ROW_H));
                let cp = bp.with_clip_rect(cr.intersect(body));
                let s = app.res_cell_text(row, *col);
                let s = if matches!(col, RCol::AccrueAt | RCol::Type) { crate::i18n::tr(&s).to_string() } else { s };
                let ink = if app.doc.resources[row].overallocated { Color32::from_rgb(0xE0, 0x1E, 0x1E) } else { theme::TEXT };
                let right = matches!(col, RCol::Max | RCol::StdRate | RCol::OvtRate | RCol::CostUse | RCol::Work | RCol::Cost);
                if right {
                    cp.text(Pos2::new(cr.right() - 4.0, cr.center().y), Align2::RIGHT_CENTER, s, font.clone(), ink);
                } else {
                    cp.text(Pos2::new(cr.left() + 4.0, cr.center().y), Align2::LEFT_CENTER, crate::thai::shape(&s), font.clone(), ink);
                }
            }
        }
        for &gx in xs.iter() {
            bp.line_segment([Pos2::new(gx, y), Pos2::new(gx, y + ROW_H)], Stroke::new(1.0_f32, theme::GRID_LINE));
        }
        bp.line_segment([Pos2::new(rect.left(), y + ROW_H), Pos2::new(xs[xs.len() - 1], y + ROW_H)], Stroke::new(1.0_f32, theme::GRID_LINE));
        if selected {
            bp.line_segment([Pos2::new(rect.left(), y + 0.5), Pos2::new(rect.right(), y + 0.5)], Stroke::new(2.0_f32, theme::ROW_SEL_LINE));
            bp.line_segment([Pos2::new(rect.left(), y + ROW_H - 0.5), Pos2::new(rect.right(), y + ROW_H - 0.5)], Stroke::new(2.0_f32, theme::ROW_SEL_LINE));
        }
        if selected && app.editing.is_none() {
            let cr = Rect::from_min_max(Pos2::new(xs[app.res_sel_col], y), Pos2::new(xs[app.res_sel_col + 1], y + ROW_H));
            bp.rect_stroke(cr.shrink(0.5), 0.0, Stroke::new(2.0_f32, theme::CELL_SEL_BORDER), egui::StrokeKind::Inside);
        }
    }

    let resp = ui.interact(body, ui.scope_id().with("res_body"), Sense::click());
    if resp.clicked() || resp.secondary_clicked() {
        app.grid_has_focus = true;
        if let Some(pos) = resp.interact_pointer_pos() {
            let row = ((pos.y - body.top() + app.scroll_y) / ROW_H).floor().max(0.0) as usize;
            let col = (0..app.res_cols.len()).find(|&c| pos.x >= xs[c] && pos.x < xs[c + 1]);
            let same = row == app.res_sel_row && col == Some(app.res_sel_col) && app.editing.is_none();
            app.commit_edit();
            app.res_sel_row = row;
            if let Some(c) = col {
                app.res_sel_col = c;
                if same && resp.clicked() && app.res_cols[c].0 != RCol::Calendar {
                    start_edit(app, row, c, None);
                }
            }
        }
    }
    if resp.double_clicked() {
        app.editing = None;
        if app.res_sel_row < app.doc.resources.len() {
            cmds.push(Cmd::ResourceInfo);
        } else {
            start_edit(app, app.res_sel_row, app.res_sel_col, None);
        }
    }
    resp.context_menu(|ui| {
        let has = app.res_sel_row < app.doc.resources.len();
        if ui.button(tr("New Work Resource")).clicked() {
            cmds.push(Cmd::AddResource(crate::model::ResourceType::Work));
            ui.close();
        }
        if ui.add_enabled(has, egui::Button::new(tr("Delete Resource"))).clicked() {
            cmds.push(Cmd::DeleteResource);
            ui.close();
        }
        if ui.add_enabled(has, egui::Button::new(tr("Information…"))).clicked() {
            cmds.push(Cmd::ResourceInfo);
            ui.close();
        }
    });
    draw_editor(app, ui, body, |c| (xs[c], xs[c + 1]), EditorKind::Resource);
}

// ======================================================================= keyboard

/// Keyboard handling for the sheets when no text field has focus.
pub fn handle_keys(app: &mut App, ctx: &egui::Context, cmds: &mut Vec<Cmd>) {
    if app.dialog.is_some() || app.backstage.is_some() || app.editing.is_some() {
        return;
    }
    if ctx.memory(|m| m.focused().is_some()) {
        return;
    }
    let events = ctx.input(|i| i.events.clone());
    let mods = ctx.input(|i| i.modifiers);
    let res = app.view == View::ResourceSheet;
    let ncols = if res { app.res_cols.len() } else { app.task_cols.len() };
    for ev in events {
        match ev {
            egui::Event::Key { key, pressed: true, modifiers, .. } => {
                use egui::Key;
                let (row, col) = if res { (app.res_sel_row, app.res_sel_col) } else { (app.sel_row, app.sel_col) };
                let set = |app: &mut App, r: usize, c: usize, extend: bool| {
                    if res {
                        app.res_sel_row = r;
                        app.res_sel_col = c;
                    } else {
                        if extend && app.sel_col_anchor.is_none() {
                            app.sel_col_anchor = Some(app.sel_col);
                        }
                        app.sel_row = r;
                        app.sel_col = c;
                        if !extend {
                            app.sel_anchor = r;
                            app.sel_col_anchor = Some(c);
                            app.sel_whole_rows = false;
                        }
                        app.scroll_to_row = Some(r);
                    }
                };
                match key {
                    Key::ArrowDown if modifiers.alt && modifiers.shift => cmds.push(Cmd::MoveDown),
                    Key::ArrowUp if modifiers.alt && modifiers.shift => cmds.push(Cmd::MoveUp),
                    Key::ArrowRight if modifiers.alt && modifiers.shift => cmds.push(Cmd::Indent),
                    Key::ArrowLeft if modifiers.alt && modifiers.shift => cmds.push(Cmd::Outdent),
                    Key::ArrowDown => set(app, row + 1, col, modifiers.shift),
                    Key::ArrowUp => set(app, row.saturating_sub(1), col, modifiers.shift),
                    Key::ArrowRight => set(app, row, (col + 1).min(ncols - 1), modifiers.shift),
                    Key::ArrowLeft => set(app, row, col.saturating_sub(1), modifiers.shift),
                    Key::PageDown => set(app, row + 20, col, modifiers.shift),
                    Key::PageUp => set(app, row.saturating_sub(20), col, modifiers.shift),
                    Key::Home if modifiers.ctrl => set(app, 0, col, false),
                    Key::Home => set(app, row, 0, false),
                    Key::End => set(app, row, ncols - 1, false),
                    Key::Enter => set(app, row + 1, col, false),
                    Key::Tab => set(app, row, if modifiers.shift { col.saturating_sub(1) } else { (col + 1).min(ncols - 1) }, false),
                    Key::F2 if modifiers.ctrl && modifiers.shift => cmds.push(Cmd::Unlink),
                    Key::F2 if modifiers.ctrl => cmds.push(Cmd::Link),
                    Key::F2 if modifiers.shift => {
                        if !res {
                            cmds.push(Cmd::TaskInfo)
                        } else {
                            cmds.push(Cmd::ResourceInfo)
                        }
                    }
                    Key::F2 => start_edit(app, row, col, None),
                    Key::F9 => cmds.push(Cmd::Calculate),
                    Key::Insert => {
                        if res {
                            cmds.push(Cmd::AddResource(crate::model::ResourceType::Work))
                        } else {
                            cmds.push(Cmd::InsertTask)
                        }
                    }
                    Key::Delete if app.drawing_sel.is_some() => {}
                    Key::Delete => {
                        if res {
                            cmds.push(Cmd::DeleteResource)
                        } else {
                            app.delete_selected()
                        }
                    }
                    Key::D if modifiers.ctrl => cmds.push(Cmd::FillDown),
                    Key::F7 => cmds.push(Cmd::Spelling),
                    Key::F if modifiers.ctrl => cmds.push(Cmd::Find),
                    Key::B if modifiers.ctrl && !res => cmds.push(Cmd::Bold),
                    Key::I if modifiers.ctrl && !res => cmds.push(Cmd::Italic),
                    Key::U if modifiers.ctrl && !res => cmds.push(Cmd::Underline),
                    Key::A if modifiers.ctrl && !res => {
                        app.sel_whole_rows = true;
                        app.sel_anchor = 0;
                        app.sel_row = app.visible_rows().len().saturating_sub(1);
                    }
                    _ => {}
                }
            }
            egui::Event::Copy if !res => cmds.push(Cmd::Copy),
            egui::Event::Cut if !res => cmds.push(Cmd::Cut),
            egui::Event::Paste(_) if !res => cmds.push(Cmd::Paste),
            egui::Event::Text(t) if !mods.ctrl && !mods.alt => {
                let (row, col) = if res { (app.res_sel_row, app.res_sel_col) } else { (app.sel_row, app.sel_col) };
                if col < ncols {
                    // gather every character typed this frame; the editor only receives input from the next frame
                    let mut typed = t;
                    for later in ctx.input(|i| i.events.clone()).into_iter().skip_while(|e| !matches!(e, egui::Event::Text(_))).skip(1) {
                        if let egui::Event::Text(more) = later {
                            typed.push_str(&more);
                        }
                    }
                    start_edit(app, row, col, Some(typed));
                }
                break;
            }
            _ => {}
        }
    }
}
