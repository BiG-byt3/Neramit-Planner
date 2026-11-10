//! Neramit Planner's own icon set, drawn as vectors so it stays sharp at every display scale.
//!
//! One drawing language for every icon: rounded shapes, one outline weight, task bars as rounded
//! bars, pages as rounded cards with a coloured band, and the action of a command (add, confirm,
//! remove, send out) shown as a round badge in the bottom-right corner. Colours are the program's
//! palette: blue / light blue bars, green accents, red for warnings, yellow for notes.

use crate::theme::{bold, Icon, DISABLED};
use eframe::egui::{self, Color32, CornerRadius, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2};
use std::f32::consts::PI;

// The icon palette: every hue has a main shade, a light fill and a deep edge, so shapes are
// outlined in their own colour (two-tone) instead of a neutral grey.
const fn c(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}
/// Slate ink for outlines of white pages, text lines and handles.
const INK: Color32 = c(0x3E, 0x4B, 0x5E);
const BLUE: Color32 = c(0x2F, 0x74, 0xD0);
const BLUE_DEEP: Color32 = c(0x1D, 0x4F, 0x99);
/// Light blue task bars.
const SKY: Color32 = c(0x8F, 0xC0, 0xF5);
const TINT: Color32 = c(0xEA, 0xF3, 0xFE);
const GREEN: Color32 = c(0x1F, 0x9D, 0x55);
const GREEN_DEEP: Color32 = c(0x13, 0x72, 0x40);
const MINT: Color32 = c(0xCF, 0xEF, 0xDB);
const RED: Color32 = c(0xE5, 0x48, 0x4D);
const RED_DEEP: Color32 = c(0xB4, 0x23, 0x30);
const ORANGE: Color32 = c(0xF5, 0x92, 0x2F);
const ORANGE_DEEP: Color32 = c(0xC7, 0x6A, 0x10);
const YELLOW: Color32 = c(0xFF, 0xC9, 0x3C);
const YELLOW_EDGE: Color32 = c(0xD6, 0x9A, 0x00);
const PURPLE: Color32 = c(0x8B, 0x5C, 0xF6);
const PURPLE_DEEP: Color32 = c(0x5B, 0x34, 0xC9);
const LAVENDER: Color32 = c(0xF0, 0xEA, 0xFE);
/// Light fills of a greyed-out icon.
const DISABLED_LIGHT: Color32 = c(0xEC, 0xEC, 0xEC);
/// Cool grey for baselines, hidden things and thin rules.
const GREY: Color32 = c(0xA3, 0xAE, 0xBC);
const TAN: Color32 = c(0xDB, 0xA9, 0x68);
const TAN_EDGE: Color32 = c(0xA8, 0x76, 0x3A);

/// What a corner badge says.
#[derive(Clone, Copy, PartialEq)]
enum Badge {
    Plus,
    Minus,
    Check,
    Cross,
    Down,
    Right,
    Info,
    Dot,
}

/// A painter scaled to the icon's box: coordinates are fractions of its width / height.
struct Pen<'a> {
    p: &'a Painter,
    r: Rect,
    w: f32,
    h: f32,
    en: bool,
}

type P = (f32, f32);

impl Pen<'_> {
    fn at(&self, x: f32, y: f32) -> Pos2 {
        Pos2::new(self.r.left() + x * self.w, self.r.top() + y * self.h)
    }
    fn rr(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
        Rect::from_min_max(self.at(x0, y0), self.at(x1, y1))
    }
    /// A colour, or the disabled grey.
    fn c(&self, c: Color32) -> Color32 {
        if self.en || c == Color32::WHITE {
            c
        } else if c.r() as u32 + c.g() as u32 + c.b() as u32 > 600 {
            // light fills stay light, so a greyed icon keeps its outline look
            DISABLED_LIGHT
        } else {
            DISABLED
        }
    }
    /// The outline weight shared by every icon.
    fn sw(&self) -> f32 {
        // 1.25 px on a 28 px ribbon icon, 1 px on the small ones
        (self.w / 22.0).max(1.0)
    }
    fn round(&self) -> f32 {
        (self.w * 0.08).max(1.5)
    }
    fn stroke(&self, c: Color32, k: f32) -> Stroke {
        Stroke::new(self.sw() * k, self.c(c))
    }
    /// A rounded card.
    fn card(&self, x0: f32, y0: f32, x1: f32, y1: f32, fill: Color32, edge: Color32) {
        let b = self.rr(x0, y0, x1, y1);
        self.p.rect_filled(b, self.round(), self.c(fill));
        self.p.rect_stroke(b, self.round(), self.stroke(edge, 1.0), StrokeKind::Inside);
    }
    /// A white rounded card with a coloured band along its top.
    fn band_card(&self, x0: f32, y0: f32, x1: f32, y1: f32, band_h: f32, band: Color32, edge: Color32) {
        let b = self.rr(x0, y0, x1, y1);
        let r = self.round();
        self.p.rect_filled(b, r, Color32::WHITE);
        let k = r.round().clamp(0.0, 255.0) as u8;
        self.p.rect_filled(self.rr(x0, y0, x1, y0 + band_h), CornerRadius { nw: k, ne: k, sw: 0, se: 0 }, self.c(band));
        self.p.rect_stroke(b, r, self.stroke(edge, 1.0), StrokeKind::Inside);
    }
    /// A rounded bar (a task in the Gantt).
    fn bar(&self, x0: f32, y0: f32, x1: f32, y1: f32, fill: Color32) {
        let b = self.rr(x0, y0, x1, y1);
        self.p.rect_filled(b, (b.height() / 2.0).min(self.round() * 1.4), self.c(fill));
    }
    /// A rounded bar with an outline.
    fn bar_o(&self, x0: f32, y0: f32, x1: f32, y1: f32, fill: Color32, edge: Color32) {
        let b = self.rr(x0, y0, x1, y1);
        let rad = (b.height() / 2.0).min(self.round() * 1.4);
        self.p.rect_filled(b, rad, self.c(fill));
        self.p.rect_stroke(b, rad, self.stroke(edge, 1.0), StrokeKind::Inside);
    }
    fn line(&self, pts: &[P], c: Color32, k: f32) {
        let pts: Vec<Pos2> = pts.iter().map(|&(x, y)| self.at(x, y)).collect();
        let ends = [pts[0], pts[pts.len() - 1]];
        self.p.add(Shape::line(pts, self.stroke(c, k)));
        // round the ends
        let rad = self.sw() * k / 2.0;
        if rad > 0.7 {
            for q in ends {
                self.p.circle_filled(q, rad, self.c(c));
            }
        }
    }
    fn dot(&self, x: f32, y: f32, rad: f32, c: Color32) {
        self.p.circle_filled(self.at(x, y), self.w * rad, self.c(c));
    }
    fn ring(&self, x: f32, y: f32, rad: f32, c: Color32, k: f32) {
        self.p.circle_stroke(self.at(x, y), self.w * rad, self.stroke(c, k));
    }
    fn poly(&self, pts: &[P], fill: Color32) {
        self.p.add(Shape::convex_polygon(pts.iter().map(|&(x, y)| self.at(x, y)).collect(), self.c(fill), Stroke::NONE));
    }
    /// An arrowhead with its tip at `tip`, pointing along `dir`, `size` of the icon wide.
    fn head(&self, tip: P, dir: Vec2, size: f32, c: Color32) {
        let d = dir.normalized();
        let n = Vec2::new(-d.y, d.x);
        let t = self.at(tip.0, tip.1);
        let s = self.w * size;
        self.p.add(Shape::convex_polygon(vec![t, t - d * s * 1.1 + n * s * 0.8, t - d * s * 1.1 - n * s * 0.8], self.c(c), Stroke::NONE));
    }
    /// A line ending in an arrowhead.
    fn arrow(&self, pts: &[P], c: Color32, k: f32) {
        let n = pts.len();
        let (a, b) = (self.at(pts[n - 2].0, pts[n - 2].1), self.at(pts[n - 1].0, pts[n - 1].1));
        let dir = (b - a).normalized();
        let size = 0.12;
        let mut body = pts.to_vec();
        let back = (b - dir * self.w * size * 0.9 - self.r.min) / Vec2::new(self.w, self.h);
        body[n - 1] = (back.x, back.y);
        self.line(&body, c, k);
        self.head(pts[n - 1], dir, size, c);
    }
    fn diamond(&self, x: f32, y: f32, d: f32, c: Color32) {
        let k = d * self.w;
        let o = self.at(x, y);
        self.p.add(Shape::convex_polygon(vec![o + Vec2::new(0.0, -k), o + Vec2::new(k, 0.0), o + Vec2::new(0.0, k), o + Vec2::new(-k, 0.0)], self.c(c), Stroke::NONE));
    }
    /// A person: round head over rounded shoulders, `s` of the icon tall, centred on `x`.
    fn person(&self, x: f32, top: f32, s: f32, c: Color32) {
        self.dot(x, top + s * 0.24, s * 0.22, c);
        let b = self.rr(x - s * 0.4, top + s * 0.54, x + s * 0.4, top + s);
        let k = (b.height() * 0.6).round().clamp(0.0, 255.0) as u8;
        self.p.rect_filled(b, CornerRadius { nw: k, ne: k, sw: 1, se: 1 }, self.c(c));
    }
    /// A magnifying glass (lens centre and radius).
    fn lens(&self, x: f32, y: f32, rad: f32) {
        self.p.circle_filled(self.at(x, y), self.w * rad, self.c(TINT));
        self.ring(x, y, rad, INK, 1.3);
        let k = rad * 0.72;
        self.line(&[(x + k, y + k), (0.88, 0.88)], INK, 2.0);
    }
    /// The round badge in the bottom-right corner.
    fn badge(&self, c: Color32, b: Badge) {
        let o = self.at(0.79, 0.79);
        let rad = self.w * 0.19;
        self.p.circle_filled(o, rad + self.sw(), Color32::WHITE);
        self.p.circle_filled(o, rad, self.c(c));
        let g = Stroke::new((self.w / 18.0).max(1.0), Color32::WHITE);
        let k = rad * 0.52;
        let v = |x: f32, y: f32| o + Vec2::new(x * k, y * k);
        match b {
            Badge::Plus => {
                self.p.line_segment([v(-1.0, 0.0), v(1.0, 0.0)], g);
                self.p.line_segment([v(0.0, -1.0), v(0.0, 1.0)], g);
            }
            Badge::Minus => {
                self.p.line_segment([v(-1.0, 0.0), v(1.0, 0.0)], g);
            }
            Badge::Check => {
                self.p.add(Shape::line(vec![v(-0.95, 0.05), v(-0.25, 0.7), v(0.95, -0.7)], g));
            }
            Badge::Cross => {
                self.p.line_segment([v(-0.75, -0.75), v(0.75, 0.75)], g);
                self.p.line_segment([v(0.75, -0.75), v(-0.75, 0.75)], g);
            }
            Badge::Down | Badge::Right => {
                let d = if b == Badge::Down { Vec2::new(0.0, 1.0) } else { Vec2::new(1.0, 0.0) };
                let n = Vec2::new(-d.y, d.x);
                self.p.line_segment([o - d * k, o + d * k * 0.3], g);
                let t = o + d * k * 1.05;
                self.p.add(Shape::convex_polygon(vec![t, t - d * k * 0.9 + n * k * 0.8, t - d * k * 0.9 - n * k * 0.8], Color32::WHITE, Stroke::NONE));
            }
            Badge::Info => {
                self.p.circle_filled(v(0.0, -0.75), k * 0.22, Color32::WHITE);
                self.p.line_segment([v(0.0, -0.2), v(0.0, 0.85)], g);
            }
            Badge::Dot => {
                self.p.circle_filled(o, k * 0.55, Color32::WHITE);
            }
        }
    }
    /// The scheduling icons' task bar: same place and size in Manual, Auto, Move and Trace.
    fn task_bar(&self, solid: bool) {
        if solid {
            self.bar(0.04, 0.62, 0.74, 0.86, BLUE);
        } else {
            self.bar_o(0.04, 0.62, 0.74, 0.86, SKY, BLUE);
        }
    }
    /// A page with text lines.
    fn page(&self, x0: f32, y0: f32, x1: f32, y1: f32) {
        self.card(x0, y0, x1, y1, Color32::WHITE, INK);
    }
    /// A sheet: a card with a header band, a first column and row lines.
    fn sheet(&self, band: Color32) {
        self.band_card(0.06, 0.12, 0.94, 0.88, 0.22, band, INK);
        let thin = self.stroke(GREY, 0.9);
        for y in [0.53, 0.7] {
            self.p.line_segment([self.at(0.06, y), self.at(0.94, y)], thin);
        }
        self.p.line_segment([self.at(0.36, 0.34), self.at(0.36, 0.88)], thin);
    }
    /// Column chart bars standing on `base` (heights as fractions of the icon).
    fn columns(&self, x0: f32, step: f32, bw: f32, base: f32, heights: &[f32], colors: &[Color32]) {
        for (i, hh) in heights.iter().enumerate() {
            let x = x0 + i as f32 * step;
            let b = self.rr(x, base - hh, x + bw, base);
            let k = (b.width() * 0.35).round().clamp(0.0, 255.0) as u8;
            self.p.rect_filled(b, CornerRadius { nw: k, ne: k, sw: 0, se: 0 }, self.c(colors[i % colors.len()]));
        }
    }
    /// Corner marks of a viewfinder around a box.
    fn brackets(&self, x0: f32, y0: f32, x1: f32, y1: f32, c: Color32) {
        let l = 0.18;
        for (x, y, dx, dy) in [(x0, y0, 1.0, 1.0), (x1, y0, -1.0, 1.0), (x0, y1, 1.0, -1.0), (x1, y1, -1.0, -1.0)] {
            self.line(&[(x + dx * l, y), (x, y), (x, y + dy * l)], c, 1.4);
        }
    }
    /// A circular arrow: centre, radius, start and end angle (half turns, counter-clockwise).
    fn turn(&self, x: f32, y: f32, rad: f32, a0: f32, a1: f32, c: Color32, k: f32) {
        let o = self.at(x, y);
        let r = self.w * rad;
        let pt = |a: f32| o + Vec2::new((PI * a).cos() * r, -(PI * a).sin() * r);
        let pts: Vec<Pos2> = (0..=24).map(|i| pt(a0 + (a1 - a0) * i as f32 / 24.0)).collect();
        self.p.add(Shape::line(pts, self.stroke(c, k)));
        let a = PI * a1;
        let sign = if a1 > a0 { 1.0 } else { -1.0 };
        let dir = Vec2::new(-a.sin(), -a.cos()) * sign;
        let tip = pt(a1) + dir * self.w * 0.16;
        let q = (tip - self.r.min) / Vec2::new(self.w, self.h);
        self.head((q.x, q.y), dir, 0.13, c);
    }
}

/// Paint one of the program's icons into `r`.
pub fn paint_icon(p: &Painter, r: Rect, icon: Icon, enabled: bool) {
    let k = Pen { p, r, w: r.width(), h: r.height(), en: enabled };
    let w = k.w;
    match icon {
        Icon::Gantt => {
            k.page(0.06, 0.1, 0.94, 0.9);
            k.bar(0.16, 0.24, 0.54, 0.38, BLUE);
            k.bar(0.32, 0.45, 0.72, 0.59, ORANGE);
            k.bar(0.5, 0.66, 0.86, 0.8, GREEN);
        }
        Icon::Sheet => k.sheet(SKY),
        Icon::ResourceSheet => {
            k.sheet(MINT);
            k.badge(GREEN, Badge::Dot);
        }
        Icon::Paste => {
            k.card(0.1, 0.12, 0.7, 0.94, TAN, TAN_EDGE);
            k.bar(0.26, 0.04, 0.54, 0.2, INK);
            k.card(0.42, 0.38, 0.94, 0.96, Color32::WHITE, BLUE);
            for y in [0.56, 0.72] {
                k.line(&[(0.54, y), (0.82, y)], SKY, 1.2);
            }
        }
        Icon::Cut => {
            k.line(&[(0.38, 0.64), (0.72, 0.06)], INK, 1.15);
            k.line(&[(0.62, 0.64), (0.28, 0.06)], INK, 1.15);
            k.ring(0.3, 0.77, 0.14, RED, 1.25);
            k.ring(0.7, 0.77, 0.14, RED, 1.25);
            k.dot(0.5, 0.36, 0.05, INK);
        }
        Icon::Copy => {
            k.card(0.06, 0.06, 0.62, 0.66, TINT, BLUE);
            k.card(0.36, 0.32, 0.94, 0.94, Color32::WHITE, BLUE);
            for y in [0.52, 0.68, 0.82] {
                k.line(&[(0.48, y), (0.82, y)], SKY, 1.2);
            }
        }
        Icon::Painter => {
            // a paint roller
            k.bar_o(0.08, 0.08, 0.8, 0.36, YELLOW, YELLOW_EDGE);
            k.line(&[(0.8, 0.22), (0.92, 0.22), (0.92, 0.5), (0.5, 0.54), (0.5, 0.62)], INK, 1.3);
            k.bar(0.42, 0.6, 0.58, 0.96, INK);
        }
        Icon::Link | Icon::Unlink => {
            k.bar(0.04, 0.15, 0.46, 0.31, BLUE);
            k.bar(0.54, 0.7, 0.96, 0.86, BLUE);
            k.arrow(&[(0.46, 0.23), (0.7, 0.23), (0.7, 0.68)], BLUE_DEEP, 1.1);
            if icon == Icon::Unlink {
                k.badge(RED, Badge::Cross);
            }
        }
        Icon::Indent | Icon::Outdent => {
            for (i, y) in [0.14f32, 0.38, 0.62, 0.86].iter().enumerate() {
                let x0 = if i == 1 || i == 2 { 0.48 } else { 0.08 };
                k.line(&[(x0, *y), (0.92, *y)], INK, 1.1);
            }
            if icon == Icon::Indent {
                k.arrow(&[(0.06, 0.5), (0.38, 0.5)], GREEN, 1.3);
            } else {
                k.arrow(&[(0.38, 0.5), (0.06, 0.5)], GREEN, 1.3);
            }
        }
        Icon::Manual => {
            // dates set by hand: the task bar with a padlock
            k.task_bar(false);
            let shackle: Vec<Pos2> = (0..=12)
                .map(|i| {
                    let a = PI * i as f32 / 12.0;
                    k.at(0.73, 0.25) + Vec2::new(a.cos() * w * 0.12, -a.sin() * w * 0.13)
                })
                .collect();
            p.add(Shape::line(shackle, k.stroke(INK, 1.15)));
            k.line(&[(0.61, 0.25), (0.61, 0.3)], INK, 1.15);
            k.line(&[(0.85, 0.25), (0.85, 0.3)], INK, 1.15);
            k.card(0.54, 0.28, 0.92, 0.52, RED, RED_DEEP);
            k.dot(0.73, 0.39, 0.04, Color32::WHITE);
        }
        Icon::Auto => {
            // dates worked out from the links: the task bar with a turning arrow
            k.task_bar(true);
            k.turn(0.73, 0.28, 0.16, 0.35, 1.9, GREEN, 1.25);
        }
        Icon::Task => k.bar_o(0.06, 0.36, 0.94, 0.64, SKY, BLUE),
        Icon::Summary => {
            // the summary bracket over two of its tasks
            k.bar(0.06, 0.16, 0.94, 0.27, BLUE_DEEP);
            k.dot(0.1, 0.215, 0.085, BLUE_DEEP);
            k.dot(0.9, 0.215, 0.085, BLUE_DEEP);
            k.bar_o(0.14, 0.5, 0.56, 0.64, SKY, BLUE);
            k.bar_o(0.44, 0.74, 0.86, 0.88, SKY, BLUE);
        }
        Icon::Milestone => {
            k.diamond(0.5, 0.5, 0.36, ORANGE_DEEP);
            k.diamond(0.5, 0.5, 0.27, ORANGE);
            k.diamond(0.5, 0.5, 0.09, Color32::WHITE);
        }
        Icon::Info => {
            // a light disc with a blue ring and a blue "i"
            k.dot(0.5, 0.5, 0.42, TINT);
            k.ring(0.5, 0.5, 0.42, BLUE, 1.2);
            k.dot(0.5, 0.29, 0.055, BLUE);
            k.line(&[(0.5, 0.43), (0.5, 0.72)], BLUE, 1.7);
        }
        Icon::Notes => {
            k.card(0.14, 0.08, 0.86, 0.94, YELLOW, YELLOW_EDGE);
            for y in [0.38, 0.56, 0.74] {
                k.line(&[(0.3, y), (0.7, y)], INK, 0.9);
            }
            k.dot(0.5, 0.18, 0.07, RED);
        }
        Icon::Timeline => {
            // a ruler with ticks, a bar and a diamond above it
            k.line(&[(0.05, 0.8), (0.95, 0.8)], INK, 1.2);
            for x in [0.05, 0.35, 0.65, 0.95] {
                k.line(&[(x, 0.68), (x, 0.8)], INK, 1.0);
            }
            k.bar(0.06, 0.34, 0.62, 0.5, BLUE);
            k.diamond(0.8, 0.42, 0.12, ORANGE);
        }
        Icon::Scroll => {
            // a target on the bar's start
            k.bar(0.3, 0.42, 0.96, 0.58, SKY);
            k.ring(0.27, 0.5, 0.2, RED, 1.15);
            k.dot(0.27, 0.5, 0.08, RED);
        }
        Icon::Move => {
            // the task bar with a two-way arrow above it
            k.task_bar(false);
            k.arrow(&[(0.5, 0.3), (0.04, 0.3)], GREEN, 1.2);
            k.arrow(&[(0.5, 0.3), (0.96, 0.3)], GREEN, 1.2);
        }
        Icon::Find => k.lens(0.42, 0.42, 0.28),
        Icon::Calendar | Icon::ProjectInfo => {
            let band = if icon == Icon::Calendar { RED } else { BLUE };
            k.band_card(0.08, 0.16, 0.92, 0.92, 0.2, band, INK);
            for x in [0.3, 0.7] {
                k.line(&[(x, 0.08), (x, 0.24)], INK, 1.3);
            }
            if icon == Icon::Calendar {
                for ix in 0..3 {
                    for iy in 0..2 {
                        let x = 0.22 + ix as f32 * 0.21;
                        let y = 0.48 + iy as f32 * 0.2;
                        let d = if ix == 1 && iy == 1 { RED } else { SKY };
                        k.card(x, y, x + 0.14, y + 0.12, d, d);
                    }
                }
            } else {
                k.line(&[(0.22, 0.5), (0.5, 0.5)], GREY, 1.0);
                k.line(&[(0.22, 0.66), (0.44, 0.66)], GREY, 1.0);
                k.badge(BLUE, Badge::Info);
            }
        }
        Icon::Calculate => {
            k.card(0.18, 0.04, 0.82, 0.96, TINT, BLUE_DEEP);
            k.card(0.28, 0.13, 0.72, 0.3, MINT, GREEN);
            for ix in 0..3 {
                for iy in 0..3 {
                    let x = 0.28 + ix as f32 * 0.16;
                    let y = 0.42 + iy as f32 * 0.17;
                    let c = if ix == 2 && iy == 2 { ORANGE } else { SKY };
                    k.card(x, y, x + 0.12, y + 0.11, c, c);
                }
            }
        }
        Icon::AssignRes => {
            k.person(0.34, 0.08, 0.66, BLUE);
            k.bar_o(0.58, 0.64, 0.96, 0.84, SKY, BLUE);
        }
        Icon::AddRes => {
            k.person(0.36, 0.06, 0.7, BLUE);
            k.badge(GREEN, Badge::Plus);
        }
        Icon::ZoomIn | Icon::ZoomOut => {
            k.lens(0.42, 0.42, 0.3);
            k.line(&[(0.29, 0.42), (0.55, 0.42)], BLUE, 1.6);
            if icon == Icon::ZoomIn {
                k.line(&[(0.42, 0.29), (0.42, 0.55)], BLUE, 1.6);
            }
        }
        Icon::Entire => {
            k.brackets(0.04, 0.1, 0.96, 0.9, INK);
            k.bar(0.16, 0.3, 0.56, 0.46, BLUE);
            k.bar(0.42, 0.56, 0.84, 0.72, BLUE);
        }
        Icon::Selected => {
            k.bar(0.08, 0.2, 0.46, 0.34, SKY);
            k.brackets(0.3, 0.46, 0.96, 0.9, INK);
            k.bar(0.42, 0.58, 0.84, 0.76, GREEN);
        }
        Icon::Report => {
            k.page(0.12, 0.05, 0.88, 0.95);
            k.columns(0.24, 0.19, 0.13, 0.84, &[0.3, 0.5, 0.4], &[BLUE, GREEN, ORANGE]);
        }
        Icon::Undo | Icon::Redo => {
            // on the green title bar: white turning arrows
            let undo = icon == Icon::Undo;
            let (a0, a1) = if undo { (-0.15, 1.0) } else { (1.15, 0.0) };
            let o = k.at(0.5, 0.62);
            let rad = w * 0.32;
            let pt = |a: f32| o + Vec2::new((PI * a).cos() * rad, -(PI * a).sin() * rad);
            let pts: Vec<Pos2> = (0..=16).map(|i| pt(a0 + (a1 - a0) * i as f32 / 16.0)).collect();
            let white = if enabled { Color32::WHITE } else { DISABLED };
            p.add(Shape::line(pts, Stroke::new((w / 11.0).max(1.3), white)));
            let tip = pt(a1) + Vec2::new(0.0, w * 0.16);
            let s = w * 0.17;
            p.add(Shape::convex_polygon(vec![tip, tip + Vec2::new(-s, -s), tip + Vec2::new(s, -s)], white, Stroke::NONE));
        }
        Icon::Percent(v) => {
            // progress as a filled bar (empty track at 0 %, green when done)
            let frac = v as f32 / 100.0;
            k.bar_o(0.02, 0.34, 0.98, 0.66, TINT, GREY);
            if frac > 0.0 {
                let inset = k.sw() / w;
                k.bar(0.02 + inset, 0.34 + inset, 0.02 + inset + (0.96 - 2.0 * inset) * frac, 0.66 - inset, if v >= 100 { GREEN } else { BLUE });
            }
        }
        Icon::Inspect => {
            // Trace: a predecessor bar whose link leads down into the task bar
            k.task_bar(false);
            k.bar(0.04, 0.14, 0.44, 0.3, GREEN);
            k.arrow(&[(0.44, 0.22), (0.62, 0.22), (0.62, 0.58)], GREEN_DEEP, 1.1);
        }
        Icon::Delete => {
            // Clear: an eraser
            let a = Vec2::new(0.18 * w, 0.84 * w);
            let d = Vec2::new(1.0, -1.0).normalized();
            let n = Vec2::new(1.0, 1.0).normalized();
            let (len, hw) = (0.8 * w, 0.14 * w);
            let o = r.min;
            let quad = |t0: f32, t1: f32| {
                let p0 = o + a + d * len * t0;
                let p1 = o + a + d * len * t1;
                vec![p0 + n * hw, p1 + n * hw, p1 - n * hw, p0 - n * hw]
            };
            p.add(Shape::convex_polygon(quad(0.4, 1.0), Color32::WHITE, Stroke::NONE));
            p.add(Shape::convex_polygon(quad(0.0, 0.4), k.c(RED), Stroke::NONE));
            p.add(Shape::closed_line(quad(0.0, 1.0), k.stroke(RED_DEEP, 1.0)));
        }
        Icon::Export => {
            k.page(0.06, 0.06, 0.66, 0.94);
            for y in [0.3, 0.46, 0.62] {
                k.line(&[(0.18, y), (0.54, y)], SKY, 1.2);
            }
            k.badge(GREEN, Badge::Right);
        }
        Icon::Update => {
            k.dot(0.5, 0.5, 0.42, MINT);
            k.ring(0.5, 0.5, 0.42, GREEN, 1.2);
            k.arrow(&[(0.5, 0.22), (0.5, 0.78)], GREEN_DEEP, 1.5);
        }
        Icon::Ai => {
            k.dot(0.5, 0.5, 0.42, LAVENDER);
            k.ring(0.5, 0.5, 0.42, PURPLE, 1.2);
            // a four-pointed sparkle, and a small one at its top right
            let star = |cx: f32, cy: f32, rad: f32| {
                let q = rad * 0.28;
                let pts = [k.at(cx, cy - rad), k.at(cx + q, cy - q), k.at(cx + rad, cy), k.at(cx + q, cy + q), k.at(cx, cy + rad), k.at(cx - q, cy + q), k.at(cx - rad, cy), k.at(cx - q, cy - q)];
                for i in 0..4 {
                    p.add(Shape::convex_polygon(vec![k.at(cx, cy), pts[i * 2], pts[(i * 2 + 1) % 8], pts[(i * 2 + 2) % 8]], k.c(PURPLE_DEEP), Stroke::NONE));
                }
            };
            star(0.45, 0.55, 0.26);
            star(0.7, 0.3, 0.11);
        }
    }
}

/// Icons of the ribbon commands that have no `Icon` of their own, looked up by command name.
pub fn paint_named(p: &Painter, r: Rect, name: &str, enabled: bool) {
    let k = Pen { p, r, w: r.width(), h: r.height(), en: enabled };
    let h = k.h;
    let text = |x: f32, y: f32, s: &str, size: f32, c: Color32| {
        p.text(k.at(x, y), egui::Align2::CENTER_CENTER, s, bold(h * size), k.c(c));
    };
    match name {
        "Task Usage" | "Resource Usage" => {
            k.sheet(if name == "Task Usage" { SKY } else { MINT });
            k.bar(0.44, 0.4, 0.66, 0.5, BLUE);
            k.bar(0.6, 0.57, 0.88, 0.67, BLUE);
        }
        "Task Board" | "Task Boards" => {
            for (i, x) in [0.06f32, 0.38, 0.7].iter().enumerate() {
                k.card(*x, 0.1, x + 0.25, 0.9, TINT, SKY);
                let y = 0.18 + i as f32 * 0.12;
                k.card(x + 0.04, y, x + 0.21, y + 0.16, Color32::WHITE, [BLUE, ORANGE, GREEN][i]);
            }
        }
        "Network Diagram" => {
            k.line(&[(0.36, 0.25), (0.62, 0.5)], INK, 1.0);
            k.line(&[(0.36, 0.75), (0.62, 0.5)], INK, 1.0);
            for (x, y) in [(0.04f32, 0.12f32), (0.04, 0.62)] {
                k.card(x, y, x + 0.36, y + 0.26, SKY, BLUE);
            }
            k.card(0.58, 0.37, 0.94, 0.63, MINT, GREEN);
        }
        "Calendar" => {
            k.band_card(0.08, 0.16, 0.92, 0.92, 0.2, GREEN, INK);
            for x in [0.3, 0.7] {
                k.line(&[(x, 0.08), (x, 0.24)], INK, 1.3);
            }
            for ix in 0..3 {
                for iy in 0..2 {
                    let x = 0.22 + ix as f32 * 0.21;
                    let y = 0.48 + iy as f32 * 0.2;
                    let d = if ix == 2 && iy == 1 { GREEN } else { MINT };
                    k.card(x, y, x + 0.14, y + 0.12, d, GREEN_DEEP);
                }
            }
        }
        "Other Views" | "Tables" | "Column Settings" => k.sheet(SKY),
        "Custom Fields" => {
            k.sheet(MINT);
            k.badge(GREEN, Badge::Plus);
        }
        "Sort" => {
            text(0.3, 0.28, "A", 0.34, BLUE);
            text(0.3, 0.72, "Z", 0.34, BLUE);
            k.arrow(&[(0.72, 0.1), (0.72, 0.92)], INK, 1.2);
        }
        "Outline" => {
            for (i, y) in [0.16f32, 0.39, 0.62, 0.85].iter().enumerate() {
                let x0 = if i == 1 || i == 2 { 0.34 } else { 0.08 };
                k.card(x0, y - 0.07, x0 + 0.13, y + 0.07, BLUE, BLUE);
                k.line(&[(x0 + 0.22, *y), (0.92, *y)], INK, 1.1);
            }
        }
        "Highlight" => {
            k.poly(&[(0.22, 0.62), (0.62, 0.1), (0.84, 0.28), (0.44, 0.8)], YELLOW);
            k.poly(&[(0.22, 0.62), (0.44, 0.8), (0.18, 0.8)], INK);
            k.line(&[(0.08, 0.92), (0.92, 0.92)], YELLOW_EDGE, 1.3);
        }
        "Filter" => {
            k.poly(&[(0.08, 0.12), (0.92, 0.12), (0.6, 0.5), (0.4, 0.5)], TINT);
            k.poly(&[(0.4, 0.5), (0.6, 0.5), (0.6, 0.86), (0.4, 0.76)], TINT);
            k.line(&[(0.08, 0.12), (0.92, 0.12), (0.6, 0.5), (0.6, 0.86), (0.4, 0.76), (0.4, 0.5), (0.08, 0.12)], BLUE, 1.2);
        }
        "Group by" => {
            for y in [0.1f32, 0.4, 0.7] {
                k.card(0.06, y, 0.3, y + 0.2, BLUE, BLUE);
                k.line(&[(0.4, y + 0.1), (0.92, y + 0.1)], INK, 1.1);
            }
        }
        "Zoom" => k.lens(0.42, 0.42, 0.3),
        "Spelling..." => {
            k.lens(0.42, 0.42, 0.3);
            k.line(&[(0.28, 0.42), (0.38, 0.54), (0.56, 0.3)], GREEN, 1.5);
        }
        "New Window" | "Arrange All" | "Switch Windows" | "Hide" => match name {
            "Arrange All" => {
                for x in [0.04f32, 0.54] {
                    k.band_card(x, 0.16, x + 0.42, 0.84, 0.16, SKY, INK);
                }
            }
            "Switch Windows" => {
                k.band_card(0.04, 0.08, 0.7, 0.66, 0.16, GREY, INK);
                k.band_card(0.3, 0.34, 0.96, 0.92, 0.16, SKY, INK);
            }
            _ => {
                k.band_card(0.04, 0.12, 0.88, 0.8, 0.18, if name == "Hide" { GREY } else { SKY }, INK);
                k.badge(if name == "Hide" { GREY } else { GREEN }, if name == "Hide" { Badge::Minus } else { Badge::Plus });
            }
        },
        "Macros" | "View Macros" => {
            k.page(0.12, 0.06, 0.78, 0.94);
            for y in [0.28f32, 0.44, 0.6] {
                k.line(&[(0.26, y), (0.64, y)], GREY, 1.0);
            }
            k.badge(RED, Badge::Dot);
        }
        "Language" => {
            let c = r.center();
            let rad = k.w * 0.4;
            p.circle_filled(c, rad, k.c(TINT));
            p.circle_stroke(c, rad, k.stroke(BLUE, 1.2));
            p.add(Shape::ellipse_stroke(c, Vec2::new(rad * 0.45, rad), k.stroke(BLUE, 1.0)));
            p.line_segment([c + Vec2::new(-rad, 0.0), c + Vec2::new(rad, 0.0)], k.stroke(BLUE, 1.0));
            for s in [-0.5f32, 0.5] {
                p.line_segment([c + Vec2::new(-rad * 0.85, rad * s), c + Vec2::new(rad * 0.85, rad * s)], k.stroke(BLUE, 0.9));
            }
        }
        "Help" | "Feedback" | "Show Training" => {
            k.dot(0.5, 0.5, 0.42, TINT);
            k.ring(0.5, 0.5, 0.42, BLUE, 1.2);
            let mark = match name {
                "Help" => "?",
                "Feedback" => "!",
                _ => "i",
            };
            text(0.5, 0.52, mark, 0.62, BLUE);
        }
        "Text Styles..." | "Font..." => {
            text(0.4, 0.5, "A", 0.62, INK);
            k.bar(0.6, 0.66, 0.94, 0.86, BLUE);
        }
        "Gridlines" | "Layout" => {
            k.page(0.06, 0.1, 0.94, 0.9);
            for y in [0.37f32, 0.63] {
                k.line(&[(0.06, y), (0.94, y)], SKY, 1.0);
            }
            if name == "Layout" {
                k.bar(0.18, 0.18, 0.62, 0.3, BLUE);
                k.bar(0.4, 0.7, 0.84, 0.82, BLUE);
                k.line(&[(0.62, 0.24), (0.7, 0.24), (0.7, 0.66)], INK, 1.0);
            }
        }
        "Align Left" | "Center" | "Align Right" | "Wrap Text" => {
            for (i, y) in [0.2f32, 0.4, 0.6, 0.8].iter().enumerate() {
                let len = if i % 2 == 0 { 0.8 } else { 0.5 };
                let x0 = match name {
                    "Align Left" | "Wrap Text" => 0.1,
                    "Center" => 0.5 - len / 2.0,
                    _ => 0.9 - len,
                };
                k.line(&[(x0, *y), (x0 + len, *y)], INK, 1.1);
            }
        }
        "Format" | "Bar Styles" | "Gantt Chart Gallery" => {
            // a small gallery of bar styles
            k.bar_o(0.04, 0.12, 0.6, 0.28, SKY, BLUE);
            k.bar(0.24, 0.42, 0.8, 0.58, GREEN);
            k.bar(0.4, 0.72, 0.96, 0.88, ORANGE);
        }
        "Task Path" => {
            k.bar(0.06, 0.16, 0.6, 0.34, RED);
            k.arrow(&[(0.6, 0.25), (0.72, 0.25), (0.72, 0.56)], RED_DEEP, 1.0);
            k.bar(0.4, 0.62, 0.94, 0.8, SKY);
        }
        "Baseline" => {
            k.bar(0.06, 0.2, 0.7, 0.4, BLUE);
            k.bar(0.06, 0.52, 0.7, 0.64, GREY);
            k.bar(0.3, 0.76, 0.94, 0.88, SKY);
        }
        "Slippage" => {
            k.bar(0.06, 0.2, 0.6, 0.36, GREY);
            k.line(&[(0.6, 0.28), (0.94, 0.28)], RED, 1.4);
            k.bar(0.4, 0.56, 0.94, 0.76, SKY);
        }
        "Drawing" => {
            // a pencil
            k.poly(&[(0.3, 0.74), (0.74, 0.12), (0.9, 0.26), (0.46, 0.88)], YELLOW);
            k.poly(&[(0.3, 0.74), (0.46, 0.88), (0.12, 0.94)], TAN);
            k.poly(&[(0.12, 0.94), (0.2, 0.82), (0.24, 0.9)], INK);
            k.poly(&[(0.74, 0.12), (0.8, 0.04), (0.96, 0.18), (0.9, 0.26)], RED);
        }
        "Subproject" => {
            k.page(0.12, 0.06, 0.88, 0.94);
            k.bar(0.24, 0.24, 0.62, 0.38, BLUE);
            k.card(0.36, 0.5, 0.78, 0.82, TINT, BLUE);
            k.bar(0.44, 0.6, 0.7, 0.7, BLUE);
        }
        "Links Between Projects..." => {
            k.page(0.04, 0.06, 0.48, 0.62);
            k.page(0.52, 0.38, 0.96, 0.94);
            k.bar(0.12, 0.2, 0.4, 0.3, BLUE);
            k.bar(0.6, 0.72, 0.88, 0.82, BLUE);
            k.arrow(&[(0.4, 0.25), (0.74, 0.25), (0.74, 0.62)], GREEN, 1.2);
        }
        "Compare Projects" => {
            k.page(0.04, 0.1, 0.5, 0.9);
            k.page(0.5, 0.1, 0.96, 0.9);
            k.bar(0.12, 0.3, 0.42, 0.42, BLUE);
            k.bar(0.58, 0.3, 0.88, 0.42, BLUE);
            k.bar(0.12, 0.58, 0.34, 0.7, BLUE);
            k.bar(0.58, 0.58, 0.88, 0.7, GREEN);
        }
        "Move Project" => {
            k.page(0.06, 0.06, 0.7, 0.94);
            k.bar(0.16, 0.28, 0.5, 0.4, BLUE);
            k.bar(0.26, 0.52, 0.6, 0.64, BLUE);
            k.badge(BLUE, Badge::Right);
        }
        "Get Add-ins" | "My Add-ins" => {
            k.card(0.1, 0.3, 0.9, 0.92, TINT, BLUE);
            k.line(&[(0.32, 0.3), (0.32, 0.12), (0.68, 0.12), (0.68, 0.3)], BLUE, 1.2);
            k.dot(0.5, 0.6, 0.1, GREEN);
        }
        "Manage Sprints" => k.turn(0.5, 0.52, 0.3, 0.35, 1.9, BLUE, 1.35),
        "WBS" => {
            for x in [0.06f32, 0.38, 0.7] {
                k.line(&[(0.5, 0.3), (0.5, 0.48), (x + 0.12, 0.48), (x + 0.12, 0.66)], INK, 1.0);
            }
            k.card(0.32, 0.08, 0.68, 0.3, BLUE_DEEP, BLUE_DEEP);
            for x in [0.06f32, 0.38, 0.7] {
                k.card(x, 0.66, x + 0.24, 0.88, SKY, BLUE);
            }
        }
        "Set Baseline" => {
            k.bar(0.06, 0.16, 0.7, 0.34, BLUE);
            k.bar(0.06, 0.44, 0.7, 0.56, GREY);
            k.badge(GREEN, Badge::Check);
        }
        "Update Project..." => {
            k.page(0.1, 0.06, 0.76, 0.94);
            k.bar(0.2, 0.24, 0.6, 0.36, BLUE);
            k.bar(0.3, 0.48, 0.66, 0.6, SKY);
            k.badge(GREEN, Badge::Check);
        }
        "New Report" | "Dashboards" | "Resources" | "Costs" | "In Progress" | "Getting Started" | "Custom" | "Recent" | "Export Data" => {
            k.page(0.14, 0.05, 0.86, 0.95);
            let accent = match name {
                "Costs" => ORANGE,
                "Resources" => GREEN,
                "In Progress" => RED,
                "Recent" => GREY,
                _ => BLUE,
            };
            k.columns(0.26, 0.17, 0.12, 0.82, &[0.22, 0.42, 0.32], &[accent]);
            match name {
                "New Report" => k.badge(GREEN, Badge::Plus),
                "Export Data" => k.badge(GREEN, Badge::Right),
                _ => {}
            }
        }
        "Resource Pool" => {
            k.person(0.3, 0.06, 0.5, BLUE);
            k.person(0.7, 0.06, 0.5, GREEN);
            k.card(0.06, 0.66, 0.94, 0.92, SKY, BLUE);
        }
        "Level Selection" | "Level Resource" | "Level All" | "Leveling Options" | "Clear Leveling" | "Next Overallocation" => {
            k.columns(0.08, 0.22, 0.16, 0.92, &[0.42, 0.72, 0.34, 0.56], &[BLUE, RED, BLUE, BLUE]);
            k.line(&[(0.04, 0.42), (0.96, 0.42)], GREEN, 1.3);
        }
        "Split Task" => {
            k.bar(0.02, 0.36, 0.4, 0.64, BLUE);
            k.bar(0.6, 0.36, 0.98, 0.64, BLUE);
            for x in [0.45f32, 0.5, 0.55] {
                k.dot(x, 0.5, 0.025, INK);
            }
        }
        "Background Color" | "Font Color" => {
            let label = if name == "Font Color" { "A" } else { "ab" };
            text(0.5, 0.42, label, 0.48, INK);
            let bar = if name == "Font Color" { Color32::from_rgb(0xED, 0x1C, 0x24) } else { Color32::from_rgb(0xFF, 0xF2, 0x00) };
            k.bar(0.1, 0.82, 0.9, 0.95, bar);
        }
        "Details" | "Notes..." => {
            k.page(0.14, 0.06, 0.86, 0.94);
            for y in [0.3f32, 0.48, 0.66] {
                k.line(&[(0.28, y), (0.72, y)], GREY, 1.0);
            }
        }
        "Import BOQ" => {
            // a green spreadsheet with an arrow into the plan's bars
            k.band_card(0.04, 0.06, 0.6, 0.76, 0.16, GREEN, INK);
            for y in [0.4f32, 0.56] {
                k.line(&[(0.04, y), (0.6, y)], GREY, 0.9);
            }
            k.line(&[(0.24, 0.22), (0.24, 0.76)], GREY, 0.9);
            k.bar(0.66, 0.5, 0.94, 0.6, BLUE);
            k.bar(0.74, 0.7, 0.96, 0.8, BLUE);
            k.arrow(&[(0.36, 0.9), (0.68, 0.9)], BLUE, 1.3);
        }
        "Payment Schedule" => {
            // a banknote with the baht sign
            k.card(0.04, 0.24, 0.96, 0.76, MINT, GREEN);
            k.ring(0.5, 0.5, 0.17, GREEN, 1.0);
            text(0.5, 0.5, "฿", 0.3, GREEN);
            for x in [0.16f32, 0.84] {
                k.dot(x, 0.5, 0.035, GREEN);
            }
        }
        "Copy Picture" | "Save Picture" => {
            // a picture: frame, sun and hills; a second sheet behind it (copy) or a save badge
            let save = name == "Save Picture";
            let (x0, y0, x1, y1) = if save { (0.04, 0.06, 0.78, 0.68) } else { (0.2, 0.22, 0.96, 0.88) };
            if !save {
                k.page(0.04, 0.06, 0.8, 0.72);
            }
            k.card(x0, y0, x1, y1, Color32::WHITE, INK);
            let fx = |f: f32| x0 + (x1 - x0) * f;
            let fy = |f: f32| y0 + (y1 - y0) * f;
            k.dot(fx(0.72), fy(0.3), 0.06, YELLOW);
            k.poly(&[(fx(0.08), fy(0.9)), (fx(0.38), fy(0.42)), (fx(0.62), fy(0.9))], GREEN);
            k.poly(&[(fx(0.4), fy(0.9)), (fx(0.66), fy(0.58)), (fx(0.92), fy(0.9))], SKY);
            if save {
                k.badge(BLUE, Badge::Down);
            }
        }
        _ => {
            k.page(0.14, 0.06, 0.86, 0.94);
            k.bar(0.28, 0.42, 0.72, 0.56, BLUE);
        }
    }
}
