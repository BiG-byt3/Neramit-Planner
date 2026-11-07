//! Task Usage and Resource Usage views: a sheet of tasks (or resources) with their assignments
//! under them, and beside it the work of every row in each unit of the timescale (the standard
//! timephased "Work" grid). Overallocated resource cells are red, as in MPP.

use crate::app::App;
use crate::format::{GridKind, TextItem};
use crate::grid::ROW_H;
use crate::i18n::tr;
use crate::model::ResourceType;
use crate::ribbon::Cmd;
use crate::theme;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use eframe::egui::{self, Align2, Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};

/// A row of a Usage view.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum URow {
    /// task index
    Task(usize),
    /// task index, assignment index
    Asg(usize, usize),
    /// resource index
    Res(usize),
    /// tasks without resources (Resource Usage)
    Unassigned,
}

/// The rows of the view in order.
pub fn rows(app: &App, resources: bool) -> Vec<URow> {
    let mut v = Vec::new();
    if !resources {
        for i in app.visible_rows() {
            v.push(URow::Task(i));
            let t = &app.doc.tasks[i];
            if !app.usage_collapsed.contains(&(false, t.uid)) {
                for k in 0..t.assignments.len() {
                    v.push(URow::Asg(i, k));
                }
            }
        }
        return v;
    }
    let unassigned: Vec<usize> = (0..app.doc.tasks.len()).filter(|&i| !app.doc.tasks[i].is_summary && app.doc.tasks[i].assignments.is_empty()).collect();
    if !unassigned.is_empty() {
        v.push(URow::Unassigned);
        if !app.usage_collapsed.contains(&(true, 0)) {
            v.extend(unassigned.into_iter().map(URow::Task));
        }
    }
    for (ri, r) in app.doc.resources.iter().enumerate() {
        v.push(URow::Res(ri));
        if app.usage_collapsed.contains(&(true, r.uid)) {
            continue;
        }
        for (i, t) in app.doc.tasks.iter().enumerate() {
            if let Some(k) = t.assignments.iter().position(|a| a.res_uid == r.uid) {
                v.push(URow::Asg(i, k));
            }
        }
    }
    v
}

/// Working minutes of task `i` inside [a, b) and its whole span (calendar minutes when elapsed).
fn task_share(app: &App, i: usize, a: NaiveDateTime, b: NaiveDateTime) -> (i64, i64) {
    let t = &app.doc.tasks[i];
    let cal = &app.doc.calendar;
    let (Some(st), Some(ft)) = (t.start_dt, t.finish_dt) else { return (0, 0) };
    let span = if t.dur_elapsed { (ft - st).num_minutes() } else { cal.work_minutes(st, ft) };
    let (x, y) = (a.max(st), b.min(ft));
    if y <= x {
        return (0, span);
    }
    (if t.dur_elapsed { (y - x).num_minutes() } else { cal.work_minutes(x, y) }, span)
}

/// Work (minutes) of an assignment between two instants (flat contour).
fn asg_work(app: &App, i: usize, k: usize, a: NaiveDateTime, b: NaiveDateTime) -> f64 {
    let t = &app.doc.tasks[i];
    let Some(asg) = t.assignments.get(k) else { return 0.0 };
    if app.doc.resource_by_uid(asg.res_uid).map(|r| r.rtype != ResourceType::Work).unwrap_or(true) || asg.work_min <= 0 {
        return 0.0;
    }
    let (m, span) = task_share(app, i, a, b);
    if span <= 0 {
        return 0.0;
    }
    asg.work_min as f64 * m as f64 / span as f64
}

/// Work of a row between two instants.
fn row_work(app: &App, row: URow, a: NaiveDateTime, b: NaiveDateTime) -> f64 {
    match row {
        URow::Asg(i, k) => asg_work(app, i, k, a, b),
        URow::Task(i) => {
            let t = &app.doc.tasks[i];
            if t.is_summary {
                let end = app.doc.children_end(i);
                (i + 1..end).filter(|&j| !app.doc.tasks[j].is_summary).map(|j| (0..app.doc.tasks[j].assignments.len()).map(|k| asg_work(app, j, k, a, b)).sum::<f64>()).sum()
            } else {
                (0..t.assignments.len()).map(|k| asg_work(app, i, k, a, b)).sum()
            }
        }
        URow::Res(ri) => {
            let uid = app.doc.resources[ri].uid;
            app.doc
                .tasks
                .iter()
                .enumerate()
                .filter_map(|(i, t)| t.assignments.iter().position(|x| x.res_uid == uid).map(|k| (i, k)))
                .map(|(i, k)| asg_work(app, i, k, a, b))
                .sum()
        }
        URow::Unassigned => 0.0,
    }
}

/// "8h", "4.5h" (the standard timephased work).
fn hours(m: f64) -> String {
    let h = crate::util::fmt_num(m / 60.0);
    if crate::i18n::is_thai() {
        format!("{h} ชม.")
    } else {
        format!("{h}h")
    }
}

fn day_start(d: NaiveDate) -> NaiveDateTime {
    d.and_time(NaiveTime::MIN)
}

pub fn usage_view(app: &mut App, ui: &mut Ui, area: Rect, cmds: &mut Vec<Cmd>, resources: bool) {
    let p = ui.painter_at(area);
    p.rect_filled(area, 0.0, Color32::WHITE);
    let rows = rows(app, resources);
    let hh = crate::grid::header_h(app);
    let sb = crate::grid::SCROLLBAR;
    // sheet columns: title, width
    let cols: Vec<(&str, f32)> = if resources {
        vec![("ID", app.id_w.max(28.0)), ("Resource Name", 220.0), ("Work", 80.0)]
    } else {
        vec![("ID", app.id_w.max(28.0)), ("Task Name", 230.0), ("Work", 74.0), ("Duration", 70.0), ("Start", 92.0), ("Finish", 92.0)]
    };
    let sheet_w: f32 = cols.iter().map(|c| c.1).sum::<f32>().min(area.width() * 0.6);
    let details_w = 58.0;
    let sheet = Rect::from_min_max(area.min, Pos2::new(area.left() + sheet_w, area.bottom() - sb));
    let split = Rect::from_min_max(Pos2::new(sheet.right(), area.top()), Pos2::new(sheet.right() + 5.0, area.bottom()));
    let chart = Rect::from_min_max(Pos2::new(split.right(), area.top()), Pos2::new(area.right() - sb, area.bottom() - sb));
    let body_top = area.top() + hh;
    let body_h = chart.bottom() - body_top;
    let content_h = rows.len() as f32 * ROW_H;
    app.scroll_y = app.scroll_y.clamp(0.0, (content_h - body_h + ROW_H).max(0.0));
    let first = (app.scroll_y / ROW_H).floor() as usize;
    let off = app.scroll_y - first as f32 * ROW_H;
    let st = app.doc.formatting.text_style(TextItem::All);
    let font = theme::style_font(&st);
    let mut bst = st.clone();
    bst.bold = true;
    let bold = theme::style_font(&bst);
    let line = theme::rgb(app.doc.formatting.gridline(GridKind::SheetRows).normal_color);
    let red = Color32::from_rgb(0xE0, 0x1E, 0x1E);

    // ---------------- sheet header
    let mut x = sheet.left();
    for (title, w) in &cols {
        let r = Rect::from_min_size(Pos2::new(x, area.top()), Vec2::new(*w, hh));
        p.rect_filled(r, 0.0, theme::HEADER_BG);
        p.line_segment([Pos2::new(r.right() - 0.5, r.top() + 3.0), Pos2::new(r.right() - 0.5, r.bottom() - 3.0)], Stroke::new(1.0_f32, theme::SHEET_HEAD_SEP));
        let tp = p.with_clip_rect(r.intersect(sheet.union(Rect::from_min_size(area.min, Vec2::new(sheet_w, hh)))));
        tp.text(r.center(), Align2::CENTER_CENTER, tr(title), font.clone(), theme::TEXT);
        x += w;
    }
    p.line_segment([Pos2::new(sheet.left(), body_top - 0.5), Pos2::new(sheet.right(), body_top - 0.5)], Stroke::new(1.0_f32, theme::SHEET_HEAD_BOTTOM));
    // ---------------- timescale header ("Details" column + tiers)
    let details_head = Rect::from_min_size(Pos2::new(chart.left(), area.top()), Vec2::new(details_w, hh));
    p.rect_filled(details_head, 0.0, theme::HEADER_BG);
    p.text(details_head.center(), Align2::CENTER_CENTER, tr("Details"), font.clone(), theme::TEXT);
    let grid_left = chart.left() + details_w;
    let theader = Rect::from_min_max(Pos2::new(grid_left, area.top()), Pos2::new(chart.right(), body_top));
    crate::gantt::draw_timescale(app, &p.with_clip_rect(theader), theader, grid_left);
    let cells = crate::gantt::bottom_cells(app, grid_left, theader.left(), theader.right());

    // ---------------- rows
    let body = Rect::from_min_max(Pos2::new(area.left(), body_top), Pos2::new(chart.right(), chart.bottom()));
    let bp = p.with_clip_rect(body);
    let sel = app.usage_sel;
    for (k, &row) in rows.iter().enumerate().skip(first) {
        let y = body_top + (k - first) as f32 * ROW_H - off;
        if y > body.bottom() {
            break;
        }
        let rr = Rect::from_min_max(Pos2::new(sheet.left(), y), Pos2::new(chart.right(), y + ROW_H));
        if k == sel {
            bp.rect_filled(Rect::from_min_max(rr.min, Pos2::new(sheet.right(), rr.bottom())), 0.0, theme::ROW_SEL_CLASSIC);
        }
        // ID column like the sheet's
        bp.rect_filled(Rect::from_min_size(rr.min, Vec2::new(cols[0].1, ROW_H)), 0.0, theme::HEADER_BG);
        let (id, name, level, strong, ink, work, dur, start, finish, collapsible) = describe(app, row);
        let mut x = sheet.left();
        let texts: Vec<String> = if resources { vec![id, name, work] } else { vec![id, name, work, dur, start, finish] };
        for (c, ((_, w), s)) in cols.iter().zip(texts.iter()).enumerate() {
            let cr = Rect::from_min_size(Pos2::new(x, y), Vec2::new(*w, ROW_H));
            let cp = bp.with_clip_rect(cr.intersect(sheet.union(Rect::from_min_max(Pos2::new(sheet.left(), body_top), Pos2::new(sheet.right(), body.bottom())))));
            let f = if strong { bold.clone() } else { font.clone() };
            match c {
                0 => {
                    cp.text(cr.center(), Align2::CENTER_CENTER, s, font.clone(), theme::TEXT);
                }
                1 => {
                    let indent = level as f32 * 14.0;
                    if let Some(open) = collapsible {
                        let c = Pos2::new(cr.left() + indent + 7.0, cr.center().y);
                        let tri = if open { vec![c + Vec2::new(-3.5, -2.0), c + Vec2::new(3.5, -2.0), c + Vec2::new(0.0, 2.5)] } else { vec![c + Vec2::new(-2.0, -3.5), c + Vec2::new(2.5, 0.0), c + Vec2::new(-2.0, 3.5)] };
                        cp.add(egui::Shape::convex_polygon(tri, theme::TEXT, Stroke::NONE));
                    }
                    cp.text(Pos2::new(cr.left() + indent + 16.0, cr.center().y), Align2::LEFT_CENTER, crate::thai::shape(s), f, ink);
                }
                _ => {
                    cp.text(Pos2::new(cr.right() - 4.0, cr.center().y), Align2::RIGHT_CENTER, s, f, ink);
                }
            }
            bp.line_segment([Pos2::new(cr.right() - 0.5, y), Pos2::new(cr.right() - 0.5, y + ROW_H)], Stroke::new(1.0_f32, line));
            x += w;
        }
        // "Work" label in the Details column
        let dr = Rect::from_min_size(Pos2::new(chart.left(), y), Vec2::new(details_w, ROW_H));
        bp.text(Pos2::new(dr.left() + 4.0, dr.center().y), Align2::LEFT_CENTER, tr("Work"), font.clone(), theme::TEXT);
        bp.line_segment([Pos2::new(dr.right() - 0.5, y), Pos2::new(dr.right() - 0.5, y + ROW_H)], Stroke::new(1.0_f32, line));
        // timephased work
        let cap = |ri: usize, a: NaiveDate, b: NaiveDate| -> f64 {
            let mut d = a;
            let mut m = 0i64;
            while d < b {
                m += app.doc.calendar.day_intervals(d).iter().map(|(x, y)| (*y as i64 - *x as i64).max(0)).sum::<i64>();
                d += chrono::Duration::days(1);
            }
            app.doc.resources[ri].max_units / 100.0 * m as f64
        };
        let gp = bp.with_clip_rect(Rect::from_min_max(Pos2::new(grid_left, body_top), body.max));
        for &(a, b, x0, x1) in &cells {
            let v = row_work(app, row, day_start(a), day_start(b));
            let over = matches!(row, URow::Res(ri) if v > cap(ri, a, b) + 0.5);
            if over {
                gp.rect_filled(Rect::from_min_max(Pos2::new(x0, y), Pos2::new(x1, y + ROW_H)), 0.0, Color32::from_rgb(0xFF, 0xE0, 0xE0));
            }
            if v >= 0.5 {
                // a value too wide for its column is not drawn (zoom in to see it), as in MPP
                let g = gp.layout_no_wrap(hours(v), if strong { bold.clone() } else { font.clone() }, if over { red } else { theme::TEXT });
                if g.size().x + 5.0 <= x1 - x0 {
                    gp.galley(Pos2::new(x1 - 3.0 - g.size().x, y + (ROW_H - g.size().y) / 2.0), g, theme::TEXT);
                }
            }
            gp.line_segment([Pos2::new(x1 - 0.5, y), Pos2::new(x1 - 0.5, y + ROW_H)], Stroke::new(1.0_f32, line));
        }
        bp.line_segment([Pos2::new(sheet.left(), y + ROW_H - 0.5), Pos2::new(chart.right(), y + ROW_H - 0.5)], Stroke::new(1.0_f32, line));
    }
    // divider
    p.rect_filled(split, 0.0, theme::SHEET_HEAD_BG);
    p.line_segment([Pos2::new(split.left() + 0.5, split.top()), Pos2::new(split.left() + 0.5, split.bottom())], Stroke::new(1.0_f32, theme::SHEET_SPLIT_EDGE));
    p.line_segment([Pos2::new(split.right() - 0.5, split.top()), Pos2::new(split.right() - 0.5, split.bottom())], Stroke::new(1.0_f32, theme::SHEET_SPLIT_EDGE));
    app.chart_view = (grid_left, (chart.right() - grid_left).max(1.0));

    // ---------------- interaction: select, expand / collapse, open information, pan
    let resp = ui.interact(body, ui.scope_id().with(("usage_body", resources)), Sense::click_and_drag());
    if let Some(pos) = resp.interact_pointer_pos() {
        let k = first + ((pos.y - body_top + off) / ROW_H).floor().max(0.0) as usize;
        if (resp.clicked() || resp.double_clicked() || resp.secondary_clicked()) && k < rows.len() {
            app.usage_sel = k;
            let row = rows[k];
            if let URow::Task(i) | URow::Asg(i, _) = row {
                if let Some(r) = app.visible_rows().iter().position(|&x| x == i) {
                    app.sel_row = r;
                    app.sel_anchor = r;
                }
            }
            if let URow::Res(ri) = row {
                app.res_sel_row = ri;
            }
            // the outline triangle
            let (_, _, level, _, _, _, _, _, _, collapsible) = describe(app, row);
            let tri_x = sheet.left() + cols[0].1 + level as f32 * 14.0;
            if collapsible.is_some() && resp.clicked() && pos.x >= tri_x && pos.x <= tri_x + 16.0 {
                let key = match row {
                    URow::Task(i) => (false, app.doc.tasks[i].uid),
                    URow::Res(ri) => (true, app.doc.resources[ri].uid),
                    _ => (true, 0),
                };
                if !app.usage_collapsed.remove(&key) {
                    app.usage_collapsed.insert(key);
                }
            }
            if resp.double_clicked() {
                match row {
                    URow::Res(_) => cmds.push(Cmd::ResourceInfo),
                    URow::Task(_) | URow::Asg(_, _) => cmds.push(Cmd::TaskInfo),
                    URow::Unassigned => {}
                }
            }
        }
    }
    if resp.dragged() && resp.interact_pointer_pos().map(|p| p.x >= grid_left).unwrap_or(false) {
        app.scroll_x = (app.scroll_x - resp.drag_delta().x).max(0.0);
    }
    // scrollbars
    let vr = Rect::from_min_max(Pos2::new(area.right() - sb, body_top), Pos2::new(area.right(), area.bottom() - sb));
    scrollbar(app, ui, vr, body_h, content_h, true);
    let hr = Rect::from_min_max(Pos2::new(grid_left, area.bottom() - sb), Pos2::new(area.right() - sb, area.bottom()));
    p.rect_filled(Rect::from_min_max(Pos2::new(area.left(), area.bottom() - sb), Pos2::new(grid_left, area.bottom())), 0.0, Color32::from_rgb(0xF0, 0xF0, 0xF0));
    let span = (crate::gantt::chart_span_px(app)).max(chart.width() + app.scroll_x);
    scrollbar(app, ui, hr, chart.width(), span, false);
}

/// ID, name, indent level, bold, ink, work, duration, start, finish, collapsible (Some(open)).
#[allow(clippy::type_complexity)]
fn describe(app: &App, row: URow) -> (String, String, u8, bool, Color32, String, String, String, String, Option<bool>) {
    let du = app.dur_units();
    let wfmt = |m: i64| crate::util::fmt_dur(m.max(0), crate::model::DurUnit::Hours, false, du, &app.doc.options, crate::i18n::is_thai());
    let red = Color32::from_rgb(0xE0, 0x1E, 0x1E);
    match row {
        URow::Task(i) => {
            let t = &app.doc.tasks[i];
            let level = t.outline_level.saturating_sub(1);
            let open = !app.usage_collapsed.contains(&(false, t.uid));
            let ink = if t.overallocated { red } else { theme::TEXT };
            (
                (i + 1).to_string(),
                t.name.clone(),
                level,
                t.is_summary,
                ink,
                wfmt(t.work_min),
                app.task_duration_text(t),
                app.fmt_date(t.start),
                app.fmt_finish(t.finish),
                (!t.assignments.is_empty()).then_some(open),
            )
        }
        URow::Asg(i, k) => {
            let t = &app.doc.tasks[i];
            let a = &t.assignments[k];
            let r = app.doc.resource_by_uid(a.res_uid);
            let resources_view = app.view == crate::app::View::ResourceUsage;
            let name = if resources_view { t.name.clone() } else { r.map(|r| r.name.clone()).unwrap_or_default() };
            let work = match r.map(|r| r.rtype) {
                Some(ResourceType::Material) => format!("{} {}", crate::util::fmt_num(a.units), r.map(|r| r.material_label.clone()).unwrap_or_default()),
                Some(ResourceType::Cost) => String::new(),
                _ => wfmt(a.work_min),
            };
            let level = if resources_view { 1 } else { t.outline_level };
            (String::new(), name, level, false, theme::TEXT, work, String::new(), app.fmt_date(t.start), app.fmt_finish(t.finish), None)
        }
        URow::Res(ri) => {
            let r = &app.doc.resources[ri];
            let open = !app.usage_collapsed.contains(&(true, r.uid));
            let has = app.doc.tasks.iter().any(|t| t.assignments.iter().any(|a| a.res_uid == r.uid));
            (
                (ri + 1).to_string(),
                r.name.clone(),
                0,
                true,
                if r.overallocated { red } else { theme::TEXT },
                if r.rtype == ResourceType::Work { wfmt(r.work_min) } else { String::new() },
                String::new(),
                String::new(),
                String::new(),
                has.then_some(open),
            )
        }
        URow::Unassigned => {
            let open = !app.usage_collapsed.contains(&(true, 0));
            (String::new(), tr("Unassigned").to_string(), 0, true, theme::TEXT, wfmt(0), String::new(), String::new(), String::new(), Some(open))
        }
    }
}

fn scrollbar(app: &mut App, ui: &mut Ui, r: Rect, view: f32, content: f32, vertical: bool) {
    let p = ui.painter();
    p.rect_filled(r, 0.0, Color32::from_rgb(0xF0, 0xF0, 0xF0));
    let content = content.max(view);
    let len = if vertical { r.height() } else { r.width() };
    let frac = (view / content).clamp(0.05, 1.0);
    let thumb_len = (len * frac).max(20.0);
    let max_scroll = (content - view).max(1.0);
    let cur = if vertical { app.scroll_y } else { app.scroll_x };
    let t = (cur / max_scroll).clamp(0.0, 1.0);
    let thumb = if vertical {
        Rect::from_min_size(Pos2::new(r.left() + 2.0, r.top() + t * (len - thumb_len)), Vec2::new(r.width() - 4.0, thumb_len))
    } else {
        Rect::from_min_size(Pos2::new(r.left() + t * (len - thumb_len), r.top() + 2.0), Vec2::new(thumb_len, r.height() - 4.0))
    };
    let resp = ui.interact(r, ui.scope_id().with(("usage_scroll", vertical)), Sense::click_and_drag());
    p.rect_filled(thumb, 3.0, if resp.dragged() || resp.hovered() { Color32::from_rgb(0xA0, 0xA0, 0xA0) } else { Color32::from_rgb(0xC2, 0xC2, 0xC2) });
    if resp.dragged() {
        let d = if vertical { resp.drag_delta().y } else { resp.drag_delta().x };
        let v = cur + d / (len - thumb_len).max(1.0) * max_scroll;
        if vertical {
            app.scroll_y = v.max(0.0);
        } else {
            app.scroll_x = v.max(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DurUnit, Project, Resource, Task};

    fn walk(shape: &egui::Shape, texts: &mut Vec<String>, fills: &mut Vec<Color32>) {
        match shape {
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, texts, fills)),
            egui::Shape::Text(t) => texts.push(t.galley.text().to_string()),
            egui::Shape::Rect(r) => fills.push(r.fill),
            _ => {}
        }
    }

    /// Resource Usage: one engineer on two parallel 2-day tasks works 16 hours a day, which
    /// MPP shows in red on the resource's row.
    #[test]
    fn resource_usage_shows_daily_work_and_overallocation() {
        let ctx = egui::Context::default();
        let mut app = App::with_ctx(&ctx, None);
        let mon = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
        app.doc = Project::new();
        app.doc.start = mon;
        app.doc.resources.push(Resource::new(1, "Engineer"));
        app.doc.next_res_uid = 2;
        for uid in 1..=2 {
            let mut t = Task::new(uid, if uid == 1 { "A" } else { "B" }, mon);
            t.set_duration_minutes(960, DurUnit::Days, false, 480);
            t.resources = vec![1];
            app.doc.tasks.push(t);
        }
        app.doc.next_uid = 3;
        app.recalc();
        assert!(app.doc.resources[0].overallocated);
        app.view = crate::app::View::ResourceUsage;
        app.px_per_day = 60.0;
        app.scroll_x = ((mon - app.gantt_origin()).num_days() - 1) as f32 * app.px_per_day;
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1400.0, 500.0));
        let mut shapes = Vec::new();
        for k in 0..12 {
            let input = egui::RawInput { screen_rect: Some(screen), time: Some(k as f64 * 0.03), ..Default::default() };
            let out = ctx.run_pass(input, |ctx| {
                egui::Area::new(egui::Id::unique("usage_test")).fixed_pos(Pos2::ZERO).show(ctx, |ui| {
                    ui.set_min_size(screen.size());
                    let mut cmds = Vec::new();
                    usage_view(&mut app, ui, screen, &mut cmds, true);
                });
            });
            shapes = out.shapes;
        }
        let (mut texts, mut fills) = (Vec::new(), Vec::new());
        for cs in &shapes {
            walk(&cs.shape, &mut texts, &mut fills);
        }
        let h = |n: &str| texts.iter().filter(|t| t.as_str() == n || t.as_str() == format!("{} ชม.", n.trim_end_matches('h'))).count();
        // the resource row: 16h on Mon and Tue; each assignment row: 8h on both days
        assert_eq!(h("16h"), 2, "texts: {texts:?}");
        assert_eq!(h("8h"), 4);
        assert!(fills.contains(&Color32::from_rgb(0xFF, 0xE0, 0xE0)), "overallocated cells are red");
        assert_eq!(rows(&app, true), vec![URow::Res(0), URow::Asg(0, 0), URow::Asg(1, 0)]);
    }
}
