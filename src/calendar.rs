//! Calendar view: a month of weeks with every task as a bar across the days it runs (the standard
//! Calendar view), non-working days shaded, the month changed with the arrows beside its name.

use crate::app::App;
use crate::i18n::tr;
use crate::ribbon::Cmd;
use crate::theme;
use chrono::{Datelike, Duration, NaiveDate};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Ui, Vec2};

const TITLE_H: f32 = 34.0;
const DAYS_H: f32 = 22.0;
const DAY_NUM_H: f32 = 18.0;
const BAR_H: f32 = 17.0;

/// First day shown: the start of the week holding the 1st of the month.
pub fn grid_start(month: NaiveDate, week_starts_on: u8) -> NaiveDate {
    let first = month.with_day(1).unwrap_or(month);
    // week_starts_on: 0 = Sunday .. 6 = Saturday
    let wd = first.weekday().num_days_from_sunday() as i64;
    first - Duration::days((wd - week_starts_on as i64).rem_euclid(7))
}

/// Bars of one week: (task index, first day column, last day column, lane).
pub fn week_bars(app: &App, week: NaiveDate) -> Vec<(usize, usize, usize, usize)> {
    let end = week + Duration::days(6);
    let mut items: Vec<(usize, usize, usize)> = app
        .visible_rows()
        .into_iter()
        .filter(|&i| !app.doc.tasks[i].is_summary && !app.doc.tasks[i].hide_bar)
        .filter_map(|i| {
            let t = &app.doc.tasks[i];
            if t.finish < week || t.start > end {
                return None;
            }
            let a = (t.start.max(week) - week).num_days() as usize;
            let b = (t.finish.min(end) - week).num_days() as usize;
            Some((i, a, b))
        })
        .collect();
    items.sort_by_key(|&(i, a, b)| (a, std::cmp::Reverse(b), i));
    let mut lanes: Vec<usize> = Vec::new(); // last column used per lane
    let mut out = Vec::new();
    for (i, a, b) in items {
        let lane = match lanes.iter().position(|&last| last < a) {
            Some(l) => {
                lanes[l] = b;
                l
            }
            None => {
                lanes.push(b);
                lanes.len() - 1
            }
        };
        out.push((i, a, b, lane));
    }
    out
}

pub fn calendar_view(app: &mut App, ui: &mut Ui, area: Rect, cmds: &mut Vec<Cmd>) {
    let p = ui.painter_at(area);
    p.rect_filled(area, 0.0, Color32::WHITE);
    let month = app.cal_month.with_day(1).unwrap_or(app.cal_month);
    let wso = app.doc.options.week_starts_on.min(6);
    let start = grid_start(month, wso);
    let weeks = {
        let next = if month.month() == 12 { NaiveDate::from_ymd_opt(month.year() + 1, 1, 1) } else { NaiveDate::from_ymd_opt(month.year(), month.month() + 1, 1) }.unwrap_or(month);
        (((next - start).num_days() + 6) / 7).max(4) as usize
    };
    // title with the month and the arrows
    let title = Rect::from_min_size(area.min, Vec2::new(area.width(), TITLE_H));
    p.rect_filled(title, 0.0, theme::HEADER_BG);
    let label = format!("{} {}", crate::thai::shape(&app.loc.month_long[month.month0() as usize]), app.loc.year(month));
    p.text(title.center(), Align2::CENTER_CENTER, label, theme::bold(15.0), theme::TEXT);
    let arrow = |x: f32| Rect::from_center_size(Pos2::new(x, title.center().y), Vec2::new(26.0, 24.0));
    let (lr, rr) = (arrow(title.center().x - 120.0), arrow(title.center().x + 120.0));
    for (r, left) in [(lr, true), (rr, false)] {
        let resp = ui.interact(r, ui.scope_id().with(("cal_arrow", left)), Sense::click());
        if resp.hovered() {
            p.rect_filled(r, 3.0, theme::HOVER_BG);
        }
        let c = r.center();
        let tri = if left { vec![c + Vec2::new(4.0, -6.0), c + Vec2::new(4.0, 6.0), c + Vec2::new(-4.0, 0.0)] } else { vec![c + Vec2::new(-4.0, -6.0), c + Vec2::new(-4.0, 6.0), c + Vec2::new(4.0, 0.0)] };
        p.add(egui::Shape::convex_polygon(tri, theme::TEXT, Stroke::NONE));
        if resp.clicked() {
            let m = month.month0() as i32 + if left { -1 } else { 1 };
            let (y, m) = (month.year() + m.div_euclid(12), m.rem_euclid(12) as u32 + 1);
            app.cal_month = NaiveDate::from_ymd_opt(y, m, 1).unwrap_or(month);
        }
    }
    // weekday names
    let col_w = area.width() / 7.0;
    let days = Rect::from_min_size(Pos2::new(area.left(), title.bottom()), Vec2::new(area.width(), DAYS_H));
    p.rect_filled(days, 0.0, theme::SHEET_HEAD_BG);
    for k in 0..7 {
        let d = start + Duration::days(k as i64);
        let r = Rect::from_min_size(Pos2::new(area.left() + k as f32 * col_w, days.top()), Vec2::new(col_w, DAYS_H));
        p.text(r.center(), Align2::CENTER_CENTER, crate::thai::shape(&app.loc.day_long[d.weekday().num_days_from_monday() as usize]), FontId::proportional(12.0), theme::TEXT);
    }
    // weeks
    let body_top = days.bottom();
    let week_h = ((area.bottom() - body_top) / weeks as f32).max(DAY_NUM_H + BAR_H + 4.0);
    let line = Color32::from_rgb(0xC8, 0xC8, 0xC8);
    let today = chrono::Local::now().date_naive();
    let font = FontId::proportional(11.0);
    let sel = app.current_task();
    let mut hits: Vec<(Rect, usize)> = Vec::new();
    for w in 0..weeks {
        let wk = start + Duration::days(w as i64 * 7);
        let y = body_top + w as f32 * week_h;
        for k in 0..7 {
            let d = wk + Duration::days(k as i64);
            let r = Rect::from_min_size(Pos2::new(area.left() + k as f32 * col_w, y), Vec2::new(col_w, week_h));
            if !app.doc.calendar.is_working(d) {
                p.rect_filled(r, 0.0, Color32::from_rgb(0xF2, 0xF2, 0xF2));
            }
            if d.month() != month.month() {
                p.rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(0xE6, 0xE6, 0xE6, 120));
            }
            p.rect_stroke(r, 0.0, Stroke::new(1.0_f32, line), egui::StrokeKind::Inside);
            let ink = if d == today { theme::BRAND_GREEN } else if d.month() == month.month() { theme::TEXT } else { theme::TEXT_DIM };
            p.text(Pos2::new(r.right() - 5.0, r.top() + 3.0), Align2::RIGHT_TOP, d.day().to_string(), if d == today { theme::bold(11.5) } else { FontId::proportional(11.5) }, ink);
        }
        let lanes_fit = ((week_h - DAY_NUM_H - 4.0) / (BAR_H + 2.0)).floor().max(1.0) as usize;
        let mut hidden = [0usize; 7];
        for (i, a, b, lane) in week_bars(app, wk) {
            if lane >= lanes_fit {
                for h in hidden.iter_mut().take(b + 1).skip(a) {
                    *h += 1;
                }
                continue;
            }
            let t = &app.doc.tasks[i];
            let by = y + DAY_NUM_H + lane as f32 * (BAR_H + 2.0);
            let br = Rect::from_min_max(Pos2::new(area.left() + a as f32 * col_w + 3.0, by), Pos2::new(area.left() + (b + 1) as f32 * col_w - 3.0, by + BAR_H));
            let fill = if t.critical && app.doc.formatting.show_critical { Color32::from_rgb(0xF4, 0xB6, 0xB6) } else if t.manual { Color32::from_rgb(0x74, 0xCC, 0xD4) } else { Color32::from_rgb(0x8A, 0xBB, 0xED) };
            p.rect_filled(br, 3.0, fill);
            if sel == Some(i) {
                p.rect_stroke(br, 3.0, Stroke::new(1.6_f32, theme::TEXT), egui::StrokeKind::Inside);
            }
            let txt = format!("{}, {}", crate::thai::shape(&t.name), app.task_duration_text(t));
            p.with_clip_rect(br.shrink(1.0)).text(Pos2::new(br.left() + 4.0, br.center().y), Align2::LEFT_CENTER, txt, font.clone(), theme::TEXT);
            hits.push((br, i));
        }
        // MPP marks days with more tasks than fit
        for (k, n) in hidden.iter().enumerate() {
            if *n > 0 {
                let r = Rect::from_min_size(Pos2::new(area.left() + k as f32 * col_w, y), Vec2::new(col_w, week_h));
                p.text(Pos2::new(r.left() + 5.0, r.top() + 3.0), Align2::LEFT_TOP, format!("+{n}"), theme::bold(10.5), theme::ACCENT);
            }
        }
    }
    let resp = ui.interact(Rect::from_min_max(Pos2::new(area.left(), body_top), area.max), ui.scope_id().with("calendar"), Sense::click());
    if let Some(pos) = resp.interact_pointer_pos() {
        if resp.clicked() || resp.double_clicked() {
            if let Some(&(_, i)) = hits.iter().find(|(r, _)| r.contains(pos)) {
                if let Some(rw) = app.visible_rows().iter().position(|&x| x == i) {
                    app.sel_row = rw;
                    app.sel_anchor = rw;
                }
                if resp.double_clicked() {
                    cmds.push(Cmd::TaskInfo);
                }
            }
        }
    }
    let _ = tr("");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DurUnit, Project, Task};

    #[test]
    fn month_grid_and_week_lanes() {
        // October 2026 begins on a Thursday: the grid starts on Sunday 27 September
        let oct = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        assert_eq!(grid_start(oct, 0), NaiveDate::from_ymd_opt(2026, 9, 27).unwrap());
        assert_eq!(grid_start(oct, 1), NaiveDate::from_ymd_opt(2026, 9, 28).unwrap());
        let ctx = egui::Context::default();
        let mut app = App::with_ctx(&ctx, None);
        let mon = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
        app.doc = Project::new();
        app.doc.start = mon;
        for (uid, days) in [(1u32, 3i64), (2, 1), (3, 2)] {
            let mut t = Task::new(uid, "T", mon);
            t.set_duration_minutes(days * 480, DurUnit::Days, false, 480);
            app.doc.tasks.push(t);
        }
        app.doc.next_uid = 4;
        app.recalc();
        // all start on Monday: three lanes, the longest first
        let week = NaiveDate::from_ymd_opt(2026, 10, 18).unwrap();
        let bars = week_bars(&app, week);
        assert_eq!(bars.len(), 3);
        assert_eq!(bars[0], (0, 1, 3, 0));
        assert!(bars.iter().all(|b| b.1 == 1));
        let lanes: Vec<usize> = bars.iter().map(|b| b.3).collect();
        assert_eq!(lanes, vec![0, 1, 2]);
    }

    /// Every new view draws a mixed project (summary, milestone, links, resources, progress)
    /// without panicking.
    #[test]
    fn views_draw() {
        let ctx = egui::Context::default();
        let mut app = App::with_ctx(&ctx, None);
        let mon = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
        app.doc = Project::new();
        app.doc.start = mon;
        app.doc.resources.push(crate::model::Resource::new(1, "Crew"));
        app.doc.next_res_uid = 2;
        let mut s = Task::new(1, "Phase", mon);
        s.outline_level = 1;
        app.doc.tasks.push(s);
        for uid in 2..=6 {
            let mut t = Task::new(uid, &format!("Task {uid}"), mon);
            t.outline_level = 2;
            t.set_duration_minutes(if uid == 6 { 0 } else { 960 }, DurUnit::Days, false, 480);
            if uid > 2 {
                t.predecessors.push(crate::model::Link::new(uid - 1, crate::model::LinkType::FS));
            }
            if uid % 2 == 0 {
                t.resources = vec![1];
            }
            app.doc.tasks.push(t);
        }
        app.doc.next_uid = 7;
        app.recalc();
        let cal = app.doc.calendar.clone();
        app.doc.tasks[1].set_percent(100, &cal);
        app.doc.tasks[2].set_percent(50, &cal);
        app.recalc();
        app.cal_month = mon;
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 700.0));
        for view in 0..4 {
            for k in 0..3 {
                let input = egui::RawInput { screen_rect: Some(screen), time: Some(k as f64 * 0.03), ..Default::default() };
                let _ = ctx.run_pass(input, |ctx| {
                    egui::Area::new(egui::Id::unique("views_test")).fixed_pos(Pos2::ZERO).show(ctx, |ui| {
                        ui.set_min_size(screen.size());
                        let mut cmds = Vec::new();
                        match view {
                            0 => calendar_view(&mut app, ui, screen, &mut cmds),
                            1 => crate::network::network_view(&mut app, ui, screen, &mut cmds),
                            2 => crate::usage::usage_view(&mut app, ui, screen, &mut cmds, false),
                            _ => crate::usage::usage_view(&mut app, ui, screen, &mut cmds, true),
                        }
                    });
                });
            }
        }
        assert_eq!(crate::network::layout(&app).len(), 5);
    }
}
