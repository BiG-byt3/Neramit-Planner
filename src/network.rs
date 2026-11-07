//! Network Diagram view: one box per task laid out "top down from left" (a task's column follows
//! its predecessors), the standard Standard box (name; Start / ID; Finish / Dur; Res), red boxes and
//! links on the critical path, a diagonal for tasks in progress and a cross for finished tasks.

use crate::app::App;
use crate::i18n::tr;
use crate::ribbon::Cmd;
use crate::theme;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2};
use std::collections::HashMap;

pub const BOX_W: f32 = 176.0;
pub const BOX_H: f32 = 82.0;
const GAP_X: f32 = 46.0;
const GAP_Y: f32 = 26.0;
const MARGIN: f32 = 24.0;

/// Grid position (column, row) of every task shown (summary tasks are left out).
pub fn layout(app: &App) -> Vec<(usize, usize, usize)> {
    let tasks: Vec<usize> = app.visible_rows().into_iter().filter(|&i| !app.doc.tasks[i].is_summary).collect();
    let shown: HashMap<u32, usize> = tasks.iter().map(|&i| (app.doc.tasks[i].uid, i)).collect();
    // column = 1 + the column of the latest predecessor (longest chain from the left)
    let mut col: HashMap<usize, usize> = tasks.iter().map(|&i| (i, 0)).collect();
    for _ in 0..tasks.len().min(2000) + 1 {
        let mut changed = false;
        for &i in &tasks {
            let c = app.doc.tasks[i].predecessors.iter().filter_map(|l| shown.get(&l.pred_uid)).map(|p| col[p] + 1).max().unwrap_or(0);
            if c > col[&i] {
                col.insert(i, c);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    // rows: next free row of the column, but not above the first predecessor's row
    let mut next: HashMap<usize, usize> = HashMap::new();
    let mut row: HashMap<usize, usize> = HashMap::new();
    let mut order = tasks.clone();
    order.sort_by_key(|&i| (col[&i], i));
    let mut out = Vec::new();
    for i in order {
        let c = col[&i];
        let pred_row = app.doc.tasks[i].predecessors.iter().filter_map(|l| shown.get(&l.pred_uid)).filter_map(|p| row.get(p)).min().copied().unwrap_or(0);
        let r = next.get(&c).copied().unwrap_or(0).max(pred_row);
        next.insert(c, r + 1);
        row.insert(i, r);
        out.push((i, c, r));
    }
    out
}

fn box_rect(origin: Pos2, zoom: f32, c: usize, r: usize) -> Rect {
    Rect::from_min_size(
        Pos2::new(origin.x + (MARGIN + c as f32 * (BOX_W + GAP_X)) * zoom, origin.y + (MARGIN + r as f32 * (BOX_H + GAP_Y)) * zoom),
        Vec2::new(BOX_W * zoom, BOX_H * zoom),
    )
}

pub fn network_view(app: &mut App, ui: &mut Ui, area: Rect, cmds: &mut Vec<Cmd>) {
    let p = ui.painter_at(area);
    p.rect_filled(area, 0.0, Color32::WHITE);
    let zoom = app.nd_zoom.clamp(0.3, 2.0);
    let boxes = layout(app);
    let cols = boxes.iter().map(|b| b.1).max().map(|c| c + 1).unwrap_or(1);
    let rows = boxes.iter().map(|b| b.2).max().map(|r| r + 1).unwrap_or(1);
    let content = Vec2::new((MARGIN * 2.0 + cols as f32 * (BOX_W + GAP_X)) * zoom, (MARGIN * 2.0 + rows as f32 * (BOX_H + GAP_Y)) * zoom);
    app.nd_scroll.x = app.nd_scroll.x.clamp(0.0, (content.x - area.width()).max(0.0));
    app.nd_scroll.y = app.nd_scroll.y.clamp(0.0, (content.y - area.height()).max(0.0));
    let origin = area.min - app.nd_scroll;
    let pos: HashMap<usize, Rect> = boxes.iter().map(|&(i, c, r)| (i, box_rect(origin, zoom, c, r))).collect();
    let fmt_crit = true;
    let red = Color32::from_rgb(0xE0, 0x24, 0x24);
    let blue = Color32::from_rgb(0x2F, 0x6F, 0xBF);

    // links (under the boxes)
    for &(i, _, _) in &boxes {
        let t = &app.doc.tasks[i];
        let Some(sr) = pos.get(&i) else { continue };
        for l in &t.predecessors {
            let Some(pi) = app.doc.index_of_uid(l.pred_uid) else { continue };
            let Some(pr) = pos.get(&pi) else { continue };
            let crit = fmt_crit && app.doc.tasks[pi].critical && t.critical;
            let col = if crit { red } else { blue };
            let a = Pos2::new(pr.right(), pr.center().y);
            let b = Pos2::new(sr.left(), sr.center().y);
            let mx = if b.x > a.x { a.x + (GAP_X * zoom) / 2.0 } else { a.x + 10.0 * zoom };
            let pts = if (a.y - b.y).abs() < 0.5 && b.x > a.x { vec![a, b] } else { vec![a, Pos2::new(mx, a.y), Pos2::new(mx, b.y), b] };
            p.add(Shape::line(pts, Stroke::new(1.2_f32, col)));
            let s = 5.0 * zoom.max(0.6);
            p.add(Shape::convex_polygon(vec![b, b + Vec2::new(-s, -s * 0.6), b + Vec2::new(-s, s * 0.6)], col, Stroke::NONE));
        }
    }

    // boxes
    let fs = (10.5 * zoom).max(6.0);
    let font = FontId::proportional(fs);
    let bold = theme::bold(fs);
    let sel_task = app.current_task();
    for &(i, c, r) in &boxes {
        let br = box_rect(origin, zoom, c, r);
        if !br.intersects(area) {
            continue;
        }
        let t = &app.doc.tasks[i];
        let crit = t.critical;
        let edge = if crit { red } else { blue };
        let fill = if crit { Color32::from_rgb(0xFF, 0xEC, 0xEC) } else { Color32::from_rgb(0xE8, 0xF1, 0xFB) };
        if t.is_milestone() {
            // MPP draws milestones as a hexagon
            let k = 10.0 * zoom;
            let poly = vec![
                Pos2::new(br.left() + k, br.top()),
                Pos2::new(br.right() - k, br.top()),
                Pos2::new(br.right(), br.center().y),
                Pos2::new(br.right() - k, br.bottom()),
                Pos2::new(br.left() + k, br.bottom()),
                Pos2::new(br.left(), br.center().y),
            ];
            p.add(Shape::convex_polygon(poly, fill, Stroke::new(1.6_f32, edge)));
        } else {
            p.rect_filled(br, 2.0, fill);
            p.rect_stroke(br, 2.0, Stroke::new(if crit { 1.8_f32 } else { 1.2_f32 }, edge), egui::StrokeKind::Inside);
        }
        if sel_task == Some(i) {
            p.rect_stroke(br.expand(2.5), 3.0, Stroke::new(2.0_f32, theme::ROW_SEL_LINE), egui::StrokeKind::Outside);
        }
        let cp = p.with_clip_rect(br.shrink(2.0).intersect(area));
        let lh = br.height() / 4.6;
        let name_r = Rect::from_min_max(br.min + Vec2::new(4.0, 2.0), Pos2::new(br.right() - 4.0, br.top() + lh * 1.5));
        let g = cp.layout(crate::thai::shape(&t.name).to_string(), bold.clone(), theme::TEXT, name_r.width());
        cp.galley(Pos2::new(name_r.center().x - g.size().x / 2.0, name_r.top()), g, theme::TEXT);
        let y1 = br.top() + lh * 1.6;
        let line = |cp: &egui::Painter, y: f32, l: String, r: String| {
            cp.text(Pos2::new(br.left() + 5.0, y), Align2::LEFT_TOP, l, font.clone(), theme::TEXT);
            cp.text(Pos2::new(br.right() - 5.0, y), Align2::RIGHT_TOP, r, font.clone(), theme::TEXT);
        };
        line(&cp, y1, format!("{} {}", tr("Start:"), app.fmt_date(t.start)), format!("{} {}", tr("ID:"), i + 1));
        line(&cp, y1 + lh, format!("{} {}", tr("Finish:"), app.fmt_finish(t.finish)), format!("{} {}", tr("Dur:"), app.task_duration_text(t)));
        let res = app.doc.resource_names(t);
        cp.text(Pos2::new(br.left() + 5.0, y1 + lh * 2.0), Align2::LEFT_TOP, format!("{} {}", tr("Res:"), crate::thai::shape(&res)), font.clone(), theme::TEXT);
        // progress marks: a diagonal once started, a cross when finished
        let ink = Stroke::new(1.0_f32, edge);
        if t.percent_complete > 0 {
            p.line_segment([br.left_bottom(), br.right_top()], ink);
        }
        if t.percent_complete == 100 {
            p.line_segment([br.left_top(), br.right_bottom()], ink);
        }
    }

    // interaction: select / open a box, pan the diagram, Ctrl+wheel zoom
    let resp = ui.interact(area, ui.scope_id().with("network"), Sense::click_and_drag());
    if let Some(pp) = resp.interact_pointer_pos() {
        let hit = boxes.iter().find(|&&(_, c, r)| box_rect(origin, zoom, c, r).contains(pp)).map(|b| b.0);
        if resp.clicked() || resp.double_clicked() || resp.secondary_clicked() {
            if let Some(i) = hit {
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
    if resp.dragged() {
        app.nd_scroll -= resp.drag_delta();
    }
    if resp.hovered() {
        let (scroll, ctrl) = ui.input(|i| (i.smooth_scroll_delta, i.modifiers.ctrl || i.modifiers.command));
        if ctrl && scroll.y != 0.0 {
            app.nd_zoom = (app.nd_zoom * 1.1_f32.powf(scroll.y / 40.0)).clamp(0.3, 2.0);
        }
    }
    if boxes.is_empty() {
        p.text(area.center(), Align2::CENTER_CENTER, tr("No tasks to show."), FontId::proportional(13.0), theme::TEXT_DIM);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DurUnit, Link, LinkType, Project, Task};

    #[test]
    fn boxes_follow_their_predecessors() {
        let ctx = egui::Context::default();
        let mut app = App::with_ctx(&ctx, None);
        let mon = chrono::NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
        app.doc = Project::new();
        app.doc.start = mon;
        // A -> B, A -> C, B -> D, C -> D
        for uid in 1..=4 {
            let mut t = Task::new(uid, &format!("T{uid}"), mon);
            t.set_duration_minutes(480, DurUnit::Days, false, 480);
            app.doc.tasks.push(t);
        }
        app.doc.tasks[1].predecessors.push(Link::new(1, LinkType::FS));
        app.doc.tasks[2].predecessors.push(Link::new(1, LinkType::FS));
        app.doc.tasks[3].predecessors.push(Link::new(2, LinkType::FS));
        app.doc.tasks[3].predecessors.push(Link::new(3, LinkType::FS));
        app.doc.next_uid = 5;
        app.recalc();
        let l = layout(&app);
        let at = |i: usize| l.iter().find(|b| b.0 == i).map(|b| (b.1, b.2)).unwrap();
        assert_eq!(at(0), (0, 0));
        assert_eq!(at(1), (1, 0));
        assert_eq!(at(2), (1, 1));
        assert_eq!(at(3), (2, 0));
    }
}
