//! View > Task Board: the standard board of task cards in four columns by progress - Not Started,
//! Next up, In Progress, Done. Dragging a card to another column sets the task's % complete as
//! MPP does (0 %, 0 %, 50 % for a task not yet started, 100 %); "Next up" is the task's board
//! status. Clicking a card selects the task, double-clicking opens Task Information.

use crate::app::App;
use crate::i18n::tr;
use crate::ribbon::Cmd;
use crate::theme;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Ui, Vec2};

/// The four columns.
pub const COLUMNS: [&str; 4] = ["Not Started", "Next up", "In Progress", "Done"];

/// The column a task's card is in.
pub fn column_of(t: &crate::model::Task) -> usize {
    match t.percent_complete {
        100 => 3,
        0 if t.board_status == 1 => 1,
        0 => 0,
        _ => 2,
    }
}

const CARD_H: f32 = 74.0;
const GAP: f32 = 8.0;

pub fn board_view(app: &mut App, ui: &mut Ui, area: Rect, cmds: &mut Vec<Cmd>) {
    let p = ui.painter_at(area);
    p.rect_filled(area, 0.0, Color32::from_rgb(0xF3, 0xF3, 0xF3));
    // the cards: tasks (not summaries) the filter shows, in ID order
    let filter = app.filter.clone();
    let tasks: Vec<usize> = (0..app.doc.tasks.len())
        .filter(|&i| !app.doc.tasks[i].is_summary && (filter.kind == crate::app::TaskFilter::All || app.task_matches(i, &filter)))
        .collect();
    let mut cols: [Vec<usize>; 4] = Default::default();
    for &i in &tasks {
        cols[column_of(&app.doc.tasks[i])].push(i);
    }
    if app.board_scroll.len() != 4 {
        app.board_scroll = vec![0.0; 4];
    }
    let cw = ((area.width() - GAP * 5.0) / 4.0).max(160.0);
    let current = app.current_task();
    let pointer = ui.ctx().pointer_latest_pos();
    let mut drop: Option<(usize, usize)> = None;
    for (c, list) in cols.iter().enumerate() {
        let x0 = area.left() + GAP + c as f32 * (cw + GAP);
        let col = Rect::from_min_max(Pos2::new(x0, area.top() + GAP), Pos2::new(x0 + cw, area.bottom() - GAP));
        let hover_col = app.board_drag.is_some() && pointer.map(|q| col.contains(q)).unwrap_or(false);
        p.rect_filled(col, 4.0, if hover_col { theme::GREEN_LIGHT } else { Color32::from_rgb(0xE8, 0xE8, 0xE8) });
        // header: column name and number of cards
        let head = Rect::from_min_size(col.min, Vec2::new(cw, 34.0));
        p.text(Pos2::new(head.left() + 12.0, head.center().y), Align2::LEFT_CENTER, tr(COLUMNS[c]), FontId::proportional(14.0), theme::TEXT);
        p.text(Pos2::new(head.right() - 12.0, head.center().y), Align2::RIGHT_CENTER, list.len().to_string(), FontId::proportional(12.0), theme::TEXT_DIM);
        let body = Rect::from_min_max(Pos2::new(col.left(), head.bottom()), col.max);
        // wheel scrolls the column under the pointer
        if pointer.map(|q| body.contains(q)).unwrap_or(false) {
            let dy = ui.input(|i| i.smooth_scroll_delta.y);
            let max = (list.len() as f32 * (CARD_H + GAP) - body.height() + GAP).max(0.0);
            app.board_scroll[c] = (app.board_scroll[c] - dy).clamp(0.0, max);
        }
        let cp = p.with_clip_rect(body);
        for (k, &i) in list.iter().enumerate() {
            let y = body.top() + GAP + k as f32 * (CARD_H + GAP) - app.board_scroll[c];
            if y > body.bottom() || y + CARD_H < body.top() {
                continue;
            }
            let card = Rect::from_min_size(Pos2::new(col.left() + GAP, y), Vec2::new(cw - 2.0 * GAP, CARD_H));
            let resp = ui.interact(card.intersect(body), ui.scope_id().with(("board_card", i)), Sense::click_and_drag());
            if app.board_drag == Some(i) {
                // the card being dragged leaves a gap
                cp.rect_stroke(card, 4.0, Stroke::new(1.0_f32, theme::TEXT_DIM), egui::StrokeKind::Inside);
            } else {
                draw_card(app, &cp, card, i, current == Some(i) || resp.hovered());
            }
            if resp.clicked() {
                app.select_task(i);
            }
            if resp.double_clicked() {
                app.select_task(i);
                cmds.push(Cmd::TaskInfo);
            }
            if resp.drag_started() {
                app.board_drag = Some(i);
            }
            if resp.drag_stopped() {
                if let Some(q) = pointer {
                    let target = (0..4).find(|&cc| {
                        let xx = area.left() + GAP + cc as f32 * (cw + GAP);
                        q.x >= xx && q.x <= xx + cw
                    });
                    if let Some(t) = target {
                        drop = Some((i, t));
                    }
                }
                app.board_drag = None;
            }
        }
    }
    // the dragged card follows the pointer
    if let (Some(i), Some(q)) = (app.board_drag, pointer) {
        let card = Rect::from_center_size(q, Vec2::new(cw - 2.0 * GAP, CARD_H));
        let fp = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::unique("board_drag")));
        draw_card(app, &fp, card, i, true);
    }
    if let Some((i, c)) = drop {
        move_card(app, i, c);
    }
}

fn draw_card(app: &App, p: &egui::Painter, r: Rect, i: usize, hot: bool) {
    let t = &app.doc.tasks[i];
    p.rect_filled(r, 4.0, Color32::WHITE);
    p.rect_stroke(r, 4.0, Stroke::new(if hot { 1.5_f32 } else { 1.0_f32 }, if hot { theme::GREEN } else { Color32::from_rgb(0xD0, 0xD0, 0xD0) }), egui::StrokeKind::Inside);
    let strip = if t.percent_complete == 100 { theme::GREEN } else if t.critical { Color32::from_rgb(0xD1, 0x3B, 0x2E) } else { theme::rgb(crate::format::TASK_BLUE) };
    p.rect_filled(Rect::from_min_size(r.min, Vec2::new(4.0, r.height())), 2.0, strip);
    let cp = p.with_clip_rect(r.shrink(2.0));
    let id = app.uid_to_id().get(&t.uid).copied().unwrap_or(i + 1);
    let name = crate::thai::shape(&t.name).to_string();
    cp.text(Pos2::new(r.left() + 12.0, r.top() + 8.0), Align2::LEFT_TOP, name, FontId::proportional(13.0), theme::TEXT);
    let res = app.doc.resource_names(t);
    let line2 = if res.is_empty() { format!("ID {id}") } else { format!("ID {id}  ·  {res}") };
    cp.text(Pos2::new(r.left() + 12.0, r.top() + 30.0), Align2::LEFT_TOP, crate::thai::shape(&line2).to_string(), FontId::proportional(11.5), theme::TEXT_DIM);
    let line3 = format!("{} - {}", app.fmt_date(t.start), app.fmt_finish(t.finish));
    cp.text(Pos2::new(r.left() + 12.0, r.bottom() - 8.0), Align2::LEFT_BOTTOM, line3, FontId::proportional(11.0), theme::TEXT_DIM);
    // % complete bar
    let bw = 60.0;
    let bar = Rect::from_min_size(Pos2::new(r.right() - bw - 10.0, r.bottom() - 16.0), Vec2::new(bw, 6.0));
    p.rect_filled(bar, 3.0, Color32::from_rgb(0xE3, 0xE3, 0xE3));
    p.rect_filled(Rect::from_min_size(bar.min, Vec2::new(bw * t.percent_complete as f32 / 100.0, 6.0)), 3.0, theme::GREEN);
    p.text(Pos2::new(bar.left() - 4.0, bar.center().y), Align2::RIGHT_CENTER, format!("{}%", t.percent_complete), FontId::proportional(11.0), theme::TEXT_DIM);
}

/// View > Sprint Planning Board: a column per sprint (and "No Sprint"); dragging a card to a column
/// puts the task in that sprint.
pub fn sprint_board(app: &mut App, ui: &mut Ui, area: Rect, cmds: &mut Vec<Cmd>) {
    let p = ui.painter_at(area);
    p.rect_filled(area, 0.0, Color32::from_rgb(0xF3, 0xF3, 0xF3));
    if app.doc.sprints.is_empty() {
        p.text(area.center(), Align2::CENTER_CENTER, tr("Add sprints with Project > Manage Sprints."), FontId::proportional(15.0), theme::TEXT_DIM);
        return;
    }
    let filter = app.filter.clone();
    let tasks: Vec<usize> = (0..app.doc.tasks.len())
        .filter(|&i| !app.doc.tasks[i].is_summary && (filter.kind == crate::app::TaskFilter::All || app.task_matches(i, &filter)))
        .collect();
    let mut keys: Vec<Option<u32>> = vec![None];
    keys.extend(app.doc.sprints.iter().map(|s| Some(s.uid)));
    let ncol = keys.len();
    if app.board_scroll.len() != ncol {
        app.board_scroll = vec![0.0; ncol];
    }
    let cw = ((area.width() - GAP * (ncol as f32 + 1.0)) / ncol as f32).max(180.0);
    let pointer = ui.ctx().pointer_latest_pos();
    let current = app.current_task();
    let mut drop: Option<(usize, Option<u32>)> = None;
    for (c, key) in keys.iter().enumerate() {
        let list: Vec<usize> = tasks.iter().copied().filter(|&i| app.doc.tasks[i].sprint == *key).collect();
        let x0 = area.left() + GAP + c as f32 * (cw + GAP);
        let col = Rect::from_min_max(Pos2::new(x0, area.top() + GAP), Pos2::new(x0 + cw, area.bottom() - GAP));
        let hover_col = app.board_drag.is_some() && pointer.map(|q| col.contains(q)).unwrap_or(false);
        p.rect_filled(col, 4.0, if hover_col { theme::GREEN_LIGHT } else { Color32::from_rgb(0xE8, 0xE8, 0xE8) });
        let head = Rect::from_min_size(col.min, Vec2::new(cw, 44.0));
        let (title, dates) = match key.and_then(|u| app.doc.sprints.iter().find(|s| s.uid == u)) {
            Some(s) => (crate::thai::shape(&s.name).to_string(), format!("{} - {}", app.fmt_date(s.start), app.fmt_finish(s.finish))),
            None => (tr("No Sprint").to_string(), String::new()),
        };
        p.text(Pos2::new(head.left() + 12.0, head.top() + 14.0), Align2::LEFT_CENTER, title, FontId::proportional(14.0), theme::TEXT);
        p.text(Pos2::new(head.left() + 12.0, head.top() + 32.0), Align2::LEFT_CENTER, dates, FontId::proportional(11.0), theme::TEXT_DIM);
        p.text(Pos2::new(head.right() - 12.0, head.top() + 14.0), Align2::RIGHT_CENTER, list.len().to_string(), FontId::proportional(12.0), theme::TEXT_DIM);
        let body = Rect::from_min_max(Pos2::new(col.left(), head.bottom()), col.max);
        if pointer.map(|q| body.contains(q)).unwrap_or(false) {
            let dy = ui.input(|i| i.smooth_scroll_delta.y);
            let max = (list.len() as f32 * (CARD_H + GAP) - body.height() + GAP).max(0.0);
            app.board_scroll[c] = (app.board_scroll[c] - dy).clamp(0.0, max);
        }
        let cp = p.with_clip_rect(body);
        for (k, &i) in list.iter().enumerate() {
            let y = body.top() + GAP + k as f32 * (CARD_H + GAP) - app.board_scroll[c];
            if y > body.bottom() || y + CARD_H < body.top() {
                continue;
            }
            let card = Rect::from_min_size(Pos2::new(col.left() + GAP, y), Vec2::new(cw - 2.0 * GAP, CARD_H));
            let resp = ui.interact(card.intersect(body), ui.scope_id().with(("sprint_card", i)), Sense::click_and_drag());
            if app.board_drag == Some(i) {
                cp.rect_stroke(card, 4.0, Stroke::new(1.0_f32, theme::TEXT_DIM), egui::StrokeKind::Inside);
            } else {
                draw_card(app, &cp, card, i, current == Some(i) || resp.hovered());
            }
            if resp.clicked() {
                app.select_task(i);
            }
            if resp.double_clicked() {
                app.select_task(i);
                cmds.push(Cmd::TaskInfo);
            }
            if resp.drag_started() {
                app.board_drag = Some(i);
            }
            if resp.drag_stopped() {
                if let Some(q) = pointer {
                    if let Some(t) = (0..ncol).find(|&cc| {
                        let xx = area.left() + GAP + cc as f32 * (cw + GAP);
                        q.x >= xx && q.x <= xx + cw
                    }) {
                        drop = Some((i, keys[t]));
                    }
                }
                app.board_drag = None;
            }
        }
    }
    if let (Some(i), Some(q)) = (app.board_drag, pointer) {
        let card = Rect::from_center_size(q, Vec2::new(cw - 2.0 * GAP, CARD_H));
        let fp = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::unique("sprint_drag")));
        draw_card(app, &fp, card, i, true);
    }
    if let Some((i, s)) = drop {
        if app.doc.tasks[i].sprint != s {
            app.checkpoint();
            app.doc.tasks[i].sprint = s;
            app.dirty = true;
        }
    }
}

/// Dropping a card on a column: the standard % complete for it.
pub fn move_card(app: &mut App, i: usize, col: usize) {
    if column_of(&app.doc.tasks[i]) == col {
        return;
    }
    app.select_task(i);
    let pct = match col {
        0 | 1 => 0,
        2 => {
            let p = app.doc.tasks[i].percent_complete;
            if p == 0 || p == 100 { 50 } else { p }
        }
        _ => 100,
    };
    app.set_percent(pct);
    app.doc.tasks[i].board_status = if col == 1 { 1 } else { 0 };
    app.dirty = true;
}
