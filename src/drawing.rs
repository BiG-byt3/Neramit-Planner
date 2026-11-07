//! Format > Drawing: text boxes, lines, arrows, rectangles and ovals drawn on the Gantt chart, as
//! in MPP. A shape is attached to the timescale (its left and right edges are dates) and to
//! the rows (its top and bottom in pixels from the first row), so it scrolls and zooms with the
//! bars. Pick a shape from the Drawing menu, then drag on the chart; click a shape to select it,
//! drag to move it, double-click a text box to edit its text, Delete removes it.

use crate::app::App;
use crate::i18n::tr;
use crate::ribbon::Cmd;
use crate::theme;
use chrono::NaiveDateTime;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Drawing {
    /// 0 text box, 1 line, 2 arrow, 3 rectangle, 4 oval
    pub kind: u8,
    pub x0: NaiveDateTime,
    pub x1: NaiveDateTime,
    /// pixels from the top of the first row
    pub y0: f32,
    pub y1: f32,
    #[serde(default)]
    pub text: String,
}

pub const SHAPES: [&str; 5] = ["Text Box", "Line", "Arrow", "Rectangle", "Oval"];

fn dist_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = if ab.length_sq() > 0.0 { ((p - a).dot(ab) / ab.length_sq()).clamp(0.0, 1.0) } else { 0.0 };
    (a + ab * t - p).length()
}

/// Draw the shapes over the chart and handle the drawing tool, selection, moving and deleting.
/// `to_x` / `from_x` convert between instants and screen x.
pub fn chart(app: &mut App, ui: &mut Ui, body: Rect, to_x: &dyn Fn(NaiveDateTime) -> f32, from_x: &dyn Fn(f32) -> NaiveDateTime, cmds: &mut Vec<Cmd>) {
    let p = ui.painter_at(body);
    let top = body.top() - app.scroll_y;
    let screen = |d: &Drawing| -> (Pos2, Pos2) { (Pos2::new(to_x(d.x0), top + d.y0), Pos2::new(to_x(d.x1), top + d.y1)) };
    // the shapes
    for (k, d) in app.doc.drawings.iter().enumerate() {
        let (a, b) = screen(d);
        let r = Rect::from_two_pos(a, b);
        if !r.expand(4.0).intersects(body) {
            continue;
        }
        let ink = Stroke::new(1.2_f32, theme::TEXT);
        match d.kind {
            0 => {
                p.rect_filled(r, 0.0, Color32::WHITE);
                p.rect_stroke(r, 0.0, ink, egui::StrokeKind::Inside);
                let g = p.layout(crate::thai::shape(&d.text).to_string(), FontId::proportional(12.0), theme::TEXT, (r.width() - 8.0).max(10.0));
                p.with_clip_rect(r).galley(r.min + Vec2::new(4.0, 3.0), g, theme::TEXT);
            }
            1 | 2 => {
                p.line_segment([a, b], ink);
                if d.kind == 2 {
                    let dir = (b - a).normalized();
                    let n = Vec2::new(-dir.y, dir.x);
                    p.add(Shape::convex_polygon(vec![b, b - dir * 9.0 + n * 4.0, b - dir * 9.0 - n * 4.0], theme::TEXT, Stroke::NONE));
                }
            }
            3 => {
                p.rect_stroke(r, 0.0, ink, egui::StrokeKind::Inside);
            }
            _ => {
                p.add(Shape::ellipse_stroke(r.center(), r.size() / 2.0, ink));
            }
        }
        if app.drawing_sel == Some(k) {
            for c in [r.left_top(), r.right_top(), r.left_bottom(), r.right_bottom()] {
                p.rect_filled(Rect::from_center_size(c, Vec2::splat(6.0)), 0.0, theme::GREEN);
            }
        }
    }
    // a shape being drawn with the tool
    if let Some(kind) = app.drawing_tool {
        let resp = ui.interact(body, ui.scope_id().with("drawing_tool"), Sense::click_and_drag());
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        if resp.drag_started() {
            app.drawing_from = resp.interact_pointer_pos();
        }
        if let (Some(a), Some(b)) = (app.drawing_from, resp.interact_pointer_pos()) {
            if resp.dragged() {
                p.rect_stroke(Rect::from_two_pos(a, b), 0.0, Stroke::new(1.0_f32, theme::GREEN), egui::StrokeKind::Inside);
            }
        }
        let done = resp.drag_stopped() || resp.clicked();
        if done {
            let b = resp.interact_pointer_pos().unwrap_or(body.center());
            let a = app.drawing_from.take().unwrap_or(b);
            // a click gives a shape of the default size
            let (a, b) = if (b - a).length() < 6.0 { (a, a + if kind == 1 || kind == 2 { Vec2::new(100.0, 0.0) } else { Vec2::new(140.0, 44.0) }) } else { (a, b) };
            let (a, b) = if kind == 1 || kind == 2 { (a, b) } else { (a.min(b), a.max(b)) };
            app.checkpoint();
            app.doc.drawings.push(Drawing { kind, x0: from_x(a.x), x1: from_x(b.x), y0: a.y - top, y1: b.y - top, text: String::new() });
            app.drawing_sel = Some(app.doc.drawings.len() - 1);
            app.drawing_tool = None;
            app.dirty = true;
            if kind == 0 {
                cmds.push(Cmd::DrawingText);
            }
        }
        return;
    }
    // select / move / edit (the top shape wins)
    let mut moved: Option<(usize, Vec2)> = None;
    for k in (0..app.doc.drawings.len()).rev() {
        let d = &app.doc.drawings[k];
        let (a, b) = screen(d);
        let r = Rect::from_two_pos(a, b).expand(4.0);
        if !r.intersects(body) {
            continue;
        }
        let resp = ui.interact(r.intersect(body), ui.scope_id().with(("drawing", k)), Sense::click_and_drag());
        let on_line = |pos: Pos2| d.kind > 2 || d.kind == 0 || dist_to_segment(pos, a, b) <= 5.0;
        let hit = resp.interact_pointer_pos().or(resp.hover_pos()).map(on_line).unwrap_or(false);
        if !hit {
            continue;
        }
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Move);
        }
        if resp.clicked() || resp.drag_started() {
            app.drawing_sel = Some(k);
        }
        if resp.double_clicked() && d.kind == 0 {
            app.drawing_sel = Some(k);
            cmds.push(Cmd::DrawingText);
        }
        if resp.drag_started() {
            app.checkpoint();
        }
        if resp.dragged() {
            moved = Some((k, resp.drag_delta()));
        }
        break;
    }
    if let Some((k, dv)) = moved {
        let shift = |t: NaiveDateTime| from_x(to_x(t) + dv.x);
        let d = &mut app.doc.drawings[k];
        d.x0 = shift(d.x0);
        d.x1 = shift(d.x1);
        d.y0 += dv.y;
        d.y1 += dv.y;
        app.dirty = true;
    }
    if let Some(k) = app.drawing_sel {
        if app.editing.is_none() && app.dialog.is_none() && ui.input(|i| i.key_pressed(egui::Key::Delete)) && k < app.doc.drawings.len() {
            app.checkpoint();
            app.doc.drawings.remove(k);
            app.drawing_sel = None;
            app.dirty = true;
        }
    }
    let _ = (Align2::CENTER_CENTER, tr(""));
}
