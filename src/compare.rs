//! Report > Compare Projects: the open project against a previous version of it (a .mpp, .xml or
//! .neramit file). Tasks are matched by unique ID, then by name; every task gets Added / Deleted /
//! Changed / Unchanged with its Start, Finish, Duration, Work, Cost and % Complete in both
//! versions and the difference, as in the standard comparison report. Shown as a report page, and
//! exported as CSV.

use crate::app::{App, View};
use crate::i18n::tr;
use crate::model::{Project, Task};
use crate::theme;
use chrono::NaiveDateTime;
use eframe::egui::{self, Color32, RichText, Ui};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Added,
    Deleted,
    Changed,
    Same,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Status::Added => "Added",
            Status::Deleted => "Deleted",
            Status::Changed => "Changed",
            Status::Same => "Unchanged",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Vals {
    pub start: Option<NaiveDateTime>,
    pub finish: Option<NaiveDateTime>,
    pub dur_min: i64,
    pub work_min: i64,
    pub cost: f64,
    pub pct: u8,
    pub summary: bool,
}

impl Vals {
    fn of(t: &Task, mpd: i64) -> Vals {
        Vals { start: t.start_dt, finish: t.finish_dt, dur_min: t.dur_minutes(mpd), work_min: t.work_min, cost: t.cost, pct: t.percent_complete, summary: t.is_summary }
    }
    fn same(&self, o: &Vals) -> bool {
        self.start == o.start && self.finish == o.finish && self.dur_min == o.dur_min && self.work_min == o.work_min && (self.cost - o.cost).abs() < 0.005 && self.pct == o.pct
    }
}

#[derive(Clone, Debug)]
pub struct Row {
    pub status: Status,
    pub name: String,
    pub level: u8,
    pub cur: Option<Vals>,
    pub prev: Option<Vals>,
}

#[derive(Clone, Debug)]
pub struct Comparison {
    pub previous: String,
    pub rows: Vec<Row>,
    /// resources: name, work and cost now and before
    pub resources: Vec<(String, Status, Option<(i64, f64)>, Option<(i64, f64)>)>,
    pub only_changes: bool,
}

/// Compare `cur` with `prev`.
pub fn compare(cur: &Project, prev: &Project, previous: &str) -> Comparison {
    let mpd_c = cur.calendar.minutes_per_day();
    let mpd_p = prev.calendar.minutes_per_day();
    // the previous task of each current one: same unique ID (and name or outline place), else name
    let key_uid = |t: &Task| t.src_uid.unwrap_or(t.uid);
    let mut used = vec![false; prev.tasks.len()];
    let mut matched: Vec<Option<usize>> = vec![None; cur.tasks.len()];
    for (i, t) in cur.tasks.iter().enumerate() {
        if let Some(k) = prev.tasks.iter().position(|p| key_uid(p) == key_uid(t) && p.name.trim() == t.name.trim()) {
            if !used[k] {
                used[k] = true;
                matched[i] = Some(k);
            }
        }
    }
    for (i, t) in cur.tasks.iter().enumerate() {
        if matched[i].is_some() {
            continue;
        }
        if let Some(k) = (0..prev.tasks.len()).find(|&k| !used[k] && prev.tasks[k].name.trim() == t.name.trim()) {
            used[k] = true;
            matched[i] = Some(k);
        }
    }
    let mut rows = Vec::new();
    // in the current order, deleted tasks after the task they followed
    let mut deleted_after: Vec<Vec<usize>> = vec![Vec::new(); cur.tasks.len() + 1];
    let mut last_cur = 0usize;
    let back: std::collections::HashMap<usize, usize> = matched.iter().enumerate().filter_map(|(i, m)| m.map(|k| (k, i))).collect();
    for k in 0..prev.tasks.len() {
        match back.get(&k) {
            Some(&i) => last_cur = i + 1,
            None => deleted_after[last_cur].push(k),
        }
    }
    let push_deleted = |rows: &mut Vec<Row>, list: &[usize]| {
        for &k in list {
            let p = &prev.tasks[k];
            rows.push(Row { status: Status::Deleted, name: p.name.clone(), level: p.outline_level, cur: None, prev: Some(Vals::of(p, mpd_p)) });
        }
    };
    push_deleted(&mut rows, &deleted_after[0]);
    for (i, t) in cur.tasks.iter().enumerate() {
        let c = Vals::of(t, mpd_c);
        let (status, prev_v) = match matched[i] {
            Some(k) => {
                let p = Vals::of(&prev.tasks[k], mpd_p);
                (if c.same(&p) { Status::Same } else { Status::Changed }, Some(p))
            }
            None => (Status::Added, None),
        };
        rows.push(Row { status, name: t.name.clone(), level: t.outline_level, cur: Some(c), prev: prev_v });
        push_deleted(&mut rows, &deleted_after[i + 1]);
    }
    // resources by name
    let mut resources = Vec::new();
    for r in &cur.resources {
        match prev.resources.iter().find(|p| p.name.trim() == r.name.trim()) {
            Some(p) => {
                let same = p.work_min == r.work_min && (p.cost - r.cost).abs() < 0.005;
                resources.push((r.name.clone(), if same { Status::Same } else { Status::Changed }, Some((r.work_min, r.cost)), Some((p.work_min, p.cost))));
            }
            None => resources.push((r.name.clone(), Status::Added, Some((r.work_min, r.cost)), None)),
        }
    }
    for p in &prev.resources {
        if !cur.resources.iter().any(|r| r.name.trim() == p.name.trim()) {
            resources.push((p.name.clone(), Status::Deleted, None, Some((p.work_min, p.cost))));
        }
    }
    Comparison { previous: previous.to_string(), rows, resources, only_changes: false }
}

/// Report > Compare Projects: pick the previous version and show the comparison.
pub fn open_compare(app: &mut App) {
    let Some(path) = rfd::FileDialog::new()
        .set_title(tr("Compare Project Versions").to_string())
        .add_filter("Project (*.mpp; *.xml; *.neramit)", &["mpp", "xml", "neramit"])
        .pick_file()
    else {
        return;
    };
    compare_with(app, &path);
}

pub fn compare_with(app: &mut App, path: &std::path::Path) {
    match crate::fileio::load(path) {
        Ok(mut prev) => {
            let _ = crate::schedule::schedule(&mut prev);
            let name = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            app.comparison = Some(compare(&app.doc, &prev, &name));
            app.view = View::Report;
            app.report = crate::reports::ReportKind::Comparison;
            app.backstage = None;
        }
        Err(e) => app.show_error(e),
    }
}

fn days(m: i64, mpd: i64) -> String {
    let d = m as f64 / mpd.max(1) as f64;
    if (d - d.round()).abs() < 0.01 {
        format!("{:.0} {}", d.round(), tr("days"))
    } else {
        format!("{d:.2} {}", tr("days"))
    }
}

/// The comparison as a report page.
pub fn comparison_page(app: &mut App, ui: &mut Ui) {
    let Some(c) = app.comparison.clone() else {
        ui.label(tr("Report > Compare Projects picks the previous version of this project."));
        return;
    };
    let mpd = app.doc.calendar.minutes_per_day();
    let count = |s: Status| c.rows.iter().filter(|r| r.status == s).count();
    ui.label(RichText::new(format!("{} {}", tr("Previous version:"), c.previous)).color(theme::TEXT_DIM));
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        for (s, col) in [(Status::Added, theme::GREEN), (Status::Deleted, Color32::from_rgb(0xD1, 0x3B, 0x2E)), (Status::Changed, Color32::from_rgb(0xC5, 0x7A, 0x00)), (Status::Same, theme::TEXT_DIM)] {
            ui.vertical(|ui| {
                ui.set_min_width(130.0);
                ui.label(RichText::new(count(s).to_string()).size(24.0).color(col));
                ui.label(RichText::new(tr(s.label())).color(theme::TEXT_DIM));
            });
        }
    });
    ui.add_space(6.0);
    let mut only = c.only_changes;
    ui.horizontal(|ui| {
        if ui.checkbox(&mut only, tr("Show only differences")).changed() {
            if let Some(cc) = app.comparison.as_mut() {
                cc.only_changes = only;
            }
        }
        if ui.button(tr("Export to CSV...")).clicked() {
            if let Some(p) = rfd::FileDialog::new().set_file_name("comparison.csv").add_filter("CSV", &["csv"]).save_file() {
                match std::fs::write(&p, to_csv(app, &c)) {
                    Ok(()) => app.status = crate::i18n::trf("Saved {}", &[&p.display().to_string()]),
                    Err(e) => app.show_error(format!("Cannot write the file: {e}")),
                }
            }
        }
    });
    ui.add_space(8.0);
    let f = |v: &Option<Vals>, k: u8| -> String {
        let Some(v) = v else { return String::new() };
        match k {
            0 => v.start.map(|d| app.fmt_date(d.date())).unwrap_or_else(|| "NA".into()),
            1 => v.finish.map(|d| app.fmt_finish(d.date())).unwrap_or_else(|| "NA".into()),
            2 => days(v.dur_min, mpd),
            3 => format!("{:.0} {}", v.work_min as f64 / 60.0, tr("hrs")),
            4 => crate::util::fmt_money(v.cost),
            _ => format!("{}%", v.pct),
        }
    };
    let diff = |r: &Row, k: u8| -> String {
        let (Some(a), Some(b)) = (&r.cur, &r.prev) else { return String::new() };
        match k {
            0 | 1 => {
                let (x, y) = if k == 0 { (a.start, b.start) } else { (a.finish, b.finish) };
                match (x, y) {
                    (Some(x), Some(y)) if x != y => days(app.doc.calendar.work_minutes(y, x), mpd),
                    _ => String::new(),
                }
            }
            2 if a.dur_min != b.dur_min => days(a.dur_min - b.dur_min, mpd),
            3 if a.work_min != b.work_min => format!("{:.0} {}", (a.work_min - b.work_min) as f64 / 60.0, tr("hrs")),
            4 if (a.cost - b.cost).abs() >= 0.005 => crate::util::fmt_money(a.cost - b.cost),
            5 if a.pct != b.pct => format!("{}%", a.pct as i32 - b.pct as i32),
            _ => String::new(),
        }
    };
    egui::Grid::new("compare_grid").striped(true).spacing([14.0, 3.0]).show(ui, |ui| {
        let mut head = vec![tr("Status").to_string(), tr("Name").to_string()];
        for k in ["Start", "Finish", "Duration", "Work", "Cost", "% Complete"] {
            head.push(format!("{}: {}", tr(k), tr("Current")));
            head.push(format!("{}: {}", tr(k), tr("Previous")));
            head.push(format!("{}: {}", tr(k), tr("Difference")));
        }
        for h in &head {
            ui.label(RichText::new(h.as_str()).strong().color(theme::GREEN));
        }
        ui.end_row();
        for r in c.rows.iter().filter(|r| !only || r.status != Status::Same).take(3000) {
            let col = match r.status {
                Status::Added => theme::GREEN,
                Status::Deleted => Color32::from_rgb(0xD1, 0x3B, 0x2E),
                Status::Changed => Color32::from_rgb(0xC5, 0x7A, 0x00),
                Status::Same => theme::TEXT_DIM,
            };
            ui.label(RichText::new(tr(r.status.label())).color(col));
            let indent = "   ".repeat(r.level.saturating_sub(1) as usize);
            let bold = r.cur.as_ref().or(r.prev.as_ref()).map(|v| v.summary).unwrap_or(false);
            let name = RichText::new(crate::thai::shape(&format!("{indent}{}", r.name)).to_string());
            ui.label(if bold { name.strong() } else { name });
            for k in 0..6u8 {
                ui.label(f(&r.cur, k));
                ui.label(f(&r.prev, k));
                let d = diff(r, k);
                ui.label(RichText::new(d).color(Color32::from_rgb(0xC5, 0x7A, 0x00)));
            }
            ui.end_row();
        }
    });
    if !c.resources.is_empty() {
        ui.add_space(14.0);
        ui.label(RichText::new(tr("Resources")).size(16.0).strong());
        egui::Grid::new("compare_res").striped(true).spacing([14.0, 3.0]).show(ui, |ui| {
            for h in ["Status", "Name", "Work: Current", "Work: Previous", "Cost: Current", "Cost: Previous"] {
                ui.label(RichText::new(tr(h)).strong().color(theme::GREEN));
            }
            ui.end_row();
            for (name, s, a, b) in c.resources.iter().filter(|r| !only || r.1 != Status::Same) {
                ui.label(tr(s.label()));
                ui.label(crate::thai::shape(name).to_string());
                ui.label(a.map(|x| format!("{:.0} {}", x.0 as f64 / 60.0, tr("hrs"))).unwrap_or_default());
                ui.label(b.map(|x| format!("{:.0} {}", x.0 as f64 / 60.0, tr("hrs"))).unwrap_or_default());
                ui.label(a.map(|x| crate::util::fmt_money(x.1)).unwrap_or_default());
                ui.label(b.map(|x| crate::util::fmt_money(x.1)).unwrap_or_default());
                ui.end_row();
            }
        });
    }
}

fn to_csv(app: &App, c: &Comparison) -> String {
    let q = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
    let mut out = String::from("\u{feff}Status,Name,Start Current,Start Previous,Finish Current,Finish Previous,Duration Current (min),Duration Previous (min),Work Current (min),Work Previous (min),Cost Current,Cost Previous,% Complete Current,% Complete Previous\r\n");
    let d = |v: Option<NaiveDateTime>| v.map(|x| app.fmt_date(x.date())).unwrap_or_default();
    for r in c.rows.iter().filter(|r| !c.only_changes || r.status != Status::Same) {
        let (a, b) = (r.cur.as_ref(), r.prev.as_ref());
        let cols = [
            r.status.label().to_string(),
            r.name.clone(),
            d(a.and_then(|v| v.start)),
            d(b.and_then(|v| v.start)),
            d(a.and_then(|v| v.finish)),
            d(b.and_then(|v| v.finish)),
            a.map(|v| v.dur_min.to_string()).unwrap_or_default(),
            b.map(|v| v.dur_min.to_string()).unwrap_or_default(),
            a.map(|v| v.work_min.to_string()).unwrap_or_default(),
            b.map(|v| v.work_min.to_string()).unwrap_or_default(),
            a.map(|v| format!("{:.2}", v.cost)).unwrap_or_default(),
            b.map(|v| format!("{:.2}", v.cost)).unwrap_or_default(),
            a.map(|v| v.pct.to_string()).unwrap_or_default(),
            b.map(|v| v.pct.to_string()).unwrap_or_default(),
        ];
        out.push_str(&cols.iter().map(|s| q(s)).collect::<Vec<_>>().join(","));
        out.push_str("\r\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Task;

    #[test]
    fn added_deleted_changed() {
        let mut a = Project::new();
        let mut b = Project::new();
        let s = a.start;
        for (k, n) in ["A", "B", "C"].iter().enumerate() {
            b.tasks.push(Task::new(k as u32 + 1, n, s));
        }
        a.tasks.push(Task::new(1, "A", s));
        let mut c = Task::new(3, "C", s);
        c.percent_complete = 50;
        a.tasks.push(c);
        a.tasks.push(Task::new(4, "D", s));
        let cmp = compare(&a, &b, "prev");
        let st: Vec<(String, Status)> = cmp.rows.iter().map(|r| (r.name.clone(), r.status)).collect();
        assert_eq!(
            st,
            vec![("A".into(), Status::Same), ("B".into(), Status::Deleted), ("C".into(), Status::Changed), ("D".into(), Status::Added)]
        );
    }
}
