//! Modal dialogs: Task/Resource/Project Information, Working Time, Assign Resources, etc.

use crate::i18n::{tr, trf};
use crate::app::App;
use crate::model::*;
use crate::theme;
use crate::util;
use eframe::egui::{self, RichText, Ui, Vec2};

pub use crate::taskinfo::{open_task_info, TaskInfoState};
use crate::fmt_dialogs::*;

#[derive(Clone, Debug)]
pub struct ProjectInfoState {
    pub start: String,
    pub status: String,
    pub title: String,
    pub error: Option<String>,
}

/// View > Zoom > Zoom...
#[derive(Clone, Debug)]
pub struct ZoomState {
    /// 0 = 1 week, 1 = 2 weeks, 2 = 1 month, 3 = 3 months, 4 = selected tasks, 5 = entire project, 6 = custom
    pub choice: u8,
    pub custom: String,
    /// 0 = days, 1 = weeks, 2 = months
    pub unit: u8,
}

#[derive(Clone, Debug)]
pub struct WorkingTimeState {
    pub name: String,
    pub days: [bool; 7],
    pub hpd: f64,
    pub holidays: Vec<(String, String, String)>,
    pub error: Option<String>,
    /// Add Thai holidays: include days only government offices observe
    pub thai_gov: bool,
    /// what the last Add Thai holidays did (added count, years without data)
    pub info: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ResInfoState {
    pub idx: usize,
    pub name: String,
    pub initials: String,
    pub group: String,
    pub rtype: ResourceType,
    pub material: String,
    pub max: f64,
    pub std: f64,
    pub ovt: f64,
    pub cpu: f64,
    pub accrue: Accrue,
    pub code: String,
    pub notes: String,
    pub email: String,
    /// 0 General, 1 Costs, 2 Notes
    pub tab: u8,
    pub error: Option<String>,
}

/// Project > Custom Fields: the definitions being edited (a working copy) and the selection.
#[derive(Clone, Debug)]
pub struct CustomFieldsState {
    pub resource: bool,
    pub kind: crate::custom::CfKind,
    pub n: u8,
    pub defs: Vec<crate::custom::CustomFieldDef>,
    pub rename: Option<String>,
    /// formula test result / error
    pub info: Option<String>,
    pub new_value: (String, String),
}

pub fn open_custom_fields(app: &mut App, field: Option<crate::custom::CustomField>) {
    let f = field.unwrap_or(crate::custom::CustomField::new(crate::custom::CfKind::Text, 1));
    app.dialog = Some(Dialog::CustomFields(Box::new(CustomFieldsState {
        resource: false,
        kind: f.kind,
        n: f.n,
        defs: app.doc.custom_defs.clone(),
        rename: None,
        info: None,
        new_value: Default::default(),
    })));
}

#[derive(Clone, Debug)]
pub struct SpellState {
    pub issues: Vec<crate::spell::Issue>,
    pub pos: usize,
    pub change_to: String,
    pub ignored: Vec<String>,
    pub changed: usize,
}

/// Project > Spelling: check every task name, task note and resource name.
pub fn open_spelling(app: &mut App) {
    app.commit_edit();
    match crate::spell::find_all(&app.doc) {
        Ok(issues) if issues.is_empty() => app.message = Some((crate::app::product().into(), "The spelling check is complete.".into())),
        Ok(issues) => {
            let change_to = issues[0].miss.suggestions.first().cloned().unwrap_or_else(|| issues[0].miss.word.clone());
            app.dialog = Some(Dialog::Spelling(Box::new(SpellState { issues, pos: 0, change_to, ignored: Vec::new(), changed: 0 })));
        }
        Err(e) => app.show_error(e),
    }
}

/// Task Dependency (double-click a link line): type and lag of one link, or delete it.
#[derive(Clone, Debug)]
pub struct DependencyState {
    pub succ_uid: u32,
    pub pred_uid: u32,
    /// 0 FS, 1 SS, 2 FF, 3 SF, 4 (None)
    pub kind: usize,
    pub lag: String,
    pub error: Option<String>,
}

const DEP_KINDS: [&str; 5] = ["Finish-to-Start (FS)", "Start-to-Start (SS)", "Finish-to-Finish (FF)", "Start-to-Finish (SF)", "(None)"];

pub fn open_dependency(app: &mut App, succ: usize, pred_uid: u32) {
    let Some(t) = app.doc.tasks.get(succ) else { return };
    let Some(l) = t.predecessors.iter().find(|l| l.pred_uid == pred_uid) else { return };
    let kind = match l.kind {
        LinkType::FS => 0,
        LinkType::SS => 1,
        LinkType::FF => 2,
        LinkType::SF => 3,
    };
    let du = app.dur_units();
    let thai = crate::i18n::is_thai();
    let lag = util::fmt_lag(l, du, &app.doc.options, thai);
    let lag = if lag.is_empty() { util::fmt_dur(0, DurUnit::Days, false, du, &app.doc.options, thai) } else { lag.trim_start_matches('+').to_string() };
    app.dialog = Some(Dialog::Dependency(DependencyState { succ_uid: t.uid, pred_uid, kind, lag, error: None }));
}

/// The prompt of a filter that asks for dates or a resource.
#[derive(Clone, Debug)]
pub struct FilterInputState {
    pub highlight: bool,
    pub kind: crate::app::TaskFilter,
    pub from: String,
    pub to: String,
    pub resource: u32,
    pub error: Option<String>,
}

/// Project > Set Baseline / Clear Baseline.
#[derive(Clone, Debug)]
pub struct BaselineState {
    pub clear: bool,
    pub number: u8,
    pub selected: bool,
    /// roll up: to all summary tasks / from subtasks into the selected summary tasks
    pub up: bool,
    pub down: bool,
    /// asking whether to overwrite a baseline already saved
    pub confirm: bool,
}

/// Task > Mark on Track > Update Tasks.
#[derive(Clone, Debug)]
pub struct UpdateTasksState {
    pub idx: Vec<usize>,
    /// % complete, actual duration, remaining duration, actual start, actual finish
    pub fields: [String; 5],
    pub orig: [String; 5],
    pub error: Option<String>,
}

/// Project > Update Project.
#[derive(Clone, Debug)]
pub struct UpdateProjectState {
    pub date: String,
    pub all_or_nothing: bool,
    pub selected: bool,
    pub error: Option<String>,
}

pub fn open_baseline(app: &mut App, clear: bool) {
    app.dialog = Some(Dialog::Baseline(BaselineState { clear, number: 0, selected: false, up: false, down: false, confirm: false }));
}

pub fn open_update_tasks(app: &mut App) {
    let idx: Vec<usize> = app.selected_tasks().into_iter().filter(|&i| !app.doc.tasks[i].is_summary).collect();
    let Some(&first) = idx.first() else { return };
    let fields = [
        app.doc.tasks[first].percent_complete.to_string(),
        app.cell_text(first, crate::app::TCol::ActualDuration),
        app.cell_text(first, crate::app::TCol::RemainingDuration),
        app.cell_text(first, crate::app::TCol::ActualStart),
        app.cell_text(first, crate::app::TCol::ActualFinish),
    ];
    app.dialog = Some(Dialog::UpdateTasks(UpdateTasksState { idx, orig: fields.clone(), fields, error: None }));
}

pub fn open_update_project(app: &mut App) {
    let d = app.doc.status_date.unwrap_or_else(|| chrono::Local::now().date_naive());
    app.dialog = Some(Dialog::UpdateProject(UpdateProjectState { date: app.fmt_date(d), all_or_nothing: false, selected: false, error: None }));
}

#[derive(Clone, Debug)]
pub enum Dialog {
    ConfirmSave,
    /// Project > Import BOQ
    Boq(crate::boq::BoqState),
    /// Project > Payment Schedule
    Payments(crate::payments::PayState),
    Baseline(BaselineState),
    FilterInput(FilterInputState),
    UpdateTasks(UpdateTasksState),
    UpdateProject(UpdateProjectState),
    Dependency(DependencyState),
    /// Task > Move > Custom: duration text, error
    MoveTasks(String, Option<String>),
    /// Project > Move Project: new start date text, move deadlines, error
    MoveProject(String, bool, Option<String>),
    TaskInfo(Box<TaskInfoState>),
    FormatBar(FormatBarState),
    BarStyles(BarStylesState),
    Layout(LayoutState),
    Gridlines(GridlinesState),
    TextStyles(TextStylesState),
    Font(FontState),
    Timescale(TimescaleState),
    FieldSettings(FieldSettingsState),
    Options(OptionsState),
    ProjectInfo(ProjectInfoState),
    WorkingTime(WorkingTimeState),
    ResourceInfo(ResInfoState),
    /// Project > WBS > Define Code: the mask being edited
    WbsCode(crate::wbs::WbsMask),
    /// Project > Custom Fields
    CustomFields(Box<CustomFieldsState>),
    /// Format > Drawing: the text of a text box (its index)
    DrawingText(usize, String),
    /// View > Macros: the macro list (selected index) / the name of a macro to record
    Macros(usize),
    RecordMacro(String),
    /// View > New Window: the view of the new window
    NewWindow(crate::app::View),
    /// Project > Links Between Projects
    /// Project > Links Between Projects: (tab 0 predecessors / 1 successors, selected row)
    ProjectLinks(usize, usize),
    /// Resource > Resource Pool > Share Resources: (own resources, pool file, sharer takes precedence)
    ShareResources(bool, String, bool),
    /// Project > Spelling: the words still to look at, where we are, the replacement typed
    Spelling(Box<SpellState>),
    /// Project > Manage Sprints: (uid, name, start text, finish text) rows and an error
    Sprints(Vec<(u32, String, String, String)>, Option<String>),
    /// Resource > Leveling Options: the options being edited, the range texts, an error
    LevelingOptions(crate::model::LevelingOptions, String, String, Option<String>),
    AssignResources,
    Find,
    Zoom(ZoomState),
    Overview,
    Inspect(usize),
    About,
    Shortcuts,
    /// open-source licences (the single-file program carries them inside)
    Licenses,
    /// Help > Check for Updates (what it shows comes from update::status)
    Update,
}

/// Licence texts of the fonts and libraries built into the program.
const LICENSES_TEXT: &str = concat!(
    include_str!("../assets/licenses/THIRD-PARTY-LICENSES.txt"),
    "\n\n----------------------------------------------------------------------\n\n",
    include_str!("../assets/licenses/NotoSans-NotoSansThai-OFL.txt")
);

pub fn open_zoom(app: &mut App) {
    // like Project, the dialog opens on "Custom" showing what is visible now
    let days = (app.chart_view.1 / app.px_per_day).round().max(1.0);
    app.dialog = Some(Dialog::Zoom(ZoomState { choice: 6, custom: format!("{days}"), unit: 0 }));
}

pub fn open_project_info(app: &mut App) {
    app.dialog = Some(Dialog::ProjectInfo(ProjectInfoState {
        start: app.fmt_date(app.doc.start),
        status: app.doc.status_date.map(|d| app.fmt_date(d)).unwrap_or_else(|| "NA".into()),
        title: app.doc.title.clone(),
        error: None,
    }));
}

pub fn open_working_time(app: &mut App) {
    let c = &app.doc.calendar;
    app.dialog = Some(Dialog::WorkingTime(WorkingTimeState {
        name: c.name.clone(),
        days: c.working_days,
        hpd: c.hours_per_day,
        holidays: c.holidays.iter().map(|h| (h.name.clone(), app.fmt_date(h.start), app.fmt_date(h.finish))).collect(),
        error: None,
        thai_gov: true,
        info: None,
    }));
}

pub fn open_resource_info(app: &mut App, idx: usize) {
    open_resource_info_tab(app, idx, 0);
}

/// Resource Information on a tab (0 General, 1 Costs, 2 Notes).
pub fn open_resource_info_tab(app: &mut App, idx: usize, tab: u8) {
    let r = app.doc.resources[idx].clone();
    app.dialog = Some(Dialog::ResourceInfo(ResInfoState {
        idx,
        name: r.name,
        initials: r.initials,
        group: r.group,
        rtype: r.rtype,
        material: r.material_label,
        max: r.max_units,
        std: r.std_rate,
        ovt: r.ovt_rate,
        cpu: r.cost_per_use,
        accrue: r.accrue_at,
        code: r.code,
        notes: r.notes,
        email: r.email,
        tab,
        error: None,
    }));
}

/// Update Tasks OK: the changed fields go to every selected task, in the standard order (actual
/// finish, actual start, % complete, actual duration, remaining duration).
fn apply_update_tasks(app: &mut App, st: &UpdateTasksState) -> Result<(), String> {
    let changed = |k: usize| st.fields[k].trim() != st.orig[k].trim();
    let cal = app.doc.calendar.clone();
    let du = app.dur_units();
    let na = |s: &str| s.trim().is_empty() || s.trim().eq_ignore_ascii_case("NA");
    let date = |app: &App, s: &str| -> Result<Option<chrono::NaiveDate>, String> {
        if na(s) {
            Ok(None)
        } else {
            app.parse_date(s).map(Some).ok_or_else(|| "The date you entered isn't supported.".to_string())
        }
    };
    let pct = if changed(0) {
        let v: f64 = st.fields[0].trim().trim_end_matches('%').trim().parse().map_err(|_| "Enter a percentage between 0 and 100.".to_string())?;
        if !(0.0..=100.0).contains(&v) {
            return Err("Enter a percentage between 0 and 100.".into());
        }
        Some(v.round() as u8)
    } else {
        None
    };
    let dur = |k: usize, unit: DurUnit| -> Result<Option<i64>, String> {
        if !changed(k) {
            return Ok(None);
        }
        util::parse_dur(&st.fields[k], du, unit).map(|x| Some(x.0)).ok_or_else(|| "The duration is not valid.".to_string())
    };
    let astart = if changed(3) { Some(date(&*app, &st.fields[3])?) } else { None };
    let afinish = if changed(4) { Some(date(&*app, &st.fields[4])?) } else { None };
    if let (Some(Some(a)), Some(Some(f))) = (astart, afinish) {
        if f < a {
            return Err("The actual finish date can't be earlier than the actual start date.".into());
        }
    }
    let unit = app.doc.tasks[st.idx[0]].dur_unit;
    let (adur, rdur) = (dur(1, unit)?, dur(2, unit)?);
    app.checkpoint();
    for &i in &st.idx {
        let t = &mut app.doc.tasks[i];
        if let Some(a) = astart {
            t.set_actual_start(a.map(|d| cal.day_start(d)));
        }
        if let Some(f) = afinish {
            t.set_actual_finish(f.map(|d| cal.day_end(d)), &cal);
        }
        if let Some(p) = pct {
            t.set_percent(p, &cal);
        } else if let Some(m) = adur {
            t.set_actual_minutes(m, &cal);
        } else if let Some(m) = rdur {
            t.set_remaining_minutes(m, &cal);
        }
    }
    app.recalc();
    Ok(())
}

pub(crate) fn buttons(ui: &mut Ui, ok_label: &str) -> (bool, bool) {
    let mut ok = false;
    let mut cancel = false;
    ui.add_space(8.0);
    ui.separator();
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        if ui.add(egui::Button::new(tr("Cancel")).min_size(Vec2::new(80.0, 26.0))).clicked() {
            cancel = true;
        }
        if ui.add(egui::Button::new(RichText::new(tr(ok_label)).strong()).min_size(Vec2::new(80.0, 26.0))).clicked() {
            ok = true;
        }
    });
    (ok, cancel)
}

pub(crate) fn error_line(ui: &mut Ui, e: &Option<String>) {
    if let Some(e) = e {
        ui.colored_label(egui::Color32::from_rgb(0xC0, 0x39, 0x2B), tr(e));
    }
}

/// Render the active dialog and the message box.
pub fn show(app: &mut App, ctx: &egui::Context) {
    if let Some((title, msg)) = app.message.clone() {
        let m = crate::theme::modal(egui::Id::unique("msgbox")).show(ctx, |ui| {
            crate::theme::dialog_spacing(ui);
            ui.set_width(380.0);
            ui.label(RichText::new(title).strong());
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                theme::paint_icon(ui.painter(), egui::Rect::from_min_size(ui.cursor().min, Vec2::splat(28.0)), theme::Icon::Info, true);
                ui.add_space(36.0);
                ui.label(tr(&msg));
            });
            ui.add_space(10.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.add(egui::Button::new(tr("OK")).min_size(Vec2::new(80.0, 26.0))).clicked()).inner
        });
        if m.inner || m.should_close() || ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            app.message = None;
        }
        return;
    }
    let Some(dialog) = app.dialog.take() else { return };
    let mut keep = true;
    let mut next: Option<Dialog> = None;
    match dialog {
        Dialog::Boq(mut st) => match crate::boq::dialog(app, ctx, &mut st) {
            Some(_) => keep = false,
            None => next = Some(Dialog::Boq(st)),
        },
        Dialog::Payments(mut st) => match crate::payments::dialog(app, ctx, &mut st) {
            Some(_) => keep = false,
            None => next = Some(Dialog::Payments(st)),
        },
        Dialog::FilterInput(mut st) => {
            let (mut ok, mut cancel) = (false, false);
            let resources: Vec<(u32, String)> = app.doc.resources.iter().map(|r| (r.uid, crate::thai::shape(&r.name).to_string())).collect();
            crate::theme::modal(egui::Id::unique("filter_input")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(380.0);
                ui.label(RichText::new(tr(st.kind.label().trim_end_matches("..."))).strong().size(14.0));
                ui.add_space(8.0);
                if st.kind == crate::app::TaskFilter::DateRange {
                    ui.label(tr("Show tasks that start or finish after:"));
                    ui.add(egui::TextEdit::singleline(&mut st.from).desired_width(140.0));
                    ui.add_space(4.0);
                    ui.label(tr("And before:"));
                    ui.add(egui::TextEdit::singleline(&mut st.to).desired_width(140.0));
                } else {
                    ui.label(tr("Show tasks using resource:"));
                    let cur = resources.iter().find(|r| r.0 == st.resource).map(|r| r.1.clone()).unwrap_or_default();
                    egui::ComboBox::from_id_salt("filter_res").width(240.0).selected_text(cur).show_ui(ui, |ui| {
                        for (u, n) in &resources {
                            ui.selectable_value(&mut st.resource, *u, n);
                        }
                    });
                }
                error_line(ui, &st.error);
                let (o, c) = buttons(ui, "OK");
                ok = o;
                cancel = c;
            });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancel = true;
            }
            if ok {
                let spec = if st.kind == crate::app::TaskFilter::DateRange {
                    match (app.parse_date(&st.from), app.parse_date(&st.to)) {
                        (Some(a), Some(b)) => Some(crate::app::FilterSpec { kind: st.kind, range: Some((a.min(b), a.max(b))), resource: None }),
                        _ => None,
                    }
                } else if resources.iter().any(|r| r.0 == st.resource) {
                    Some(crate::app::FilterSpec { kind: st.kind, range: None, resource: Some(st.resource) })
                } else {
                    None
                };
                match spec {
                    Some(s) => {
                        if st.highlight {
                            app.highlight = s;
                        } else {
                            app.filter = s;
                            app.sel_row = 0;
                            app.sel_anchor = 0;
                            app.scroll_y = 0.0;
                        }
                        keep = false;
                    }
                    None => {
                        st.error = Some(if st.kind == crate::app::TaskFilter::DateRange { "Enter valid dates." } else { "Choose a resource." }.into());
                        next = Some(Dialog::FilterInput(st));
                    }
                }
            } else if cancel {
                keep = false;
            } else {
                next = Some(Dialog::FilterInput(st));
            }
        }
        Dialog::Baseline(mut st) => {
            let (mut ok, mut cancel) = (false, false);
            let labels: Vec<String> = (0..=10u8).map(|n| app.baseline_label(n)).collect();
            let used = app.doc.tasks.iter().any(|t| t.baseline(st.number).is_some());
            crate::theme::modal(egui::Id::unique("set_baseline")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(460.0);
                ui.label(RichText::new(tr(if st.clear { "Clear Baseline" } else { "Set Baseline" })).strong().size(14.0));
                ui.add_space(8.0);
                if st.confirm {
                    let name = if st.number == 0 { tr("Baseline").to_string() } else { crate::i18n::trf("Baseline {}", &[&st.number.to_string()]) };
                    ui.label(crate::i18n::trf("The '{}' is already being used. Are you sure you want to overwrite the data?", &[&name]));
                    ui.add_space(10.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new(tr("No")).min_size(Vec2::new(80.0, 26.0))).clicked() {
                            st.confirm = false;
                        }
                        if ui.add(egui::Button::new(RichText::new(tr("Yes")).strong()).min_size(Vec2::new(80.0, 26.0))).clicked() {
                            ok = true;
                        }
                    });
                    return;
                }
                ui.horizontal(|ui| {
                    let _ = ui.radio(true, tr(if st.clear { "Clear baseline plan" } else { "Set baseline" }));
                    egui::ComboBox::from_id_salt("bl_number").width(260.0).selected_text(labels[st.number as usize].clone()).show_ui(ui, |ui| {
                        for (n, l) in labels.iter().enumerate() {
                            ui.selectable_value(&mut st.number, n as u8, l);
                        }
                    });
                });
                ui.add_enabled_ui(false, |ui| {
                    let _ = ui.radio(false, tr(if st.clear { "Clear interim plan" } else { "Set interim plan" }));
                });
                ui.add_space(6.0);
                ui.label(tr("For:"));
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    ui.radio_value(&mut st.selected, false, tr("Entire project"));
                });
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    ui.radio_value(&mut st.selected, true, tr("Selected tasks"));
                });
                if !st.clear {
                    ui.add_enabled_ui(st.selected, |ui| {
                        ui.horizontal(|ui| {
                            ui.add_space(34.0);
                            ui.label(tr("Roll up baselines:"));
                        });
                        ui.horizontal(|ui| {
                            ui.add_space(34.0);
                            ui.checkbox(&mut st.up, tr("To all summary tasks"));
                        });
                        ui.horizontal(|ui| {
                            ui.add_space(34.0);
                            ui.checkbox(&mut st.down, tr("From subtasks into selected summary task(s)"));
                        });
                    });
                }
                let (o, c) = buttons(ui, "OK");
                if o {
                    if !st.clear && used {
                        st.confirm = true;
                    } else {
                        ok = true;
                    }
                }
                cancel = c;
            });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancel = true;
            }
            if ok {
                if st.clear {
                    app.clear_baseline(st.number, st.selected);
                } else {
                    app.set_baseline(st.number, st.selected, st.up, st.down);
                }
                keep = false;
            } else if cancel {
                keep = false;
            } else {
                next = Some(Dialog::Baseline(st));
            }
        }
        Dialog::UpdateTasks(mut st) => {
            let (mut ok, mut cancel) = (false, false);
            let first = st.idx.first().copied().unwrap_or(0);
            if first >= app.doc.tasks.len() {
                app.dialog = None;
                return;
            }
            let name = if st.idx.len() == 1 { crate::thai::shape(&app.doc.tasks[first].name).to_string() } else { String::new() };
            let (dur, cur_s, cur_f) = (app.cell_text(first, crate::app::TCol::Duration), app.cell_text(first, crate::app::TCol::Start), app.cell_text(first, crate::app::TCol::Finish));
            crate::theme::modal(egui::Id::unique("update_tasks")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(520.0);
                ui.label(RichText::new(tr("Update Tasks")).strong().size(14.0));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(tr("Name:"));
                    ui.label(&name);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(if st.idx.len() == 1 { dur.clone() } else { String::new() });
                        ui.label(tr("Duration:"));
                    });
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(tr("% Complete:"));
                    ui.add(egui::TextEdit::singleline(&mut st.fields[0]).desired_width(60.0));
                    ui.add_space(10.0);
                    ui.label(tr("Actual dur:"));
                    ui.add(egui::TextEdit::singleline(&mut st.fields[1]).desired_width(80.0));
                    ui.add_space(10.0);
                    ui.label(tr("Remaining dur:"));
                    ui.add(egui::TextEdit::singleline(&mut st.fields[2]).desired_width(80.0));
                });
                ui.add_space(8.0);
                ui.columns(2, |c| {
                    c[0].label(RichText::new(tr("Actual")).strong());
                    egui::Grid::new("ut_actual").num_columns(2).spacing([8.0, 6.0]).show(&mut c[0], |ui| {
                        ui.label(tr("Start:"));
                        ui.add(egui::TextEdit::singleline(&mut st.fields[3]).desired_width(130.0));
                        ui.end_row();
                        ui.label(tr("Finish:"));
                        ui.add(egui::TextEdit::singleline(&mut st.fields[4]).desired_width(130.0));
                        ui.end_row();
                    });
                    c[1].label(RichText::new(tr("Current")).strong());
                    egui::Grid::new("ut_current").num_columns(2).spacing([8.0, 6.0]).show(&mut c[1], |ui| {
                        ui.label(tr("Start:"));
                        ui.label(if st.idx.len() == 1 { cur_s.clone() } else { String::new() });
                        ui.end_row();
                        ui.label(tr("Finish:"));
                        ui.label(if st.idx.len() == 1 { cur_f.clone() } else { String::new() });
                        ui.end_row();
                    });
                });
                error_line(ui, &st.error);
                let (o, c) = buttons(ui, "OK");
                ok = o;
                cancel = c;
            });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancel = true;
            }
            if ok {
                match apply_update_tasks(app, &st) {
                    Ok(()) => keep = false,
                    Err(e) => {
                        st.error = Some(e);
                        next = Some(Dialog::UpdateTasks(st));
                    }
                }
            } else if cancel {
                keep = false;
            } else {
                next = Some(Dialog::UpdateTasks(st));
            }
        }
        Dialog::UpdateProject(mut st) => {
            let (mut ok, mut cancel) = (false, false);
            crate::theme::modal(egui::Id::unique("update_project")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(500.0);
                ui.label(RichText::new(tr("Update Project")).strong().size(14.0));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let _ = ui.radio(true, tr("Update work as complete through:"));
                    ui.add(egui::TextEdit::singleline(&mut st.date).desired_width(130.0));
                });
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    ui.radio_value(&mut st.all_or_nothing, false, tr("Set 0% - 100% complete"));
                });
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    ui.radio_value(&mut st.all_or_nothing, true, tr("Set 0% or 100% complete only"));
                });
                ui.add_enabled_ui(false, |ui| {
                    let _ = ui.radio(false, tr("Reschedule uncompleted work to start after:"));
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(tr("For:"));
                    ui.radio_value(&mut st.selected, false, tr("Entire project"));
                    ui.radio_value(&mut st.selected, true, tr("Selected tasks"));
                });
                error_line(ui, &st.error);
                let (o, c) = buttons(ui, "OK");
                ok = o;
                cancel = c;
            });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancel = true;
            }
            if ok {
                match app.parse_date(&st.date) {
                    Some(d) => {
                        let through = app.doc.calendar.day_end(d);
                        let idx: Vec<usize> = if st.selected { app.selected_tasks() } else { (0..app.doc.tasks.len()).collect() };
                        app.checkpoint();
                        app.update_progress(&idx, through, st.all_or_nothing);
                        app.recalc();
                        keep = false;
                    }
                    None => {
                        st.error = Some("The date you entered isn't supported.".into());
                        next = Some(Dialog::UpdateProject(st));
                    }
                }
            } else if cancel {
                keep = false;
            } else {
                next = Some(Dialog::UpdateProject(st));
            }
        }
        Dialog::MoveProject(mut txt, mut deadlines, err) => {
            let (mut ok, mut cancel) = (false, false);
            let current = app.fmt_date(app.doc.start);
            crate::theme::modal(egui::Id::unique("move_project")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(380.0);
                ui.label(RichText::new(tr("Move Project")).strong().size(14.0));
                ui.add_space(8.0);
                egui::Grid::new("move_project_grid").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
                    ui.label(tr("Current project start date:"));
                    ui.label(&current);
                    ui.end_row();
                    ui.label(tr("New project start date:"));
                    ui.add(egui::TextEdit::singleline(&mut txt).desired_width(140.0));
                    ui.end_row();
                });
                ui.checkbox(&mut deadlines, tr("Move deadlines"));
                error_line(ui, &err);
                let (o, c) = buttons(ui, "OK");
                ok = o;
                cancel = c;
            });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancel = true;
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                ok = true;
            }
            if ok {
                match app.parse_date(txt.trim()) {
                    Some(d) => {
                        app.move_project(d, deadlines);
                        keep = false;
                    }
                    None => next = Some(Dialog::MoveProject(txt, deadlines, Some("The date is not valid.".into()))),
                }
            } else if cancel {
                keep = false;
            } else {
                next = Some(Dialog::MoveProject(txt, deadlines, err));
            }
        }
        Dialog::CustomFields(mut st) => {
            use crate::custom::{CfKind, CustomField, CustomFieldDef};
            let (mut ok, mut cancel) = (false, false);
            crate::theme::modal(egui::Id::unique("custom_fields")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(560.0);
                ui.label(RichText::new(tr("Custom Fields")).strong().size(14.0));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(tr("Field"));
                    ui.radio_value(&mut st.resource, false, tr("Task"));
                    ui.radio_value(&mut st.resource, true, tr("Resource"));
                    ui.add_space(20.0);
                    ui.label(tr("Type:"));
                    egui::ComboBox::from_id_salt("cf_kind").selected_text(tr(st.kind.label())).show_ui(ui, |ui| {
                        for k in CfKind::ALL {
                            if ui.selectable_value(&mut st.kind, k, tr(k.label())).clicked() {
                                st.n = 1;
                            }
                        }
                    });
                });
                let resource = st.resource;
                let cur = CustomField::new(st.kind, st.n.clamp(1, st.kind.count()));
                // the fields of this type, with their aliases
                egui::ScrollArea::vertical().id_salt("cf_list").max_height(200.0).show(ui, |ui| {
                    for n in 1..=st.kind.count() {
                        let f = CustomField::new(st.kind, n);
                        let alias = st.defs.iter().find(|d| d.field == f && d.resource == resource).map(|d| d.alias.clone()).unwrap_or_default();
                        let label = if alias.is_empty() { f.name().to_string() } else { format!("{} ({})", f.name(), alias) };
                        if ui.selectable_label(cur == f, crate::thai::shape(&label).to_string()).clicked() {
                            st.n = n;
                            st.rename = None;
                            st.info = None;
                        }
                    }
                });
                let k = match st.defs.iter().position(|d| d.field == cur && d.resource == resource) {
                    Some(k) => k,
                    None => {
                        st.defs.push(CustomFieldDef::new(cur, resource));
                        st.defs.len() - 1
                    }
                };
                ui.horizontal(|ui| {
                    if ui.button(tr("Rename...")).clicked() {
                        st.rename = Some(st.defs[k].alias.clone());
                    }
                    if ui.button(tr("Delete")).clicked() {
                        st.defs[k] = CustomFieldDef::new(cur, resource);
                    }
                });
                if let Some(mut name) = st.rename.clone() {
                    ui.horizontal(|ui| {
                        ui.label(format!("{} {}:", tr("New name for"), cur.name()));
                        ui.add(egui::TextEdit::singleline(&mut name).desired_width(200.0));
                        if ui.button(tr("OK")).clicked() {
                            // MPP: a field's name cannot be another field's (or another alias)
                            let n = name.trim().to_string();
                            let taken = !n.is_empty()
                                && (crate::cfdefs::field_by_name(&n, resource).is_some_and(|id| id != cur.field_id(resource))
                                    || st.defs.iter().enumerate().any(|(j, d)| j != k && d.resource == resource && d.alias.trim().eq_ignore_ascii_case(&n)));
                            if taken {
                                app.message = Some((crate::app::product().into(), format!("{} '{}'", tr("The name is already used by another field:"), n)));
                            } else {
                                st.defs[k].alias = n;
                            }
                            st.rename = None;
                        } else {
                            st.rename = Some(name);
                        }
                    });
                }
                ui.separator();
                ui.label(RichText::new(tr("Custom attributes")).strong());
                // lookup table
                ui.label(tr("Lookup table (value, description):"));
                let mut remove: Option<usize> = None;
                egui::Grid::new("cf_values").num_columns(3).spacing([6.0, 3.0]).show(ui, |ui| {
                    for (j, (v, d)) in st.defs[k].values.iter_mut().enumerate() {
                        ui.add_sized([160.0, 20.0], egui::TextEdit::singleline(v));
                        ui.add_sized([240.0, 20.0], egui::TextEdit::singleline(d));
                        if ui.small_button("x").clicked() {
                            remove = Some(j);
                        }
                        ui.end_row();
                    }
                    ui.add_sized([160.0, 20.0], egui::TextEdit::singleline(&mut st.new_value.0).hint_text(tr("Value")));
                    ui.add_sized([240.0, 20.0], egui::TextEdit::singleline(&mut st.new_value.1).hint_text(tr("Description")));
                    if ui.small_button("+").clicked() && !st.new_value.0.trim().is_empty() {
                        let nv = std::mem::take(&mut st.new_value);
                        st.defs[k].values.push((nv.0.trim().to_string(), nv.1.trim().to_string()));
                    }
                    ui.end_row();
                });
                if let Some(j) = remove {
                    st.defs[k].values.remove(j);
                }
                ui.checkbox(&mut st.defs[k].restrict, tr("Restrict field to items in the value list"));
                ui.add_space(4.0);
                // formula
                ui.label(tr("Formula:"));
                ui.add(egui::TextEdit::multiline(&mut st.defs[k].formula).desired_width(520.0).desired_rows(2).hint_text("IIf([% Complete]=100, \"Done\", \"Open\")"));
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("cf_insert_field").selected_text(tr("Insert field")).show_ui(ui, |ui| {
                        for name in ["Name", "Duration", "Work", "Cost", "% Complete", "Start", "Finish", "Baseline Start", "Baseline Finish", "Baseline Cost", "Total Slack", "Critical", "Priority", "Resource Names", "Status Date"] {
                            if ui.button(tr(name)).clicked() {
                                st.defs[k].formula.push_str(&format!("[{name}]"));
                            }
                        }
                    });
                    if ui.button(tr("Test")).clicked() {
                        st.info = Some(match app.current_task().map(|i| crate::custom::eval(&app.doc, &app.doc.tasks[i], &st.defs[k].formula, cur.kind)) {
                            Some(Ok(v)) => format!("= {}", v.map(|v| crate::custom::xml_value(&v)).unwrap_or_else(|| "NA".into())),
                            Some(Err(e)) => e,
                            None => tr("Select a task to test the formula on.").to_string(),
                        });
                    }
                });
                if let Some(i) = &st.info {
                    ui.label(RichText::new(i.as_str()).color(theme::TEXT_DIM));
                }
                let (a, b) = buttons(ui, "OK");
                ok = a;
                cancel = b;
            });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancel = true;
            }
            if ok {
                app.checkpoint();
                st.defs.retain(|d| !d.alias.trim().is_empty() || !d.formula.trim().is_empty() || !d.values.is_empty() || d.restrict);
                app.doc.custom_defs = st.defs.clone();
                app.dirty = true;
                app.recalc();
                keep = false;
            } else if cancel {
                keep = false;
            } else {
                next = Some(Dialog::CustomFields(st));
            }
        }
        Dialog::Macros(mut sel) => {
            let (mut close, mut run, mut delete) = (false, false, false);
            crate::theme::modal(egui::Id::unique("macros")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(380.0);
                ui.label(RichText::new(tr("Macros")).strong().size(14.0));
                ui.add_space(4.0);
                if app.macros.is_empty() {
                    ui.label(RichText::new(tr("No macros yet: View > Macros > Record Macro records the commands you use.")).color(theme::TEXT_DIM));
                }
                for (k, (name, cmds)) in app.macros.iter().enumerate() {
                    if ui.selectable_label(sel == k, format!("{}  ({} {})", crate::thai::shape(name), cmds.len(), tr("steps"))).clicked() {
                        sel = k;
                    }
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.add_enabled(sel < app.macros.len(), egui::Button::new(tr("Run"))).clicked() {
                        run = true;
                    }
                    if ui.add_enabled(sel < app.macros.len(), egui::Button::new(tr("Delete"))).clicked() {
                        delete = true;
                    }
                    if ui.button(tr("Close")).clicked() {
                        close = true;
                    }
                });
            });
            if run {
                let steps = app.macros[sel].1.clone();
                keep = false;
                for c in steps {
                    crate::ribbon::run(app, ctx, c);
                }
            } else if delete {
                app.macros.remove(sel);
                crate::app::save_macros(&app.macros);
                next = Some(Dialog::Macros(0));
            } else if close || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                keep = false;
            } else {
                next = Some(Dialog::Macros(sel));
            }
        }
        Dialog::NewWindow(mut view) => {
            let (mut ok, mut cancel) = (false, false);
            let name = app.doc_name();
            crate::theme::modal(egui::Id::unique("new_window")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(340.0);
                ui.label(RichText::new(tr("New Window")).strong().size(14.0));
                ui.add_space(4.0);
                ui.label(tr("Projects:"));
                egui::Frame::new().stroke(egui::Stroke::new(1.0, theme::RIBBON_BORDER)).inner_margin(4.0).show(ui, |ui| {
                    ui.set_width(320.0);
                    let _ = ui.selectable_label(true, crate::thai::shape(&name).to_string());
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(tr("View:"));
                    let views = [
                        (crate::app::View::Gantt, "Gantt Chart"),
                        (crate::app::View::TrackingGantt, "Tracking Gantt"),
                        (crate::app::View::TaskUsage, "Task Usage"),
                        (crate::app::View::TaskSheet, "Task Sheet"),
                        (crate::app::View::NetworkDiagram, "Network Diagram"),
                        (crate::app::View::Calendar, "Calendar"),
                        (crate::app::View::TaskBoard, "Task Board"),
                        (crate::app::View::ResourceSheet, "Resource Sheet"),
                        (crate::app::View::ResourceUsage, "Resource Usage"),
                    ];
                    let cur = views.iter().find(|v| v.0 == view).map(|v| v.1).unwrap_or("Gantt Chart");
                    egui::ComboBox::from_id_salt("new_window_view").width(200.0).selected_text(tr(cur)).show_ui(ui, |ui| {
                        for (v, l) in views {
                            ui.selectable_value(&mut view, v, tr(l));
                        }
                    });
                });
                let (a, b) = buttons(ui, "OK");
                ok = a;
                cancel = b;
            });
            if ok {
                app.new_window(view);
                keep = false;
            } else if cancel || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                keep = false;
            } else {
                next = Some(Dialog::NewWindow(view));
            }
        }
        Dialog::RecordMacro(mut name) => {
            let (mut ok, mut cancel) = (false, false);
            crate::theme::modal(egui::Id::unique("record_macro")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(340.0);
                ui.label(RichText::new(tr("Record Macro")).strong().size(14.0));
                ui.horizontal(|ui| {
                    ui.label(tr("Macro name:"));
                    ui.add(egui::TextEdit::singleline(&mut name).desired_width(200.0));
                });
                let (a, b) = buttons(ui, "OK");
                ok = a;
                cancel = b;
            });
            if ok && !name.trim().is_empty() {
                app.recording = Some((name.trim().to_string(), Vec::new()));
                app.status = crate::i18n::tr("Recording a macro: the commands you use are recorded until View > Macros > Stop Recording").into();
                keep = false;
            } else if cancel || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                keep = false;
            } else {
                next = Some(Dialog::RecordMacro(name));
            }
        }
        Dialog::Spelling(mut st) => {
            // skip what was ignored
            while st.pos < st.issues.len() && st.ignored.contains(&st.issues[st.pos].miss.word) {
                st.pos += 1;
            }
            if st.pos >= st.issues.len() {
                app.message = Some((crate::app::product().into(), "The spelling check is complete.".into()));
                if st.changed > 0 {
                    app.recalc();
                }
                return;
            }
            let cur = st.issues[st.pos].clone();
            let (mut ignore, mut ignore_all, mut change, mut change_all, mut cancel) = (false, false, false, false, false);
            crate::theme::modal(egui::Id::unique("spelling")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(460.0);
                ui.label(RichText::new(tr("Spelling")).strong().size(14.0));
                ui.add_space(4.0);
                let field = match cur.field {
                    0 => tr("Task Name"),
                    1 => tr("Notes"),
                    _ => tr("Resource Name"),
                };
                let ctx_text = crate::spell::text_of(&app.doc, &cur).unwrap_or("").chars().take(80).collect::<String>();
                ui.label(format!("{field}: {}", crate::thai::shape(&ctx_text)));
                ui.horizontal(|ui| {
                    ui.label(tr("Not in Dictionary:"));
                    ui.label(RichText::new(crate::thai::shape(&cur.miss.word).to_string()).strong().color(egui::Color32::from_rgb(0xC0, 0x39, 0x2B)));
                });
                ui.horizontal(|ui| {
                    ui.label(tr("Change To:"));
                    ui.add(egui::TextEdit::singleline(&mut st.change_to).desired_width(220.0));
                });
                ui.label(tr("Suggestions:"));
                egui::ScrollArea::vertical().max_height(110.0).show(ui, |ui| {
                    for s in &cur.miss.suggestions {
                        if ui.selectable_label(st.change_to == *s, crate::thai::shape(s).to_string()).clicked() {
                            st.change_to = s.clone();
                        }
                    }
                    if cur.miss.suggestions.is_empty() {
                        ui.label(RichText::new(tr("(no suggestions)")).color(theme::TEXT_DIM));
                    }
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ignore = ui.button(tr("Ignore")).clicked();
                    ignore_all = ui.button(tr("Ignore All")).clicked();
                    change = ui.button(tr("Change")).clicked();
                    change_all = ui.button(tr("Change All")).clicked();
                    cancel = ui.button(tr("Cancel")).clicked();
                });
            });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancel = true;
            }
            let advance = |st: &mut SpellState| {
                st.pos += 1;
                if let Some(n) = st.issues.get(st.pos) {
                    st.change_to = n.miss.suggestions.first().cloned().unwrap_or_else(|| n.miss.word.clone());
                }
            };
            if cancel {
                if st.changed > 0 {
                    app.recalc();
                }
                keep = false;
            } else if ignore {
                advance(&mut st);
                next = Some(Dialog::Spelling(st));
            } else if ignore_all {
                st.ignored.push(cur.miss.word.clone());
                advance(&mut st);
                next = Some(Dialog::Spelling(st));
            } else if change || change_all {
                if st.changed == 0 {
                    app.checkpoint();
                }
                let with = st.change_to.clone();
                let targets: Vec<usize> = if change_all { (st.pos..st.issues.len()).filter(|&k| st.issues[k].miss.word == cur.miss.word).collect() } else { vec![st.pos] };
                for k in targets {
                    let is = st.issues[k].clone();
                    if crate::spell::replace(&mut app.doc, &is, &with) {
                        st.changed += 1;
                        // later words in the same text move with the new length
                        let delta = with.chars().count() as isize - is.miss.len as isize;
                        for other in st.issues.iter_mut().skip(k + 1) {
                            if other.field == is.field && other.index == is.index && other.miss.start > is.miss.start {
                                other.miss.start = (other.miss.start as isize + delta).max(0) as usize;
                            }
                        }
                    }
                }
                if change_all {
                    st.ignored.push(cur.miss.word.clone());
                }
                app.dirty = true;
                advance(&mut st);
                next = Some(Dialog::Spelling(st));
            } else {
                next = Some(Dialog::Spelling(st));
            }
        }
        Dialog::ShareResources(mut own, mut path, mut sharer_first) => {
            let (mut ok, mut cancel, mut browse) = (false, false, false);
            let name = app.doc_name();
            crate::theme::modal(egui::Id::unique("share_resources")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(420.0);
                ui.label(RichText::new(tr("Share Resources")).strong().size(14.0));
                ui.add_space(4.0);
                ui.label(format!("{} '{}'", tr("Resources for"), crate::thai::shape(&name)));
                ui.radio_value(&mut own, true, tr("Use own resources"));
                ui.radio_value(&mut own, false, tr("Use resources (requires resource pool)"));
                ui.add_enabled_ui(!own, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(20.0);
                        ui.label(tr("From:"));
                        ui.add(egui::TextEdit::singleline(&mut path).desired_width(250.0));
                        browse = ui.button(tr("Browse...")).clicked();
                    });
                    ui.add_space(4.0);
                    ui.label(tr("On conflict with calendar or resource information"));
                    ui.horizontal(|ui| {
                        ui.add_space(20.0);
                        ui.radio_value(&mut sharer_first, false, tr("Pool takes precedence"));
                        ui.radio_value(&mut sharer_first, true, tr("Sharer takes precedence"));
                    });
                });
                let (a, b) = buttons(ui, "OK");
                ok = a;
                cancel = b;
            });
            if browse {
                if let Some(p) = rfd::FileDialog::new().set_title(tr("Share Resources").to_string()).add_filter("Project (*.mpp; *.xml; *.neramit)", &["mpp", "xml", "neramit"]).pick_file() {
                    path = p.display().to_string();
                }
                next = Some(Dialog::ShareResources(own, path, sharer_first));
            } else if ok {
                if own {
                    // MPP: the resources this project uses stay in it as its own
                    app.checkpoint();
                    app.doc.resource_pool = None;
                    app.dirty = true;
                    keep = false;
                } else if path.trim().is_empty() || !std::path::Path::new(path.trim()).exists() {
                    app.message = Some((tr("Share Resources").into(), tr("Choose the resource pool file.").into()));
                    next = Some(Dialog::ShareResources(own, path, sharer_first));
                } else {
                    app.checkpoint();
                    app.doc.resource_pool = Some(crate::model::ResourcePool { path: path.trim().to_string(), sharer_first });
                    app.dirty = true;
                    match app.refresh_pool() {
                        Ok((a, u)) => app.status = crate::i18n::trf("Resources refreshed from the pool ({} added, {} updated)", &[&a.to_string(), &u.to_string()]),
                        Err(e) => app.show_error(e),
                    }
                    keep = false;
                }
            } else if cancel || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                keep = false;
            } else {
                next = Some(Dialog::ShareResources(own, path, sharer_first));
            }
        }
        Dialog::ProjectLinks(mut tab, mut sel) => {
            let rows = app.external_links();
            let (mut close, mut accept, mut accept_all, mut delete) = (false, false, false, false);
            let fmt_dt = |d: Option<chrono::NaiveDateTime>| d.map(|d| app.fmt_date(d.date())).unwrap_or_default();
            crate::theme::modal(egui::Id::unique("project_links")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(720.0);
                ui.label(RichText::new(format!("{} {}", tr("Links Between Projects in"), crate::thai::shape(&app.doc_name()))).strong().size(14.0));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut tab, 0, tr("External Predecessors"));
                    ui.selectable_value(&mut tab, 1, tr("External Successors"));
                });
                let shown: Vec<&crate::app::ExtLinkRow> = rows.iter().filter(|r| r.predecessor == (tab == 0)).collect();
                if sel >= shown.len() {
                    sel = 0;
                }
                if shown.is_empty() {
                    ui.label(RichText::new(tr("This project has no links to tasks in other projects.")).color(theme::TEXT_DIM));
                } else {
                    egui::Grid::new("ext_links").num_columns(7).striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
                        for h in ["ID", "Task", "Path", "Type", "Date", "% Comp.", "Differences"] {
                            ui.label(RichText::new(tr(h)).strong());
                        }
                        ui.end_row();
                        let ids = app.uid_to_id();
                        for (k, r) in shown.iter().enumerate() {
                            let g = app.doc.tasks.iter().find(|t| t.uid == r.ghost);
                            let local_id = ids.get(&r.local).map(|i| i.to_string()).unwrap_or_default();
                            let x = g.and_then(|g| g.external.as_ref());
                            if ui.selectable_label(sel == k, local_id).clicked() {
                                sel = k;
                            }
                            ui.label(crate::thai::shape(&g.map(|g| g.name.clone()).unwrap_or_default()).to_string());
                            ui.label(x.map(|x| format!("{}\\{}", x.path, x.id)).unwrap_or_default());
                            ui.label(format!("{:?}", r.kind));
                            ui.label(g.map(|g| format!("{} - {}", fmt_dt(g.start_dt), fmt_dt(g.finish_dt))).unwrap_or_default());
                            ui.label(g.map(|g| format!("{}%", g.percent_complete)).unwrap_or_default());
                            let diff = match (&r.now, g) {
                                (None, _) => tr("The project or task cannot be found").to_string(),
                                (Some(n), Some(g)) if r.changed => {
                                    let mut d = Vec::new();
                                    if n.0 != g.name {
                                        d.push(format!("{} {}", tr("Name:"), n.0));
                                    }
                                    if n.1 != g.start_dt || n.2 != g.finish_dt {
                                        d.push(format!("{} {} - {}", tr("Dates:"), fmt_dt(n.1), fmt_dt(n.2)));
                                    }
                                    if n.3 != g.percent_complete {
                                        d.push(format!("{} {}%", tr("% Complete:"), n.3));
                                    }
                                    if d.is_empty() {
                                        d.push(format!("ID {}", n.5));
                                    }
                                    d.join("; ")
                                }
                                _ => String::new(),
                            };
                            ui.label(RichText::new(crate::thai::shape(&diff).to_string()).color(theme::TEXT_DIM));
                            ui.end_row();
                        }
                    });
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let any = !shown.is_empty();
                    let can = shown.get(sel).map(|r| r.changed && r.now.is_some()).unwrap_or(false);
                    delete = ui.add_enabled(any, egui::Button::new(tr("Delete Link"))).clicked();
                    accept = ui.add_enabled(can, egui::Button::new(tr("Accept"))).clicked();
                    accept_all = ui.add_enabled(rows.iter().any(|r| r.changed && r.now.is_some()), egui::Button::new(tr("Accept All"))).clicked();
                    close = ui.button(tr("Close")).clicked();
                });
            });
            let shown: Vec<crate::app::ExtLinkRow> = rows.iter().filter(|r| r.predecessor == (tab == 0)).cloned().collect();
            if accept || accept_all || delete {
                app.checkpoint();
                if delete {
                    if let Some(r) = shown.get(sel) {
                        app.delete_external_link(r.ghost, r.local, r.predecessor);
                    }
                } else {
                    let pick: Vec<crate::app::ExtLinkRow> = if accept_all { rows.clone() } else { shown.get(sel).cloned().into_iter().collect() };
                    for r in pick.iter().filter(|r| r.changed) {
                        if let Some(n) = &r.now {
                            app.accept_external(r.ghost, n);
                        }
                    }
                }
                app.dirty = true;
                app.recalc();
            }
            if close || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                keep = false;
            } else {
                next = Some(Dialog::ProjectLinks(tab, sel));
            }
        }
        Dialog::DrawingText(k, mut text) => {
            let (mut ok, mut cancel) = (false, false);
            crate::theme::modal(egui::Id::unique("drawing_text")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(360.0);
                ui.label(RichText::new(tr("Text Box")).strong().size(14.0));
                ui.add(egui::TextEdit::multiline(&mut text).desired_width(340.0).desired_rows(4));
                let (a, b) = buttons(ui, "OK");
                ok = a;
                cancel = b;
            });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancel = true;
            }
            if ok {
                if let Some(d) = app.doc.drawings.get_mut(k) {
                    d.text = text;
                    app.dirty = true;
                }
                keep = false;
            } else if cancel {
                keep = false;
            } else {
                next = Some(Dialog::DrawingText(k, text));
            }
        }
        Dialog::Sprints(mut rows, err) => {
            let (mut ok, mut cancel) = (false, false);
            crate::theme::modal(egui::Id::unique("sprints")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(520.0);
                ui.label(RichText::new(tr("Manage Sprints")).strong().size(14.0));
                ui.add_space(6.0);
                let mut remove: Option<usize> = None;
                egui::Grid::new("sprint_rows").num_columns(4).spacing([8.0, 4.0]).show(ui, |ui| {
                    for h in ["Sprint", "Start", "Finish", ""] {
                        ui.label(RichText::new(tr(h)).strong());
                    }
                    ui.end_row();
                    for (k, r) in rows.iter_mut().enumerate() {
                        ui.add_sized([180.0, 20.0], egui::TextEdit::singleline(&mut r.1));
                        ui.add_sized([120.0, 20.0], egui::TextEdit::singleline(&mut r.2));
                        ui.add_sized([120.0, 20.0], egui::TextEdit::singleline(&mut r.3));
                        if ui.small_button("x").clicked() {
                            remove = Some(k);
                        }
                        ui.end_row();
                    }
                });
                if let Some(k) = remove {
                    rows.remove(k);
                }
                if ui.button(tr("Add Sprint")).clicked() {
                    // the next two weeks after the last sprint (or from the project start)
                    let last = rows.last().and_then(|r| app.parse_date(r.3.trim()));
                    let s = last.map(|d| d + chrono::Duration::days(1)).unwrap_or(app.doc.start);
                    let f = s + chrono::Duration::days(13);
                    // (UID 1 is the standard "No Sprint")
                    let uid = rows.iter().map(|r| r.0).max().unwrap_or(0).max(app.doc.sprints.iter().map(|s| s.uid).max().unwrap_or(0)).max(1) + 1;
                    rows.push((uid, format!("Sprint {}", rows.len() + 1), app.fmt_date(s), app.fmt_finish(f)));
                }
                error_line(ui, &err);
                let (a, b) = buttons(ui, "OK");
                ok = a;
                cancel = b;
            });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancel = true;
            }
            if ok {
                let mut out = Vec::new();
                let mut bad = None;
                for (uid, name, s, f) in &rows {
                    match (app.parse_date(s.trim()), app.parse_date(f.trim())) {
                        (Some(a), Some(b)) if a <= b && !name.trim().is_empty() => out.push(crate::model::Sprint { uid: *uid, name: name.trim().to_string(), start: a, finish: b }),
                        _ => bad = Some(format!("{}: {}", name, "The dates are not valid.")),
                    }
                }
                match bad {
                    Some(e) => next = Some(Dialog::Sprints(rows, Some(e))),
                    None => {
                        app.checkpoint();
                        let keep_uids: Vec<u32> = out.iter().map(|s| s.uid).collect();
                        for t in app.doc.tasks.iter_mut() {
                            if t.sprint.map(|u| !keep_uids.contains(&u)).unwrap_or(false) {
                                t.sprint = None;
                            }
                        }
                        app.doc.sprints = out;
                        app.dirty = true;
                        keep = false;
                    }
                }
            } else if cancel {
                keep = false;
            } else {
                next = Some(Dialog::Sprints(rows, err));
            }
        }
        Dialog::WbsCode(mut m) => {
            let (mut ok, mut cancel) = (false, false);
            let title = if app.doc.title.trim().is_empty() { "Project1".to_string() } else { app.doc.title.clone() };
            crate::theme::modal(egui::Id::unique("wbs_code")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(460.0);
                ui.label(RichText::new(crate::thai::shape(&format!("{} {}", tr("WBS Code Definition in"), title)).to_string()).strong().size(14.0));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(tr("Code preview:"));
                    let preview = crate::wbs::code(&m, &(1..=m.levels.len().max(1)).map(|_| 1).collect::<Vec<_>>());
                    ui.label(RichText::new(crate::thai::shape(&preview).to_string()).strong());
                });
                ui.horizontal(|ui| {
                    ui.label(tr("Project Code Prefix:"));
                    ui.add(egui::TextEdit::singleline(&mut m.prefix).desired_width(160.0));
                });
                ui.label(tr("Code mask (excluding prefix):"));
                let mut remove: Option<usize> = None;
                egui::Grid::new("wbs_levels").num_columns(5).spacing([8.0, 4.0]).show(ui, |ui| {
                    // ("Level" is also Resource > Level, translated as leveling)
                    let level = if crate::i18n::is_thai() { crate::thai::shape("ระดับ").to_string() } else { "Level".to_string() };
                    ui.label(RichText::new(level).strong());
                    for h in ["Sequence", "Length", "Separator", ""] {
                        ui.label(RichText::new(tr(h)).strong());
                    }
                    ui.end_row();
                    for (k, l) in m.levels.iter_mut().enumerate() {
                        ui.label((k + 1).to_string());
                        egui::ComboBox::from_id_salt(("wbs_seq", k)).width(190.0).selected_text(tr(crate::wbs::SEQUENCES[l.seq.min(3) as usize])).show_ui(ui, |ui| {
                            for (s, n) in crate::wbs::SEQUENCES.iter().enumerate() {
                                ui.selectable_value(&mut l.seq, s as u8, tr(n));
                            }
                        });
                        let len_text = if l.len == 0 { tr("Any").to_string() } else { l.len.to_string() };
                        egui::ComboBox::from_id_salt(("wbs_len", k)).width(60.0).selected_text(len_text).show_ui(ui, |ui| {
                            ui.selectable_value(&mut l.len, 0, tr("Any"));
                            for n in 1..=10u8 {
                                ui.selectable_value(&mut l.len, n, n.to_string());
                            }
                        });
                        egui::ComboBox::from_id_salt(("wbs_sep", k)).width(40.0).selected_text(l.sep.clone()).show_ui(ui, |ui| {
                            for s in [".", "-", "+", "/"] {
                                ui.selectable_value(&mut l.sep, s.to_string(), s);
                            }
                        });
                        if ui.small_button("x").clicked() {
                            remove = Some(k);
                        }
                        ui.end_row();
                    }
                });
                if let Some(k) = remove {
                    m.levels.remove(k);
                }
                if m.levels.len() < 10 && ui.button(tr("Add level")).clicked() {
                    m.levels.push(crate::wbs::WbsLevel { seq: 0, len: 0, sep: ".".into() });
                }
                ui.checkbox(&mut m.generate, tr("Generate WBS code for new task"));
                ui.checkbox(&mut m.verify_unique, tr("Verify uniqueness of new WBS codes"));
                let (a, b) = buttons(ui, "OK");
                ok = a;
                cancel = b;
            });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancel = true;
            }
            if ok {
                app.checkpoint();
                app.doc.wbs_mask = if m.levels.is_empty() && m.prefix.is_empty() { None } else { Some(m) };
                app.dirty = true;
                keep = false;
            } else if cancel {
                keep = false;
            } else {
                next = Some(Dialog::WbsCode(m));
            }
        }
        Dialog::LevelingOptions(mut o, mut from, mut to, err) => {
            let (mut ok, mut cancel, mut level_now, mut clear_now) = (false, false, false, false);
            crate::theme::modal(egui::Id::unique("leveling_options")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(470.0);
                ui.label(RichText::new(tr("Resource Leveling")).strong().size(14.0));
                ui.add_space(6.0);
                ui.label(RichText::new(tr("Leveling calculations")).strong());
                ui.horizontal(|ui| {
                    ui.radio_value(&mut o.automatic, true, tr("Automatic"));
                    ui.radio_value(&mut o.automatic, false, tr("Manual"));
                });
                ui.horizontal(|ui| {
                    ui.label(tr("Look for overallocations on a"));
                    let names = ["Minute by Minute", "Hour by Hour", "Day by Day", "Week by Week", "Month by Month"];
                    egui::ComboBox::from_id_salt("lvl_basis").selected_text(tr(names[o.basis.min(4) as usize])).show_ui(ui, |ui| {
                        for (k, n) in names.iter().enumerate() {
                            ui.selectable_value(&mut o.basis, k as u8, tr(n));
                        }
                    });
                    ui.label(tr("basis"));
                });
                ui.checkbox(&mut o.clear_first, tr("Clear leveling values before leveling"));
                ui.add_space(6.0);
                ui.label(RichText::new(tr("Leveling range for this project")).strong());
                let mut whole = o.range.is_none() && from.trim().is_empty();
                if ui.radio(whole, tr("Level entire project")).clicked() {
                    whole = true;
                    from.clear();
                    to.clear();
                }
                ui.horizontal(|ui| {
                    if ui.radio(!whole, tr("Level")).clicked() && whole {
                        from = app.fmt_date(app.doc.start);
                        to = app.fmt_date(app.doc.finish_date());
                    }
                    ui.label(tr("From:"));
                    ui.add_enabled(!whole || !from.is_empty(), egui::TextEdit::singleline(&mut from).desired_width(100.0));
                    ui.label(tr("To:"));
                    ui.add_enabled(!whole || !to.is_empty(), egui::TextEdit::singleline(&mut to).desired_width(100.0));
                });
                ui.add_space(6.0);
                ui.label(RichText::new(tr("Resolving overallocations")).strong());
                ui.horizontal(|ui| {
                    ui.label(tr("Leveling order:"));
                    let names = ["ID Only", "Standard", "Priority, Standard"];
                    egui::ComboBox::from_id_salt("lvl_order").selected_text(tr(names[o.order.min(2) as usize])).show_ui(ui, |ui| {
                        for (k, n) in names.iter().enumerate() {
                            ui.selectable_value(&mut o.order, k as u8, tr(n));
                        }
                    });
                });
                ui.checkbox(&mut o.within_slack, tr("Level only within available slack"));
                ui.checkbox(&mut o.adjust_assignments, tr("Leveling can adjust individual assignments on a task"));
                ui.checkbox(&mut o.create_splits, tr("Leveling can create splits in remaining work"));
                ui.checkbox(&mut o.proposed, tr("Level resources with the proposed booking type"));
                ui.checkbox(&mut o.manual_tasks, tr("Level manually scheduled tasks"));
                error_line(ui, &err);
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button(tr("Clear Leveling...")).clicked() {
                        clear_now = true;
                    }
                    if ui.button(tr("Level All")).clicked() {
                        level_now = true;
                    }
                });
                let (a, b) = buttons(ui, "OK");
                ok = a;
                cancel = b;
            });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancel = true;
            }
            if ok || level_now || clear_now {
                let range = if from.trim().is_empty() && to.trim().is_empty() {
                    Ok(None)
                } else {
                    match (app.parse_date(from.trim()), app.parse_date(to.trim())) {
                        (Some(a), Some(b)) if a <= b => Ok(Some((a, b))),
                        _ => Err("The date is not valid.".to_string()),
                    }
                };
                match range {
                    Ok(r) => {
                        o.range = r;
                        if app.doc.options.leveling != o {
                            app.checkpoint();
                            app.doc.options.leveling = o;
                            app.dirty = true;
                        }
                        if clear_now {
                            app.clear_leveling();
                        }
                        if level_now {
                            app.level_resources(None);
                        }
                        keep = false;
                    }
                    Err(e) => next = Some(Dialog::LevelingOptions(o, from, to, Some(e))),
                }
            } else if cancel {
                keep = false;
            } else {
                next = Some(Dialog::LevelingOptions(o, from, to, err));
            }
        }
        Dialog::MoveTasks(mut txt, err) => {
            let (mut ok, mut cancel) = (false, false);
            crate::theme::modal(egui::Id::unique("move_tasks")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(360.0);
                ui.label(RichText::new(tr("Move Tasks")).strong().size(14.0));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(tr("Move selected tasks forward by:"));
                    ui.add(egui::TextEdit::singleline(&mut txt).desired_width(90.0));
                });
                ui.label(RichText::new(tr("Enter a negative value to move the tasks back.")).small());
                error_line(ui, &err);
                let (o, c) = buttons(ui, "OK");
                ok = o;
                cancel = c;
            });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancel = true;
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                ok = true;
            }
            if ok {
                let s = txt.trim();
                let (neg, rest) = match s.strip_prefix('-') {
                    Some(r) => (true, r),
                    None => (false, s.trim_start_matches('+')),
                };
                match util::parse_dur(rest, app.dur_units(), DurUnit::Days) {
                    Some((m, _, _)) => {
                        app.move_tasks_by(if neg { -m } else { m });
                        keep = false;
                    }
                    None => next = Some(Dialog::MoveTasks(txt, Some("The duration is not valid.".into()))),
                }
            } else if cancel {
                keep = false;
            } else {
                next = Some(Dialog::MoveTasks(txt, err));
            }
        }
        Dialog::Dependency(mut st) => {
            let (Some(si), Some(pi)) = (app.doc.index_of_uid(st.succ_uid), app.doc.index_of_uid(st.pred_uid)) else {
                app.dialog = None;
                return;
            };
            let label = |i: usize| format!("{}  {}", i + 1, crate::thai::shape(&app.doc.tasks[i].name));
            let (from, to) = (label(pi), label(si));
            let (mut ok, mut cancel, mut delete) = (false, false, false);
            crate::theme::modal(egui::Id::unique("task_dependency")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(430.0);
                ui.label(RichText::new(tr("Task Dependency")).strong().size(14.0));
                ui.add_space(8.0);
                egui::Grid::new("dep_names").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                    ui.label(tr("From:"));
                    ui.label(from);
                    ui.end_row();
                    ui.label(tr("To:"));
                    ui.label(to);
                    ui.end_row();
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(tr("Type:"));
                    egui::ComboBox::from_id_salt("dep_type").width(180.0).selected_text(tr(DEP_KINDS[st.kind])).show_ui(ui, |ui| {
                        for (k, s) in DEP_KINDS.iter().enumerate() {
                            ui.selectable_value(&mut st.kind, k, tr(s));
                        }
                    });
                    ui.add_space(12.0);
                    ui.label(tr("Lag:"));
                    ui.add(egui::TextEdit::singleline(&mut st.lag).desired_width(90.0));
                });
                error_line(ui, &st.error);
                ui.add_space(8.0);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(tr("Delete")).min_size(Vec2::new(80.0, 26.0))).clicked() {
                        delete = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new(tr("Cancel")).min_size(Vec2::new(80.0, 26.0))).clicked() {
                            cancel = true;
                        }
                        if ui.add(egui::Button::new(RichText::new(tr("OK")).strong()).min_size(Vec2::new(80.0, 26.0))).clicked() {
                            ok = true;
                        }
                    });
                });
            });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancel = true;
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                ok = true;
            }
            let pred = st.pred_uid;
            if delete || (ok && st.kind == 4) {
                app.checkpoint();
                app.doc.tasks[si].predecessors.retain(|l| l.pred_uid != pred);
                app.recalc();
                keep = false;
            } else if ok {
                let du = app.dur_units();
                let Some(mut l) = app.doc.tasks[si].predecessors.iter().find(|l| l.pred_uid == pred).cloned() else {
                    app.dialog = None;
                    return;
                };
                l.kind = [LinkType::FS, LinkType::SS, LinkType::FF, LinkType::SF][st.kind.min(3)];
                let lag = st.lag.trim();
                let parsed = if lag.is_empty() {
                    l.set_lag_minutes(0, DurUnit::Days, false, du.mpd);
                    Some(())
                } else {
                    util::parse_lag_into(&lag.to_lowercase(), &mut l, du)
                };
                if parsed.is_some() {
                    app.checkpoint();
                    if let Some(x) = app.doc.tasks[si].predecessors.iter_mut().find(|x| x.pred_uid == pred) {
                        *x = l;
                    }
                    app.recalc();
                    keep = false;
                } else {
                    st.error = Some("The lag is not valid.".into());
                    next = Some(Dialog::Dependency(st));
                }
            } else if cancel {
                keep = false;
            } else {
                next = Some(Dialog::Dependency(st));
            }
        }
        Dialog::ConfirmSave => {
            let name = app.path.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Project1".into());
            let mut choice = 0;
            crate::theme::modal(egui::Id::unique("confirm_save")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(380.0);
                ui.label(RichText::new(crate::app::product()).strong());
                ui.add_space(8.0);
                ui.label(trf("Do you want to save changes to '{}'?", &[&name]));
                ui.add_space(12.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(egui::Button::new(tr("Cancel")).min_size(Vec2::new(80.0, 26.0))).clicked() {
                        choice = 3;
                    }
                    if ui.add(egui::Button::new(tr("No")).min_size(Vec2::new(80.0, 26.0))).clicked() {
                        choice = 2;
                    }
                    if ui.add(egui::Button::new(RichText::new(tr("Yes")).strong()).min_size(Vec2::new(80.0, 26.0))).clicked() {
                        choice = 1;
                    }
                });
            });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                choice = 3;
            }
            match choice {
                1 => {
                    keep = false;
                    let action = app.pending.take();
                    if app.cmd_save() {
                        if let Some(a) = action {
                            app.run_pending(a);
                        }
                    } else {
                        crate::update::cancel_restart();
                    }
                }
                2 => {
                    keep = false;
                    app.dirty = false;
                    if let Some(a) = app.pending.take() {
                        app.run_pending(a);
                    }
                }
                3 => {
                    keep = false;
                    app.pending = None;
                    crate::update::cancel_restart();
                }
                // no answer yet: the prompt stays (it used to vanish after one frame, so closing
                // the program with unsaved changes did nothing)
                _ => next = Some(Dialog::ConfirmSave),
            }
        }
        Dialog::TaskInfo(st) => match crate::taskinfo::show(app, ctx, *st) {
            Some(s) => next = Some(Dialog::TaskInfo(Box::new(s))),
            None => keep = false,
        },
        Dialog::FormatBar(st) => match show_format_bar(app, ctx, st) {
            Some(s) => next = Some(Dialog::FormatBar(s)),
            None => keep = false,
        },
        Dialog::BarStyles(st) => match show_bar_styles(app, ctx, st) {
            Some(s) => next = Some(Dialog::BarStyles(s)),
            None => keep = false,
        },
        Dialog::Layout(st) => match show_layout(app, ctx, st) {
            Some(s) => next = Some(Dialog::Layout(s)),
            None => keep = false,
        },
        Dialog::Gridlines(st) => match show_gridlines(app, ctx, st) {
            Some(s) => next = Some(Dialog::Gridlines(s)),
            None => keep = false,
        },
        Dialog::TextStyles(st) => match show_text_styles(app, ctx, st) {
            Some(s) => next = Some(Dialog::TextStyles(s)),
            None => keep = false,
        },
        Dialog::Font(st) => match show_font(app, ctx, st) {
            Some(s) => next = Some(Dialog::Font(s)),
            None => keep = false,
        },
        Dialog::Timescale(st) => match show_timescale(app, ctx, st) {
            Some(s) => next = Some(Dialog::Timescale(s)),
            None => keep = false,
        },
        Dialog::FieldSettings(st) => match show_field_settings(app, ctx, st) {
            Some(s) => next = Some(Dialog::FieldSettings(s)),
            None => keep = false,
        },
        Dialog::Options(st) => match show_options(app, ctx, st) {
            Some(s) => next = Some(Dialog::Options(s)),
            None => keep = false,
        },
        Dialog::ProjectInfo(mut st) => {
            let mut done = None;
            crate::theme::modal(egui::Id::unique("proj_info")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(440.0);
                ui.label(RichText::new(tr("Project Details")).strong().size(14.0));
                let ro = false;
                ui.add_enabled_ui(!ro, |ui| {
                ui.add_space(8.0);
                egui::Grid::new("pi").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                    ui.label(tr("Title:"));
                    ui.add(egui::TextEdit::singleline(&mut st.title).desired_width(260.0));
                    ui.end_row();
                    ui.label(tr("Schedule from:"));
                    ui.label(tr("Project Start Date"));
                    ui.end_row();
                    ui.label(tr("Start date:"));
                    ui.add(egui::TextEdit::singleline(&mut st.start).desired_width(140.0));
                    ui.end_row();
                    ui.label(tr("Finish date:"));
                    ui.label(app.fmt_date(app.doc.finish_date()));
                    ui.end_row();
                    ui.label(tr("Current date:"));
                    ui.label(app.fmt_date(chrono::Local::now().date_naive()));
                    ui.end_row();
                    ui.label(tr("Status date:"));
                    ui.add(egui::TextEdit::singleline(&mut st.status).desired_width(140.0).hint_text(tr("NA")));
                    ui.end_row();
                    ui.label(tr("Calendar:"));
                    ui.label(&app.doc.calendar.name);
                    ui.end_row();
                });
                });
                error_line(ui, &st.error);
                let (ok, cancel) = buttons(ui, "OK");
                if ok {
                    done = Some(true);
                } else if cancel {
                    done = Some(false);
                }
            });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                done = Some(false);
            }
            match done {
                Some(true) => {
                    let start = app.parse_date(&st.start);
                    let status_txt = st.status.trim();
                    let status = if status_txt.is_empty() || status_txt.eq_ignore_ascii_case("NA") { Ok(None) } else { app.parse_date(status_txt).map(Some).ok_or(()) };
                    match (start, status) {
                        (Some(s), Ok(sd)) => {
                            app.checkpoint();
                            app.doc.start = app.doc.calendar.next_working(s);
                            app.doc.status_date = sd;
                            app.doc.title = st.title.clone();
                            app.recalc();
                            keep = false;
                        }
                        _ => {
                            st.error = Some("Enter valid dates.".into());
                            next = Some(Dialog::ProjectInfo(st));
                        }
                    }
                }
                Some(false) => keep = false,
                None => next = Some(Dialog::ProjectInfo(st)),
            }
        }
        Dialog::WorkingTime(mut st) => {
            let mut done = None;
            crate::theme::modal(egui::Id::unique("working_time")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(520.0);
                ui.label(RichText::new(tr("Working Days & Holidays")).strong().size(14.0));
                let ro = false;
                ui.add_enabled_ui(!ro, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(tr("Calendar name:"));
                    ui.add(egui::TextEdit::singleline(&mut st.name).desired_width(200.0));
                });
                ui.add_space(6.0);
                ui.label(RichText::new(tr("Work Weeks")).strong());
                ui.horizontal(|ui| {
                    for (k, d) in ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].iter().enumerate() {
                        ui.checkbox(&mut st.days[k], tr(d));
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new(tr("Presets:")).color(theme::TEXT_DIM));
                    if ui.small_button(tr("Office (Mon–Fri)")).clicked() {
                        st.days = [true, true, true, true, true, false, false];
                    }
                    if ui.small_button(tr("Construction site (Mon–Sat)")).clicked() {
                        st.days = [true, true, true, true, true, true, false];
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(tr("Hours per working day:"));
                    ui.add(egui::DragValue::new(&mut st.hpd).range(1.0..=24.0).speed(0.5));
                });
                ui.add_space(8.0);
                ui.label(RichText::new(tr("Exceptions (non-working days / holidays)")).strong());
                let mut remove = None;
                egui::ScrollArea::vertical().max_height(220.0).auto_shrink([false, true]).show(ui, |ui| {
                    egui::Grid::new("wt_ex").striped(true).num_columns(4).spacing([8.0, 4.0]).show(ui, |ui| {
                        ui.label(RichText::new(tr("Name")).strong());
                        ui.label(RichText::new(tr("Start")).strong());
                        ui.label(RichText::new(tr("Finish")).strong());
                        ui.label(tr(""));
                        ui.end_row();
                        for (k, h) in st.holidays.iter_mut().enumerate() {
                            // fixed sizes: a grid inside a scroll area would otherwise shrink them
                            ui.add_sized([250.0, 22.0], egui::TextEdit::singleline(&mut h.0));
                            ui.add_sized([100.0, 22.0], egui::TextEdit::singleline(&mut h.1));
                            ui.add_sized([100.0, 22.0], egui::TextEdit::singleline(&mut h.2));
                            if ui.small_button("×").clicked() {
                                remove = Some(k);
                            }
                            ui.end_row();
                        }
                    });
                });
                if let Some(k) = remove {
                    st.holidays.remove(k);
                }
                ui.horizontal(|ui| {
                    if ui.button(tr("+ Add exception")).clicked() {
                        let today = app.fmt_date(chrono::Local::now().date_naive());
                        st.holidays.push(("Holiday".into(), today.clone(), today));
                    }
                    if ui.button(tr("Add Thai holidays")).on_hover_text(tr("Public holidays and substitution days for every year of the project")).clicked() {
                        add_thai_holidays(app, &mut st);
                    }
                    ui.checkbox(&mut st.thai_gov, tr("Include government-only days"))
                        .on_hover_text(tr("Royal Ploughing Ceremony and Buddhist Lent Day"));
                });
                });
                if let Some(info) = &st.info {
                    ui.label(RichText::new(crate::thai::shape(info)).color(theme::TEXT_DIM).size(11.5));
                }
                error_line(ui, &st.error);
                let (ok, cancel) = buttons(ui, "OK");
                if ok {
                    done = Some(true);
                } else if cancel {
                    done = Some(false);
                }
            });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                done = Some(false);
            }
            match done {
                Some(true) => {
                    let mut hol = Vec::new();
                    let mut err = None;
                    for (n, a, b) in &st.holidays {
                        match (app.parse_date(a), app.parse_date(b)) {
                            (Some(a), Some(b)) if b >= a => hol.push(Holiday { name: n.clone(), start: a, finish: b }),
                            _ => err = Some(format!("Invalid dates for exception '{n}'.")),
                        }
                    }
                    if !st.days.iter().any(|d| *d) {
                        err = Some("At least one day of the week must be a working day.".into());
                    }
                    if let Some(e) = err {
                        st.error = Some(e);
                        next = Some(Dialog::WorkingTime(st));
                    } else {
                        app.checkpoint();
                        app.doc.calendar.name = st.name.clone();
                        app.doc.calendar.working_days = st.days;
                        app.doc.calendar.hours_per_day = st.hpd;
                        app.doc.calendar.holidays = hol;
                        app.recalc();
                        keep = false;
                    }
                }
                Some(false) => keep = false,
                None => next = Some(Dialog::WorkingTime(st)),
            }
        }
        Dialog::ResourceInfo(mut st) => {
            let mut done = None;
            crate::theme::modal(egui::Id::unique("res_info")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(440.0);
                ui.label(RichText::new(tr("Resource Details")).strong().size(14.0));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    for (k, l) in [(0u8, "General"), (1, "Costs"), (2, "Notes")] {
                        if ui.selectable_label(st.tab == k, tr(l)).clicked() {
                            st.tab = k;
                        }
                    }
                });
                ui.separator();
                if st.tab == 2 {
                    ui.label(tr("Notes:"));
                    ui.add(egui::TextEdit::multiline(&mut st.notes).desired_width(420.0).desired_rows(10));
                }
                if st.tab == 1 {
                    egui::Grid::new("ri_cost").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                        if st.rtype != ResourceType::Cost {
                            ui.label(if st.rtype == ResourceType::Work { tr("Standard rate (/hr):") } else { tr("Standard rate:") });
                            ui.add(egui::DragValue::new(&mut st.std).range(0.0..=1e9).prefix(crate::util::currency_symbol()));
                            ui.end_row();
                        }
                        if st.rtype == ResourceType::Work {
                            ui.label(tr("Overtime rate (/hr):"));
                            ui.add(egui::DragValue::new(&mut st.ovt).range(0.0..=1e9).prefix(crate::util::currency_symbol()));
                            ui.end_row();
                        }
                        if st.rtype != ResourceType::Cost {
                            ui.label(tr("Cost/Use:"));
                            ui.add(egui::DragValue::new(&mut st.cpu).range(0.0..=1e9).prefix(crate::util::currency_symbol()));
                            ui.end_row();
                        }
                        ui.label(tr("Accrue at:"));
                        egui::ComboBox::from_id_salt("accrue2").selected_text(tr(st.accrue.label())).show_ui(ui, |ui| {
                            for a in Accrue::ALL {
                                ui.selectable_value(&mut st.accrue, a, tr(a.label()));
                            }
                        });
                        ui.end_row();
                    });
                }
                let ro = st.tab != 0;
                if !ro {
                ui.add_space(8.0);
                egui::Grid::new("ri").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                    ui.label(tr("Resource name:"));
                    ui.add(egui::TextEdit::singleline(&mut st.name).desired_width(240.0));
                    ui.end_row();
                    ui.label(tr("Type:"));
                    egui::ComboBox::from_id_salt("rtype").selected_text(tr(st.rtype.label())).show_ui(ui, |ui| {
                        for t in [ResourceType::Work, ResourceType::Material, ResourceType::Cost] {
                            ui.selectable_value(&mut st.rtype, t, tr(t.label()));
                        }
                    });
                    ui.end_row();
                    ui.label(tr("Initials:"));
                    ui.add(egui::TextEdit::singleline(&mut st.initials).desired_width(80.0));
                    ui.end_row();
                    ui.label(tr("Group:"));
                    ui.add(egui::TextEdit::singleline(&mut st.group).desired_width(160.0));
                    ui.end_row();
                    ui.label(tr("Email:"));
                    ui.add(egui::TextEdit::singleline(&mut st.email).desired_width(240.0));
                    ui.end_row();
                    if st.rtype == ResourceType::Material {
                        ui.label(tr("Material label:"));
                        ui.add(egui::TextEdit::singleline(&mut st.material).desired_width(80.0));
                        ui.end_row();
                    }
                    if st.rtype == ResourceType::Work {
                        ui.label(tr("Max. units:"));
                        ui.add(egui::DragValue::new(&mut st.max).range(1.0..=10000.0).suffix("%"));
                        ui.end_row();
                    }
                    if st.rtype != ResourceType::Cost {
                        ui.label(if st.rtype == ResourceType::Work { "Standard rate (/hr):" } else { "Standard rate:" });
                        ui.add(egui::DragValue::new(&mut st.std).range(0.0..=1e9).prefix(crate::util::currency_symbol()));
                        ui.end_row();
                    }
                    if st.rtype == ResourceType::Work {
                        ui.label(tr("Overtime rate (/hr):"));
                        ui.add(egui::DragValue::new(&mut st.ovt).range(0.0..=1e9).prefix(crate::util::currency_symbol()));
                        ui.end_row();
                    }
                    ui.label(tr("Accrue at:"));
                    egui::ComboBox::from_id_salt("accrue").selected_text(tr(st.accrue.label())).show_ui(ui, |ui| {
                        for a in Accrue::ALL {
                            ui.selectable_value(&mut st.accrue, a, tr(a.label()));
                        }
                    });
                    ui.end_row();
                    ui.label(tr("Code:"));
                    ui.add(egui::TextEdit::singleline(&mut st.code).desired_width(120.0));
                    ui.end_row();
                    if st.rtype != ResourceType::Cost {
                        ui.label(tr("Cost/Use:"));
                        ui.add(egui::DragValue::new(&mut st.cpu).range(0.0..=1e9).prefix(crate::util::currency_symbol()));
                        ui.end_row();
                    }
                });
                }
                error_line(ui, &st.error);
                let (ok, cancel) = buttons(ui, "OK");
                if ok {
                    done = Some(true);
                } else if cancel {
                    done = Some(false);
                }
            });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                done = Some(false);
            }
            match done {
                Some(true) => {
                    let name = st.name.trim().to_string();
                    let dup = app.doc.resources.iter().enumerate().any(|(k, r)| k != st.idx && r.name.eq_ignore_ascii_case(&name));
                    if name.is_empty() || dup || name.contains([',', '[', ']']) {
                        st.error = Some("Enter a unique resource name without commas or brackets.".into());
                        next = Some(Dialog::ResourceInfo(st));
                    } else if st.idx < app.doc.resources.len() {
                        app.checkpoint();
                        let r = &mut app.doc.resources[st.idx];
                        r.name = name;
                        r.initials = st.initials.clone();
                        r.group = st.group.clone();
                        r.rtype = st.rtype;
                        r.material_label = st.material.clone();
                        r.max_units = st.max;
                        r.std_rate = st.std;
                        r.ovt_rate = st.ovt;
                        r.cost_per_use = st.cpu;
                        r.accrue_at = st.accrue;
                        r.code = st.code.trim().to_string();
                        r.notes = st.notes.clone();
                        r.email = st.email.trim().to_string();
                        app.recalc();
                        keep = false;
                    } else {
                        keep = false;
                    }
                }
                Some(false) => keep = false,
                None => next = Some(Dialog::ResourceInfo(st)),
            }
        }
        Dialog::AssignResources => {
            let mut close = false;
            let sel = app.selected_tasks();
            egui::Window::new(tr("Assign Resources"))
                .id(egui::Id::unique("assign_resources"))
                .frame(crate::theme::dialog_frame())
                .collapsible(false)
                .resizable(true)
                .default_size([380.0, 360.0])
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.content_rect().center())
                .show(ctx, |ui| {
                let names: Vec<String> = sel.iter().map(|&i| app.doc.tasks[i].name.clone()).collect();
                ui.label(crate::thai::shape(&crate::i18n::trf("Task: {}", &[&names.join(", ")])).to_string());
                ui.separator();
                let mut new_name = ui.data_mut(|d| d.get_temp::<String>(egui::Id::unique("assign_new")).unwrap_or_default());
                egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                    egui::Grid::new("assign").striped(true).num_columns(4).show(ui, |ui| {
                        ui.label(RichText::new(tr("Assign")).strong());
                        ui.label(RichText::new(tr("Resource Name")).strong());
                        ui.label(RichText::new(tr("Units")).strong());
                        ui.label(RichText::new(tr("Cost")).strong());
                        ui.end_row();
                        for r in app.doc.resources.clone() {
                            let assigned = !sel.is_empty() && sel.iter().all(|&i| app.doc.tasks[i].resources.contains(&r.uid));
                            let mut on = assigned;
                            ui.checkbox(&mut on, tr(""));
                            ui.label(crate::thai::shape(&r.name).to_string());
                            // units of the assignment (the first selected task), editable like the standard grid
                            let first = sel.first().copied();
                            let asg = first.and_then(|i| app.doc.tasks[i].assignments.iter().find(|a| a.res_uid == r.uid).cloned());
                            match (assigned, asg, first) {
                                (true, Some(a), Some(i)) => {
                                    let mut u = if r.rtype == ResourceType::Work { a.units * 100.0 } else { a.units };
                                    let resp = ui.add(egui::DragValue::new(&mut u).range(0.0..=100000.0).speed(if r.rtype == ResourceType::Work { 5.0 } else { 1.0 }).suffix(if r.rtype == ResourceType::Work { "%" } else { "" }));
                                    if resp.changed() {
                                        app.checkpoint();
                                        let v = if r.rtype == ResourceType::Work { u / 100.0 } else { u };
                                        for &j in &sel {
                                            app.set_assignment_units(j, r.uid, v);
                                        }
                                        app.recalc();
                                    }
                                    let cost = match r.rtype {
                                        ResourceType::Work => a.work_min.max(0) as f64 / 60.0 * r.std_rate + r.cost_per_use * a.units,
                                        ResourceType::Material => a.units * r.std_rate + r.cost_per_use,
                                        ResourceType::Cost => a.cost,
                                    };
                                    ui.label(util::fmt_money(cost));
                                    let _ = i;
                                }
                                _ => {
                                    ui.label("");
                                    ui.label("");
                                }
                            }
                            ui.end_row();
                            if on != assigned {
                                app.checkpoint();
                                for &i in &sel {
                                    let t = &mut app.doc.tasks[i];
                                    if t.is_summary {
                                        continue;
                                    }
                                    if on {
                                        if !t.resources.contains(&r.uid) {
                                            t.resources.push(r.uid);
                                        }
                                    } else {
                                        t.resources.retain(|u| *u != r.uid);
                                    }
                                }
                                app.recalc();
                            }
                        }
                    });
                });
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut new_name).hint_text(tr("New resource name")).desired_width(200.0));
                    if ui.button(tr("Add")).clicked() && !new_name.trim().is_empty() && app.doc.resource_by_name(new_name.trim()).is_none() {
                        app.checkpoint();
                        let u = app.doc.alloc_res_uid();
                        app.doc.resources.push(Resource::new(u, new_name.trim()));
                        new_name.clear();
                    }
                });
                ui.data_mut(|d| d.insert_temp(egui::Id::unique("assign_new"), new_name));
                ui.separator();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(egui::Button::new(tr("Close")).min_size(Vec2::new(80.0, 26.0))).clicked() {
                        close = true;
                    }
                });
            });
            if close || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                keep = false;
            } else {
                next = Some(Dialog::AssignResources);
            }
        }
        Dialog::Find => {
            let mut close = false;
            crate::theme::modal(egui::Id::unique("find")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(380.0);
                ui.label(RichText::new(tr("Find")).strong().size(14.0));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(tr("Find what:"));
                    let r = ui.add(egui::TextEdit::singleline(&mut app.find_text).desired_width(240.0));
                    r.request_focus();
                    if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        app.find_next();
                    }
                });
                ui.label(RichText::new(tr("Look in field: Name")).small().color(theme::TEXT_DIM));
                ui.add_space(8.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(egui::Button::new(tr("Close")).min_size(Vec2::new(80.0, 26.0))).clicked() {
                        close = true;
                    }
                    if ui.add(egui::Button::new(tr("Find Next")).min_size(Vec2::new(80.0, 26.0))).clicked() {
                        app.find_next();
                    }
                });
            });
            if close || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                keep = false;
            } else if app.message.is_none() {
                next = Some(Dialog::Find);
            } else {
                keep = false;
            }
        }
        Dialog::Zoom(mut st) => {
            let mut close = false;
            let mut ok = false;
            crate::theme::modal(egui::Id::unique("zoom")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(300.0);
                ui.label(RichText::new(tr("Zoom")).strong().size(14.0));
                ui.add_space(6.0);
                ui.label(tr("Zoom to"));
                ui.indent("zoom_to", |ui| {
                    for (i, lbl) in ["1 week", "2 weeks", "1 month", "3 months", "Selected tasks", "Entire project"].iter().enumerate() {
                        ui.radio_value(&mut st.choice, i as u8, tr(lbl));
                    }
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut st.choice, 6, tr("Custom:"));
                        let r = ui.add(egui::TextEdit::singleline(&mut st.custom).desired_width(40.0));
                        if r.gained_focus() || r.changed() {
                            st.choice = 6;
                        }
                        egui::ComboBox::from_id_salt("zoom_unit")
                            .width(80.0)
                            .selected_text(tr(["Days", "Weeks", "Months"][st.unit as usize]))
                            .show_ui(ui, |ui| {
                                for (i, u) in ["Days", "Weeks", "Months"].iter().enumerate() {
                                    if ui.selectable_value(&mut st.unit, i as u8, tr(u)).clicked() {
                                        st.choice = 6;
                                    }
                                }
                            });
                    });
                });
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(tr("Reset")).min_size(Vec2::new(70.0, 26.0))).clicked() {
                        // back to the default Weeks-over-Days timescale
                        app.set_zoom(24.0, None);
                        close = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new(tr("Cancel")).min_size(Vec2::new(70.0, 26.0))).clicked() {
                            close = true;
                        }
                        if ui.add(egui::Button::new(tr("OK")).min_size(Vec2::new(70.0, 26.0))).clicked() {
                            ok = true;
                        }
                    });
                });
            });
            if ok || ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                let (left, width) = app.chart_view;
                let span = |days: f32, app: &mut App| app.set_zoom(width / days.max(0.05), Some(left));
                match st.choice {
                    0 => span(7.0, app),
                    1 => span(14.0, app),
                    2 => span(30.0, app),
                    3 => span(91.0, app),
                    4 | 5 => {
                        let idx: Vec<usize> = if st.choice == 4 { app.selected_tasks() } else { (0..app.doc.tasks.len()).collect() };
                        let a = idx.iter().map(|&i| app.doc.tasks[i].start).min();
                        let b = idx.iter().map(|&i| app.doc.tasks[i].finish).max();
                        if let (Some(a), Some(b)) = (a, b) {
                            app.zoom_to_range(a, b, width);
                        }
                    }
                    _ => {
                        let n: f32 = st.custom.trim().parse().unwrap_or(0.0);
                        if n > 0.0 {
                            span(n * [1.0, 7.0, 30.44][st.unit as usize], app);
                        }
                    }
                }
                keep = false;
            } else if close || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                keep = false;
            } else {
                next = Some(Dialog::Zoom(st));
            }
        }
        Dialog::Overview => {
            let mut close = false;
            crate::theme::modal(egui::Id::unique("overview")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(520.0);
                let title = if app.doc.title.is_empty() { "Project Overview".to_string() } else { app.doc.title.clone() };
                ui.label(RichText::new(title).size(22.0).color(theme::GREEN));
                ui.label(format!(
                    "{} - {}",
                    app.fmt_date(app.doc.start),
                    app.fmt_date(app.doc.finish_date())
                ));
                ui.add_space(10.0);
                let leaf: Vec<&Task> = app.doc.tasks.iter().filter(|t| !t.is_summary).collect();
                let total = leaf.len();
                let done = leaf.iter().filter(|t| t.percent_complete == 100).count();
                let late = leaf
                    .iter()
                    .filter(|t| t.finish < chrono::Local::now().date_naive() && t.percent_complete < 100)
                    .count();
                let milestones: Vec<&&Task> = leaf.iter().filter(|t| t.is_milestone()).collect();
                let crit = leaf.iter().filter(|t| t.critical).count();
                let cost: f64 = leaf.iter().map(|t| t.cost).sum();
                let w: f64 = leaf.iter().map(|t| t.duration.max(1) as f64).sum();
                let pct = if w > 0.0 { leaf.iter().map(|t| t.duration.max(1) as f64 * t.percent_complete as f64).sum::<f64>() / w } else { 0.0 };
                ui.horizontal(|ui| {
                    stat(ui, &format!("{pct:.0}%"), "% Complete");
                    stat(ui, &total.to_string(), "Tasks");
                    stat(ui, &done.to_string(), "Completed");
                    stat(ui, &late.to_string(), "Late tasks");
                    stat(ui, &crit.to_string(), "Critical");
                });
                ui.add_space(6.0);
                ui.add(egui::ProgressBar::new((pct / 100.0) as f32).fill(theme::GREEN).show_percentage());
                ui.add_space(8.0);
                ui.label(RichText::new(format!("Total cost: {}", util::fmt_money(cost))).strong());
                ui.add_space(8.0);
                ui.label(RichText::new(tr("Milestones due")).strong());
                if milestones.is_empty() {
                    ui.label(RichText::new(tr("No milestones")).color(theme::TEXT_DIM));
                }
                for m in milestones.iter().take(10) {
                    ui.label(crate::thai::shape(&format!("•  {}  —  {}", m.name, app.fmt_date(m.start))).to_string());
                }
                ui.add_space(8.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(egui::Button::new(tr("Close")).min_size(Vec2::new(80.0, 26.0))).clicked() {
                        close = true;
                    }
                });
            });
            if close || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                keep = false;
            } else {
                next = Some(Dialog::Overview);
            }
        }
        Dialog::Inspect(i) => {
            let mut close = false;
            crate::theme::modal(egui::Id::unique("inspect")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(400.0);
                if let Some(t) = app.doc.tasks.get(i) {
                    ui.label(RichText::new(crate::thai::shape(&format!("Task: {}", t.name)).to_string()).strong().size(14.0));
                    ui.add_space(6.0);
                    ui.label(format!("Start: {}", app.fmt_date(t.start)));
                    ui.add_space(6.0);
                    ui.label(RichText::new(tr("Factors affecting the start date:")).strong());
                    if t.manual {
                        ui.label(tr("• Manually scheduled — the start date is the date you entered."));
                    }
                    if let Some(c) = t.constraint_date {
                        ui.label(format!("• Start No Earlier Than constraint: {}", app.fmt_date(c)));
                    }
                    let (ids, _) = app.id_maps();
                    for l in &t.predecessors {
                        if let Some(k) = app.doc.index_of_uid(l.pred_uid) {
                            let p = &app.doc.tasks[k];
                            ui.label(format!(
                                "• Predecessor {} '{}' ({}{}), finishes {}",
                                ids[&p.uid],
                                p.name,
                                l.kind.code(),
                                {
                                    let lt = crate::util::fmt_lag(l, app.dur_units(), &app.doc.options, crate::i18n::is_thai());
                                    if lt.is_empty() { lt } else { format!(" {lt}") }
                                },
                                app.fmt_date(p.finish)
                            ));
                        }
                    }
                    if let Some(par) = app.doc.parent_of(i) {
                        if !app.doc.tasks[par].predecessors.is_empty() {
                            ui.label(format!("• Summary task '{}' has predecessors", app.doc.tasks[par].name));
                        }
                    }
                    if t.predecessors.is_empty() && t.constraint_date.is_none() && !t.manual {
                        ui.label(tr("• Project start date"));
                    }
                    if !app.doc.calendar.is_working(t.start) {
                        ui.colored_label(egui::Color32::from_rgb(0xC0, 0x39, 0x2B), "Warning: the task starts on a non-working day.");
                    }
                }
                ui.add_space(8.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(egui::Button::new(tr("Close")).min_size(Vec2::new(80.0, 26.0))).clicked() {
                        close = true;
                    }
                });
            });
            if close || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                keep = false;
            } else {
                next = Some(Dialog::Inspect(i));
            }
        }
        Dialog::Update => {
            if show_update(app, ctx) {
                next = Some(Dialog::Update);
            } else {
                keep = false;
            }
        }
        Dialog::Licenses => {
            let mut close = false;
            crate::theme::modal(egui::Id::unique("licenses")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(640.0);
                ui.label(RichText::new(tr("Open-source licenses")).strong().size(14.0));
                ui.add_space(6.0);
                // the full licence texts run to thousands of lines: only the visible rows are laid out
                static LINES: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
                let lines = LINES.get_or_init(|| LICENSES_TEXT.trim_start_matches('\u{feff}').lines().collect());
                let row_h = ui.text_style_height(&egui::TextStyle::Monospace).max(14.0);
                egui::ScrollArea::vertical().max_height(420.0).auto_shrink([false, false]).show_rows(ui, row_h, lines.len(), |ui, rows| {
                    for line in &lines[rows] {
                        ui.add(egui::Label::new(RichText::new(*line).monospace().size(11.0)).extend());
                    }
                });
                ui.add_space(8.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(egui::Button::new(tr("Close")).min_size(Vec2::new(80.0, 26.0))).clicked() {
                        close = true;
                    }
                });
            });
            if close || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                keep = false;
            } else {
                next = Some(dialog);
            }
        }
        Dialog::About | Dialog::Shortcuts => {
            let about = matches!(dialog, Dialog::About);
            let mut open_licenses = false;
            let mut check_updates = false;
            let mut close = false;
            crate::theme::modal(egui::Id::unique("about")).show(ctx, |ui| {
                crate::theme::dialog_spacing(ui);
                ui.set_width(440.0);
                if about {
                    let tex = theme::logo_texture(ui.ctx(), true);
                    ui.add(egui::Image::new(&tex).fit_to_exact_size(Vec2::splat(96.0)));
                    ui.label(RichText::new(crate::app::product()).size(24.0).color(theme::GREEN));
                    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                    crate::ribbon::credit_lines(ui);
                    if ui.link(tr("Open-source licenses...")).clicked() {
                        open_licenses = true;
                    }
                    ui.add_space(6.0);
                    ui.label(tr("Desktop project scheduling with Gantt chart. Works fully offline — no sign-in."));
                    ui.label(tr("File formats: Neramit Planner (.neramit), .mpp / .mpt and XML (.xml); export to PDF, Excel and CSV."));
                    crate::ribbon::trademark_note(ui);
                } else {
                    ui.label(RichText::new(tr("Keyboard Shortcuts")).strong().size(14.0));
                    ui.add_space(6.0);
                    egui::Grid::new("keys").striped(true).num_columns(2).show(ui, |ui| {
                        let editor_keys = [
                            ("Ctrl+N / Ctrl+O / Ctrl+S", "New / Open / Save"),
                            ("F12", "Save As"),
                            ("Ctrl+Z / Ctrl+Y", "Undo / Redo"),
                            ("Insert", "Insert task"),
                            ("Delete", "Delete task"),
                            ("F2", "Edit cell"),
                            ("Shift+F2", "Task Details"),
                            ("Ctrl+F2 / Ctrl+Shift+F2", "Link / Unlink tasks"),
                            ("Alt+Shift+Right / Left", "Indent / Outdent"),
                            ("Alt+Shift+Up / Down", "Move task up / down"),
                            ("Ctrl+C / Ctrl+X / Ctrl+V", "Copy / Cut / Paste task"),
                            ("Ctrl+D", "Fill down"),
                            ("Ctrl+F", "Find"),
                            ("F9", "Calculate project"),
                            ("Ctrl+Mouse wheel", "Zoom timescale at the pointer"),
                            ("Ctrl+/ or Ctrl++ / Ctrl+-", "Zoom in / Zoom out"),
                            ("Alt+Home / Alt+End", "Go to project start / finish"),
                            ("Alt+Left / Alt+Right", "Scroll timescale"),
                            ("Ctrl+Shift+F5", "Scroll to task"),
                        ];
                        let keys: &[(&str, &str)] = &editor_keys;
                        for &(k, d) in keys {
                            ui.label(RichText::new(k).strong());
                            ui.label(tr(d));
                            ui.end_row();
                        }
                    });
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if about && update_button(ui) {
                        check_updates = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new(tr("OK")).min_size(Vec2::new(80.0, 26.0))).clicked() {
                            close = true;
                        }
                    });
                });
            });
            if open_licenses {
                keep = false;
                app.dialog = Some(Dialog::Licenses);
            } else if check_updates {
                keep = false;
                crate::update::check(ctx);
                app.dialog = Some(Dialog::Update);
            } else if close || ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::Enter)) {
                keep = false;
            } else {
                next = Some(dialog);
            }
        }
    }
    if keep {
        if let Some(n) = next {
            // only restore if no other dialog was opened meanwhile
            if app.dialog.is_none() {
                app.dialog = Some(n);
            }
        }
    }
}

/// "Check for Updates..." with the update icon (About dialog and the About page); true when clicked.
pub fn update_button(ui: &mut egui::Ui) -> bool {
    // room on the left of the label for the icon
    let b = ui.add(egui::Button::new(format!("       {}", tr("Check for Updates..."))).min_size(Vec2::new(0.0, 30.0)));
    let icon = egui::Rect::from_center_size(egui::pos2(b.rect.left() + 17.0, b.rect.center().y), Vec2::splat(20.0));
    theme::paint_icon(ui.painter(), icon, theme::Icon::Update, true);
    b.clicked()
}

/// The update dialog; false once it is closed. A check or download keeps running when it is
/// closed, and the dialog opens again by itself when the download is ready. A finished check
/// (up to date / failed) starts a new one when the dialog is opened again.
fn show_update(app: &mut App, ctx: &egui::Context) -> bool {
    use crate::update::{self, Status};
    let status = update::status();
    let mut open = true;
    crate::theme::modal(egui::Id::unique("update")).show(ctx, |ui| {
        crate::theme::dialog_spacing(ui);
        ui.set_width(440.0);
        ui.horizontal(|ui| {
            theme::paint_icon(ui.painter(), egui::Rect::from_min_size(ui.cursor().min, Vec2::splat(28.0)), theme::Icon::Update, true);
            ui.add_space(36.0);
            ui.label(RichText::new(tr("Neramit Planner Updates")).strong().size(14.0));
        });
        ui.add_space(10.0);
        let button = |ui: &mut Ui, text: &str, strong: bool| {
            let t = if strong { RichText::new(tr(text)).strong() } else { RichText::new(tr(text)) };
            ui.add(egui::Button::new(t).min_size(Vec2::new(96.0, 26.0))).clicked()
        };
        match &status {
            Status::Idle | Status::Checking => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(tr("Checking for updates..."));
                });
                ui.add_space(10.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if button(ui, "Close", false) {
                        open = false;
                    }
                });
            }
            Status::UpToDate | Status::Failed(_) => {
                if let Status::Failed(e) = &status {
                    ui.label(tr("Could not check for updates."));
                    ui.label(RichText::new(tr(e)).color(theme::TEXT_DIM));
                } else {
                    ui.label(trf("Neramit Planner {} is the latest version.", &[update::current()]));
                }
                ui.add_space(10.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if button(ui, "OK", true) {
                        open = false;
                    }
                });
                if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                    open = false;
                }
            }
            Status::Available(rel) => {
                ui.label(RichText::new(trf("Neramit Planner {} is available.", &[&rel.version])).strong());
                ui.label(trf("You have version {}.", &[update::current()]));
                let notes = if crate::i18n::is_thai() && !rel.notes_th.is_empty() { &rel.notes_th } else { &rel.notes_en };
                if !notes.is_empty() {
                    ui.add_space(8.0);
                    ui.label(RichText::new(tr("What's new")).strong());
                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        ui.label(crate::thai::shape(notes).to_string());
                    });
                }
                if !update::can_download(rel) {
                    ui.add_space(6.0);
                    ui.label(RichText::new(tr("This release has no portable update. Open the download page instead.")).color(theme::TEXT_DIM));
                }
                ui.add_space(10.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if button(ui, "Later", false) {
                        open = false;
                    }
                    if button(ui, "Update now", true) {
                        if update::can_download(rel) {
                            update::download(ctx, rel.clone());
                        } else {
                            update::open_page();
                            open = false;
                        }
                    }
                });
            }
            Status::Downloading(rel, f) => {
                ui.label(trf("Downloading Neramit Planner {}...", &[&rel.version]));
                ui.add_space(6.0);
                ui.add(egui::ProgressBar::new(*f).show_percentage());
                ui.add_space(10.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if button(ui, "Hide", false) {
                        open = false;
                    }
                });
            }
            Status::Ready(rel, path) => {
                ui.label(RichText::new(trf("Neramit Planner {} is ready to install.", &[&rel.version])).strong());
                ui.label(tr("Neramit Planner closes, updates and opens again. You are asked to save your changes first."));
                ui.add_space(10.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if button(ui, "Later", false) {
                        // installed (without reopening) when the program is closed anyway
                        update::install_on_exit(rel.clone(), path.clone(), false);
                        open = false;
                    }
                    if button(ui, "Install and restart", true) {
                        update::install_on_exit(rel.clone(), path.clone(), true);
                        open = false;
                        app.cmd_exit();
                    }
                });
            }
        }
    });
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        open = false;
    }
    open
}

fn stat(ui: &mut Ui, big: &str, small: &str) {
    ui.vertical(|ui| {
        ui.set_width(88.0);
        ui.label(RichText::new(big).size(24.0).color(theme::GREEN));
        ui.label(RichText::new(small).color(theme::TEXT_DIM));
    });
}

/// Change Working Time > Add Thai holidays: every year from the project start to its finish.
fn add_thai_holidays(app: &App, st: &mut WorkingTimeState) {
    use chrono::Datelike;
    let today = chrono::Local::now().date_naive();
    let first = app.doc.start.year();
    let last = app.doc.tasks.iter().map(|t| t.finish.year()).max().unwrap_or(first).max(first);
    let (first, last) = if app.doc.tasks.is_empty() { (first.min(today.year()), first.max(today.year())) } else { (first, last) };
    let set = crate::thai_holidays::holidays(first, last, st.thai_gov);
    let thai = crate::i18n::is_thai();
    let mut added = 0;
    for h in &set.days {
        let s = app.fmt_date(h.date);
        let covered = st.holidays.iter().any(|x| match (app.parse_date(&x.1), app.parse_date(&x.2)) {
            (Some(a), Some(b)) => a <= h.date && h.date <= b,
            _ => x.1 == s,
        });
        if !covered {
            st.holidays.push((h.name(thai).to_string(), s.clone(), s));
            added += 1;
        }
    }
    st.holidays.sort_by_key(|x| app.parse_date(&x.1));
    let loc = &app.loc;
    let years = |ys: &[i32]| ys.iter().map(|y| (y + loc.year_offset).to_string()).collect::<Vec<_>>().join(", ");
    let span = if first == last { (first + loc.year_offset).to_string() } else { format!("{}–{}", first + loc.year_offset, last + loc.year_offset) };
    let mut info = crate::i18n::trf("Added {} holidays for {}.", &[&added.to_string(), &span]);
    if !set.provisional.is_empty() {
        info.push(' ');
        info.push_str(&crate::i18n::trf("Dates for {} are provisional until the cabinet announces them.", &[&years(&set.provisional)]));
    }
    if !set.missing.is_empty() {
        info.push(' ');
        info.push_str(&crate::i18n::trf("{}: Buddhist holidays not included; add them by hand or in thai-holidays.txt in the settings folder.", &[&years(&set.missing)]));
    }
    st.info = Some(info);
}
