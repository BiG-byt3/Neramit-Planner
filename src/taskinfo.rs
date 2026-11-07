//! Task Information dialog — General, Predecessors, Resources, Advanced, Notes and Custom Fields
//! tabs with the task's fields.

use crate::i18n::{tr};
use crate::app::App;
use crate::model::*;
use crate::theme;
use eframe::egui::{self, RichText, Ui, Vec2};

#[derive(Clone, Debug)]
pub struct PredRow {
    pub id: String,
    pub kind: LinkType,
    pub lag: String,
}

#[derive(Clone, Debug)]
pub struct TaskInfoState {
    pub idx: usize,
    pub tab: usize,
    pub name: String,
    pub duration: String,
    pub estimated: bool,
    pub start: String,
    pub finish: String,
    pub percent: u8,
    pub priority: String,
    pub manual: bool,
    pub inactive: bool,
    pub milestone: bool,
    pub on_timeline: bool,
    pub hide_bar: bool,
    pub rollup: bool,
    pub notes: String,
    pub preds: Vec<PredRow>,
    pub res: Vec<u32>,
    /// units typed per resource (percent for work resources, quantity for materials)
    pub units: Vec<(u32, f64)>,
    pub deadline: String,
    pub constraint_type: ConstraintType,
    pub constraint_date: String,
    pub task_type: TaskType,
    pub effort_driven: bool,
    pub wbs: String,
    pub error: Option<String>,
}

pub fn open_task_info(app: &mut App, tab: usize) {
    app.commit_edit();
    let Some(i) = app.current_task() else { return };
    let st = state_for(app, i, tab);
    app.dialog = Some(crate::dialogs::Dialog::TaskInfo(Box::new(st)));
}

/// The fields of task `i` as Task Information (and the Task Form) edit them.
pub fn state_for(app: &App, i: usize, tab: usize) -> TaskInfoState {
    let t = app.doc.tasks[i].clone();
    let (ids, _) = app.id_maps();
    let preds = t
        .predecessors
        .iter()
        .map(|l| PredRow {
            id: ids.get(&l.pred_uid).map(|v| v.to_string()).unwrap_or_default(),
            kind: l.kind,
            lag: {
                let lt = crate::util::fmt_lag(l, app.dur_units(), &app.doc.options, crate::i18n::is_thai());
                if lt.is_empty() { "0d".into() } else { lt }
            },
        })
        .collect();
    let wbs = if t.wbs.is_empty() { app.doc.outline_numbers()[i].clone() } else { t.wbs.clone() };
    TaskInfoState {
        idx: i,
        tab,
        name: t.name.clone(),
        duration: match &t.manual_text[2] {
            Some(s) if t.manual => s.clone(),
            _ => {
                let mut s = app.task_duration_text(&t);
                if s.ends_with('?') {
                    s.pop();
                }
                s
            }
        },
        estimated: t.estimated,
        start: match &t.manual_text[0] {
            Some(s) if t.manual => s.clone(),
            _ => app.fmt_date(t.start),
        },
        finish: match &t.manual_text[1] {
            Some(s) if t.manual => s.clone(),
            _ => app.fmt_finish(t.finish),
        },
        percent: t.percent_complete,
        priority: t.priority.to_string(),
        manual: t.manual,
        inactive: !t.active,
        milestone: t.is_milestone(),
        on_timeline: t.on_timeline,
        hide_bar: t.hide_bar,
        rollup: t.rollup,
        notes: t.notes.clone(),
        preds,
        res: t.resources.clone(),
        units: t
            .assignments
            .iter()
            .map(|a| {
                let work = app.doc.resource_by_uid(a.res_uid).map(|r| r.rtype == ResourceType::Work).unwrap_or(true);
                (a.res_uid, if work { a.units * 100.0 } else { a.units })
            })
            .collect(),
        deadline: t.deadline.map(|d| app.fmt_finish(d)).unwrap_or_else(|| "NA".into()),
        constraint_type: t.constraint_type,
        constraint_date: t.constraint_date.filter(|_| t.constraint_type.needs_date()).map(|d| app.fmt_date(d)).unwrap_or_else(|| "NA".into()),
        task_type: t.task_type,
        effort_driven: t.effort_driven,
        wbs,
        error: None,
    }
}

/// View > Details: the standard Task Form under the chart - the selected task's name, duration,
/// dates, type and progress, its resources and its predecessors; OK applies them, Previous / Next
/// move through the tasks.
pub fn task_form(app: &mut App, ui: &mut Ui, rect: egui::Rect) {
    ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(0xF7, 0xF7, 0xF7));
    ui.painter().line_segment([rect.left_top(), rect.right_top()], egui::Stroke::new(1.0_f32, theme::SHEET_SPLIT_EDGE));
    let Some(i) = app.current_task() else {
        app.details = None;
        let p = ui.painter();
        p.text(rect.center(), egui::Align2::CENTER_CENTER, tr("Select a task to see its details."), egui::FontId::proportional(13.0), theme::TEXT_DIM);
        return;
    };
    // a fresh form when another task is selected (or the task changed elsewhere)
    let stale = app.details.as_ref().map(|s| s.idx != i).unwrap_or(true) || app.details_rev != app.undo_len();
    if stale {
        app.details = Some(state_for(app, i, 0));
        app.details_rev = app.undo_len();
    }
    let Some(mut st) = app.details.take() else { return };
    let is_sum = app.doc.tasks[i].is_summary;
    let mut ok = false;
    let mut step: i32 = 0;
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink2(Vec2::new(10.0, 6.0))).layout(egui::Layout::top_down(egui::Align::LEFT)));
    egui::ScrollArea::vertical().id_salt("task_form").auto_shrink([false, false]).show(&mut child, |ui| {
        ui.horizontal(|ui| {
            ui.label(tr("Name:"));
            ui.add(egui::TextEdit::singleline(&mut st.name).desired_width(260.0));
            ui.label(tr("Duration:"));
            ui.add_enabled(!is_sum, egui::TextEdit::singleline(&mut st.duration).desired_width(70.0));
            ui.add_enabled(!is_sum, egui::Checkbox::new(&mut st.effort_driven, tr("Effort driven")));
            ui.add_enabled_ui(!is_sum, |ui| {
                ui.radio_value(&mut st.manual, true, tr("Manually Scheduled"));
                ui.radio_value(&mut st.manual, false, tr("Auto Scheduled"));
            });
            if ui.button(tr("Previous")).clicked() {
                step = -1;
            }
            if ui.button(tr("Next")).clicked() {
                step = 1;
            }
            if ui.add(egui::Button::new(RichText::new(tr("OK")).strong())).clicked() {
                ok = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label(tr("Start:"));
            ui.add_enabled(!is_sum, egui::TextEdit::singleline(&mut st.start).desired_width(150.0));
            ui.label(tr("Finish:"));
            ui.add_enabled(!is_sum, egui::TextEdit::singleline(&mut st.finish).desired_width(150.0));
            ui.label(tr("Task type:"));
            egui::ComboBox::from_id_salt("tf_type").selected_text(tr(st.task_type.label())).show_ui(ui, |ui| {
                for tt in TaskType::ALL {
                    ui.selectable_value(&mut st.task_type, tt, tr(tt.label()));
                }
            });
            ui.label(tr("% Complete:"));
            let mut pct = st.percent as u32;
            ui.add(egui::DragValue::new(&mut pct).range(0..=100).suffix("%"));
            st.percent = pct as u8;
        });
        if let Some(e) = &st.error {
            ui.label(RichText::new(tr(e)).color(egui::Color32::from_rgb(0xC0, 0x39, 0x2B)));
        }
        ui.add_space(4.0);
        ui.columns(2, |cols| {
            resources_tab(app, &mut cols[0], &mut st);
            predecessors_tab(app, &mut cols[1], &mut st);
        });
    });
    if ok {
        match apply(app, &st) {
            Ok(()) => {
                app.recalc();
                app.details = None;
                return;
            }
            Err(e) => st.error = Some(e),
        }
    }
    if step != 0 {
        let vis = app.visible_rows();
        let r = (app.sel_row as i32 + step).clamp(0, vis.len().saturating_sub(1) as i32) as usize;
        app.sel_row = r;
        app.sel_anchor = r;
        app.scroll_to_row = Some(r);
        app.details = None;
        return;
    }
    app.details = Some(st);
}

fn link_label(k: LinkType) -> &'static str {
    match k {
        LinkType::FS => "Finish-to-Start (FS)",
        LinkType::SS => "Start-to-Start (SS)",
        LinkType::FF => "Finish-to-Finish (FF)",
        LinkType::SF => "Start-to-Finish (SF)",
    }
}

const TABS: [&str; 6] = ["General", "Predecessors", "Resources", "Advanced", "Notes", "Custom Fields"];

/// Draws the dialog; returns the state to keep showing, or None when it closed.
pub fn show(app: &mut App, ctx: &egui::Context, mut st: TaskInfoState) -> Option<TaskInfoState> {
    let mut done: Option<bool> = None;
    let mut open_project_info = false;
    let is_sum = app.doc.tasks.get(st.idx).map(|t| t.is_summary).unwrap_or(false);
    let multi = app.selected_tasks().len() > 1;
    crate::theme::modal(egui::Id::unique("task_info")).show(ctx, |ui| {
        crate::theme::dialog_spacing(ui);
        ui.set_width(580.0);
        let caption = if multi { "Multiple Task Information" } else if is_sum { "Summary Task Information" } else { "Task Information" };
        ui.label(RichText::new(tr(caption)).strong().size(14.0));
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            for (k, t) in TABS.iter().enumerate() {
                if ui.selectable_label(st.tab == k, tr(t)).clicked() {
                    st.tab = k;
                }
            }
        });
        ui.separator();
        let ro = false;
        ui.add_enabled_ui(!ro, |ui| {
        // Name / Duration / Estimated appear on every tab
        ui.horizontal(|ui| {
            ui.label(tr("Name:"));
            ui.add(egui::TextEdit::singleline(&mut st.name).desired_width(290.0));
            ui.label(tr("Duration:"));
            ui.add_enabled(!is_sum, egui::TextEdit::singleline(&mut st.duration).desired_width(70.0));
            ui.add_enabled(!is_sum, egui::Checkbox::new(&mut st.estimated, tr("Estimated")));
        });
        ui.add_space(6.0);
        egui::Frame::NONE.inner_margin(4.0).show(ui, |ui| {
            ui.set_min_height(250.0);
            match st.tab {
                0 => general_tab(ui, &mut st, is_sum),
                1 => predecessors_tab(app, ui, &mut st),
                2 => resources_tab(app, ui, &mut st),
                3 => advanced_tab(app, ui, &mut st, is_sum),
                4 => notes_tab(ui, &mut st),
                _ => {
                    ui.label(tr("Custom Fields:"));
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_min_size(Vec2::new(ui.available_width(), 200.0));
                        egui::Grid::new("ti_cf").num_columns(2).striped(true).show(ui, |ui| {
                            ui.label(RichText::new(tr("Custom Field Name")).strong());
                            ui.label(RichText::new(tr("Value")).strong());
                            ui.end_row();
                        });
                        ui.label(RichText::new(tr("No custom fields are defined for this project.")).color(theme::TEXT_DIM));
                    });
                }
            }
        });
        });
        if let Some(e) = &st.error {
            ui.colored_label(egui::Color32::from_rgb(0xC0, 0x39, 0x2B), tr(e));
        }
        ui.add_space(6.0);
        ui.separator();
        ui.horizontal(|ui| {
            let _ = ui.add_enabled(false, egui::Button::new(tr("Help")).min_size(Vec2::new(70.0, 26.0)));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ro {
                    if ui.add(egui::Button::new(tr("Close")).min_size(Vec2::new(70.0, 26.0))).clicked() {
                        done = Some(false);
                    }
                } else {
                    if ui.add(egui::Button::new(tr("Cancel")).min_size(Vec2::new(70.0, 26.0))).clicked() {
                        done = Some(false);
                    }
                    if ui.add(egui::Button::new(RichText::new(tr("OK")).strong()).min_size(Vec2::new(70.0, 26.0))).clicked() {
                        done = Some(true);
                    }
                }
                if ui.add(egui::Button::new(tr("Project Information...")).min_size(Vec2::new(140.0, 26.0))).clicked() {
                    open_project_info = true;
                }
            });
        });
    });
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        done = Some(false);
    }
    if open_project_info {
        crate::dialogs::open_project_info(app);
        return None;
    }
    match done {
        Some(true) => match apply(app, &st) {
            Ok(()) => None,
            Err(e) => {
                st.error = Some(e);
                Some(st)
            }
        },
        Some(false) => None,
        None => Some(st),
    }
}

fn general_tab(ui: &mut Ui, st: &mut TaskInfoState, is_sum: bool) {
    egui::Grid::new("ti_gen").num_columns(4).spacing([12.0, 8.0]).show(ui, |ui| {
        ui.label(tr("Percent complete:"));
        let mut pct = st.percent as u32;
        ui.add(egui::DragValue::new(&mut pct).range(0..=100).suffix("%"));
        st.percent = pct as u8;
        ui.label(tr("Priority:"));
        ui.add(egui::TextEdit::singleline(&mut st.priority).desired_width(60.0));
        ui.end_row();
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(tr("Schedule Mode:"));
        ui.add_enabled_ui(!is_sum, |ui| {
            ui.radio_value(&mut st.manual, true, tr("Manually Scheduled"));
            ui.radio_value(&mut st.manual, false, tr("Auto Scheduled"));
        });
        ui.add_space(20.0);
        ui.checkbox(&mut st.inactive, tr("Inactive"));
    });
    ui.add_space(4.0);
    egui::Frame::group(ui.style()).inner_margin(8.0).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(tr("Dates")).strong());
        ui.horizontal(|ui| {
            ui.label(tr("Start:"));
            ui.add_enabled(!is_sum, egui::TextEdit::singleline(&mut st.start).desired_width(170.0));
            ui.add_space(12.0);
            ui.label(tr("Finish:"));
            ui.add_enabled(!is_sum, egui::TextEdit::singleline(&mut st.finish).desired_width(170.0));
        });
    });
    ui.add_space(4.0);
    ui.checkbox(&mut st.on_timeline, tr("Display on Timeline"));
    ui.checkbox(&mut st.hide_bar, tr("Hide Bar"));
    ui.checkbox(&mut st.rollup, tr("Rollup"));
}

fn predecessors_tab(app: &App, ui: &mut Ui, st: &mut TaskInfoState) {
    ui.label(tr("Predecessors:"));
    let mut remove = None;
    egui::Frame::group(ui.style()).inner_margin(2.0).show(ui, |ui| {
        egui::ScrollArea::vertical().max_height(190.0).show(ui, |ui| {
            egui::Grid::new("ti_preds").striped(true).num_columns(5).spacing([10.0, 4.0]).show(ui, |ui| {
                ui.label(RichText::new(tr("ID")).strong());
                ui.label(RichText::new(tr("Task Name")).strong());
                ui.label(RichText::new(tr("Type")).strong());
                ui.label(RichText::new(tr("Lag")).strong());
                ui.label(tr(""));
                ui.end_row();
                for (k, pr) in st.preds.iter_mut().enumerate() {
                    ui.add(egui::TextEdit::singleline(&mut pr.id).desired_width(40.0));
                    let nm = pr.id.trim().parse::<usize>().ok().and_then(|n| app.doc.tasks.get(n.wrapping_sub(1))).map(|t| t.name.clone()).unwrap_or_default();
                    ui.add_sized([200.0, 18.0], egui::Label::new(crate::thai::shape(&nm).to_string()).truncate());
                    egui::ComboBox::from_id_salt(("ptype", k)).width(150.0).selected_text(tr(link_label(pr.kind))).show_ui(ui, |ui| {
                        for lt in [LinkType::FS, LinkType::SS, LinkType::FF, LinkType::SF] {
                            ui.selectable_value(&mut pr.kind, lt, tr(link_label(lt)));
                        }
                    });
                    ui.add(egui::TextEdit::singleline(&mut pr.lag).desired_width(60.0));
                    if ui.small_button("×").clicked() {
                        remove = Some(k);
                    }
                    ui.end_row();
                }
            });
        });
    });
    if let Some(k) = remove {
        st.preds.remove(k);
    }
    if ui.button(tr("+ Add predecessor")).clicked() {
        st.preds.push(PredRow { id: String::new(), kind: LinkType::FS, lag: "0d".into() });
    }
}

fn resources_tab(app: &App, ui: &mut Ui, st: &mut TaskInfoState) {
    ui.label(tr("Resources:"));
    egui::Frame::group(ui.style()).inner_margin(2.0).show(ui, |ui| {
        egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
            egui::Grid::new("ti_res").striped(true).num_columns(3).spacing([10.0, 4.0]).show(ui, |ui| {
                ui.label(RichText::new(tr("Resource Name")).strong());
                ui.label(RichText::new(tr("Units")).strong());
                ui.label(tr(""));
                ui.end_row();
                for r in &app.doc.resources {
                    let mut on = st.res.contains(&r.uid);
                    if ui.checkbox(&mut on, crate::thai::shape(&r.name).to_string()).changed() {
                        if on {
                            st.res.push(r.uid);
                        } else {
                            st.res.retain(|u| *u != r.uid);
                        }
                    }
                    if on {
                        let work = r.rtype == ResourceType::Work;
                        let dflt = if work { r.max_units.min(100.0) } else { 1.0 };
                        let pos = st.units.iter().position(|(u, _)| *u == r.uid);
                        let mut v = pos.map(|p| st.units[p].1).unwrap_or(dflt);
                        if ui.add(egui::DragValue::new(&mut v).range(0.0..=100000.0).speed(if work { 5.0 } else { 1.0 }).suffix(if work { "%" } else { "" })).changed() {
                            match pos {
                                Some(p) => st.units[p].1 = v,
                                None => st.units.push((r.uid, v)),
                            }
                        }
                    } else {
                        ui.label(String::new());
                    }
                    ui.label(RichText::new(tr(r.rtype.label())).color(theme::TEXT_DIM));
                    ui.end_row();
                }
            });
            if app.doc.resources.is_empty() {
                ui.label(RichText::new(tr("No resources yet. Add them in the Resource Sheet, or type names into the Resource Names column.")).color(theme::TEXT_DIM));
            }
        });
    });
}

fn advanced_tab(app: &App, ui: &mut Ui, st: &mut TaskInfoState, is_sum: bool) {
    egui::Frame::group(ui.style()).inner_margin(8.0).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(tr("Constrain task")).strong());
        egui::Grid::new("ti_con").num_columns(4).spacing([12.0, 8.0]).show(ui, |ui| {
            ui.label(tr("Deadline:"));
            ui.add(egui::TextEdit::singleline(&mut st.deadline).desired_width(170.0).hint_text(tr("NA")));
            ui.end_row();
            ui.label(tr("Constraint type:"));
            ui.add_enabled_ui(!is_sum && !st.manual, |ui| {
                egui::ComboBox::from_id_salt("ti_ctype").width(170.0).selected_text(tr(st.constraint_type.label())).show_ui(ui, |ui| {
                    for c in ConstraintType::ALL {
                        if ui.selectable_label(st.constraint_type == c, tr(c.label())).clicked() {
                            st.constraint_type = c;
                            if !c.needs_date() {
                                st.constraint_date = "NA".into();
                            } else if st.constraint_date.trim().eq_ignore_ascii_case("NA") || st.constraint_date.trim().is_empty() {
                                st.constraint_date = st.start.clone();
                            }
                        }
                    }
                });
            });
            ui.label(tr("Constraint date:"));
            ui.add_enabled(st.constraint_type.needs_date() && !is_sum && !st.manual, egui::TextEdit::singleline(&mut st.constraint_date).desired_width(170.0).hint_text(tr("NA")));
            ui.end_row();
        });
    });
    ui.add_space(6.0);
    egui::Grid::new("ti_adv").num_columns(4).spacing([12.0, 8.0]).show(ui, |ui| {
        ui.label(tr("Task type:"));
        egui::ComboBox::from_id_salt("ti_ttype").width(170.0).selected_text(tr(st.task_type.label())).show_ui(ui, |ui| {
            for t in TaskType::ALL {
                ui.selectable_value(&mut st.task_type, t, tr(t.label()));
            }
        });
        ui.checkbox(&mut st.effort_driven, tr("Effort driven"));
        ui.end_row();
        ui.label(tr("Calendar:"));
        ui.add_enabled(false, egui::Button::new(tr("None")).min_size(Vec2::new(170.0, 20.0)));
        let mut ignore = false;
        ui.add_enabled(false, egui::Checkbox::new(&mut ignore, tr("Scheduling ignores resource calendars")));
        ui.end_row();
        ui.label(tr("WBS code:"));
        ui.add(egui::TextEdit::singleline(&mut st.wbs).desired_width(170.0));
        ui.end_row();
        ui.label(tr("Earned value method:"));
        ui.add_enabled(false, egui::Button::new(tr("% Complete")).min_size(Vec2::new(170.0, 20.0)));
        ui.end_row();
    });
    ui.add_space(6.0);
    ui.add_enabled(!is_sum, egui::Checkbox::new(&mut st.milestone, tr("Mark task as milestone")));
    if let Some(t) = app.doc.tasks.get(st.idx) {
        ui.add_space(6.0);
        ui.label(RichText::new(format!("Total slack: {}    Critical: {}    Unique ID: {}", crate::fileio::format_duration(t.total_slack), if t.critical { "Yes" } else { "No" }, t.uid)).color(theme::TEXT_DIM));
    }
    if st.manual {
        ui.label(RichText::new(tr("Some of the fields above are not editable because the task is Manually Scheduled.")).small().color(theme::TEXT_DIM));
    }
}

fn notes_tab(ui: &mut Ui, st: &mut TaskInfoState) {
    ui.label(tr("Notes:"));
    ui.horizontal(|ui| {
        for b in ["A", "≡", "≡", "≡", "•", "▣"] {
            let _ = ui.add_enabled(false, egui::Button::new(b).min_size(Vec2::new(22.0, 20.0)));
        }
    });
    ui.add(egui::TextEdit::multiline(&mut st.notes).desired_width(f32::INFINITY).desired_rows(9));
}

pub(crate) fn apply(app: &mut App, st: &TaskInfoState) -> Result<(), String> {
    if st.idx >= app.doc.tasks.len() {
        return Ok(());
    }
    let cal = app.doc.calendar.clone();
    let du = app.dur_units();
    let is_sum = app.doc.tasks[st.idx].is_summary;
    let cur = &app.doc.tasks[st.idx];
    // a manually scheduled task's Start / Finish / Duration may be text (MPP keeps it)
    let manual = st.manual && !is_sum;
    let mut texts = if cur.manual { cur.manual_text.clone() } else { [None, None, None] };
    let typed = [&st.start, &st.finish, &st.duration];
    let ok = [app.parse_date(&st.start).is_some(), app.parse_date(&st.finish).is_some(), is_sum || crate::util::parse_dur(&st.duration, du, cur.dur_unit).is_some()];
    for k in 0..3 {
        if manual && (!ok[k] || texts[k].as_deref() == Some(typed[k].trim())) {
            texts[k] = Some(typed[k].trim().to_string());
        } else {
            texts[k] = None;
        }
    }
    let dur: (i64, DurUnit, bool) = if is_sum || texts[2].is_some() {
        (cur.dur_minutes(du.mpd), if is_sum { DurUnit::Days } else { cur.dur_unit }, if is_sum { false } else { cur.dur_elapsed })
    } else {
        crate::util::parse_dur(&st.duration, du, cur.dur_unit).ok_or("The duration is not valid.")?
    };
    let start = if texts[0].is_some() { cur.start } else { app.parse_date(&st.start).ok_or("The start date is not valid.")? };
    let finish = if texts[1].is_some() { cur.finish } else { app.parse_date(&st.finish).ok_or("The finish date is not valid.")? };
    let priority: u32 = st.priority.trim().parse().map_err(|_| "Priority must be a number between 0 and 1000.")?;
    if priority > 1000 {
        return Err("Priority must be a number between 0 and 1000.".into());
    }
    let na = |s: &str| s.trim().is_empty() || s.trim().eq_ignore_ascii_case("NA");
    let deadline = if na(&st.deadline) { None } else { Some(app.parse_date(&st.deadline).ok_or("The deadline date is not valid.")?) };
    let cdate = if st.constraint_type.needs_date() {
        if na(&st.constraint_date) {
            Some(start)
        } else {
            Some(app.parse_date(&st.constraint_date).ok_or("The constraint date is not valid.")?)
        }
    } else {
        None
    };
    let (_, id_to_uid) = app.id_maps();
    let me = app.doc.tasks[st.idx].uid;
    let mut links = Vec::new();
    for pr in &st.preds {
        if pr.id.trim().is_empty() {
            continue;
        }
        let id: usize = pr.id.trim().parse().map_err(|_| format!("'{}' is not a valid task ID.", pr.id))?;
        let uid = *id_to_uid.get(&id).ok_or_else(|| format!("Task ID {id} does not exist."))?;
        if app.is_outline_relative(id - 1, st.idx) {
            return Err("You cannot link a summary task to one of its subtasks.".into());
        }
        let lag_txt = pr.lag.trim();
        let mut link = Link::new(uid, pr.kind);
        if !lag_txt.is_empty() {
            crate::util::parse_lag_into(lag_txt, &mut link, du).ok_or("Lag is not valid.")?;
        }
        links.push(link);
    }
    let old_preds = std::mem::take(&mut app.doc.tasks[st.idx].predecessors);
    for l in &links {
        if crate::schedule::creates_cycle(&app.doc, l.pred_uid, me) {
            app.doc.tasks[st.idx].predecessors = old_preds;
            return Err("A predecessor would create a circular relationship.".into());
        }
    }
    app.doc.tasks[st.idx].predecessors = old_preds;

    let before_start = app.doc.tasks[st.idx].start;
    let before_finish = app.doc.tasks[st.idx].finish;
    app.checkpoint();
    let t = &mut app.doc.tasks[st.idx];
    t.name = st.name.trim().to_string();
    if t.percent_complete != st.percent.min(100) {
        t.set_percent(st.percent.min(100), &cal);
    }
    t.priority = priority as u16;
    t.manual = st.manual && !is_sum;
    t.manual_text = texts;
    t.sync_manual_text();
    t.active = !st.inactive;
    t.on_timeline = st.on_timeline;
    t.hide_bar = st.hide_bar;
    t.rollup = st.rollup;
    t.notes = st.notes.clone();
    t.predecessors = links;
    t.resources = if is_sum { Vec::new() } else { st.res.clone() };
    // units typed for new assignments start them at those units; changed units follow the task type
    let typed: Vec<(u32, f64)> = st.units.iter().filter(|(u, _)| st.res.contains(u)).copied().collect();
    t.deadline = deadline;
    t.task_type = st.task_type;
    t.effort_driven = st.effort_driven;
    t.wbs = st.wbs.trim().to_string();
    if !is_sum {
        t.estimated = st.estimated || st.duration.contains('?');
        t.constraint_type = st.constraint_type;
        if t.constraint_date != cdate {
            t.constraint_dt = None;
        }
        t.constraint_date = cdate;
        let (m, unit, el) = dur;
        let m = if st.milestone { 0 } else if t.milestone && m == 0 { du.mpd } else { m };
        t.set_duration_minutes(m, unit, el, du.mpd);
        t.milestone = st.milestone;
        if finish != before_finish && start == before_start && finish >= start {
            if t.manual {
                let s = t.start_dt.filter(|x| x.date() == start).unwrap_or_else(|| cal.day_start(start));
                let m = cal.work_minutes(s, cal.day_end(finish)).max(if st.milestone { 0 } else { 1 });
                let unit = t.dur_unit;
                t.set_duration_minutes(m, unit, false, du.mpd);
            } else if !t.constraint_type.needs_date() {
                // a finish typed on an auto-scheduled task is a Finish No Earlier Than constraint
                t.constraint_type = ConstraintType::Fnet;
                t.constraint_date = Some(finish);
                t.constraint_dt = None;
            }
        }
        if start != before_start {
            if t.manual {
                t.start = start;
                t.start_dt = Some(cal.day_start(start));
            } else if !t.constraint_type.needs_date() || t.constraint_type == ConstraintType::Snet {
                t.set_snet(start);
            }
        }
    }
    let mut changed_units = Vec::new();
    for (u, v) in typed {
        let work = app.doc.resource_by_uid(u).map(|r| r.rtype == ResourceType::Work).unwrap_or(true);
        let units = if work { v / 100.0 } else { v };
        let t = &mut app.doc.tasks[st.idx];
        match t.assignments.iter_mut().find(|a| a.res_uid == u) {
            Some(a) if (a.units - units).abs() > 1e-9 => changed_units.push((u, units)),
            Some(_) => {}
            None => t.assignments.push(crate::model::Assignment { res_uid: u, units, work_min: -1, cost: 0.0 }),
        }
    }
    for (u, units) in changed_units {
        app.set_assignment_units(st.idx, u, units);
    }
    app.recalc();
    Ok(())
}
