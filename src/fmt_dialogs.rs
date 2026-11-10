//! Format tab dialogs — Text Styles, Bar Styles, Format Bar, Layout, Gridlines, Timescale,
//! Field Settings, Font — and the Project Options dialog. Layouts follow the dialog templates in
//! the scheduling dialogs (groups, labels and button order).

use crate::i18n::{tr, trs};
use crate::app::{App, TCol, UserSettings};
use crate::format::*;
use crate::model::{ProjectOptions, TaskType};
use crate::theme;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke, Ui, Vec2};

// ====================================================================== shared widgets

fn title(ui: &mut Ui, t: &str) {
    ui.label(RichText::new(tr(t)).strong().size(14.0));
    ui.add_space(6.0);
}

/// Help / [extra] ... OK / Cancel row. Returns (ok, cancel, extra pressed).
fn buttons(ui: &mut Ui, extra: Option<&str>) -> (bool, bool, bool) {
    let (mut ok, mut cancel, mut ex) = (false, false, false);
    ui.add_space(8.0);
    ui.separator();
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add(egui::Button::new(tr("Cancel")).min_size(Vec2::new(70.0, 26.0))).clicked() {
                cancel = true;
            }
            if ui.add(egui::Button::new(RichText::new(tr("OK")).strong()).min_size(Vec2::new(70.0, 26.0))).clicked() {
                ok = true;
            }
            if let Some(e) = extra {
                if ui.add(egui::Button::new(e).min_size(Vec2::new(70.0, 26.0))).clicked() {
                    ex = true;
                }
            }
        });
    });
    (ok, cancel, ex)
}

fn swatch(ui: &mut Ui, c: Rgb) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(14.0, 12.0), egui::Sense::hover());
    ui.painter().rect_filled(r, 0.0, theme::rgb(c));
    ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0_f32, Color32::from_rgb(0x80, 0x80, 0x80)), egui::StrokeKind::Inside);
}

/// MPP-style colour box: Automatic (optional), the 16 named colours, theme colours, More Colors.
fn color_combo(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, value: &mut Option<Rgb>, allow_auto: bool, width: f32) {
    let text = match value {
        None => tr("Automatic").to_string(),
        Some(c) => trs(color_name(*c)),
    };
    ui.horizontal(|ui| {
        if let Some(c) = *value {
            swatch(ui, c);
        }
        egui::ComboBox::from_id_salt(id).width(width).selected_text(text).show_ui(ui, |ui| {
            if allow_auto && ui.selectable_label(value.is_none(), "Automatic").clicked() {
                *value = None;
            }
            for (name, c) in NAMED_COLORS {
                ui.horizontal(|ui| {
                    swatch(ui, c);
                    if ui.selectable_label(*value == Some(c), tr(name)).clicked() {
                        *value = Some(c);
                    }
                });
            }
            ui.separator();
            ui.label(RichText::new(tr("Theme Colors")).small());
            ui.horizontal_wrapped(|ui| {
                for c in THEME_COLORS {
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(16.0, 16.0), egui::Sense::click());
                    ui.painter().rect_filled(r, 0.0, theme::rgb(c));
                    ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0_f32, Color32::from_rgb(0x80, 0x80, 0x80)), egui::StrokeKind::Inside);
                    if resp.clicked() {
                        *value = Some(c);
                    }
                }
            });
            ui.separator();
            let mut rgb = value.unwrap_or(BLACK);
            ui.horizontal(|ui| {
                ui.label(tr("More Colors..."));
                if ui.color_edit_button_srgb(&mut rgb).changed() {
                    *value = Some(rgb);
                }
            });
        });
    });
}

fn rgb_combo(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, value: &mut Rgb, width: f32) {
    let mut v = Some(*value);
    color_combo(ui, id, &mut v, false, width);
    if let Some(c) = v {
        *value = c;
    }
}

fn pick<T: Copy + PartialEq>(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, value: &mut T, items: &[T], label: impl Fn(T) -> String, width: f32) {
    egui::ComboBox::from_id_salt(id).width(width).selected_text(trs(label(*value))).show_ui(ui, |ui| {
        for it in items {
            if ui.selectable_label(*value == *it, trs(label(*it))).clicked() {
                *value = *it;
            }
        }
    });
}

/// Draw a sample bar with the given appearance (Format Bar / Bar Styles "Sample").
fn bar_sample(ui: &mut Ui, bar: &BarAppearance, text: Option<&[BarText; 5]>, height: f32) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), egui::Sense::hover());
    let p = ui.painter_at(r);
    p.rect_filled(r, 0.0, Color32::WHITE);
    p.rect_stroke(r, 0.0, Stroke::new(1.0_f32, Color32::from_rgb(0xC8, 0xC8, 0xC8)), egui::StrokeKind::Inside);
    let cy = r.center().y;
    let (x0, x1) = (r.left() + r.width() * 0.3, r.left() + r.width() * 0.7);
    let bh = 12.0;
    let col = theme::rgb(bar.middle.color);
    let full = Rect::from_min_max(Pos2::new(x0, cy - bh / 2.0), Pos2::new(x1, cy + bh / 2.0));
    let mr = match bar.middle.shape {
        MidShape::None => None,
        MidShape::Full => Some(full),
        MidShape::Top => Some(Rect::from_min_max(full.min, Pos2::new(full.right(), full.top() + 6.0))),
        MidShape::Middle => Some(Rect::from_center_size(full.center(), Vec2::new(full.width(), 5.0))),
        MidShape::Bottom => Some(Rect::from_min_max(Pos2::new(full.left(), full.bottom() - 6.0), full.max)),
        MidShape::LineTop => Some(Rect::from_min_max(full.min, Pos2::new(full.right(), full.top() + 2.0))),
        MidShape::LineMiddle => Some(Rect::from_center_size(full.center(), Vec2::new(full.width(), 2.0))),
        MidShape::LineBottom => Some(Rect::from_min_max(Pos2::new(full.left(), full.bottom() - 2.0), full.max)),
    };
    if let Some(m) = mr {
        theme::pattern_fill(&p, m, bar.middle.pattern, col);
        p.rect_stroke(m, 0.0, Stroke::new(1.0_f32, col), egui::StrokeKind::Inside);
    }
    theme::end_marker(&p, Pos2::new(x0, cy), bh, bar.start);
    theme::end_marker(&p, Pos2::new(x1, cy), bh, bar.end);
    if let Some(t) = text {
        let f = egui::FontId::proportional(10.5);
        let lbl = |k: usize| if t[k] == BarText::None { String::new() } else { t[k].label().to_string() };
        p.text(Pos2::new(x0 - 10.0, cy), egui::Align2::RIGHT_CENTER, lbl(0), f.clone(), theme::TEXT);
        p.text(Pos2::new(x1 + 10.0, cy), egui::Align2::LEFT_CENTER, lbl(1), f.clone(), theme::TEXT);
        p.text(Pos2::new((x0 + x1) / 2.0, cy - 9.0), egui::Align2::CENTER_BOTTOM, lbl(2), f.clone(), theme::TEXT);
        p.text(Pos2::new((x0 + x1) / 2.0, cy + 9.0), egui::Align2::CENTER_TOP, lbl(3), f.clone(), theme::TEXT);
        p.text(Pos2::new((x0 + x1) / 2.0, cy), egui::Align2::CENTER_CENTER, lbl(4), f, Color32::WHITE);
    }
}

fn end_style_ui(ui: &mut Ui, id: &str, st: &mut EndStyle) {
    egui::Grid::new(id).num_columns(2).spacing([6.0, 6.0]).show(ui, |ui| {
        ui.label(tr("Shape:"));
        pick(ui, (id, "shape"), &mut st.shape, &EndShape::ALL, |s| s.label().to_string(), 110.0);
        ui.end_row();
        ui.label(tr("Type:"));
        pick(ui, (id, "type"), &mut st.kind, &EndType::ALL, |s| s.label().to_string(), 110.0);
        ui.end_row();
        ui.label(tr("Color:"));
        rgb_combo(ui, (id, "color"), &mut st.color, 90.0);
        ui.end_row();
    });
}

fn mid_style_ui(ui: &mut Ui, id: &str, st: &mut MidStyle) {
    egui::Grid::new(id).num_columns(2).spacing([6.0, 6.0]).show(ui, |ui| {
        ui.label(tr("Shape:"));
        pick(ui, (id, "shape"), &mut st.shape, &MidShape::ALL, |s| s.label().to_string(), 110.0);
        ui.end_row();
        ui.label(tr("Pattern:"));
        pick(ui, (id, "pattern"), &mut st.pattern, &Pattern::ALL, |s| s.label().to_string(), 110.0);
        ui.end_row();
        ui.label(tr("Color:"));
        rgb_combo(ui, (id, "color"), &mut st.color, 90.0);
        ui.end_row();
    });
}

fn group(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui)) {
    egui::Frame::group(ui.style()).inner_margin(8.0).show(ui, |ui| {
        ui.label(RichText::new(tr(label)).strong());
        add(ui);
    });
}

fn esc(ctx: &egui::Context) -> bool {
    ctx.input(|i| i.key_pressed(egui::Key::Escape))
}

fn bar_text_ui(ui: &mut Ui, id: &str, text: &mut [BarText; 5]) {
    egui::Grid::new(id).num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
        for (k, lbl) in ["Left", "Right", "Top", "Bottom", "Inside"].iter().enumerate() {
            ui.label(tr(lbl));
            pick(ui, (id, k), &mut text[k], &BarText::ALL, |t| if t == BarText::None { String::new() } else { t.label().to_string() }, 200.0);
            ui.end_row();
        }
    });
}

// ====================================================================== Format Bar

#[derive(Clone, Debug)]
pub struct FormatBarState {
    pub idx: usize,
    pub tab: u8,
    pub bar: BarAppearance,
    pub text: [BarText; 5],
}

pub fn open_format_bar(app: &mut App) {
    app.commit_edit();
    let Some(i) = app.current_task() else { return };
    let t = &app.doc.tasks[i];
    // the style that currently draws this task's bar is the starting point
    let base = app
        .doc
        .formatting
        .bar_styles
        .iter()
        .find(|s| {
            let ms = t.is_milestone();
            if t.is_summary {
                s.name.contains("Summary") && !s.name.contains("Project")
            } else if ms {
                s.name == "Milestone"
            } else {
                s.name == "Task" || s.name == "Manual Task"
            }
        })
        .cloned()
        .unwrap_or_else(|| default_bar_styles()[0].clone());
    app.dialog = Some(crate::dialogs::Dialog::FormatBar(FormatBarState { idx: i, tab: 0, bar: t.bar_format.unwrap_or(base.bar), text: t.bar_text.unwrap_or(base.text) }));
}

pub fn show_format_bar(app: &mut App, ctx: &egui::Context, mut st: FormatBarState) -> Option<FormatBarState> {
    let mut result: Option<bool> = None;
    let mut reset = false;
    crate::theme::modal(egui::Id::unique("format_bar")).show(ctx, |ui| {
        crate::theme::dialog_spacing(ui);
        ui.set_width(560.0);
        title(ui, "Format Bar");
        ui.horizontal(|ui| {
            for (k, t) in ["Bar Shape", "Bar Text"].iter().enumerate() {
                if ui.selectable_label(st.tab == k as u8, tr(t)).clicked() {
                    st.tab = k as u8;
                }
            }
        });
        ui.separator();
        if st.tab == 0 {
            ui.horizontal_top(|ui| {
                group(ui, "Start", |ui| end_style_ui(ui, "fb_start", &mut st.bar.start));
                group(ui, "Middle", |ui| mid_style_ui(ui, "fb_mid", &mut st.bar.middle));
                group(ui, "End", |ui| end_style_ui(ui, "fb_end", &mut st.bar.end));
            });
        } else {
            bar_text_ui(ui, "fb_text", &mut st.text);
        }
        ui.add_space(8.0);
        ui.label(tr("Sample:"));
        bar_sample(ui, &st.bar, Some(&st.text), 64.0);
        let (ok, cancel, ex) = buttons(ui, Some("Reset"));
        if ok {
            result = Some(true);
        } else if cancel {
            result = Some(false);
        }
        reset = ex;
    });
    if esc(ctx) {
        result = Some(false);
    }
    if reset {
        if st.idx < app.doc.tasks.len() {
            app.checkpoint();
            app.doc.tasks[st.idx].bar_format = None;
            app.doc.tasks[st.idx].bar_text = None;
        }
        return None;
    }
    match result {
        Some(true) => {
            if st.idx < app.doc.tasks.len() {
                app.checkpoint();
                app.doc.tasks[st.idx].bar_format = Some(st.bar);
                app.doc.tasks[st.idx].bar_text = Some(st.text);
            }
            None
        }
        Some(false) => None,
        None => Some(st),
    }
}

// ====================================================================== Bar Styles

#[derive(Clone, Debug)]
pub struct BarStylesState {
    pub styles: Vec<BarStyle>,
    pub sel: usize,
    pub tab: u8,
    pub clipboard: Option<BarStyle>,
}

pub fn open_bar_styles(app: &mut App) {
    app.commit_edit();
    let styles = if app.view == crate::app::View::TrackingGantt { app.doc.formatting.tracking_bar_styles.clone() } else { app.doc.formatting.bar_styles.clone() };
    app.dialog = Some(crate::dialogs::Dialog::BarStyles(BarStylesState { styles, sel: 0, tab: 1, clipboard: None }));
}

fn parse_show_for(s: &str) -> Vec<ShowFor> {
    let mut v = Vec::new();
    for part in s.split(',') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        let (not, name) = match p.strip_prefix("Not ").or_else(|| p.strip_prefix("not ")) {
            Some(n) => (true, n.trim()),
            None => (false, p),
        };
        if let Some(cat) = Category::ALL.iter().copied().find(|c| c.label().eq_ignore_ascii_case(name)) {
            v.push(ShowFor { cat, not });
        } else if let Some(n) = name.strip_prefix("Flag").and_then(|n| n.parse::<u8>().ok()) {
            v.push(ShowFor { cat: Category::Flag(n), not });
        }
    }
    v
}

pub fn show_bar_styles(app: &mut App, ctx: &egui::Context, mut st: BarStylesState) -> Option<BarStylesState> {
    let mut result: Option<bool> = None;
    crate::theme::modal(egui::Id::unique("bar_styles")).show(ctx, |ui| {
        crate::theme::dialog_spacing(ui);
        ui.set_width(700.0);
        title(ui, "Bar Styles");
        ui.horizontal(|ui| {
            if ui.button(tr("Cut Row")).clicked() && st.sel < st.styles.len() {
                st.clipboard = Some(st.styles.remove(st.sel));
                st.sel = st.sel.min(st.styles.len().saturating_sub(1));
            }
            if ui.add_enabled(st.clipboard.is_some(), egui::Button::new(tr("Paste Row"))).clicked() {
                if let Some(c) = st.clipboard.clone() {
                    let at = st.sel.min(st.styles.len());
                    st.styles.insert(at, c);
                }
            }
            if ui.button(tr("Insert Row")).clicked() {
                let at = st.sel.min(st.styles.len());
                let mut n = default_bar_styles()[0].clone();
                n.name = String::new();
                n.show_for = vec![];
                st.styles.insert(at, n);
                st.sel = at;
            }
        });
        ui.add_space(4.0);
        // table
        egui::Frame::group(ui.style()).inner_margin(2.0).show(ui, |ui| {
            egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                ui.set_min_width(660.0);
                egui::Grid::new("bs_table").num_columns(6).striped(true).spacing([6.0, 2.0]).min_col_width(40.0).show(ui, |ui| {
                    for h in ["Name", "Appearance", "Show For ... Tasks", "Row", "From", "To"] {
                        ui.label(RichText::new(tr(h)).strong());
                    }
                    ui.end_row();
                    let n = st.styles.len();
                    for k in 0..n {
                        let selected = k == st.sel;
                        let s = &mut st.styles[k];
                        let name_resp = ui.add_sized([120.0, 20.0], egui::TextEdit::singleline(&mut s.name));
                        if name_resp.gained_focus() || name_resp.clicked() {
                            st.sel = k;
                        }
                        let s = &mut st.styles[k];
                        // appearance preview
                        let (r, resp) = ui.allocate_exact_size(Vec2::new(90.0, 18.0), egui::Sense::click());
                        {
                            let p = ui.painter_at(r);
                            if selected {
                                p.rect_filled(r, 0.0, theme::ROW_SEL_CLASSIC);
                            }
                            let cy = r.center().y;
                            let col = theme::rgb(s.bar.middle.color);
                            if s.bar.middle.shape != MidShape::None {
                                let h = if matches!(s.bar.middle.shape, MidShape::LineTop | MidShape::LineMiddle | MidShape::LineBottom) { 2.0 } else { 8.0 };
                                theme::pattern_fill(&p, Rect::from_center_size(Pos2::new(r.center().x, cy), Vec2::new(60.0, h)), s.bar.middle.pattern, col);
                                p.rect_stroke(Rect::from_center_size(Pos2::new(r.center().x, cy), Vec2::new(60.0, h)), 0.0, Stroke::new(1.0_f32, col), egui::StrokeKind::Inside);
                            }
                            theme::end_marker(&p, Pos2::new(r.center().x - 30.0, cy), 10.0, s.bar.start);
                            if s.from != s.to {
                                theme::end_marker(&p, Pos2::new(r.center().x + 30.0, cy), 10.0, s.bar.end);
                            }
                        }
                        if resp.clicked() {
                            st.sel = k;
                        }
                        let s = &mut st.styles[k];
                        let mut sf = show_for_text(&s.show_for);
                        let sresp = ui.add_sized([190.0, 20.0], egui::TextEdit::singleline(&mut sf));
                        if sresp.changed() {
                            s.show_for = parse_show_for(&sf);
                        }
                        if sresp.gained_focus() {
                            st.sel = k;
                        }
                        let s = &mut st.styles[k];
                        pick(ui, ("bs_row", k), &mut s.row, &[1u8, 2, 3, 4], |r| r.to_string(), 36.0);
                        pick(ui, ("bs_from", k), &mut s.from, &BarField::ALL, |f| f.label().to_string(), 100.0);
                        pick(ui, ("bs_to", k), &mut s.to, &BarField::ALL, |f| f.label().to_string(), 100.0);
                        ui.end_row();
                    }
                });
            });
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            for (k, t) in ["Text", "Bars"].iter().enumerate() {
                if ui.selectable_label(st.tab == k as u8, tr(t)).clicked() {
                    st.tab = k as u8;
                }
            }
        });
        ui.separator();
        if st.sel < st.styles.len() {
            let s = &mut st.styles[st.sel];
            if st.tab == 1 {
                ui.horizontal_top(|ui| {
                    group(ui, "Start", |ui| end_style_ui(ui, "bs_start", &mut s.bar.start));
                    group(ui, "Middle", |ui| mid_style_ui(ui, "bs_mid", &mut s.bar.middle));
                    group(ui, "End", |ui| end_style_ui(ui, "bs_end", &mut s.bar.end));
                });
            } else {
                bar_text_ui(ui, "bs_text", &mut s.text);
            }
            ui.add_space(6.0);
            bar_sample(ui, &s.bar, Some(&s.text), 44.0);
        }
        let (ok, cancel, _) = buttons(ui, None);
        if ok {
            result = Some(true);
        } else if cancel {
            result = Some(false);
        }
    });
    if esc(ctx) {
        result = Some(false);
    }
    match result {
        Some(true) => {
            app.checkpoint();
            if app.view == crate::app::View::TrackingGantt {
                app.doc.formatting.tracking_bar_styles = st.styles;
            } else {
                app.doc.formatting.bar_styles = st.styles;
            }
            None
        }
        Some(false) => None,
        None => Some(st),
    }
}

// ====================================================================== Layout

#[derive(Clone, Debug)]
pub struct LayoutState {
    pub layout: Layout,
}

pub fn open_layout(app: &mut App) {
    app.dialog = Some(crate::dialogs::Dialog::Layout(LayoutState { layout: app.doc.formatting.layout.clone() }));
}

fn link_picture(ui: &mut Ui, style: LinkStyle, selected: bool) -> bool {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(96.0, 54.0), egui::Sense::click());
    let p = ui.painter_at(r);
    p.rect_filled(r, 2.0, Color32::WHITE);
    p.rect_stroke(r, 2.0, Stroke::new(if selected { 2.0 } else { 1.0 }, if selected { theme::ACCENT } else { Color32::from_rgb(0xB0, 0xB0, 0xB0) }), egui::StrokeKind::Inside);
    let blue = theme::rgb(TASK_BLUE);
    let a = Rect::from_min_size(r.min + Vec2::new(10.0, 12.0), Vec2::new(34.0, 8.0));
    let b = Rect::from_min_size(r.min + Vec2::new(50.0, 34.0), Vec2::new(36.0, 8.0));
    p.rect_filled(a, 0.0, blue);
    p.rect_filled(b, 0.0, blue);
    let s = Stroke::new(1.0_f32, theme::LINK);
    match style {
        LinkStyle::None => {}
        LinkStyle::Rectilinear => {
            p.add(egui::Shape::line(vec![a.right_center(), Pos2::new(b.left() + 4.0, a.center().y), Pos2::new(b.left() + 4.0, b.top())], s));
            p.add(egui::Shape::convex_polygon(vec![Pos2::new(b.left() + 4.0, b.top()), Pos2::new(b.left() + 1.0, b.top() - 5.0), Pos2::new(b.left() + 7.0, b.top() - 5.0)], theme::LINK, Stroke::NONE));
        }
        LinkStyle::Straight => {
            p.line_segment([a.right_center(), b.left_center()], s);
            p.add(egui::Shape::convex_polygon(vec![b.left_center(), b.left_center() + Vec2::new(-6.0, -4.0), b.left_center() + Vec2::new(-6.0, 3.0)], theme::LINK, Stroke::NONE));
        }
    }
    resp.clicked()
}

pub fn show_layout(app: &mut App, ctx: &egui::Context, mut st: LayoutState) -> Option<LayoutState> {
    let mut result: Option<bool> = None;
    let loc = app.loc.clone();
    let names = crate::locale::format_names(&loc);
    crate::theme::modal(egui::Id::unique("layout")).show(ctx, |ui| {
        crate::theme::dialog_spacing(ui);
        ui.set_width(380.0);
        title(ui, "Layout");
        group(ui, "Links", |ui| {
            ui.horizontal(|ui| {
                for ls in [LinkStyle::None, LinkStyle::Rectilinear, LinkStyle::Straight] {
                    if link_picture(ui, ls, st.layout.links == ls) {
                        st.layout.links = ls;
                    }
                }
            });
        });
        group(ui, "Bars", |ui| {
            egui::Grid::new("lay_bars").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label(tr("Date format:"));
                let cur = if st.layout.date_format == 255 { tr("Default").to_string() } else { names.get(st.layout.date_format as usize).cloned().unwrap_or_default() };
                egui::ComboBox::from_id_salt("lay_date").width(190.0).selected_text(cur).show_ui(ui, |ui| {
                    if ui.selectable_label(st.layout.date_format == 255, tr("Default")).clicked() {
                        st.layout.date_format = 255;
                    }
                    for (k, n) in names.iter().enumerate() {
                        if ui.selectable_label(st.layout.date_format == k as u8, n).clicked() {
                            st.layout.date_format = k as u8;
                        }
                    }
                });
                ui.end_row();
                ui.label(tr("Bar height:"));
                pick(ui, "lay_h", &mut st.layout.bar_height, &BAR_HEIGHTS, |h| h.to_string(), 60.0);
                ui.end_row();
            });
            ui.checkbox(&mut st.layout.always_roll_up, tr("Always roll up Gantt bars"));
            ui.add_enabled_ui(st.layout.always_roll_up, |ui| {
                ui.indent("lay_hide", |ui| ui.checkbox(&mut st.layout.hide_rollup_when_expanded, tr("Hide rollup bars when summary expanded")));
            });
            ui.checkbox(&mut st.layout.round_to_whole_days, tr("Round bars to whole days"));
            ui.checkbox(&mut st.layout.show_bar_splits, tr("Show bar splits"));
        });
        group(ui, "Other", |ui| {
            ui.checkbox(&mut st.layout.show_drawings, tr("Show drawings"));
        });
        let (ok, cancel, _) = buttons(ui, None);
        if ok {
            result = Some(true);
        } else if cancel {
            result = Some(false);
        }
    });
    if esc(ctx) {
        result = Some(false);
    }
    match result {
        Some(true) => {
            app.checkpoint();
            app.doc.formatting.layout = st.layout;
            None
        }
        Some(false) => None,
        None => Some(st),
    }
}

// ====================================================================== Gridlines

#[derive(Clone, Debug)]
pub struct GridlinesState {
    pub kind: GridKind,
    pub lines: Vec<Gridline>,
    pub other: String,
}

pub fn open_gridlines(app: &mut App) {
    let lines = GridKind::ALL.iter().map(|k| app.doc.formatting.gridline(*k)).collect();
    app.dialog = Some(crate::dialogs::Dialog::Gridlines(GridlinesState { kind: GridKind::GanttRows, lines, other: "5".into() }));
}

fn line_type_combo(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, value: &mut LineType, width: f32) {
    egui::ComboBox::from_id_salt(id).width(width).selected_text(tr(value.label())).show_ui(ui, |ui| {
        for lt in LineType::ALL {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::new(40.0, 10.0), egui::Sense::hover());
                theme::styled_line(ui.painter(), r.left_center(), r.right_center(), lt, theme::TEXT);
                if ui.selectable_label(*value == lt, tr(if lt == LineType::None { "(none)" } else { lt.label() })).clicked() {
                    *value = lt;
                }
            });
        }
    });
}

pub fn show_gridlines(app: &mut App, ctx: &egui::Context, mut st: GridlinesState) -> Option<GridlinesState> {
    let mut result: Option<bool> = None;
    crate::theme::modal(egui::Id::unique("gridlines")).show(ctx, |ui| {
        crate::theme::dialog_spacing(ui);
        ui.set_width(560.0);
        title(ui, "Gridlines");
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.label(tr("Line to change:"));
                egui::Frame::group(ui.style()).inner_margin(2.0).show(ui, |ui| {
                    ui.set_width(170.0);
                    egui::ScrollArea::vertical().max_height(190.0).show(ui, |ui| {
                        for k in GridKind::ALL {
                            if ui.selectable_label(st.kind == k, tr(k.label())).clicked() {
                                st.kind = k;
                            }
                        }
                    });
                });
            });
            ui.add_space(12.0);
            let kind = st.kind;
            let has_interval = kind.has_interval();
            if let Some(g) = st.lines.iter_mut().find(|g| g.kind == kind) {
                ui.vertical(|ui| {
                    group(ui, "Normal", |ui| {
                        ui.horizontal(|ui| {
                            ui.label(tr("Type:"));
                            line_type_combo(ui, "gl_type", &mut g.normal, 110.0);
                            ui.add_space(10.0);
                            ui.label(tr("Color:"));
                            rgb_combo(ui, "gl_color", &mut g.normal_color, 90.0);
                        });
                    });
                    ui.add_enabled_ui(has_interval, |ui| {
                        group(ui, "At interval", |ui| {
                            ui.horizontal(|ui| {
                                ui.radio_value(&mut g.interval, 0, tr("None"));
                                ui.radio_value(&mut g.interval, 2, tr("2"));
                                ui.radio_value(&mut g.interval, 3, tr("3"));
                                ui.radio_value(&mut g.interval, 4, tr("4"));
                                let other = g.interval > 4 || g.interval == 1;
                                if ui.radio(other, tr("Other:")).clicked() {
                                    g.interval = st.other.parse().unwrap_or(5);
                                }
                                let r = ui.add(egui::TextEdit::singleline(&mut st.other).desired_width(36.0));
                                if r.changed() && other {
                                    g.interval = st.other.parse().unwrap_or(5);
                                }
                            });
                            ui.add_enabled_ui(g.interval > 0, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(tr("Type:"));
                                    line_type_combo(ui, "gl_itype", &mut g.interval_type, 110.0);
                                    ui.add_space(10.0);
                                    ui.label(tr("Color:"));
                                    rgb_combo(ui, "gl_icolor", &mut g.interval_color, 90.0);
                                });
                            });
                        });
                    });
                });
            }
        });
        let (ok, cancel, _) = buttons(ui, None);
        if ok {
            result = Some(true);
        } else if cancel {
            result = Some(false);
        }
    });
    if esc(ctx) {
        result = Some(false);
    }
    match result {
        Some(true) => {
            app.checkpoint();
            app.doc.formatting.gridlines = st.lines;
            None
        }
        Some(false) => None,
        None => Some(st),
    }
}

// ====================================================================== Text Styles

#[derive(Clone, Debug)]
pub struct TextStylesState {
    pub item: TextItem,
    /// full working set, one entry per item
    pub styles: Vec<(TextItem, TextStyle)>,
}

pub fn open_text_styles(app: &mut App) {
    let styles = TextItem::ALL.iter().map(|i| (*i, app.doc.formatting.text_style(*i))).collect();
    app.dialog = Some(crate::dialogs::Dialog::TextStyles(TextStylesState { item: TextItem::All, styles }));
}

/// Font / style / size / effects / colours block shared by Text Styles and Font.
fn font_block(ui: &mut Ui, id: &str, st: &mut TextStyle, show_bg: bool) {
    let fonts = theme::available_fonts();
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.label(tr("Font:"));
            ui.add(egui::TextEdit::singleline(&mut st.font).desired_width(150.0));
            egui::Frame::group(ui.style()).inner_margin(2.0).show(ui, |ui| {
                ui.set_width(150.0);
                // typing part of a name narrows the list (every font on the PC is in it)
                let exact = fonts.iter().any(|f| f.eq_ignore_ascii_case(st.font.trim()));
                let shown = if exact { fonts.iter().collect() } else { theme::find_fonts(&st.font) };
                egui::ScrollArea::vertical().id_salt((id, "fonts")).max_height(110.0).auto_shrink([false, true]).show_rows(ui, ui.spacing().interact_size.y, shown.len(), |ui, rows| {
                    ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                        for f in &shown[rows] {
                            if ui.selectable_label(st.font.eq_ignore_ascii_case(f), f.as_str()).clicked() {
                                st.font = (*f).clone();
                            }
                        }
                    });
                });
            });
        });
        ui.add_space(8.0);
        ui.vertical(|ui| {
            ui.label(tr("Font style:"));
            let cur = match (st.bold, st.italic) {
                (false, false) => "Regular",
                (false, true) => "Italic",
                (true, false) => "Bold",
                (true, true) => "Bold Italic",
            };
            ui.add(egui::TextEdit::singleline(&mut cur.to_string()).desired_width(100.0).interactive(false));
            egui::Frame::group(ui.style()).inner_margin(2.0).show(ui, |ui| {
                ui.set_width(100.0);
                for (lbl, b, i) in [("Regular", false, false), ("Italic", false, true), ("Bold", true, false), ("Bold Italic", true, true)] {
                    if ui.selectable_label(st.bold == b && st.italic == i, lbl).clicked() {
                        st.bold = b;
                        st.italic = i;
                    }
                }
            });
        });
        ui.add_space(8.0);
        ui.vertical(|ui| {
            ui.label(tr("Size:"));
            let mut sz = format!("{}", st.size);
            if ui.add(egui::TextEdit::singleline(&mut sz).desired_width(50.0)).changed() {
                if let Ok(v) = sz.trim().parse::<f32>() {
                    st.size = v.clamp(6.0, 72.0);
                }
            }
            egui::Frame::group(ui.style()).inner_margin(2.0).show(ui, |ui| {
                ui.set_width(50.0);
                egui::ScrollArea::vertical().id_salt((id, "sizes")).max_height(110.0).show(ui, |ui| {
                    for s in [8.0f32, 9.0, 10.0, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0, 24.0, 28.0, 36.0] {
                        if ui.selectable_label((st.size - s).abs() < 0.01, format!("{s}")).clicked() {
                            st.size = s;
                        }
                    }
                });
            });
        });
    });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.checkbox(&mut st.underline, tr("Underline"));
        ui.checkbox(&mut st.strike, tr("Strikethrough"));
    });
    egui::Grid::new((id, "colors")).num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label(tr("Color:"));
        color_combo(ui, (id, "fc"), &mut st.color, true, 120.0);
        ui.end_row();
        if show_bg {
            ui.label(tr("Background Color:"));
            color_combo(ui, (id, "bc"), &mut st.background, true, 120.0);
            ui.end_row();
        }
    });
    ui.add_space(6.0);
    ui.label(tr("Sample:"));
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 46.0), egui::Sense::hover());
    let p = ui.painter_at(r);
    p.rect_filled(r, 0.0, st.background.map(theme::rgb).unwrap_or(Color32::WHITE));
    p.rect_stroke(r, 0.0, Stroke::new(1.0_f32, Color32::from_rgb(0xC8, 0xC8, 0xC8)), egui::StrokeKind::Inside);
    let mut job = egui::text::LayoutJob::default();
    let color = st.color.map(theme::rgb).unwrap_or(theme::TEXT);
    job.append(
        "AaBbYyZz กขค",
        0.0,
        egui::TextFormat {
            font_id: theme::style_font(st),
            color,
            italics: st.italic,
            underline: if st.underline { Stroke::new(1.0_f32, color) } else { Stroke::NONE },
            strikethrough: if st.strike { Stroke::new(1.0_f32, color) } else { Stroke::NONE },
            ..Default::default()
        },
    );
    let g = ui.painter().layout_job(job);
    let pos = r.center() - g.size() / 2.0;
    p.galley(pos, g, color);
}

pub fn show_text_styles(app: &mut App, ctx: &egui::Context, mut st: TextStylesState) -> Option<TextStylesState> {
    let mut result: Option<bool> = None;
    crate::theme::modal(egui::Id::unique("text_styles")).show(ctx, |ui| {
        crate::theme::dialog_spacing(ui);
        ui.set_width(440.0);
        title(ui, "Text Styles");
        ui.horizontal(|ui| {
            ui.label(tr("Item to Change:"));
            pick(ui, "ts_item", &mut st.item, &TextItem::ALL, |i| i.label().to_string(), 220.0);
        });
        ui.add_space(6.0);
        let item = st.item;
        let before = st.styles.iter().find(|(i, _)| *i == item).map(|(_, s)| s.clone()).unwrap_or_default();
        if let Some((_, s)) = st.styles.iter_mut().find(|(i, _)| *i == item) {
            font_block(ui, "ts", s, true);
        }
        // changing "All" changes every item (keeping their own bold / colour emphasis)
        if item == TextItem::All {
            let after = st.styles.iter().find(|(i, _)| *i == item).map(|(_, s)| s.clone()).unwrap_or_default();
            if after != before {
                for (i, s) in st.styles.iter_mut() {
                    if *i != TextItem::All {
                        if after.font != before.font {
                            s.font = after.font.clone();
                        }
                        if (after.size - before.size).abs() > 0.01 && (s.size - before.size).abs() < 0.01 {
                            s.size = after.size;
                        }
                        if after.color != before.color && s.color == before.color {
                            s.color = after.color;
                        }
                        if after.background != before.background && s.background == before.background {
                            s.background = after.background;
                        }
                        if after.bold != before.bold && s.bold == before.bold {
                            s.bold = after.bold;
                        }
                        if after.italic != before.italic && s.italic == before.italic {
                            s.italic = after.italic;
                        }
                        if after.underline != before.underline {
                            s.underline = after.underline;
                        }
                    }
                }
            }
        }
        let (ok, cancel, _) = buttons(ui, None);
        if ok {
            result = Some(true);
        } else if cancel {
            result = Some(false);
        }
    });
    if esc(ctx) {
        result = Some(false);
    }
    match result {
        Some(true) => {
            app.checkpoint();
            for (i, s) in st.styles {
                theme::ensure_font(ctx, &s.font);
                app.doc.formatting.set_text_style(i, s);
            }
            None
        }
        Some(false) => None,
        None => Some(st),
    }
}

// ====================================================================== Font (cells)

#[derive(Clone, Debug)]
pub struct FontState {
    pub style: TextStyle,
}

pub fn open_font(app: &mut App) {
    app.commit_edit();
    if app.current_task().is_none() {
        return;
    }
    app.dialog = Some(crate::dialogs::Dialog::Font(FontState { style: app.selected_text_style() }));
}

pub fn show_font(app: &mut App, ctx: &egui::Context, mut st: FontState) -> Option<FontState> {
    let mut result: Option<bool> = None;
    crate::theme::modal(egui::Id::unique("font_dialog")).show(ctx, |ui| {
        crate::theme::dialog_spacing(ui);
        ui.set_width(440.0);
        title(ui, "Font");
        font_block(ui, "font", &mut st.style, true);
        let (ok, cancel, _) = buttons(ui, None);
        if ok {
            result = Some(true);
        } else if cancel {
            result = Some(false);
        }
    });
    if esc(ctx) {
        result = Some(false);
    }
    match result {
        Some(true) => {
            theme::ensure_font(ctx, &st.style.font);
            let s = st.style.clone();
            let base = app.doc.formatting.text_style(TextItem::All);
            app.format_selected_cells(move |c| {
                c.bold = s.bold;
                c.italic = s.italic;
                c.underline = s.underline;
                c.strike = s.strike;
                c.color = s.color;
                c.background = s.background;
                c.size = if (s.size - base.size).abs() < 0.01 { None } else { Some(s.size) };
                c.font = if s.font == base.font || s.font.is_empty() { None } else { Some(s.font.clone()) };
            });
            for i in app.selected_tasks() {
                let t = &mut app.doc.tasks[i];
                t.bold = false;
                t.italic = false;
                t.underline = false;
            }
            None
        }
        Some(false) => None,
        None => Some(st),
    }
}

// ====================================================================== Timescale

#[derive(Clone, Debug)]
pub struct TimescaleState {
    pub tab: u8,
    pub ts: Timescale,
}

pub fn open_timescale(app: &mut App, tab: u8) {
    let mut ts = app.doc.formatting.timescale.clone();
    if ts.auto {
        // start from what is on screen now
        let (middle, bottom) = crate::gantt::auto_tiers(app.px_per_day);
        ts.middle.units = middle;
        ts.bottom.units = bottom;
        ts.middle.label = default_label(middle);
        ts.bottom.label = default_label(bottom);
        ts.show = 2;
    }
    app.dialog = Some(crate::dialogs::Dialog::Timescale(TimescaleState { tab: tab.min(3), ts }));
}

fn default_label(u: TsUnit) -> u8 {
    match u {
        TsUnit::Years => 0,
        TsUnit::HalfYears => 2,
        TsUnit::Quarters => 1,
        TsUnit::Months => 0,
        TsUnit::ThirdsOfMonths => 0,
        TsUnit::Weeks => 1,
        TsUnit::Days => 6,
        TsUnit::Hours => 2,
        TsUnit::Minutes => 1,
    }
}

fn tier_ui(ui: &mut Ui, id: &str, t: &mut Tier, loc: &crate::locale::Locale, start: chrono::NaiveDate) {
    egui::Grid::new((id, "grid")).num_columns(4).spacing([8.0, 6.0]).show(ui, |ui| {
        ui.label(tr("Units:"));
        let before = t.units;
        pick(ui, (id, "units"), &mut t.units, &TsUnit::ALL, |u| u.label().to_string(), 130.0);
        if t.units != before {
            t.label = default_label(t.units);
        }
        ui.label(tr("Label:"));
        let fmts = t.units.label_formats();
        let sample = chrono::NaiveDate::from_ymd_opt(2009, 1, 26).unwrap();
        let name = |k: u8| crate::gantt::tier_label(t.units, k, sample, loc, start, start);
        egui::ComboBox::from_id_salt((id, "label")).width(200.0).selected_text(name(t.label)).show_ui(ui, |ui| {
            for k in 0..fmts.len() as u8 {
                if ui.selectable_label(t.label == k, name(k)).clicked() {
                    t.label = k;
                }
            }
        });
        ui.end_row();
        ui.label(tr("Count:"));
        let mut c = t.count as i32;
        ui.add(egui::DragValue::new(&mut c).range(1..=99));
        t.count = c as u8;
        ui.label(tr("Align:"));
        pick(ui, (id, "align"), &mut t.align, &[0u8, 1, 2], |a| ["Left", "Center", "Right"][a as usize].to_string(), 100.0);
        ui.end_row();
        ui.label(tr(""));
        ui.checkbox(&mut t.tick_lines, tr("Tick lines"));
        ui.label(tr(""));
        ui.checkbox(&mut t.fiscal_year, tr("Use fiscal year"));
        ui.end_row();
    });
}

pub fn show_timescale(app: &mut App, ctx: &egui::Context, mut st: TimescaleState) -> Option<TimescaleState> {
    let mut result: Option<bool> = None;
    let loc = app.loc.clone();
    let start = app.doc.start;
    crate::theme::modal(egui::Id::unique("timescale")).show(ctx, |ui| {
        crate::theme::dialog_spacing(ui);
        ui.set_width(560.0);
        title(ui, "Timescale");
        ui.horizontal(|ui| {
            for (k, t) in ["Top Tier", "Middle Tier", "Bottom Tier", "Non-working time"].iter().enumerate() {
                let enabled = match k {
                    0 => st.ts.show == 3,
                    2 => st.ts.show >= 2,
                    _ => true,
                };
                if ui.add_enabled(enabled, egui::Button::selectable(st.tab == k as u8, tr(t))).clicked() {
                    st.tab = k as u8;
                }
            }
        });
        ui.separator();
        if st.tab < 3 {
            let name = tr(["Top tier formatting", "Middle tier formatting", "Bottom tier formatting"][st.tab as usize]);
            group(ui, name, |ui| {
                let tier = match st.tab {
                    0 => &mut st.ts.top,
                    1 => &mut st.ts.middle,
                    _ => &mut st.ts.bottom,
                };
                tier_ui(ui, "tier", tier, &loc, start);
            });
            group(ui, "Timescale options", |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr("Show:"));
                    pick(ui, "ts_show", &mut st.ts.show, &[1u8, 2, 3], |s| ["", "One tier (Middle)", "Two tiers (Middle, Bottom)", "Three tiers (Top, Middle, Bottom)"][s as usize].to_string(), 230.0);
                    ui.add_space(10.0);
                    ui.label(tr("Size:"));
                    let mut sz = st.ts.size as i32;
                    ui.add(egui::DragValue::new(&mut sz).range(25..=1000).suffix(" %"));
                    st.ts.size = sz.clamp(25, 255) as u8;
                    ui.checkbox(&mut st.ts.separator, tr("Scale separator"));
                });
            });
            group(ui, "Preview", |ui| {
                timescale_preview(ui, &st.ts, &loc, start);
            });
        } else {
            group(ui, "Draw", |ui| {
                ui.radio_value(&mut st.ts.nonworking.draw, 0, tr("Behind task bars"));
                ui.radio_value(&mut st.ts.nonworking.draw, 1, tr("In front of task bars"));
                ui.radio_value(&mut st.ts.nonworking.draw, 2, tr("Do not draw"));
            });
            group(ui, "Formatting options", |ui| {
                egui::Grid::new("nw").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label(tr("Color:"));
                    rgb_combo(ui, "nw_color", &mut st.ts.nonworking.color, 110.0);
                    ui.end_row();
                    ui.label(tr("Pattern:"));
                    pick(ui, "nw_pat", &mut st.ts.nonworking.pattern, &Pattern::ALL, |p| p.label().to_string(), 120.0);
                    ui.end_row();
                });
            });
        }
        let (ok, cancel, _) = buttons(ui, None);
        if ok {
            result = Some(true);
        } else if cancel {
            result = Some(false);
        }
    });
    if esc(ctx) {
        result = Some(false);
    }
    match result {
        Some(true) => {
            app.checkpoint();
            st.ts.auto = false;
            // the size percentage scales the zoom like MPP
            let old = app.doc.formatting.timescale.size.max(1) as f32;
            let factor = st.ts.size as f32 / old;
            app.doc.formatting.timescale = st.ts;
            if (factor - 1.0).abs() > 0.01 {
                app.zoom_by(factor, None);
            }
            None
        }
        Some(false) => None,
        None => Some(st),
    }
}

fn timescale_preview(ui: &mut Ui, ts: &Timescale, loc: &crate::locale::Locale, start: chrono::NaiveDate) {
    let tiers: Vec<&Tier> = match ts.show {
        3 => vec![&ts.top, &ts.middle, &ts.bottom],
        2 => vec![&ts.middle, &ts.bottom],
        _ => vec![&ts.middle],
    };
    let h = 18.0 * tiers.len() as f32;
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), egui::Sense::hover());
    let p = ui.painter_at(r);
    p.rect_filled(r, 0.0, theme::HEADER_BG);
    let font = egui::FontId::proportional(10.5);
    for (k, t) in tiers.iter().enumerate() {
        let tr = Rect::from_min_max(Pos2::new(r.left(), r.top() + k as f32 * 18.0), Pos2::new(r.right(), r.top() + (k as f32 + 1.0) * 18.0));
        p.line_segment([tr.left_bottom(), tr.right_bottom()], Stroke::new(1.0_f32, theme::GRID_LINE));
        // a few consecutive units from the project start
        let step = match t.units {
            TsUnit::Years => 365,
            TsUnit::HalfYears => 182,
            TsUnit::Quarters => 91,
            TsUnit::Months => 30,
            TsUnit::ThirdsOfMonths => 10,
            TsUnit::Weeks => 7,
            _ => 1,
        } * t.count.max(1) as i64;
        let n = 6;
        let w = tr.width() / n as f32;
        for i in 0..n {
            let d = start + chrono::Duration::days(step * i as i64);
            let x0 = tr.left() + i as f32 * w;
            if t.tick_lines {
                p.line_segment([Pos2::new(x0, tr.top()), Pos2::new(x0, tr.bottom())], Stroke::new(1.0_f32, theme::GRID_LINE));
            }
            let label = crate::gantt::tier_label(t.units, t.label, d, loc, start, start);
            let clip = p.with_clip_rect(Rect::from_min_max(Pos2::new(x0, tr.top()), Pos2::new(x0 + w, tr.bottom())));
            let (pos, al) = match t.align {
                0 => (Pos2::new(x0 + 3.0, tr.center().y), egui::Align2::LEFT_CENTER),
                2 => (Pos2::new(x0 + w - 3.0, tr.center().y), egui::Align2::RIGHT_CENTER),
                _ => (Pos2::new(x0 + w / 2.0, tr.center().y), egui::Align2::CENTER_CENTER),
            };
            clip.text(pos, al, label, font.clone(), theme::TEXT);
        }
    }
}

// ====================================================================== Field Settings

#[derive(Clone, Debug)]
pub struct FieldSettingsState {
    pub col_index: usize,
    pub col: TCol,
    pub title: String,
    pub title_align: u8,
    pub data_align: u8,
    pub width: f32,
    pub header_wrap: bool,
    pub wrap: bool,
}

pub fn open_field_settings(app: &mut App) {
    let Some(&(col, w)) = app.task_cols.get(app.sel_col) else { return };
    let title = app.col_titles.iter().find(|(c, _)| *c == col).map(|(_, t)| t.clone()).unwrap_or_default();
    let title_align = app.col_title_align.iter().find(|(c, _)| *c == col).map(|(_, a)| *a).unwrap_or(1);
    app.dialog = Some(crate::dialogs::Dialog::FieldSettings(FieldSettingsState {
        col_index: app.sel_col,
        col,
        title,
        title_align,
        data_align: app.col_alignment(col),
        width: w,
        header_wrap: true,
        wrap: app.wrap_cols.contains(&col),
    }));
}

pub fn show_field_settings(app: &mut App, ctx: &egui::Context, mut st: FieldSettingsState) -> Option<FieldSettingsState> {
    let mut result: Option<bool> = None;
    let mut best_fit = false;
    crate::theme::modal(egui::Id::unique("field_settings")).show(ctx, |ui| {
        crate::theme::dialog_spacing(ui);
        ui.set_width(400.0);
        title(ui, "Field Settings");
        egui::Grid::new("fs").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
            ui.label(tr("Field name:"));
            pick(ui, "fs_field", &mut st.col, &TCol::ALL, |c| c.title().to_string(), 220.0);
            ui.end_row();
            ui.label(tr("Title:"));
            ui.add(egui::TextEdit::singleline(&mut st.title).desired_width(220.0).hint_text(st.col.title()));
            ui.end_row();
            ui.label(tr("Align title:"));
            pick(ui, "fs_ta", &mut st.title_align, &[0u8, 1, 2], |a| ["Left", "Center", "Right"][a as usize].to_string(), 120.0);
            ui.end_row();
            ui.label(tr("Align data:"));
            pick(ui, "fs_da", &mut st.data_align, &[0u8, 1, 2], |a| ["Left", "Center", "Right"][a as usize].to_string(), 120.0);
            ui.end_row();
            ui.label(tr("Width:"));
            let mut w = (st.width / 7.0).round() as i32; // MPP measures widths in characters
            ui.add(egui::DragValue::new(&mut w).range(1..=200));
            st.width = w as f32 * 7.0;
            ui.end_row();
            ui.label(tr(""));
            ui.checkbox(&mut st.header_wrap, tr("Header Text Wrapping"));
            ui.end_row();
            ui.label(tr(""));
            ui.checkbox(&mut st.wrap, tr("Wrap Text"));
            ui.end_row();
        });
        ui.add_space(8.0);
        ui.separator();
        ui.horizontal(|ui| {
            if ui.add(egui::Button::new(tr("Best Fit")).min_size(Vec2::new(70.0, 26.0))).clicked() {
                best_fit = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(egui::Button::new(tr("Cancel")).min_size(Vec2::new(70.0, 26.0))).clicked() {
                    result = Some(false);
                }
                if ui.add(egui::Button::new(RichText::new(tr("OK")).strong()).min_size(Vec2::new(70.0, 26.0))).clicked() {
                    result = Some(true);
                }
            });
        });
    });
    if esc(ctx) {
        result = Some(false);
    }
    if best_fit {
        // widest cell text + padding
        let col = st.col;
        let mut w: f32 = ctx.fonts_mut(|f| f.layout_no_wrap(st.col.title().to_string(), egui::FontId::proportional(12.0), theme::TEXT).size().x) + 24.0;
        for i in 0..app.doc.tasks.len() {
            let s = app.cell_text(i, col);
            let indent = if col == TCol::Name { app.doc.tasks[i].outline_level as f32 * 14.0 + 10.0 } else { 0.0 };
            w = w.max(ctx.fonts_mut(|f| f.layout_no_wrap(s, egui::FontId::proportional(12.0), theme::TEXT).size().x) + 12.0 + indent);
        }
        st.width = w.min(600.0);
        result = Some(true);
    }
    match result {
        Some(true) => {
            if st.col_index < app.task_cols.len() {
                let old = app.task_cols[st.col_index].0;
                app.task_cols[st.col_index] = (st.col, st.width.max(16.0));
                app.col_titles.retain(|(c, _)| *c != old && *c != st.col);
                if !st.title.trim().is_empty() && st.title.trim() != st.col.title() {
                    app.col_titles.push((st.col, st.title.trim().to_string()));
                }
                app.col_title_align.retain(|(c, _)| *c != st.col);
                app.col_title_align.push((st.col, st.title_align));
                app.col_align.retain(|(c, _)| *c != st.col);
                app.col_align.push((st.col, st.data_align));
                app.wrap_cols.retain(|c| *c != old && *c != st.col);
                if st.wrap {
                    app.wrap_cols.push(st.col);
                }
            }
            None
        }
        Some(false) => None,
        None => Some(st),
    }
}

// ====================================================================== Project Options

#[derive(Clone, Debug)]
pub struct OptionsState {
    pub page: u8,
    pub settings: UserSettings,
    pub opts: ProjectOptions,
    pub new_tasks_auto: bool,
    pub hours_per_day: f64,
    pub show_project_summary: bool,
    pub start_time: String,
    pub end_time: String,
}

pub fn open_options(app: &mut App, page: u8) {
    app.commit_edit();
    // the AI assistants may have been installed or changed since the page was last shown
    crate::aiconnect::refresh();
    app.backstage = None;
    let o = &app.doc.options;
    app.dialog = Some(crate::dialogs::Dialog::Options(OptionsState {
        page,
        settings: app.settings.clone(),
        opts: o.clone(),
        new_tasks_auto: app.doc.new_tasks_auto,
        hours_per_day: app.doc.calendar.hours_per_day,
        show_project_summary: app.doc.show_project_summary,
        start_time: app.loc.time(o.default_start_time.0, o.default_start_time.1),
        end_time: app.loc.time(o.default_end_time.0, o.default_end_time.1),
    }));
}

const OPTION_PAGES: [&str; 12] = ["General", "Display", "Schedule", "Proofing", "Save", "Language", "Advanced", "Customize Ribbon", "Quick Access Toolbar", "Add-ins", "Trust Center", "AI Assistants"];

/// File > Options page of the AI assistants (Help > AI Assistants opens it).
pub const AI_PAGE: u8 = 11;
/// Options pages with nothing to set in Neramit Planner (not listed).
const EMPTY_PAGES: [usize; 6] = [3, 4, 7, 8, 9, 10];

fn section(ui: &mut Ui, text: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(crate::i18n::trp(text)).strong().color(theme::ACCENT).size(13.0));
    ui.separator();
}

fn parse_time(s: &str, loc: &crate::locale::Locale) -> Option<(u32, u32)> {
    let s = s.trim().to_uppercase();
    let pm = s.ends_with(&loc.pm.to_uppercase()) || s.ends_with("PM");
    let am = s.ends_with(&loc.am.to_uppercase()) || s.ends_with("AM");
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit() || *c == ':' || *c == '.').collect();
    let (h, m) = match digits.split_once([':', '.']) {
        Some((h, m)) => (h.trim().parse::<u32>().ok()?, m.trim().parse::<u32>().ok()?),
        None => (digits.trim().parse::<u32>().ok()?, 0),
    };
    let h = if pm && h < 12 { h + 12 } else if am && h == 12 { 0 } else { h };
    (h < 24 && m < 60).then_some((h, m))
}

pub fn show_options(app: &mut App, ctx: &egui::Context, mut st: OptionsState) -> Option<OptionsState> {
    let mut result: Option<bool> = None;
    let loc = app.loc.clone();
    let date_names = crate::locale::format_names(&loc);
    crate::theme::modal(egui::Id::unique("project_options")).show(ctx, |ui| {
        crate::theme::dialog_spacing(ui);
        ui.set_width(780.0);
        title(ui, "Project Options");
        ui.horizontal_top(|ui| {
            // left navigation
            egui::Frame::NONE.fill(Color32::from_rgb(0xF3, 0xF3, 0xF3)).inner_margin(6.0).show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                    ui.set_width(150.0);
                    ui.set_min_height(470.0);
                    ui.spacing_mut().button_padding = Vec2::new(10.0, 5.0);
                    for (k, p) in OPTION_PAGES.iter().enumerate() {
                        if EMPTY_PAGES.contains(&k) {
                            continue;
                        }
                        let sel = st.page == k as u8;
                        let r = ui.add(egui::Button::selectable(sel, tr(p)));
                        if r.clicked() {
                            st.page = k as u8;
                        }
                    }
                });
            });
            ui.add_space(10.0);
            ui.vertical(|ui| {
                ui.set_width(590.0);
                ui.set_max_width(590.0);
                egui::ScrollArea::vertical().max_height(470.0).show(ui, |ui| {
                    match st.page {
                    0 => {
                        ui.label(RichText::new(crate::i18n::trp("General options for working with Neramit Planner.")).size(13.0));
                        section(ui, "User Interface options");
                        ui.horizontal(|ui| {
                            ui.label(tr("ScreenTip style:"));
                            let mut s = if st.settings.screentips { 0u8 } else { 2 };
                            pick(ui, "opt_tips", &mut s, &[0u8, 1, 2], |v| ["Show feature descriptions in ScreenTips", "Don't show feature descriptions in ScreenTips", "Don't show ScreenTips"][v as usize].to_string(), 300.0);
                            st.settings.screentips = s != 2;
                        });
                        section(ui, "Project view");
                        egui::Grid::new("opt_view").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                            ui.label(tr("Default view:"));
                            pick(ui, "opt_defview", &mut st.settings.default_view, &[0u8, 1, 2], |v| ["Gantt with Timeline", "Task Sheet", "Resource Sheet"][v as usize].to_string(), 200.0);
                            ui.end_row();
                            ui.label(tr("Date format:"));
                            egui::ComboBox::from_id_salt("opt_date").width(200.0).selected_text(date_names.get(st.settings.date_format as usize).cloned().unwrap_or_default()).show_ui(ui, |ui| {
                                for (k, n) in date_names.iter().enumerate() {
                                    if ui.selectable_label(st.settings.date_format == k as u8, n).clicked() {
                                        st.settings.date_format = k as u8;
                                    }
                                }
                            });
                            ui.end_row();
                        });
                        section(ui, "Personalize your copy of Neramit Planner");
                        egui::Grid::new("opt_user").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                            ui.label(tr("User name:"));
                            ui.add(egui::TextEdit::singleline(&mut st.settings.user_name).desired_width(240.0));
                            ui.end_row();
                            ui.label(tr("Initials:"));
                            ui.add(egui::TextEdit::singleline(&mut st.settings.initials).desired_width(60.0));
                            ui.end_row();
                            ui.label(tr("Theme:"));
                            let mut t = if st.settings.classic_ui { 0u8 } else { 1 };
                            pick(ui, "opt_theme", &mut t, &[0u8, 1], |v| ["Classic", "Modern"][v as usize].to_string(), 240.0);
                            st.settings.classic_ui = t == 0;
                            ui.end_row();
                        });
                        section(ui, "File types");
                        if crate::setup::install_location().is_none() {
                            ui.label(RichText::new(tr("Install Neramit Planner with Setup to open files with a double-click.")).color(theme::TEXT_DIM));
                        } else if crate::setup::is_mpp_default() {
                            ui.label(tr(".mpp and .mpt files open with Neramit Planner."));
                        } else if ui.button(tr("Open .mpp and .mpt files with Neramit Planner")).clicked() && !crate::setup::set_mpp_default() {
                            // the user picked another program in Windows: only Windows' own dialog may change that
                            crate::setup::choose_mpp_default();
                        }
                    }
                    1 => {
                        ui.label(RichText::new(tr("Change how Project content is displayed on the screen.")).size(13.0));
                        section(ui, "Calendar");
                        ui.horizontal(|ui| {
                            ui.label(tr("Calendar Type:"));
                            pick(ui, "opt_cal", &mut st.settings.calendar_type, &[0u8, 1, 2], |v| ["Automatic (display language)", "Gregorian Calendar", "Thai Buddhist Calendar"][v as usize].to_string(), 220.0);
                        });
                        section(ui, "Currency options for this project");
                        egui::Grid::new("opt_cur").num_columns(4).spacing([10.0, 6.0]).show(ui, |ui| {
                            ui.label(tr("Symbol:"));
                            ui.add(egui::TextEdit::singleline(&mut st.opts.currency_symbol).desired_width(50.0));
                            ui.label(tr("Decimal digits:"));
                            let mut d = st.opts.currency_digits as i32;
                            ui.add(egui::DragValue::new(&mut d).range(0..=4));
                            st.opts.currency_digits = d as u8;
                            ui.end_row();
                            ui.label(tr("Placement:"));
                            pick(ui, "opt_pl", &mut st.opts.currency_position, &[0u8, 1, 2, 3], |v| ["$1", "1$", "$ 1", "1 $"][v as usize].to_string(), 70.0);
                            ui.label(tr("Currency:"));
                            ui.add(egui::TextEdit::singleline(&mut st.opts.currency_code).desired_width(60.0));
                            ui.end_row();
                        });
                        section(ui, "Show these elements");
                        ui.checkbox(&mut st.settings.show_status_bar, tr("Status bar"));
                    }
                    2 => {
                        ui.label(RichText::new(tr("Change options related to scheduling, calendars, and calculations.")).size(13.0));
                        section(ui, "Calendar options for this project");
                        egui::Grid::new("opt_calopt").num_columns(4).spacing([10.0, 6.0]).show(ui, |ui| {
                            ui.label(tr("Week starts on:"));
                            pick(ui, "opt_wk", &mut st.opts.week_starts_on, &[0u8, 1, 2, 3, 4, 5, 6], |v| ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"][v as usize].to_string(), 110.0);
                            ui.label(tr("Fiscal year starts in:"));
                            pick(ui, "opt_fy", &mut st.opts.fiscal_year_start, &[1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12], |v| loc.month_long[v as usize - 1].clone(), 110.0);
                            ui.end_row();
                            ui.label(tr("Default start time:"));
                            ui.add(egui::TextEdit::singleline(&mut st.start_time).desired_width(80.0));
                            ui.label(tr("Default end time:"));
                            ui.add(egui::TextEdit::singleline(&mut st.end_time).desired_width(80.0));
                            ui.end_row();
                            ui.label(tr("Hours per day:"));
                            ui.add(egui::DragValue::new(&mut st.hours_per_day).range(1.0..=24.0).speed(0.5));
                            ui.label(tr("Hours per week:"));
                            ui.add(egui::DragValue::new(&mut st.opts.hours_per_week).range(1.0..=168.0).speed(0.5));
                            ui.end_row();
                            ui.label(tr("Days per month:"));
                            ui.add(egui::DragValue::new(&mut st.opts.days_per_month).range(1.0..=31.0).speed(0.5));
                            ui.end_row();
                        });
                        section(ui, "Schedule");
                        section(ui, "Scheduling options for this project");
                        egui::Grid::new("opt_sched").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                            ui.label(tr("New tasks created:"));
                            let mut v = if st.new_tasks_auto { 1u8 } else { 0 };
                            pick(ui, "opt_new", &mut v, &[0u8, 1], |x| ["Manually Scheduled", "Auto Scheduled"][x as usize].to_string(), 180.0);
                            st.new_tasks_auto = v == 1;
                            ui.end_row();
                            ui.label(tr("Auto scheduled tasks scheduled on:"));
                            let mut v = if st.opts.new_tasks_start_today { 1u8 } else { 0 };
                            pick(ui, "opt_son", &mut v, &[0u8, 1], |x| ["Project Start Date", "Current Date"][x as usize].to_string(), 180.0);
                            st.opts.new_tasks_start_today = v == 1;
                            ui.end_row();
                            ui.label(tr("Duration is entered in:"));
                            pick(ui, "opt_du", &mut st.opts.duration_units, &[0u8, 1, 2, 3, 4], |x| ["Minutes", "Hours", "Days", "Weeks", "Months"][x as usize].to_string(), 180.0);
                            ui.end_row();
                            ui.label(tr("Work is entered in:"));
                            pick(ui, "opt_wu", &mut st.opts.work_units, &[0u8, 1, 2, 3, 4], |x| ["Minutes", "Hours", "Days", "Weeks", "Months"][x as usize].to_string(), 180.0);
                            ui.end_row();
                            ui.label(tr("Default task type:"));
                            pick(ui, "opt_tt", &mut st.opts.default_task_type, &TaskType::ALL, |x| x.label().to_string(), 180.0);
                            ui.end_row();
                        });
                        ui.checkbox(&mut st.opts.new_tasks_effort_driven, tr("New tasks are effort driven"));
                        ui.checkbox(&mut st.opts.autolink, tr("Autolink inserted or moved tasks"));
                        ui.checkbox(&mut st.opts.split_in_progress, tr("Split in-progress tasks"));
                        ui.checkbox(&mut st.opts.honor_constraints, tr("Tasks will always honor their constraint dates"));
                        ui.checkbox(&mut st.opts.show_estimated, tr("Show that scheduled tasks have estimated durations"));
                        ui.checkbox(&mut st.opts.new_tasks_estimated, tr("New scheduled tasks have estimated durations"));
                        ui.checkbox(&mut st.opts.keep_on_working_day, tr("Keep task on nearest working day when changing to Automatically Scheduled mode"));
                        section(ui, "Calculation");
                        ui.horizontal(|ui| {
                            ui.label(tr("Calculate project after each edit:"));
                            ui.radio_value(&mut st.settings.auto_calculate, true, tr("On"));
                            ui.radio_value(&mut st.settings.auto_calculate, false, tr("Off"));
                        });
                    }
                    6 => {
                        ui.label(RichText::new(crate::i18n::trp("Advanced options for working with Neramit Planner.")).size(13.0));
                        section(ui, "General");
                        ui.horizontal(|ui| {
                            ui.label(tr("Undo levels:"));
                            let mut u = st.settings.undo_levels as i32;
                            ui.add(egui::DragValue::new(&mut u).range(1..=99));
                            st.settings.undo_levels = u as u8;
                        });
                        section(ui, "Display");
                        ui.horizontal(|ui| {
                            ui.label(tr("Show this number of Recent Projects:"));
                            let mut r = st.settings.recent_count as i32;
                            ui.add(egui::DragValue::new(&mut r).range(0..=50));
                            st.settings.recent_count = r as u8;
                        });
                        ui.checkbox(&mut st.settings.show_status_bar, tr("Show status bar"));
                        ui.checkbox(&mut st.settings.screentips, tr("Show project screentips"));
                        section(ui, "Display options for this project");
                        egui::Grid::new("opt_labels").num_columns(6).spacing([8.0, 6.0]).show(ui, |ui| {
                            let choices: [&[&str]; 6] = [&["m", "min", "minute"], &["h", "hr", "hour"], &["d", "dy", "day"], &["w", "wk", "week"], &["mo", "mon", "month"], &["y", "yr", "year"]];
                            for (k, lbl) in ["Minutes:", "Hours:", "Days:", "Weeks:", "Months:", "Years:"].iter().enumerate() {
                                ui.label(tr(lbl));
                                let cur = st.opts.duration_labels[k].clone();
                                egui::ComboBox::from_id_salt(("opt_lbl", k)).width(70.0).selected_text(cur.clone()).show_ui(ui, |ui| {
                                    for c in choices[k] {
                                        if ui.selectable_label(cur == *c, *c).clicked() {
                                            st.opts.duration_labels[k] = c.to_string();
                                        }
                                    }
                                });
                                if k % 3 == 2 {
                                    ui.end_row();
                                }
                            }
                        });
                        ui.checkbox(&mut st.opts.space_before_label, tr("Add space before label"));
                        ui.checkbox(&mut st.show_project_summary, tr("Show project summary task"));
                        ui.checkbox(&mut st.opts.underline_hyperlinks, tr("Underline hyperlinks"));
                        ui.checkbox(&mut st.opts.indent_name, tr("Indent name"));
                        ui.checkbox(&mut st.opts.show_outline_symbol, tr("Show outline symbol"));
                    }
                    5 => {
                        ui.label(RichText::new(tr("Choose the language of menus, ribbons and dialogs.")).size(13.0));
                        section(ui, "Display Language");
                        let a = ui.radio_value(&mut st.settings.language, 0, "English").changed();
                        let b = ui.radio_value(&mut st.settings.language, 1, crate::thai::shape("ไทย (Thai)").to_string()).changed();
                        if a || b {
                            st.settings.language_chosen = true;
                        }
                        ui.add_space(8.0);
                        ui.label(RichText::new(tr("Day and month names follow the display language; the date order follows the Windows regional settings (Display > Calendar Type).")).color(theme::TEXT_DIM));
                    }
                    11 => {
                        use crate::aiconnect::{Assistant, State};
                        ui.label(RichText::new(tr("Let AI assistants such as Claude and Codex open, read and change your projects.")).size(13.0));
                        section(ui, "Assistants on this PC");
                        let states = crate::aiconnect::states();
                        let busy = crate::aiconnect::busy();
                        egui::Grid::new("opt_ai").num_columns(3).spacing([24.0, 10.0]).min_col_width(120.0).show(ui, |ui| {
                            for (a, s) in Assistant::ALL.into_iter().zip(states) {
                                ui.label(RichText::new(a.name()).strong());
                                let (text, color, button) = match s {
                                    State::Missing => (tr("Not installed"), theme::TEXT_DIM, None),
                                    State::On => (tr("Connected"), theme::GREEN, Some((tr("Disconnect"), false))),
                                    State::Off => (tr("Not connected"), theme::TEXT, Some((tr("Connect"), true))),
                                };
                                ui.label(RichText::new(text).color(color));
                                match button {
                                    Some((label, on)) => {
                                        if ui.add_enabled(!busy, egui::Button::new(label).min_size(Vec2::new(110.0, 0.0))).clicked() {
                                            let c = ui.ctx().clone();
                                            crate::aiconnect::set_in_background(a, on, move || c.request_repaint());
                                        }
                                    }
                                    None => {
                                        ui.label("");
                                    }
                                }
                                ui.end_row();
                            }
                        });
                        ui.add_space(4.0);
                        if busy {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(tr("Working..."));
                            });
                        } else if let Some((a, on, ok)) = crate::aiconnect::last_result() {
                            let (msg, color) = match (on, ok) {
                                (true, true) => (crate::i18n::trf("Connected. Restart {} to use Neramit Planner there.", &[a.name()]), theme::GREEN),
                                (false, true) => (crate::i18n::trf("Disconnected. Restart {} to finish.", &[a.name()]), theme::TEXT),
                                (_, false) => (crate::i18n::trf("{} could not be changed. Check that it is installed for this Windows user.", &[a.name()]), Color32::from_rgb(0xC0, 0x39, 0x2B)),
                            };
                            ui.label(RichText::new(msg).color(color));
                        }
                        section(ui, "How it works");
                        for line in [
                            "The assistant starts Neramit Planner in the background (NeramitPlanner.exe --mcp) only while it works on a project: nothing keeps running.",
                            "Changes stay in the assistant's copy until it saves. A .mpp file is never overwritten, nor a file changed since the assistant opened it.",
                            "Only Neramit Planner's own entry is added to or removed from each assistant's settings.",
                        ] {
                            ui.label(RichText::new(format!("•  {}", tr(line))).color(theme::TEXT_DIM));
                        }
                        section(ui, "Other assistants");
                        ui.label(tr("Add an MCP server (stdio) that runs this command:"));
                        let cmd = format!("\"{}\" --mcp", std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default());
                        ui.horizontal(|ui| {
                            let mut shown = cmd.clone();
                            ui.add(egui::TextEdit::singleline(&mut shown).font(egui::TextStyle::Monospace).desired_width(470.0).interactive(false));
                            if ui.button(tr("Copy")).clicked() {
                                ui.ctx().copy_text(cmd.clone());
                            }
                        });
                    }
                    _ => {
                        ui.add_space(20.0);
                        ui.label(RichText::new(format!("{} - {}", tr(OPTION_PAGES[st.page as usize]), crate::i18n::trp("Neramit Planner works offline without add-ins, proofing tools or a trust center."))).color(theme::TEXT_DIM));
                        ui.label(RichText::new(crate::i18n::trp("Neramit Planner works offline without add-ins, proofing tools or a trust center.")).color(theme::TEXT_DIM));
                    }
                }});
            });
        });
        let (ok, cancel, _) = buttons(ui, None);
        if ok {
            result = Some(true);
        } else if cancel {
            result = Some(false);
        }
    });
    if esc(ctx) {
        result = Some(false);
    }
    match result {
        Some(true) => {
            app.settings = st.settings.clone();
            app.apply_settings();
            let mut opts = st.opts.clone();
            if let Some(t) = parse_time(&st.start_time, &loc) {
                opts.default_start_time = t;
            }
            if let Some(t) = parse_time(&st.end_time, &loc) {
                opts.default_end_time = t;
            }
            let changed = opts != app.doc.options
                || st.new_tasks_auto != app.doc.new_tasks_auto
                || (st.hours_per_day - app.doc.calendar.hours_per_day).abs() > f64::EPSILON
                || st.show_project_summary != app.doc.show_project_summary;
            if changed {
                app.checkpoint();
                app.doc.options = opts;
                app.doc.new_tasks_auto = st.new_tasks_auto;
                app.doc.calendar.hours_per_day = st.hours_per_day;
                app.doc.show_project_summary = st.show_project_summary;
                app.recalc();
            }
            None
        }
        Some(false) => None,
        None => Some(st),
    }
}

// ====================================================================== Gantt Chart Style gallery

/// The Gantt Chart Style gallery: task bar fill and the dark colour of summaries / milestones.
/// The first one is the default look of new plans (see assets/gantt-bar-styles.json).
pub const GALLERY: [(Rgb, Rgb); 6] = [
    ([90, 155, 234], [62, 75, 94]),
    ([92, 190, 130], [19, 114, 64]),
    ([247, 170, 90], [199, 106, 16]),
    ([160, 172, 188], [45, 55, 70]),
    ([180, 156, 248], [91, 52, 201]),
    ([94, 200, 200], [12, 110, 120]),
];
pub const MILESTONE_ORANGE: Rgb = [245, 146, 47];

/// Apply one of the gallery colour schemes to the task / summary / milestone bar styles.
pub fn apply_gallery(app: &mut App, g: u8) {
    let (task, dark) = GALLERY[(g as usize).min(GALLERY.len() - 1)];
    app.checkpoint();
    app.doc.formatting.gallery = g;
    for s in app.doc.formatting.bar_styles.iter_mut() {
        match s.name.as_str() {
            "Task" | "Split" => s.bar.middle.color = task,
            "Summary" => {
                s.bar.start.color = dark;
                s.bar.middle.color = dark;
                s.bar.end.color = dark;
            }
            // the default scheme keeps its orange milestones
            "Milestone" => s.bar.start.color = if g == 0 { MILESTONE_ORANGE } else { dark },
            _ => {}
        }
    }
}
