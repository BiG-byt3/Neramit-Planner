//! Gantt chart (timescale, bars, links, drag editing) and the Timeline strip.
//!
//! Everything drawn here is driven by the project's formatting (`format.rs`): bar styles decide
//! which bars a task gets and how they look, the Layout dialog sets links / bar height / rollup,
//! the Gridlines dialog the lines, the Timescale dialog the tiers. Dates and tier labels follow the
//! Windows regional settings, like MPP.

use crate::i18n::{tr};
use crate::app::{App, BarDrag, BarDragKind};
use crate::format::{BarField, BarStyle, BarText, Category, GridKind, LineType, LinkStyle, MidShape, Pattern, TextItem, TsUnit};
use crate::grid::{clamp_scroll, header_h, SCROLLBAR};
use crate::locale::Locale;
use crate::model::{DurUnit, LinkType, Project, Task};
use crate::ribbon::Cmd;
use crate::theme;
use chrono::{Datelike, Duration, NaiveDate};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2};

struct Scale {
    origin: NaiveDate,
    ppd: f32,
    x0: f32,
    scroll: f32,
}

impl Scale {
    fn x(&self, d: NaiveDate) -> f32 {
        self.x0 + (d - self.origin).num_days() as f32 * self.ppd - self.scroll
    }
    fn date_at(&self, x: f32) -> NaiveDate {
        let days = ((x - self.x0 + self.scroll) / self.ppd).floor() as i64;
        self.origin + Duration::days(days)
    }
}

fn add_months(d: NaiveDate, m: i32) -> NaiveDate {
    let total = d.year() * 12 + d.month0() as i32 + m;
    NaiveDate::from_ymd_opt(total.div_euclid(12), total.rem_euclid(12) as u32 + 1, 1).unwrap_or(d)
}

/// Timescale units (the standard TimescaleUnit, minus Minutes).
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Unit {
    /// hours, labelled every `n` hours
    Hour(u32),
    Day,
    Week,
    ThirdOfMonth,
    Month,
    Quarter,
    HalfYear,
    Year,
    FiveYears,
}

/// Working hours used to place bars and shade non-working time at hour zoom (Standard calendar).
const WORK_START_H: f32 = 8.0;
const WORK_END_H: f32 = 17.0;
/// px per day from which the bottom tier shows hours and bars snap to working hours.
const HOURS_PPD: f32 = 120.0;

pub(crate) fn unit_start(d: NaiveDate, u: Unit) -> NaiveDate {
    let ymd = |y, m, day| NaiveDate::from_ymd_opt(y, m, day).unwrap();
    match u {
        Unit::Hour(_) | Unit::Day => d,
        Unit::Week => d - Duration::days(d.weekday().num_days_from_sunday() as i64),
        Unit::ThirdOfMonth => ymd(d.year(), d.month(), [1, 11, 21][((d.day() - 1) / 10).min(2) as usize]),
        Unit::Month => ymd(d.year(), d.month(), 1),
        Unit::Quarter => ymd(d.year(), (d.month0() / 3) * 3 + 1, 1),
        Unit::HalfYear => ymd(d.year(), (d.month0() / 6) * 6 + 1, 1),
        Unit::Year => ymd(d.year(), 1, 1),
        Unit::FiveYears => ymd(d.year() - d.year().rem_euclid(5), 1, 1),
    }
}

pub(crate) fn unit_next(d: NaiveDate, u: Unit) -> NaiveDate {
    match u {
        Unit::Hour(_) | Unit::Day => d + Duration::days(1),
        Unit::Week => d + Duration::days(7),
        Unit::ThirdOfMonth if d.day() < 21 => d + Duration::days(10),
        Unit::ThirdOfMonth | Unit::Month => add_months(d, 1),
        Unit::Quarter => add_months(d, 3),
        Unit::HalfYear => add_months(d, 6),
        Unit::Year => add_months(d, 12),
        Unit::FiveYears => add_months(d, 60),
    }
}

/// Automatic tier labels (Zoom presets), in the Windows locale like MPP.
pub(crate) fn unit_label(d: NaiveDate, u: Unit, top: bool, width: f32, loc: &Locale) -> String {
    let yy = format!("'{:02}", loc.year(d).rem_euclid(100));
    match u {
        Unit::Hour(_) => String::new(),
        Unit::Day => {
            if top {
                format!("{} {}, {yy}", loc.day_abbr(d), loc.mmm_dd(d))
            } else if width >= 64.0 {
                format!("{} {}", loc.day_abbr(d), d.day())
            } else if width >= 34.0 {
                loc.day_abbr(d).to_string()
            } else {
                loc.day_abbr(d).chars().take(1).collect()
            }
        }
        Unit::Week => {
            if top {
                format!("{}, {yy}", loc.mmm_dd(d))
            } else {
                d.day().to_string()
            }
        }
        // MPP labels thirds of months B(eginning) / M(iddle) / E(nd)
        Unit::ThirdOfMonth => ["B", "M", "E"][((d.day() - 1) / 10).min(2) as usize].to_string(),
        Unit::Month => {
            if top && width > 110.0 {
                format!("{} {}", loc.month_long[d.month0() as usize], loc.year(d))
            } else if top {
                format!("{} {yy}", loc.month_abbr(d))
            } else if width > 26.0 {
                loc.month_abbr(d).to_string()
            } else {
                loc.month_abbr(d).chars().take(1).collect()
            }
        }
        Unit::Quarter => {
            if top {
                format!("Qtr {}, {}", d.month0() / 3 + 1, loc.year(d))
            } else {
                format!("Q{}", d.month0() / 3 + 1)
            }
        }
        Unit::HalfYear => {
            if top {
                format!("H{}, {}", d.month0() / 6 + 1, loc.year(d))
            } else {
                format!("H{}", d.month0() / 6 + 1)
            }
        }
        Unit::Year => {
            if top || width >= 34.0 {
                loc.year(d).to_string()
            } else {
                yy
            }
        }
        Unit::FiveYears => format!("{} - {}", loc.year(d), loc.year(d) + 4),
    }
}

/// Top and bottom tier for a zoom level, following the standard zoom presets.
pub(crate) fn tiers(ppd: f32) -> (Unit, Unit) {
    if ppd >= HOURS_PPD {
        let per_hour = ppd / 24.0;
        let n = [1, 2, 3, 6, 12].into_iter().find(|n| *n as f32 * per_hour >= 30.0).unwrap_or(12);
        (Unit::Day, Unit::Hour(n))
    } else if ppd >= 12.0 {
        (Unit::Week, Unit::Day)
    } else if ppd >= 4.0 {
        (Unit::Month, Unit::Week)
    } else if ppd >= 2.0 {
        (Unit::Month, Unit::ThirdOfMonth)
    } else if ppd >= 0.8 {
        (Unit::Quarter, Unit::Month)
    } else if ppd >= 0.3 {
        (Unit::Year, Unit::Quarter)
    } else if ppd >= 0.12 {
        (Unit::Year, Unit::HalfYear)
    } else {
        (Unit::FiveYears, Unit::Year)
    }
}

/// The automatic (middle, bottom) tier units at a zoom level, for the Timescale dialog.
pub fn auto_tiers(ppd: f32) -> (TsUnit, TsUnit) {
    let conv = |u: Unit| match u {
        Unit::Hour(_) => TsUnit::Hours,
        Unit::Day => TsUnit::Days,
        Unit::Week => TsUnit::Weeks,
        Unit::ThirdOfMonth => TsUnit::ThirdsOfMonths,
        Unit::Month => TsUnit::Months,
        Unit::Quarter => TsUnit::Quarters,
        Unit::HalfYear => TsUnit::HalfYears,
        Unit::Year | Unit::FiveYears => TsUnit::Years,
    };
    let (a, b) = tiers(ppd);
    (conv(a), conv(b))
}

/// Name of the bottom timescale tier (View > Zoom > Timescale box).
pub fn timescale_name(ppd: f32) -> &'static str {
    match tiers(ppd).1 {
        Unit::Hour(_) => "Hours",
        Unit::Day => "Days",
        Unit::Week => "Weeks",
        Unit::ThirdOfMonth => "Thirds of Months",
        Unit::Month => "Months",
        Unit::Quarter => "Quarters",
        Unit::HalfYear => "Half Years",
        Unit::Year | Unit::FiveYears => "Years",
    }
}

/// Timescale presets offered in the Timescale box: (bottom tier, px per day).
pub const TIMESCALES: [(&str, f32); 8] = [
    ("Hours", 300.0),
    ("Days", 24.0),
    ("Weeks", 7.0),
    ("Thirds of Months", 2.6),
    ("Months", 1.2),
    ("Quarters", 0.45),
    ("Half Years", 0.2),
    ("Years", 0.08),
];

pub(crate) fn ts_unit(u: TsUnit, ppd: f32) -> Unit {
    match u {
        TsUnit::Years => Unit::Year,
        TsUnit::HalfYears => Unit::HalfYear,
        TsUnit::Quarters => Unit::Quarter,
        TsUnit::Months => Unit::Month,
        TsUnit::ThirdsOfMonths => Unit::ThirdOfMonth,
        TsUnit::Weeks => Unit::Week,
        TsUnit::Days => Unit::Day,
        TsUnit::Hours | TsUnit::Minutes => {
            let per_hour = ppd / 24.0;
            Unit::Hour([1, 2, 3, 6, 12].into_iter().find(|n| *n as f32 * per_hour >= 30.0).unwrap_or(12))
        }
    }
}

/// Label of a configured tier (`Tier::label` index into `TsUnit::label_formats`).
pub fn tier_label(u: TsUnit, fmt: u8, d: NaiveDate, loc: &Locale, project_start: NaiveDate, project_finish: NaiveDate) -> String {
    let yy = format!("'{:02}", loc.year(d).rem_euclid(100));
    let year = loc.year(d);
    let months_from = |a: NaiveDate, b: NaiveDate| (a.year() * 12 + a.month0() as i32) - (b.year() * 12 + b.month0() as i32);
    let from_start = |diff: i32| if diff >= 0 { diff + 1 } else { diff };
    match u {
        TsUnit::Years => match fmt {
            0 => year.to_string(),
            1 => yy,
            2 => format!("Year {}", from_start(d.year() - project_start.year())),
            3 => format!("Y{}", from_start(d.year() - project_start.year())),
            _ => format!("Year {}", from_start(project_finish.year() - d.year())),
        },
        TsUnit::HalfYears => {
            let h = d.month0() / 6 + 1;
            match fmt {
                0 => format!("{} Half {year}", if h == 1 { "1st" } else { "2nd" }),
                1 => format!("{} Half", if h == 1 { "1st" } else { "2nd" }),
                2 => format!("H{h} {yy}"),
                3 => format!("H{h}"),
                4 => format!("Half {h}"),
                5 => format!("Half {}", from_start(months_from(d, project_start).div_euclid(6))),
                _ => format!("Half {h}, {year}"),
            }
        }
        TsUnit::Quarters => {
            let q = d.month0() / 3 + 1;
            match fmt {
                0 => format!("{} Quarter {year}", ["1st", "2nd", "3rd", "4th"][q as usize - 1]),
                1 => format!("Qtr {q}, {year}"),
                2 => format!("Q{q} {yy}"),
                3 => format!("Q{q}"),
                4 => q.to_string(),
                5 => format!("Quarter {q}"),
                _ => format!("Q{} (From Start)", from_start(months_from(d, project_start).div_euclid(3))),
            }
        }
        TsUnit::Months => match fmt {
            0 => format!("{} {year}", loc.month_long[d.month0() as usize]),
            1 => format!("{} {yy}", loc.month_abbr(d)),
            2 => loc.month_abbr(d).to_string(),
            3 => loc.month_abbr(d).chars().take(1).collect(),
            4 => d.month().to_string(),
            5 => format!("Month {}", from_start(months_from(d, project_start))),
            6 => format!("M{}", from_start(months_from(d, project_start))),
            _ => format!("Month {} (From Start)", from_start(months_from(d, project_start))),
        },
        TsUnit::ThirdsOfMonths => {
            let k = ((d.day() - 1) / 10).min(2) as usize;
            match fmt {
                0 => ["B", "M", "E"][k].to_string(),
                1 => ["1", "11", "21"][k].to_string(),
                _ => ["Beginning", "Middle", "End"][k].to_string(),
            }
        }
        TsUnit::Weeks => match fmt {
            0 => format!("{}, {year}", loc.mmmm_dd(d)),
            1 => format!("{}, {yy}", loc.mmm_dd(d)),
            2 => loc.mmm_dd(d),
            3 => d.day().to_string(),
            4 => format!("{} {}", loc.day_abbr(d), loc.mmm_dd(d)),
            5 => format!("W{}", d.iso_week().week()),
            6 => format!("Week {}", d.iso_week().week()),
            7 => format!("W{} (From Start)", from_start(((d - project_start).num_days() as i32).div_euclid(7))),
            _ => loc.month_day(d),
        },
        TsUnit::Days => match fmt {
            0 => format!("{}, {}, {year}", loc.day_long[d.weekday().num_days_from_monday() as usize], loc.mmmm_dd(d)),
            1 => format!("{} {}, {yy}", loc.day_abbr(d), loc.mmm_dd(d)),
            2 => format!("{} {}", loc.day_abbr(d), loc.mmm_dd(d)),
            3 => format!("{} {}", loc.day_abbr(d), d.day()),
            4 => loc.mmm_dd(d),
            5 => d.day().to_string(),
            6 => loc.day_abbr(d).chars().take(1).collect(),
            7 => loc.month_day(d),
            8 => format!("Day {}", from_start((d - project_start).num_days() as i32)),
            _ => format!("D{}", from_start((d - project_start).num_days() as i32)),
        },
        TsUnit::Hours | TsUnit::Minutes => match fmt {
            0 => format!("{}, {}, {year}", loc.day_long[d.weekday().num_days_from_monday() as usize], loc.mmmm_dd(d)),
            1 => format!("{}, {yy}", loc.mmm_dd(d)),
            _ => String::new(),
        },
    }
}

/// One header tier to draw.
struct TierSpec {
    unit: Unit,
    /// tier of the Timescale dialog it corresponds to (fonts / configured labels)
    item: TextItem,
    cfg: Option<crate::format::Tier>,
    /// first (top-most) tier uses left-aligned labels in automatic mode
    top: bool,
}

fn tier_specs(app: &App) -> Vec<TierSpec> {
    let ts = &app.doc.formatting.timescale;
    let ppd = app.px_per_day;
    if ts.auto || app.usage_scale {
        let (a, b) = tiers(ppd);
        vec![TierSpec { unit: a, item: TextItem::MiddleTier, cfg: None, top: true }, TierSpec { unit: b, item: TextItem::BottomTier, cfg: None, top: false }]
    } else {
        let mut v = Vec::new();
        if ts.show == 3 {
            v.push(TierSpec { unit: ts_unit(ts.top.units, ppd), item: TextItem::TopTier, cfg: Some(ts.top.clone()), top: true });
        }
        v.push(TierSpec { unit: ts_unit(ts.middle.units, ppd), item: TextItem::MiddleTier, cfg: Some(ts.middle.clone()), top: v.is_empty() });
        if ts.show >= 2 {
            v.push(TierSpec { unit: ts_unit(ts.bottom.units, ppd), item: TextItem::BottomTier, cfg: Some(ts.bottom.clone()), top: false });
        }
        v
    }
}

fn draw_tier(app: &App, p: &egui::Painter, s: &Scale, r: Rect, spec: &TierSpec, title_lines: (LineType, Color32), tick_bottom: f32) {
    // the standard timescale header (measured): filled once, light ticks, no line between tiers
    let msp = true;
    let st = app.doc.formatting.text_style(spec.item);
    let mut font = theme::chrome_font(&st);
    let mut color = st.color.map(theme::rgb).unwrap_or(theme::TEXT);
    // the standard grey timescale text when the style's colour is automatic
    if msp && st.color.is_none() {
        color = theme::SHEET_HEAD_TEXT;
    }
    // the header is filled once (an upper tier's ticks run down through the tiers below)
    if !msp {
        p.rect_filled(r, 0.0, theme::HEADER_BG);
    }
    // MPP: light ticks from 5 px under the tier's top down to the header's bottom line
    let (tick_top, tick_end, title_lines) = if msp { (r.top() + 5.0, tick_bottom, (LineType::Solid, theme::SHEET_HEAD_SEP)) } else { (r.top(), r.bottom(), title_lines) };
    let pad = if msp { 6.0 } else { 4.0 };
    let _ = &mut font;
    let loc = &app.loc;
    let count = spec.cfg.as_ref().map(|c| c.count.max(1) as usize).unwrap_or(1);
    let ticks = spec.cfg.as_ref().map(|c| c.tick_lines).unwrap_or(true);
    let align = spec.cfg.as_ref().map(|c| c.align).unwrap_or(if spec.top { 0 } else { 1 });
    let (pstart, pfinish) = (app.doc.start, app.doc.finish_date());
    let mut d = unit_start(s.date_at(r.left()), spec.unit);
    // align counted units to the project start (every 2 weeks from the start, ...)
    if count > 1 {
        let mut anchor = unit_start(pstart, spec.unit);
        while anchor > d {
            for _ in 0..count {
                anchor = prev_unit(anchor, spec.unit);
            }
        }
        while {
            let mut n = anchor;
            for _ in 0..count {
                n = unit_next(n, spec.unit);
            }
            n <= d
        } {
            for _ in 0..count {
                anchor = unit_next(anchor, spec.unit);
            }
        }
        d = anchor;
    }
    let end = s.date_at(r.right()) + Duration::days(1);
    while d <= end {
        let mut nx = d;
        for _ in 0..count {
            nx = unit_next(nx, spec.unit);
        }
        let (x0, x1) = (s.x(d), s.x(nx));
        if ticks {
            theme::styled_line(p, Pos2::new(x0, tick_top), Pos2::new(x0, tick_end), title_lines.0, title_lines.1);
        }
        if let Unit::Hour(n) = spec.unit {
            // hour ticks inside the day: "12 AM", "6 AM", "12 PM" ..., or bare numbers when tight
            let per_hour = s.ppd / 24.0;
            for h in (0..24).step_by(n as usize) {
                let hx = x0 + h as f32 * per_hour;
                let w = n as f32 * per_hour;
                if h > 0 && ticks {
                    theme::styled_line(p, Pos2::new(hx, r.top() + 4.0), Pos2::new(hx, r.bottom()), title_lines.0, title_lines.1);
                }
                let label = match spec.cfg.as_ref().map(|c| c.label).unwrap_or(2) {
                    3 => (if h % 12 == 0 { 12 } else { h % 12 }).to_string(),
                    4 => format!("Hour {}", h + 1),
                    _ => {
                        let t = loc.time(h, 0);
                        if w >= 44.0 { t } else { (if h % 12 == 0 { 12 } else { h % 12 }).to_string() }
                    }
                };
                p.text(Pos2::new(hx + w / 2.0, r.center().y), Align2::CENTER_CENTER, label, font.clone(), color);
            }
            d = nx;
            continue;
        }
        let label = match &spec.cfg {
            Some(c) => tier_label(c.units, c.label, d, loc, pstart, pfinish),
            None => unit_label(d, spec.unit, spec.top, x1 - x0, loc),
        };
        let clip = p.with_clip_rect(Rect::from_min_max(Pos2::new(x0.max(r.left()), r.top()), Pos2::new(x1.min(r.right()), r.bottom())));
        let mut label = label;
        let mut lw = p.layout_no_wrap(label.clone(), font.clone(), color).size().x;
        if lw > x1 - x0 - 2.0 * pad {
            // the file's label format does not fit the unit: the shorter automatic label instead
            // ("Aug '26" or "Aug" rather than a cut "August 20")
            let short = unit_label(d, spec.unit, spec.top, x1 - x0, loc);
            let sw = p.layout_no_wrap(short.clone(), font.clone(), color).size().x;
            if sw < lw {
                label = short;
                lw = sw;
            }
        }
        match align {
            0 => {
                // keep the label of a partly scrolled-off unit readable at the left edge
                let lx = (x0 + pad).max(r.left() + pad).min(x1 - lw - 4.0).max(x0 + pad);
                clip.text(Pos2::new(lx, r.center().y), Align2::LEFT_CENTER, label, font.clone(), color);
            }
            2 => {
                clip.text(Pos2::new(x1 - 4.0, r.center().y), Align2::RIGHT_CENTER, label, font.clone(), color);
            }
            _ => {
                clip.text(Pos2::new((x0 + x1) / 2.0, r.center().y), Align2::CENTER_CENTER, label, font.clone(), color);
            }
        }
        d = nx;
    }
    if !msp {
        p.line_segment([r.left_bottom(), r.right_bottom()], Stroke::new(1.0_f32, title_lines.1));
    }
}

fn prev_unit(d: NaiveDate, u: Unit) -> NaiveDate {
    match u {
        Unit::Hour(_) | Unit::Day => d - Duration::days(1),
        Unit::Week => d - Duration::days(7),
        Unit::ThirdOfMonth => unit_start(d - Duration::days(1), u),
        Unit::Month => add_months(d, -1),
        Unit::Quarter => add_months(d, -3),
        Unit::HalfYear => add_months(d, -6),
        Unit::Year => add_months(d, -12),
        Unit::FiveYears => add_months(d, -60),
    }
}

/// Arrow head pointing in direction `dir` at `tip`.
fn arrow(p: &egui::Painter, tip: Pos2, dir: Vec2, col: Color32) {
    let d = dir.normalized() * 5.0;
    let n = Vec2::new(-d.y, d.x) * 0.8;
    p.add(Shape::convex_polygon(vec![tip, tip - d + n, tip - d - n], col, Stroke::NONE));
}

// ======================================================================= bar styles engine

/// Categories a task belongs to, for "Show For ... Tasks".
fn categories(doc: &Project, t: &Task) -> Vec<Category> {
    let mut v = Vec::new();
    if t.external.is_some() {
        v.push(Category::External);
    }
    let milestone = t.is_milestone();
    if !t.is_summary && !milestone {
        v.push(Category::Normal);
    }
    if milestone {
        v.push(Category::Milestone);
    }
    if t.is_summary {
        v.push(Category::Summary);
    }
    if t.is_summary && t.outline_level == 0 {
        v.push(Category::ProjectSummary);
    }
    if t.critical && doc.formatting.show_critical {
        v.push(Category::Critical);
    } else {
        v.push(Category::Noncritical);
    }
    if t.marked {
        v.push(Category::Marked);
    }
    match t.percent_complete {
        100 => v.push(Category::Finished),
        0 => v.push(Category::NotStarted),
        _ => v.push(Category::InProgress),
    }
    if t.percent_complete < 100 {
        v.push(Category::NotFinished);
    }
    if t.rollup {
        v.push(Category::RolledUp);
    }
    if !t.splits.is_empty() {
        v.push(Category::Split);
    }
    if t.manual {
        v.push(Category::ManuallyScheduled);
    }
    if t.placeholder {
        v.push(Category::Placeholder);
    }
    if t.active {
        v.push(Category::Active);
    }
    if t.deadline.map(|d| t.finish > d).unwrap_or(false) && doc.formatting.show_late {
        v.push(Category::Late);
    }
    v
}

fn style_applies(doc: &Project, st: &BarStyle, t: &Task, cats: &[Category]) -> bool {
    if st.from == BarField::Deadline || st.to == BarField::Deadline {
        return t.deadline.is_some() && !t.is_summary;
    }
    if st.show_for.is_empty() {
        return false;
    }
    // Slack bars only with Format > Slack
    if st.to == BarField::LateFinish && !doc.formatting.show_slack {
        return false;
    }
    st.show_for.iter().all(|sf| cats.contains(&sf.cat) != sf.not)
}

/// Text shown next to a bar for a bar-text field.
fn bar_text_value(app: &App, i: usize, field: BarText) -> String {
    let t = &app.doc.tasks[i];
    match field {
        BarText::None => String::new(),
        BarText::Name => t.name.clone(),
        BarText::Start => app.fmt_bar_date(t.start),
        BarText::Finish => app.fmt_bar_date(t.finish),
        BarText::Duration => app.task_duration_text(t),
        BarText::PercentComplete => format!("{}%", t.percent_complete),
        BarText::ResourceNames => app.doc.resource_names(t),
        BarText::ResourceInitials => t.resources.iter().filter_map(|u| app.doc.resource_by_uid(*u)).map(|r| r.initials.clone()).collect::<Vec<_>>().join(","),
        BarText::Deadline => t.deadline.map(|d| app.fmt_bar_date(d)).unwrap_or_default(),
        BarText::Predecessors => app.cell_text(i, crate::app::TCol::Predecessors),
        BarText::Successors => app.cell_text(i, crate::app::TCol::Successors),
        BarText::Priority => t.priority.to_string(),
        BarText::Notes => t.notes.replace('\n', " "),
        BarText::Wbs => app.cell_text(i, crate::app::TCol::Wbs),
        BarText::Cost => crate::util::fmt_money(t.cost),
        BarText::Id => (i + 1).to_string(),
        BarText::UniqueId => t.uid.to_string(),
        BarText::OutlineNumber => app.doc.outline_numbers()[i].clone(),
        BarText::TotalSlack => crate::fileio::format_duration(t.total_slack),
        BarText::Contact => t.contact.clone(),
    }
}

/// Screen x of a bar date field (None when the task has no such date). Returns (x, end_of_day).
/// Screen x of an instant: MPP draws bars by the exact time of day (a 1-day task runs from
/// 08:00 to 17:00, so consecutive days leave the night between their bars).
/// Part of a bar under the pointer.
#[derive(Clone, Copy, PartialEq)]
enum BarZone {
    Middle,
    End,
    Progress,
}

/// What releasing a bar drag does.
enum DragEnd {
    None,
    Move(chrono::NaiveDateTime),
    Resize(i64),
    Progress(u8),
    Link(usize),
    Draw(chrono::NaiveDateTime, i64),
    /// split k's new gap (working minutes; 0 joins the pieces)
    Gap(usize, i64),
}

fn dist_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_sq();
    if len2 <= f32::EPSILON {
        return (p - a).length();
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    (p - (a + ab * t)).length()
}

fn x_at(s: &Scale, t: chrono::NaiveDateTime) -> f32 {
    use chrono::Timelike;
    s.x(t.date()) + s.ppd * (t.hour() * 60 + t.minute()) as f32 / 1440.0
}

/// Exact start / finish of a task (from the scheduler; the working day's hours otherwise).
pub(crate) fn task_times(app: &App, t: &Task) -> (chrono::NaiveDateTime, chrono::NaiveDateTime) {
    let cal = &app.doc.calendar;
    let st = t.start_dt.unwrap_or_else(|| cal.day_start(t.start));
    let ft = t.finish_dt.unwrap_or_else(|| if t.is_milestone() { st } else { cal.day_end(t.finish) });
    (st, ft)
}

fn field_x(app: &App, s: &Scale, t: &Task, f: BarField, _hours: bool) -> Option<f32> {
    let cal = &app.doc.calendar;
    let end_of = |d: NaiveDate| x_at(s, cal.day_end(d));
    let (st, ft) = task_times(app, t);
    Some(match f {
        BarField::Start | BarField::EarlyStart => x_at(s, st),
        BarField::Finish | BarField::EarlyFinish => x_at(s, ft),
        BarField::ActualStart => {
            if t.percent_complete == 0 && t.actual_start.is_none() {
                return None;
            }
            x_at(s, t.actual_start.unwrap_or(st))
        }
        BarField::ActualFinish => x_at(s, t.actual_finish?),
        BarField::CompleteThrough => {
            if t.percent_complete == 0 {
                return None;
            }
            match t.actual_start {
                // the actual duration counted from the actual start (skips non-working time)
                Some(a) if !t.is_summary => {
                    let through = if t.percent_complete == 100 { t.actual_finish.unwrap_or(ft) } else { t.advance(cal, a, t.actual_minutes(cal.minutes_per_day())) };
                    x_at(s, through)
                }
                _ => {
                    let a = x_at(s, st);
                    let b = x_at(s, ft);
                    a + (b - a) * t.percent_complete as f32 / 100.0
                }
            }
        }
        BarField::Deadline => end_of(t.deadline?),
        BarField::BaselineStart => x_at(s, t.baseline(0)?.start?),
        BarField::BaselineFinish => x_at(s, t.baseline(0)?.finish?),
        BarField::LateStart | BarField::LateFinish => {
            let k = t.slack_min;
            let x = if f == BarField::LateStart { st } else { ft };
            let y = if k > 0 {
                let y = cal.add_work_minutes(x, k);
                if f == BarField::LateStart { cal.next_work_instant(y) } else { y }
            } else if k < 0 {
                cal.sub_work_minutes(x, -k)
            } else {
                x
            };
            x_at(s, y)
        }
    })
}

/// Draw the middle part of a bar between x0 and x1 in the task row.
fn draw_middle(p: &egui::Painter, x0: f32, x1: f32, cy: f32, bar_h: f32, m: crate::format::MidStyle) -> Option<Rect> {
    let col = theme::rgb(m.color);
    let full = Rect::from_min_max(Pos2::new(x0, cy - bar_h / 2.0), Pos2::new(x1.max(x0 + 1.0), cy + bar_h / 2.0));
    let r = match m.shape {
        MidShape::None => return None,
        MidShape::Full => full,
        MidShape::Top => Rect::from_min_max(full.min, Pos2::new(full.right(), full.top() + (bar_h * 0.5).max(3.0))),
        MidShape::Middle => Rect::from_center_size(full.center(), Vec2::new(full.width(), (bar_h * 0.4).max(3.0))),
        MidShape::Bottom => Rect::from_min_max(Pos2::new(full.left(), full.bottom() - (bar_h * 0.5).max(3.0)), full.max),
        MidShape::LineTop => Rect::from_min_max(full.min, Pos2::new(full.right(), full.top() + 2.0)),
        MidShape::LineMiddle => Rect::from_center_size(full.center(), Vec2::new(full.width(), 2.0)),
        MidShape::LineBottom => Rect::from_min_max(Pos2::new(full.left(), full.bottom() - 2.0), full.max),
    };
    if matches!(m.shape, MidShape::LineTop | MidShape::LineMiddle | MidShape::LineBottom) {
        p.rect_filled(r, 0.0, col);
    } else {
        theme::pattern_fill(p, r, m.pattern, col);
        if m.pattern != Pattern::Solid || true {
            p.rect_stroke(r, 0.0, Stroke::new(1.0_f32, col), egui::StrokeKind::Inside);
        }
    }
    Some(r)
}

pub fn gantt(app: &mut App, ui: &mut Ui, rect: Rect, cmds: &mut Vec<Cmd>) {
    let hh = header_h(app);
    let vscroll_rect = Rect::from_min_max(Pos2::new(rect.right() - SCROLLBAR, rect.top() + hh), Pos2::new(rect.right(), rect.bottom() - SCROLLBAR));
    let hscroll_rect = Rect::from_min_max(Pos2::new(rect.left(), rect.bottom() - SCROLLBAR), Pos2::new(rect.right() - SCROLLBAR, rect.bottom()));
    let chart = Rect::from_min_max(rect.min, Pos2::new(rect.right() - SCROLLBAR, rect.bottom() - SCROLLBAR));
    let header = Rect::from_min_size(chart.min, Vec2::new(chart.width(), hh));
    let body = Rect::from_min_max(Pos2::new(chart.left(), chart.top() + hh), chart.max);
    // Format > Project Summary Task: its row above the tasks (as in the sheet)
    let ps_band = app.shows_project_summary().then(|| Rect::from_min_size(body.min, Vec2::new(body.width(), crate::grid::ROW_H)));
    let body = match ps_band {
        Some(b) => Rect::from_min_max(Pos2::new(body.left(), b.bottom()), body.max),
        None => body,
    };
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, Color32::WHITE);

    app.chart_view = (chart.left(), chart.width());
    let origin = app.gantt_origin();
    let ppd = app.px_per_day;
    let hours = ppd >= HOURS_PPD;
    let s = Scale { origin, ppd, x0: chart.left(), scroll: app.scroll_x };
    let fmt = app.doc.formatting.clone();
    let specs = tier_specs(app);
    let gl = |k: GridKind| fmt.gridline(k);

    // ---------------- timescale header
    let hp = p.with_clip_rect(header);
    let tv = gl(GridKind::TitleVertical);
    let tv_line = (if tv.normal == LineType::None { LineType::Solid } else { tv.normal }, theme::rgb(tv.normal_color));
    let tier_h = hh / specs.len() as f32;
    hp.rect_filled(header, 0.0, theme::HEADER_BG);
    for (k, spec) in specs.iter().enumerate() {
        let r = Rect::from_min_max(Pos2::new(header.left(), header.top() + k as f32 * tier_h), Pos2::new(header.right(), header.top() + (k as f32 + 1.0) * tier_h));
        draw_tier(app, &hp, &s, r, spec, tv_line, header.bottom() - 1.0);
    }
    {
        // MPP: one grey line under the timescale, like under the column titles
        hp.line_segment([Pos2::new(header.left(), header.bottom() - 0.5), Pos2::new(header.right(), header.bottom() - 0.5)], Stroke::new(1.0_f32, theme::SHEET_HEAD_BOTTOM));
    }

    // ---------------- body background
    clamp_scroll(app, body.height());
    let bp = p.with_clip_rect(body);
    let nonwork = &fmt.timescale.nonworking;
    let shade_nonworking = |painter: &egui::Painter| {
        if nonwork.draw == 2 || ppd < 5.0 {
            return;
        }
        let col = theme::rgb(nonwork.color);
        let mut d = s.date_at(body.left());
        let end = s.date_at(body.right());
        while d <= end {
            let x = s.x(d);
            if !app.doc.calendar.is_working(d) {
                theme::pattern_fill(painter, Rect::from_min_max(Pos2::new(x, body.top()), Pos2::new(x + ppd, body.bottom())), nonwork.pattern, col);
            } else if hours {
                for (h0, h1) in [(0.0, WORK_START_H), (12.0, 13.0), (WORK_END_H, 24.0)] {
                    let r = Rect::from_min_max(Pos2::new(x + ppd * h0 / 24.0, body.top()), Pos2::new(x + ppd * h1 / 24.0, body.bottom()));
                    theme::pattern_fill(painter, r, nonwork.pattern, col);
                }
            }
            d += Duration::days(1);
        }
    };
    if nonwork.draw == 0 {
        shade_nonworking(&bp);
    }

    // vertical gridlines at tier boundaries
    let col_lines: Vec<(Unit, GridKind)> = match specs.len() {
        3 => vec![(specs[0].unit, GridKind::TopTierColumn), (specs[1].unit, GridKind::MiddleTierColumn), (specs[2].unit, GridKind::BottomTierColumn)],
        2 => vec![(specs[0].unit, GridKind::MiddleTierColumn), (specs[1].unit, GridKind::BottomTierColumn)],
        _ => vec![(specs[0].unit, GridKind::MiddleTierColumn)],
    };
    for (unit, kind) in col_lines.iter().rev() {
        let g = gl(*kind);
        if g.normal == LineType::None && g.interval == 0 {
            continue;
        }
        let mut d = unit_start(s.date_at(body.left()), *unit);
        let end = s.date_at(body.right());
        let mut n = 0usize;
        while d <= end {
            let x = s.x(d);
            let (lt, lc) = if g.interval > 1 && n % g.interval as usize == 0 && g.interval_type != LineType::None { (g.interval_type, g.interval_color) } else { (g.normal, g.normal_color) };
            theme::styled_line(&bp, Pos2::new(x, body.top()), Pos2::new(x, body.bottom()), lt, theme::rgb(lc));
            d = unit_next(d, *unit);
            n += 1;
        }
    }

    let vis = app.visible_rows();
    let first = app.row_at(app.scroll_y);
    let row_y = |app: &App, row: usize| body.top() + app.row_top(row) - app.scroll_y;
    let (sa, sb) = (app.sel_anchor.min(app.sel_row), app.sel_anchor.max(app.sel_row));

    // horizontal gridlines (Gantt Rows)
    let gr = gl(GridKind::GanttRows);
    if gr.normal != LineType::None || gr.interval > 1 {
        let mut row = first;
        loop {
            let y = row_y(app, row) + app.row_height(row);
            if y > body.bottom() || row > first + 400 {
                break;
            }
            let (lt, lc) = if gr.interval > 1 && (row + 1) % gr.interval as usize == 0 && gr.interval_type != LineType::None { (gr.interval_type, gr.interval_color) } else { (gr.normal, gr.normal_color) };
            theme::styled_line(&bp, Pos2::new(body.left(), y), Pos2::new(body.right(), y), lt, theme::rgb(lc));
            row += 1;
        }
    }

    // today, project start / finish, status date lines
    let now = chrono::Local::now().naive_local();
    let today = now.date();
    let cd = gl(GridKind::CurrentDate);
    if cd.normal != LineType::None {
        let tx = if hours {
            use chrono::Timelike;
            s.x(today) + ppd * (now.hour() as f32 + now.minute() as f32 / 60.0) / 24.0
        } else {
            s.x(today) + ppd / 2.0
        };
        theme::styled_line(&bp, Pos2::new(tx, body.top()), Pos2::new(tx, body.bottom()), cd.normal, theme::rgb(cd.normal_color));
    }
    let ps = gl(GridKind::ProjectStart);
    if ps.normal != LineType::None {
        let x = s.x(app.doc.start);
        theme::styled_line(&bp, Pos2::new(x, body.top()), Pos2::new(x, body.bottom()), ps.normal, theme::rgb(ps.normal_color));
    }
    let pf = gl(GridKind::ProjectFinish);
    if pf.normal != LineType::None {
        let x = s.x(app.doc.finish_date()) + ppd;
        theme::styled_line(&bp, Pos2::new(x, body.top()), Pos2::new(x, body.bottom()), pf.normal, theme::rgb(pf.normal_color));
    }
    if let Some(sd) = app.doc.status_date {
        let g = gl(GridKind::StatusDate);
        let lt = if g.normal == LineType::None { LineType::Dashed } else { g.normal };
        let x = s.x(sd) + ppd;
        theme::styled_line(&bp, Pos2::new(x, body.top()), Pos2::new(x, body.bottom()), lt, theme::rgb(g.normal_color));
    }

    // ---------------- bar geometry (main bar of every visible task)
    let bar_h = fmt.layout.bar_height as f32;
    // (bar, milestone, row, critical) by UID
    let mut geom: std::collections::HashMap<u32, (Rect, bool, usize, bool)> = std::collections::HashMap::with_capacity(vis.len());
    for (row, &i) in vis.iter().enumerate() {
        let t = &app.doc.tasks[i];
        let cy = row_y(app, row) + app.row_height(row) / 2.0;
        let (st, ft) = task_times(app, t);
        let (x0, x1) = (x_at(&s, st), x_at(&s, ft));
        let r = Rect::from_min_max(Pos2::new(x0, cy - bar_h / 2.0), Pos2::new(x1.max(x0 + 2.0), cy + bar_h / 2.0));
        geom.insert(t.uid, (r, t.is_milestone(), row, t.critical));
    }
    // last row on screen: links with both ends above or both below the view are not drawn
    let last = app.row_at(app.scroll_y + body.height());

    // ---------------- links (drawn below bars)
    // (successor index, predecessor UID, path) of every link drawn
    let mut link_hits: Vec<(usize, u32, Vec<Pos2>)> = Vec::new();
    if fmt.layout.links != LinkStyle::None {
        for &i in &vis {
            let t = &app.doc.tasks[i];
            let Some(&(sr, s_ms, s_row, _)) = geom.get(&t.uid) else { continue };
            if t.hide_bar {
                continue;
            }
            for l in &t.predecessors {
                let Some(&(pr, p_ms, p_row, pred_crit)) = geom.get(&l.pred_uid) else { continue };
                if (p_row < first && s_row < first) || (p_row > last + 1 && s_row > last + 1) {
                    continue;
                }
                // MPP colours a link line like its predecessor (red out of a critical task)
                let crit = fmt.show_critical && pred_crit;
                let col = if crit { theme::SHEET_LINK_CRIT } else { theme::SHEET_LINK };
                let st = Stroke::new(1.0_f32, col);
                let p_end = if p_ms { pr.left() + 6.0 } else { pr.right() };
                let p_start = if p_ms { pr.left() - 6.0 } else { pr.left() };
                let s_start = if s_ms { sr.left() - 6.0 } else { sr.left() };
                let s_end = if s_ms { sr.left() + 6.0 } else { sr.right() };
                let py = pr.center().y;
                let down = s_row > p_row;
                let s_edge_y = if down { sr.top() } else { sr.bottom() };
                let pts: Vec<Pos2> = if fmt.layout.links == LinkStyle::Straight {
                    let (a, b) = match l.kind {
                        LinkType::FS => (Pos2::new(p_end, py), Pos2::new(s_start, sr.center().y)),
                        LinkType::SS => (Pos2::new(p_start, py), Pos2::new(s_start, sr.center().y)),
                        LinkType::FF => (Pos2::new(p_end, py), Pos2::new(s_end, sr.center().y)),
                        LinkType::SF => (Pos2::new(p_start, py), Pos2::new(s_end, sr.center().y)),
                    };
                    arrow(&bp, b, b - a, col);
                    vec![a, b]
                } else {
                    match l.kind {
                        LinkType::FS => {
                            // MPP drops onto the successor whenever it starts at or after the
                            // predecessor's end (a 1-day LOI -> next day task gets the down arrow too)
                            // (-2.5: a 1-day bar is widened to 2 px when zoomed out)
                            let room = -2.5;
                            if s_start >= p_end + room {
                                let ax = s_start + 4.0_f32.min((s_end - s_start) / 2.0);
                                arrow(&bp, Pos2::new(ax, s_edge_y), if down { Vec2::Y } else { -Vec2::Y }, col);
                                vec![Pos2::new(p_end, py), Pos2::new(ax, py), Pos2::new(ax, s_edge_y)]
                            } else {
                                let mid_y = if down { row_y(app, s_row) } else { row_y(app, s_row) + app.row_height(s_row) };
                                arrow(&bp, Pos2::new(s_start, sr.center().y), Vec2::X, col);
                                vec![
                                    Pos2::new(p_end, py),
                                    Pos2::new(p_end + 6.0, py),
                                    Pos2::new(p_end + 6.0, mid_y),
                                    Pos2::new(s_start - 8.0, mid_y),
                                    Pos2::new(s_start - 8.0, sr.center().y),
                                    Pos2::new(s_start, sr.center().y),
                                ]
                            }
                        }
                        LinkType::SS => {
                            let lx = p_start.min(s_start) - 8.0;
                            arrow(&bp, Pos2::new(s_start, sr.center().y), Vec2::X, col);
                            vec![Pos2::new(p_start, py), Pos2::new(lx, py), Pos2::new(lx, sr.center().y), Pos2::new(s_start, sr.center().y)]
                        }
                        LinkType::FF => {
                            let rx = p_end.max(s_end) + 8.0;
                            arrow(&bp, Pos2::new(s_end, sr.center().y), -Vec2::X, col);
                            vec![Pos2::new(p_end, py), Pos2::new(rx, py), Pos2::new(rx, sr.center().y), Pos2::new(s_end, sr.center().y)]
                        }
                        LinkType::SF => {
                            let lx = p_start - 8.0;
                            let rx = s_end + 8.0;
                            let mid_y = if down { row_y(app, s_row) } else { row_y(app, s_row) + app.row_height(s_row) };
                            arrow(&bp, Pos2::new(s_end, sr.center().y), -Vec2::X, col);
                            vec![Pos2::new(p_start, py), Pos2::new(lx, py), Pos2::new(lx, mid_y), Pos2::new(rx, mid_y), Pos2::new(rx, sr.center().y), Pos2::new(s_end, sr.center().y)]
                        }
                    }
                };
                bp.add(Shape::line(pts.clone(), st));
                link_hits.push((i, l.pred_uid, pts));
            }
        }
    }

    // ---------------- Format > Baseline / Slippage (under the bars)
    if fmt.baseline_bars.is_some() || fmt.slippage.is_some() {
        for (row, &i) in vis.iter().enumerate() {
            let y = row_y(app, row);
            let rh = app.row_height(row);
            if y + rh < body.top() || y > body.bottom() {
                continue;
            }
            let t = &app.doc.tasks[i];
            let cy = y + rh / 2.0;
            let grey = Color32::from_rgb(0xA6, 0xA6, 0xA6);
            if let Some(n) = fmt.slippage {
                if let (Some(bs), Some(st)) = (t.baseline(n).and_then(|b| b.start), t.start_dt) {
                    let (x0, x1) = (x_at(&s, bs), x_at(&s, st));
                    if (x1 - x0).abs() > 0.5 {
                        bp.line_segment([Pos2::new(x0.min(x1), cy), Pos2::new(x0.max(x1), cy)], Stroke::new(1.0_f32, Color32::from_rgb(0x40, 0x40, 0x40)));
                    }
                }
            }
            if let Some(n) = fmt.baseline_bars {
                let Some(b) = t.baseline(n) else { continue };
                let (Some(bs), Some(bf)) = (b.start, b.finish) else { continue };
                let (x0, x1) = (x_at(&s, bs), x_at(&s, bf));
                let by = cy + bar_h / 2.0 + 1.0;
                if bs == bf {
                    let c = Pos2::new(x0, by + 3.0);
                    bp.add(Shape::convex_polygon(vec![c + Vec2::new(0.0, -4.0), c + Vec2::new(4.0, 0.0), c + Vec2::new(0.0, 4.0), c + Vec2::new(-4.0, 0.0)], grey, Stroke::NONE));
                } else if t.is_summary {
                    bp.rect_filled(Rect::from_min_max(Pos2::new(x0, by), Pos2::new(x1.max(x0 + 1.0), by + 2.5)), 0.0, Color32::from_rgb(0x7F, 0x7F, 0x7F));
                } else {
                    bp.rect_filled(Rect::from_min_max(Pos2::new(x0, by), Pos2::new(x1.max(x0 + 1.0), by + 4.0)), 0.0, grey);
                }
            }
        }
    }

    // ---------------- Format > Task Path colours
    let path = app.task_path_sets();

    // ---------------- bars
    let text_style = |item: TextItem| fmt.text_style(item);
    for (row, &i) in vis.iter().enumerate() {
        let y = row_y(app, row);
        let rh = app.row_height(row);
        if y + rh < body.top() || y > body.bottom() {
            continue;
        }
        let t = &app.doc.tasks[i];
        // (MPP draws nothing for a manual task with nothing entered)
        if t.hide_bar || t.nothing_entered() {
            continue;
        }
        let cy = y + rh / 2.0;
        let cats = categories(&app.doc, t);
        let mut main_done = false;
        let styles = if app.view == crate::app::View::TrackingGantt { &fmt.tracking_bar_styles } else { &fmt.bar_styles };
        for st in styles {
            if !style_applies(&app.doc, st, t, &cats) {
                continue;
            }
            let (Some(x0), Some(x1)) = (field_x(app, &s, t, st.from, hours), field_x(app, &s, t, st.to, hours)) else { continue };
            let (x0, x1) = (x0.min(x1), x0.max(x1));
            // the first style that draws the task is its "bar"; Format Bar overrides it
            let is_main = !main_done && (st.bar.middle.shape != MidShape::None || st.bar.start.shape != crate::format::EndShape::None);
            let mut look = if is_main { t.bar_format.unwrap_or(st.bar) } else { st.bar };
            // Task Path: MPP paints the path's bars (driving ones darker)
            if let Some(&bits) = path.get(&i).filter(|_| is_main && !t.is_summary && !t.is_milestone()) {
                look.middle.color = if bits & 2 != 0 {
                    [0xE4, 0x6C, 0x0A]
                } else if bits & 8 != 0 {
                    [0x70, 0x30, 0xA0]
                } else if bits & 1 != 0 {
                    [0xF4, 0xB1, 0x83]
                } else {
                    [0xB4, 0x9C, 0xDC]
                };
            }
            let texts = if is_main { t.bar_text.unwrap_or(st.text) } else { st.text };
            if is_main {
                main_done = true;
            }
            // a split task's bar is drawn piece by piece (the Split style shows the gaps)
            let pieces = !t.splits.is_empty()
                && !st.show_for.iter().any(|f| f.cat == Category::Split && !f.not)
                && matches!(st.from, BarField::Start | BarField::ActualStart)
                && matches!(st.to, BarField::Finish | BarField::CompleteThrough);
            let mid = if pieces {
                let mut first = None;
                for (a, b) in t.segments(&app.doc.calendar) {
                    let (sx0, sx1) = (x_at(&s, a).max(x0), x_at(&s, b).min(x1));
                    if sx1 > sx0 {
                        let r = draw_middle(&bp, sx0, sx1, cy, bar_h, look.middle);
                        first = first.or(r);
                    }
                }
                first
            } else {
                draw_middle(&bp, x0, x1, cy, bar_h, look.middle)
            };
            theme::end_marker(&bp, Pos2::new(x0, cy), bar_h, look.start);
            if st.from != st.to {
                theme::end_marker(&bp, Pos2::new(x1, cy), bar_h, look.end);
            }
            // bar text: left, right, top, bottom, inside
            let marker_w = if look.start.shape != crate::format::EndShape::None { bar_h / 2.0 } else { 0.0 };
            for (k, field) in texts.iter().enumerate() {
                if *field == BarText::None {
                    continue;
                }
                let value = bar_text_value(app, i, *field);
                if value.is_empty() {
                    continue;
                }
                let item = [TextItem::BarTextLeft, TextItem::BarTextRight, TextItem::BarTextTop, TextItem::BarTextBottom, TextItem::BarTextInside][k];
                let ts = text_style(item);
                let font = theme::chrome_font(&ts);
                let color = ts.color.map(theme::rgb).unwrap_or(theme::TEXT);
                let value = crate::thai::shape(&value).to_string();
                match k {
                    0 => {
                        bp.text(Pos2::new(x0 - marker_w - 4.0, cy), Align2::RIGHT_CENTER, value, font, color);
                    }
                    1 => {
                        bp.text(Pos2::new(x1 + marker_w + 4.0, cy), Align2::LEFT_CENTER, value, font, color);
                    }
                    2 => {
                        bp.text(Pos2::new((x0 + x1) / 2.0, cy - bar_h / 2.0 - 1.0), Align2::CENTER_BOTTOM, value, font, color);
                    }
                    3 => {
                        bp.text(Pos2::new((x0 + x1) / 2.0, cy + bar_h / 2.0 + 1.0), Align2::CENTER_TOP, value, font, color);
                    }
                    _ => {
                        if let Some(r) = mid {
                            let cp = bp.with_clip_rect(r);
                            cp.text(r.center(), Align2::CENTER_CENTER, value, font, color);
                        }
                    }
                }
            }
        }
    }

    if let Some(band) = ps_band {
        let pp = p.with_clip_rect(band);
        pp.rect_filled(band, 0.0, Color32::WHITE);
        let t = app.project_summary();
        let cats = categories(&app.doc, &t);
        let cy = band.center().y;
        let styles = if app.view == crate::app::View::TrackingGantt { &fmt.tracking_bar_styles } else { &fmt.bar_styles };
        for st in styles {
            if !style_applies(&app.doc, st, &t, &cats) {
                continue;
            }
            let (Some(x0), Some(x1)) = (field_x(app, &s, &t, st.from, hours), field_x(app, &s, &t, st.to, hours)) else { continue };
            let (x0, x1) = (x0.min(x1), x0.max(x1));
            draw_middle(&pp, x0, x1, cy, bar_h, st.bar.middle);
            theme::end_marker(&pp, Pos2::new(x0, cy), bar_h, st.bar.start);
            if st.from != st.to {
                theme::end_marker(&pp, Pos2::new(x1, cy), bar_h, st.bar.end);
            }
            let marker_w = if st.bar.start.shape != crate::format::EndShape::None { bar_h / 2.0 } else { 0.0 };
            for (k, field) in st.text.iter().enumerate() {
                let value = match field {
                    BarText::Name => t.name.clone(),
                    BarText::Start => app.fmt_bar_date(t.start),
                    BarText::Finish => app.fmt_bar_date(t.finish),
                    BarText::Duration => app.task_duration_text(&t),
                    BarText::PercentComplete => format!("{}%", t.percent_complete),
                    BarText::Cost => crate::util::fmt_money(t.cost),
                    BarText::Id | BarText::UniqueId | BarText::OutlineNumber | BarText::Wbs => "0".into(),
                    _ => String::new(),
                };
                if value.is_empty() || k > 1 {
                    continue;
                }
                let ts = fmt.text_style([TextItem::BarTextLeft, TextItem::BarTextRight][k]);
                let color = ts.color.map(theme::rgb).unwrap_or(theme::TEXT);
                let value = crate::thai::shape(&value).to_string();
                if k == 0 {
                    pp.text(Pos2::new(x0 - marker_w - 4.0, cy), Align2::RIGHT_CENTER, value, theme::chrome_font(&ts), color);
                } else {
                    pp.text(Pos2::new(x1 + marker_w + 4.0, cy), Align2::LEFT_CENTER, value, theme::chrome_font(&ts), color);
                }
            }
        }
        let gr = gl(GridKind::GanttRows);
        if gr.normal != LineType::None {
            theme::styled_line(&pp, Pos2::new(band.left(), band.bottom()), Pos2::new(band.right(), band.bottom()), gr.normal, theme::rgb(gr.normal_color));
        }
    }

    // Format > Gridlines > Progress Lines: from the status date (today without one) through each
    // task's progress - tasks behind schedule pull the line left, tasks ahead push it right
    if fmt.progress_lines {
        let sd = app.doc.status_date.unwrap_or(today);
        let sx = s.x(sd) + ppd;
        let mut pts: Vec<Pos2> = vec![Pos2::new(sx, body.top())];
        for (row, &i) in vis.iter().enumerate() {
            let t = &app.doc.tasks[i];
            let y = row_y(app, row);
            let cy = y + app.row_height(row) / 2.0;
            if cy < body.top() || cy > body.bottom() {
                continue;
            }
            // only tasks that should have started by the status date and are not done
            let x = if !t.is_summary && t.start <= sd && t.percent_complete < 100 && t.active {
                field_x(app, &s, t, BarField::CompleteThrough, hours).unwrap_or_else(|| x_at(&s, app.doc.calendar.day_start(t.start)))
            } else {
                sx
            };
            pts.push(Pos2::new(x, cy));
        }
        pts.push(Pos2::new(sx, body.bottom()));
        bp.add(Shape::line(pts, Stroke::new(1.5_f32, Color32::from_rgb(0xD1, 0x3B, 0x2E))));
    }

    // View > Group by: the group header bands with a summary bar across the group's dates
    for (k, (r, _)) in app.group_bands.iter().enumerate() {
        let y1 = row_y(app, *r);
        let y0 = y1 - crate::app::GROUP_H;
        if y1 < body.top() || y0 > body.bottom() {
            continue;
        }
        bp.rect_filled(Rect::from_min_max(Pos2::new(body.left(), y0), Pos2::new(body.right(), y1)), 0.0, Color32::from_rgb(0xFF, 0xF4, 0xB8));
        if let Some((st, fi, _, _)) = app.group_totals(k) {
            let (x0, x1) = (x_at(&s, st), x_at(&s, fi));
            let cy = (y0 + y1) / 2.0;
            bp.rect_filled(Rect::from_min_max(Pos2::new(x0, cy - 2.5), Pos2::new(x1.max(x0 + 2.0), cy + 2.5)), 0.0, theme::TEXT);
            for x in [x0, x1] {
                bp.add(Shape::convex_polygon(vec![Pos2::new(x - 3.5, cy + 2.5), Pos2::new(x + 3.5, cy + 2.5), Pos2::new(x, cy + 7.0)], theme::TEXT, Stroke::NONE));
            }
        }
    }

    // rolled-up subtasks on their summary rows (Layout > Always roll up / task Rollup)
    for (row, &i) in vis.iter().enumerate() {
        let t = &app.doc.tasks[i];
        if !t.is_summary || (fmt.layout.hide_rollup_when_expanded && !t.collapsed) {
            continue;
        }
        let y = row_y(app, row);
        let rh = app.row_height(row);
        if y + rh < body.top() || y > body.bottom() {
            continue;
        }
        let cy = y + rh / 2.0;
        let end = app.doc.children_end(i);
        for j in i + 1..end {
            let c = &app.doc.tasks[j];
            if c.is_summary || !(fmt.layout.always_roll_up || c.rollup) {
                continue;
            }
            if c.is_milestone() {
                theme::end_marker(&bp, Pos2::new(s.x(c.start), cy), bar_h, crate::format::EndStyle::solid(crate::format::EndShape::Diamond, crate::format::BLACK));
            } else {
                let r = Rect::from_min_max(Pos2::new(s.x(c.start), cy - 2.0), Pos2::new(s.x(c.finish) + ppd, cy + 2.0));
                bp.rect_filled(r, 0.0, theme::rgb(crate::format::TASK_BLUE));
            }
        }
    }

    if nonwork.draw == 1 {
        shade_nonworking(&bp);
    }

    // one selected cell: the standard grey edges of the selected row continue across the chart
    if app.single_cell_selected() {
        let y0 = row_y(app, app.sel_row);
        let y1 = y0 + app.row_height(app.sel_row);
        for ly in [y0 + 0.5, y1 + 0.5] {
            if ly >= body.top() && ly <= body.bottom() {
                bp.line_segment([Pos2::new(body.left(), ly), Pos2::new(body.right(), ly)], Stroke::new(1.0_f32, theme::SHEET_SEL_ROW_LINE));
            }
        }
    } else if sb >= first.saturating_sub(1) {
        // selected rows: the green lines of the sheet continue across the chart
        let y0 = row_y(app, sa);
        let y1 = row_y(app, sb) + app.row_height(sb);
        if y0 >= body.top() - 2.0 && y0 <= body.bottom() {
            bp.line_segment([Pos2::new(body.left(), y0 + 0.5), Pos2::new(body.right(), y0 + 0.5)], Stroke::new(2.0_f32, theme::ROW_SEL_LINE));
        }
        if y1 >= body.top() && y1 <= body.bottom() + 2.0 {
            bp.line_segment([Pos2::new(body.left(), y1 - 0.5), Pos2::new(body.right(), y1 - 0.5)], Stroke::new(2.0_f32, theme::ROW_SEL_LINE));
        }
    }

    // ---------------- interactions
    let resp = ui.interact(body, ui.scope_id().with("gantt_body"), Sense::click_and_drag());
    if resp.clicked() || resp.drag_started() {
        app.drawing_sel = None;
    }
    // Format > Drawing: the shapes over the bars (they take the pointer where they are)
    {
        let origin_t = origin.and_time(chrono::NaiveTime::MIN);
        let to_x = |t: chrono::NaiveDateTime| x_at(&s, t);
        let from_x = |x: f32| -> chrono::NaiveDateTime { origin_t + Duration::minutes(((x - s.x0 + s.scroll) as f64 / ppd as f64 * 1440.0).round() as i64) };
        crate::drawing::chart(app, ui, body, &to_x, &from_x, cmds);
    }
    let hresp = ui.interact(header, ui.scope_id().with("gantt_header"), Sense::click());
    let hover_pos = ui.ctx().pointer_hover_pos();
    // hit test bars: the middle moves the bar (or links it when dragged into another row), its
    // right end changes the duration and the end of its progress sets % complete, as in MPP
    let hit_list: Vec<(usize, Rect, bool, bool, u8)> = vis
        .iter()
        .filter_map(|&i| {
            let t = &app.doc.tasks[i];
            if t.hide_bar {
                return None;
            }
            // only bars on screen can be under the pointer
            geom.get(&t.uid).filter(|g| g.2 + 1 >= first && g.2 <= last + 1).map(|&(r, ms, _, _)| (i, r, ms, t.is_summary, t.percent_complete))
        })
        .collect();
    let hit_bar = move |pos: Pos2| -> Option<(usize, BarZone)> {
        for &(i, r, ms, summary, pct) in &hit_list {
            let hr = if ms { Rect::from_center_size(Pos2::new(r.left(), r.center().y), Vec2::splat(14.0)) } else { r.expand2(Vec2::new(2.0, 3.0)) };
            if hr.contains(pos) && pos.y >= body.top() {
                if ms || summary {
                    return Some((i, BarZone::Middle));
                }
                if (pos.x - r.right()).abs() <= 4.0 {
                    return Some((i, BarZone::End));
                }
                let px = r.left() + r.width() * pct as f32 / 100.0;
                if (pos.x - px).abs() <= 3.0 && r.width() > 10.0 {
                    return Some((i, BarZone::Progress));
                }
                return Some((i, BarZone::Middle));
            }
        }
        None
    };
    let link_at = |pos: Pos2| -> Option<(usize, u32)> {
        link_hits.iter().find(|(_, _, pts)| pts.windows(2).any(|w| dist_to_segment(pos, w[0], w[1]) <= 3.0)).map(|(i, u, _)| (*i, *u))
    };
    // drags snap to whole days, or to hours when the timescale shows hours
    let snap_min: i64 = if hours { 60 } else { 1440 };
    let origin_t = origin.and_time(chrono::NaiveTime::MIN);
    let instant_at = |x: f32| -> chrono::NaiveDateTime { origin_t + Duration::minutes(((x - s.x0 + s.scroll) as f64 / ppd as f64 * 1440.0).round() as i64) };
    let snap_at = |t: chrono::NaiveDateTime| -> chrono::NaiveDateTime {
        let m = (t - origin_t).num_minutes() as f64;
        origin_t + Duration::minutes((m / snap_min as f64).round() as i64 * snap_min)
    };
    let show_t = |app: &App, t: chrono::NaiveDateTime| -> String {
        if hours {
            format!("{} {}", app.fmt_date(t.date()), t.format("%H:%M"))
        } else {
            app.fmt_date(t.date())
        }
    };
    if let Some(pos) = hover_pos {
        if body.contains(pos) && app.bar_drag.is_none() {
            if let Some((i, zone)) = hit_bar(pos) {
                let t = &app.doc.tasks[i];
                match zone {
                    BarZone::End => ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal),
                    BarZone::Progress => {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeEast);
                        bp.text(pos + Vec2::new(9.0, -9.0), Align2::LEFT_BOTTOM, "%", FontId::proportional(11.0), theme::TEXT);
                    }
                    BarZone::Middle if !t.is_summary => ui.ctx().set_cursor_icon(egui::CursorIcon::Move),
                    _ => {}
                }
                let kind = if t.is_summary {
                    "Summary"
                } else if t.is_milestone() {
                    "Milestone"
                } else if t.manual {
                    "Manual Task"
                } else {
                    "Task"
                };
                let txt = format!(
                    "{}\n{}:  {}\nStart:  {}\nFinish:  {}\nDuration:  {}\n% Complete:  {}%",
                    crate::thai::shape(&t.name),
                    kind,
                    if t.critical && fmt.show_critical { "Critical" } else { "" },
                    app.fmt_date(t.start),
                    app.fmt_finish(t.finish),
                    app.task_duration_text(t),
                    t.percent_complete
                );
                resp.clone().on_hover_text_at_pointer(txt);
            } else if let Some((si, pu)) = link_at(pos) {
                // the standard link tip: type, from and to
                if let (Some(l), Some(pi)) = (app.doc.tasks[si].predecessors.iter().find(|l| l.pred_uid == pu), app.doc.index_of_uid(pu)) {
                    let kind = match l.kind {
                        LinkType::FS => "Finish-to-Start (FS)",
                        LinkType::SS => "Start-to-Start (SS)",
                        LinkType::FF => "Finish-to-Finish (FF)",
                        LinkType::SF => "Start-to-Finish (SF)",
                    };
                    let lag = crate::util::fmt_lag(l, app.dur_units(), &app.doc.options, crate::i18n::is_thai());
                    let txt = format!(
                        "{} {}{}\n{} ({}) {}\n{} ({}) {}",
                        tr("Link:"),
                        tr(kind),
                        if lag.is_empty() { String::new() } else { format!(" {lag}") },
                        tr("From:"),
                        pi + 1,
                        crate::thai::shape(&app.doc.tasks[pi].name),
                        tr("To:"),
                        si + 1,
                        crate::thai::shape(&app.doc.tasks[si].name)
                    );
                    resp.clone().on_hover_text_at_pointer(txt);
                }
            }
        }
    }
    if app.split_mode {
        if let Some(pos) = hover_pos.filter(|p| body.contains(*p)) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
            resp.clone().on_hover_text_at_pointer(tr("Split Task: click on a task bar where it should be split"));
            if resp.clicked() {
                if let Some((i, _)) = hit_bar(pos) {
                    let t = snap_at(instant_at(pos.x));
                    let gap = if hours { 60 } else { app.doc.calendar.minutes_per_day() };
                    app.split_task_at(i, t, gap);
                }
                app.split_mode = false;
            }
        }
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            app.split_mode = false;
        }
    }
    if resp.drag_started() {
        // where the button went down (the drag only starts after a few pixels of movement)
        if let Some(pos) = ui.input(|i| i.pointer.press_origin()).or_else(|| resp.interact_pointer_pos()) {
            let row = app.row_at(pos.y - body.top() + app.scroll_y);
            if let Some((i, zone)) = hit_bar(pos) {
                let t = &app.doc.tasks[i];
                let (st, ft) = task_times(app, t);
                let kind = match zone {
                    BarZone::End => BarDragKind::Resize,
                    BarZone::Progress => BarDragKind::Progress,
                    BarZone::Middle => BarDragKind::Press,
                };
                let brow = geom.get(&t.uid).map(|g| g.2).unwrap_or(row);
                // a press on a later piece of a split task moves that piece (its gap)
                let piece = if zone == BarZone::Middle && !t.splits.is_empty() {
                    let segs = t.segments(&app.doc.calendar);
                    segs.iter().position(|(a, b)| pos.x >= x_at(&s, *a) - 2.0 && pos.x <= x_at(&s, *b) + 2.0).unwrap_or(0)
                } else {
                    0
                };
                let (st, ft) = if piece > 0 { t.segments(&app.doc.calendar)[piece] } else { (st, ft) };
                app.bar_drag = Some(BarDrag { task_idx: i, kind, orig_s: st, orig_f: ft, press: (pos.x, pos.y), row: brow, piece });
            } else if row >= vis.len() {
                // an empty row: draw a new task there
                let st = instant_at(pos.x);
                app.bar_drag = Some(BarDrag { task_idx: usize::MAX, kind: BarDragKind::Draw, orig_s: st, orig_f: st, press: (pos.x, pos.y), row: vis.len(), piece: 0 });
            }
        }
    }
    if let Some(mut drag) = app.bar_drag.clone() {
        let pos = ui.ctx().pointer_interact_pos().unwrap_or_default();
        let released = resp.drag_stopped() || !ui.input(|i| i.pointer.primary_down());
        let cal = app.doc.calendar.clone();
        let mpd = cal.minutes_per_day();
        let du = app.dur_units();
        let thai = crate::i18n::is_thai();
        let ti = drag.task_idx;
        if matches!(drag.kind, BarDragKind::Press | BarDragKind::Move | BarDragKind::Link) && ti < app.doc.tasks.len() {
            // along the bar's row the bar moves; into another row it links
            let ry0 = row_y(app, drag.row);
            let in_row = pos.y >= ry0 && pos.y <= ry0 + app.row_height(drag.row);
            let moved = (pos.x - drag.press.0).abs() >= 3.0;
            drag.kind = if !in_row {
                BarDragKind::Link
            } else if moved && !app.doc.tasks[ti].is_summary {
                BarDragKind::Move
            } else {
                BarDragKind::Press
            };
        }
        let dx_min = {
            let m = (pos.x - drag.press.0) as f64 / ppd as f64 * 1440.0;
            (m / snap_min as f64).round() as i64 * snap_min
        };
        // finish of a task with duration `dur` starting at `st`
        let finish_from = |t: &Task, st: chrono::NaiveDateTime, dur: i64| -> chrono::NaiveDateTime {
            if dur <= 0 {
                st
            } else if t.dur_elapsed {
                st + Duration::minutes(dur)
            } else {
                cal.add_work_minutes(st, dur)
            }
        };
        let ghost = |row: usize, a: chrono::NaiveDateTime, b: chrono::NaiveDateTime, milestone: bool| {
            let cy = row_y(app, row) + app.row_height(row) / 2.0;
            let (x0, x1) = (x_at(&s, a), x_at(&s, b));
            let r = if milestone { Rect::from_center_size(Pos2::new(x0, cy), Vec2::splat(bar_h)) } else { Rect::from_min_max(Pos2::new(x0, cy - bar_h / 2.0), Pos2::new(x1.max(x0 + 2.0), cy + bar_h / 2.0)) };
            bp.rect_stroke(r, 1.0, Stroke::new(1.5_f32, theme::TEXT), egui::StrokeKind::Inside);
        };
        let mut tip = String::new();
        let mut end = DragEnd::None;
        match drag.kind {
            BarDragKind::Press => {}
            BarDragKind::Move if ti < app.doc.tasks.len() && drag.piece > 0 => {
                // a later piece of a split task: its gap grows or shrinks (0 joins the pieces)
                let t = &app.doc.tasks[ti];
                let k = drag.piece - 1;
                let mut sp = t.splits.clone();
                sp.sort();
                if let Some(&(_, old)) = sp.get(k) {
                    let cand = drag.orig_s + Duration::minutes(dx_min);
                    let delta = cal.work_minutes(drag.orig_s, cal.next_work_instant(cand));
                    let gap = (old + delta).max(0);
                    let ns = cal.next_work_instant(cand);
                    let nf = t.advance(&cal, ns, if t.dur_elapsed { (drag.orig_f - drag.orig_s).num_minutes() } else { cal.work_minutes(drag.orig_s, drag.orig_f) });
                    ghost(drag.row, ns, nf, false);
                    tip = format!("{}  {}\n{}  {}\n{}  {}", tr("Task:"), crate::thai::shape(&t.name), tr("Start:"), show_t(app, ns), tr("Finish:"), show_t(app, nf));
                    end = DragEnd::Gap(k, gap);
                }
            }
            BarDragKind::Move if ti < app.doc.tasks.len() => {
                let t = &app.doc.tasks[ti];
                let dur = t.dur_minutes(mpd);
                let cand = drag.orig_s + Duration::minutes(dx_min);
                // a milestone shown at the end of a day lands on the start of the target day
                let cand = if t.is_milestone() && !hours { cal.day_start(cand.date()) } else { cand };
                let ns = cal.next_work_instant(cand);
                let nf = finish_from(t, ns, dur);
                ghost(drag.row, ns, nf, t.is_milestone());
                tip = format!("{}  {}\n{}  {}\n{}  {}", tr("Task:"), crate::thai::shape(&t.name), tr("Start:"), show_t(app, ns), tr("Finish:"), show_t(app, nf));
                end = DragEnd::Move(ns);
            }
            BarDragKind::Resize if ti < app.doc.tasks.len() => {
                let t = &app.doc.tasks[ti];
                let st = drag.orig_s;
                let cand = drag.orig_f + Duration::minutes(dx_min);
                let unit = if hours { 60 } else if t.dur_elapsed { 1440 } else { mpd };
                let min = unit.min(t.dur_minutes(mpd)).max(1);
                let nd = if t.dur_elapsed { (cand - st).num_minutes() } else { cal.work_minutes(st, cal.prev_work_end(cand.max(st))) }.max(min);
                let nf = finish_from(t, st, nd);
                ghost(drag.row, st, nf, false);
                let dtxt = crate::util::fmt_dur(nd, t.dur_unit, t.dur_elapsed, du, &app.doc.options, thai);
                tip = format!("{}  {}\n{}  {}\n{}  {}", tr("Task:"), crate::thai::shape(&t.name), tr("Finish:"), show_t(app, nf), tr("Duration:"), dtxt);
                end = DragEnd::Resize(nd);
            }
            BarDragKind::Progress if ti < app.doc.tasks.len() => {
                let t = &app.doc.tasks[ti];
                let (st, ft) = (drag.orig_s, drag.orig_f);
                let total = if t.dur_elapsed { (ft - st).num_minutes() } else { t.dur_minutes(mpd) }.max(1);
                let through = snap_at(instant_at(pos.x)).clamp(st, ft);
                let done = if t.dur_elapsed { (through - st).num_minutes() } else { cal.work_minutes(st, through) };
                let pct = (done as f64 / total as f64 * 100.0).round().clamp(0.0, 100.0) as u8;
                let cy = row_y(app, drag.row) + app.row_height(drag.row) / 2.0;
                let (x0, x1) = (x_at(&s, st), x_at(&s, through));
                if x1 > x0 {
                    bp.rect_filled(Rect::from_min_max(Pos2::new(x0, cy - 1.5), Pos2::new(x1, cy + 1.5)), 0.0, theme::TEXT);
                }
                tip = format!("{}  {}\n{}  {}\n{}  {}%", tr("Task:"), crate::thai::shape(&t.name), tr("Complete Through:"), show_t(app, through), tr("% Complete:"), pct);
                end = DragEnd::Progress(pct);
            }
            BarDragKind::Link if ti < app.doc.tasks.len() => {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Alias);
                let target = hit_bar(pos).map(|(j, _)| j).filter(|&j| j != ti);
                let name = |j: usize| crate::thai::shape(&app.doc.tasks[j].name).to_string();
                tip = format!(
                    "{}\n{}  {}\n{}  {}",
                    tr("Finish-to-Start Link"),
                    tr("From Finish Of:"),
                    name(ti),
                    tr("To Start Of:"),
                    target.map(name).unwrap_or_default()
                );
                if let Some(j) = target {
                    end = DragEnd::Link(j);
                }
            }
            BarDragKind::Draw => {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                let (a, b) = (snap_at(drag.orig_s), snap_at(instant_at(pos.x)));
                let (a, b) = (a.min(b), a.max(b));
                let ns = cal.next_work_instant(a);
                let nd = cal.work_minutes(ns, b).max(if hours { 60 } else { mpd });
                let nf = cal.add_work_minutes(ns, nd);
                ghost(drag.row, ns, nf, false);
                let dtxt = crate::util::fmt_dur(nd, DurUnit::Days, false, du, &app.doc.options, thai);
                tip = format!("{}  {}\n{}  {}\n{}  {}", tr("Start:"), show_t(app, ns), tr("Finish:"), show_t(app, nf), tr("Duration:"), dtxt);
                end = DragEnd::Draw(ns, nd);
            }
            _ => {}
        }
        if !tip.is_empty() {
            egui::Tooltip::always_open(ui.ctx().clone(), ui.layer_id(), egui::Id::unique("bar_drag_tip"), egui::PopupAnchor::Pointer).show(|ui| {
                ui.label(tip);
            });
        }
        if released {
            app.bar_drag = None;
            match end {
                DragEnd::Move(ns) if ns != drag.orig_s => {
                    app.checkpoint();
                    let t = &mut app.doc.tasks[ti];
                    if t.manual {
                        t.start_dt = Some(ns);
                        t.start = ns.date();
                    } else {
                        // like MPP: moving an auto-scheduled bar sets Start No Earlier Than
                        t.set_snet_at(ns);
                    }
                    app.recalc();
                }
                DragEnd::Resize(nd) if nd != app.doc.tasks[ti].dur_minutes(mpd) => {
                    app.checkpoint();
                    let t = &mut app.doc.tasks[ti];
                    let (unit, el) = (t.dur_unit, t.dur_elapsed);
                    t.set_duration_minutes(nd, unit, el, mpd);
                    t.milestone = false;
                    t.estimated = false;
                    app.recalc();
                }
                DragEnd::Progress(pct) if pct != app.doc.tasks[ti].percent_complete => {
                    app.checkpoint();
                    app.doc.tasks[ti].percent_complete = pct;
                    app.recalc();
                }
                DragEnd::Link(j) => app.link_tasks(ti, j),
                DragEnd::Gap(k, gap) => {
                    app.checkpoint();
                    let t = &mut app.doc.tasks[ti];
                    t.splits.sort();
                    if gap <= 0 {
                        // dragged back onto the previous piece: one piece again
                        if k < t.splits.len() {
                            t.splits.remove(k);
                        }
                    } else if let Some(s) = t.splits.get_mut(k) {
                        s.1 = gap;
                    }
                    app.recalc();
                }
                DragEnd::Draw(ns, nd) => app.draw_new_task(ns, nd),
                _ => {}
            }
        } else {
            app.bar_drag = Some(drag);
        }
    } else if resp.dragged() {
        // pan the chart
        app.scroll_x = (app.scroll_x - resp.drag_delta().x).max(0.0);
        app.scroll_y -= resp.drag_delta().y;
    }
    if resp.clicked() || resp.double_clicked() || resp.secondary_clicked() {
        app.commit_edit();
        if let Some(pos) = resp.interact_pointer_pos() {
            let row = app.row_at(pos.y - body.top() + app.scroll_y);
            let on_bar = hit_bar(pos).is_some();
            if on_bar && !(resp.secondary_clicked() && row >= sa && row <= sb) {
                app.sel_row = row;
                if !ui.input(|i| i.modifiers.shift) {
                    app.sel_anchor = row;
                }
            } else if !on_bar && resp.clicked() {
                app.sel_row = row;
                if !ui.input(|i| i.modifiers.shift) {
                    app.sel_anchor = row;
                }
            }
            if resp.double_clicked() {
                // like MPP: a bar opens Format Bar, a link line Task Dependency, the chart
                // background Bar Styles
                if on_bar {
                    cmds.push(Cmd::FormatBar);
                } else if let Some((si, pu)) = link_at(pos) {
                    crate::dialogs::open_dependency(app, si, pu);
                } else {
                    cmds.push(Cmd::BarStyles);
                }
            }
        }
    }
    if hresp.double_clicked() {
        cmds.push(Cmd::TimescaleDialog(0));
    }
    let on_bar_menu = hover_pos.map(|pos| hit_bar(pos).is_some()).unwrap_or(false);
    resp.context_menu(|ui| {
        ui.set_min_width(190.0);
        let has = app.current_task().is_some();
        let b = |ui: &mut Ui, enabled: bool, label: &str, cmds: &mut Vec<Cmd>, cmd: Cmd| {
            if ui.add_enabled(enabled, egui::Button::new(tr(label))).clicked() {
                cmds.push(cmd);
                ui.close();
            }
        };
        if on_bar_menu || has && ui.ctx().input(|i| i.pointer.secondary_down()) {
            // bar menu
            b(ui, has, "Cut", cmds, Cmd::Cut);
            b(ui, has, "Copy", cmds, Cmd::Copy);
            b(ui, !app.clipboard.is_empty(), "Paste", cmds, Cmd::Paste);
            ui.separator();
            b(ui, has, "Go to Task", cmds, Cmd::ScrollToTask);
            b(ui, has, "Information...", cmds, Cmd::TaskInfo);
            b(ui, has, "Notes...", cmds, Cmd::Notes);
            b(ui, has, "Show on Timeline", cmds, Cmd::AddToTimeline);
            ui.separator();
            b(ui, has, "Format Bar...", cmds, Cmd::FormatBar);
            ui.separator();
            b(ui, has, "Manually Schedule", cmds, Cmd::Manual);
            b(ui, has, "Auto Schedule", cmds, Cmd::Auto);
            b(ui, has, "Assign Resources...", cmds, Cmd::AssignResources);
        } else {
            b(ui, true, "Gridlines...", cmds, Cmd::Gridlines);
            b(ui, true, "Bar Styles...", cmds, Cmd::BarStyles);
            b(ui, true, "Layout...", cmds, Cmd::Layout);
            b(ui, true, "Nonworking Time...", cmds, Cmd::TimescaleDialog(3));
            ui.separator();
            let mut rollup = app.doc.formatting.layout.always_roll_up;
            if ui.checkbox(&mut rollup, tr("Show Bar Rollup")).clicked() {
                app.doc.formatting.layout.always_roll_up = rollup;
                ui.close();
            }
            let mut tl = app.show_timeline;
            if ui.checkbox(&mut tl, tr("Show Timeline")).clicked() {
                cmds.push(Cmd::ToggleTimeline);
                ui.close();
            }
            ui.separator();
            b(ui, true, "Zoom In", cmds, Cmd::ZoomIn);
            b(ui, true, "Zoom Out", cmds, Cmd::ZoomOut);
            b(ui, true, "Entire Project", cmds, Cmd::EntireProject);
        }
    });
    hresp.context_menu(|ui| {
        ui.set_min_width(190.0);
        let b = |ui: &mut Ui, label: &str, cmds: &mut Vec<Cmd>, cmd: Cmd| {
            if ui.button(tr(label)).clicked() {
                cmds.push(cmd);
                ui.close();
            }
        };
        b(ui, "Timescale...", cmds, Cmd::TimescaleDialog(0));
        b(ui, "Zoom...", cmds, Cmd::ZoomDialog);
        b(ui, "Nonworking Time...", cmds, Cmd::TimescaleDialog(3));
        ui.separator();
        b(ui, "Working Days & Holidays...", cmds, Cmd::WorkingTime);
    });

    // ---------------- scrollbars
    crate::grid::vscrollbar(app, ui, vscroll_rect, body.height());
    hscrollbar(app, ui, hscroll_rect, chart.width());
    p.rect_filled(Rect::from_min_max(Pos2::new(rect.right() - SCROLLBAR, rect.top()), Pos2::new(rect.right(), rect.top() + hh)), 0.0, theme::HEADER_BG);
    p.rect_filled(Rect::from_min_max(Pos2::new(rect.right() - SCROLLBAR, rect.bottom() - SCROLLBAR), rect.max), 0.0, Color32::from_rgb(0xF0, 0xF0, 0xF0));
}

/// The chart's timescale header drawn in `header` with the chart starting at `x0` (shared by the
/// Usage views).
pub(crate) fn draw_timescale(app: &App, p: &egui::Painter, header: Rect, x0: f32) {
    let s = Scale { origin: app.gantt_origin(), ppd: app.px_per_day, x0, scroll: app.scroll_x };
    let specs = tier_specs(app);
    let tv = app.doc.formatting.gridline(GridKind::TitleVertical);
    let tv_line = (if tv.normal == LineType::None { LineType::Solid } else { tv.normal }, theme::rgb(tv.normal_color));
    let tier_h = header.height() / specs.len().max(1) as f32;
    p.rect_filled(header, 0.0, theme::HEADER_BG);
    for (k, spec) in specs.iter().enumerate() {
        let r = Rect::from_min_max(Pos2::new(header.left(), header.top() + k as f32 * tier_h), Pos2::new(header.right(), header.top() + (k as f32 + 1.0) * tier_h));
        draw_tier(app, p, &s, r, spec, tv_line, header.bottom() - 1.0);
    }
    p.line_segment([Pos2::new(header.left(), header.bottom() - 0.5), Pos2::new(header.right(), header.bottom() - 0.5)], Stroke::new(1.0_f32, theme::SHEET_HEAD_BOTTOM));
}

/// The bottom tier's units between two x positions: (first date, next unit's date, x0, x1).
pub(crate) fn bottom_cells(app: &App, x0: f32, left: f32, right: f32) -> Vec<(NaiveDate, NaiveDate, f32, f32)> {
    let s = Scale { origin: app.gantt_origin(), ppd: app.px_per_day, x0, scroll: app.scroll_x };
    let Some(spec) = tier_specs(app).pop() else { return Vec::new() };
    let mut out = Vec::new();
    let mut d = unit_start(s.date_at(left), spec.unit);
    let end = s.date_at(right);
    while d <= end && out.len() < 2000 {
        let n = unit_next(d, spec.unit);
        out.push((d, n, s.x(d), s.x(n)));
        d = n;
    }
    out
}

/// Width of the scrollable chart in px (from the chart origin to two months after the finish).
pub(crate) fn chart_span_px(app: &App) -> f32 {
    let finish = app.doc.tasks.iter().map(|t| t.finish).max().unwrap_or(app.doc.start);
    ((finish - app.gantt_origin()).num_days() + 60).max(120) as f32 * app.px_per_day
}

fn hscrollbar(app: &mut App, ui: &mut Ui, r: Rect, view_w: f32) {
    let p = ui.painter();
    p.rect_filled(r, 0.0, Color32::from_rgb(0xF0, 0xF0, 0xF0));
    let finish = app.doc.tasks.iter().map(|t| t.finish).max().unwrap_or(app.doc.start);
    let span_days = ((finish - app.gantt_origin()).num_days() + 60).max(120) as f32;
    let content = (span_days * app.px_per_day).max(view_w + app.scroll_x);
    let frac = (view_w / content).clamp(0.05, 1.0);
    let thumb_w = (r.width() * frac).max(24.0);
    let max_scroll = (content - view_w).max(1.0);
    let t = (app.scroll_x / max_scroll).clamp(0.0, 1.0);
    let thumb = Rect::from_min_size(Pos2::new(r.left() + t * (r.width() - thumb_w), r.top() + 2.0), Vec2::new(thumb_w, r.height() - 4.0));
    let resp = ui.interact(r, ui.scope_id().with("hscroll"), Sense::click_and_drag());
    p.rect_filled(thumb, 3.0, if resp.hovered() || resp.dragged() { Color32::from_rgb(0xA0, 0xA0, 0xA0) } else { Color32::from_rgb(0xC2, 0xC2, 0xC2) });
    if resp.dragged() {
        app.scroll_x += resp.drag_delta().x / (r.width() - thumb_w).max(1.0) * max_scroll;
    } else if resp.clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            if pos.x < thumb.left() {
                app.scroll_x -= view_w;
            } else if pos.x > thumb.right() {
                app.scroll_x += view_w;
            }
        }
    }
    app.scroll_x = app.scroll_x.max(0.0);
}

// ======================================================================= timeline

/// Timeline bars take these soft colours in turn; Today is marked in orange.
const TIMELINE_COLORS: [Color32; 4] = [Color32::from_rgb(0x9F, 0xC8, 0xF5), Color32::from_rgb(0xA8, 0xDF, 0xBE), Color32::from_rgb(0xFF, 0xD3, 0x9E), Color32::from_rgb(0xCF, 0xC0, 0xF9)];
const TIMELINE_TODAY: Color32 = Color32::from_rgb(0xF5, 0x92, 0x2F);

pub fn timeline(app: &mut App, ui: &mut Ui, rect: Rect) {
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, theme::TIMELINE_BG);
    p.line_segment([rect.left_bottom(), rect.right_bottom()], Stroke::new(1.0_f32, theme::RIBBON_BORDER));
    // the same slim green stripe as the main view's left edge
    let lbl = Rect::from_min_size(rect.min, Vec2::new(crate::grid::VIEW_BAR_W, rect.height()));
    p.rect_filled(lbl, 0.0, theme::GREEN);

    let area = Rect::from_min_max(Pos2::new(lbl.right() + 92.0, rect.top() + 24.0), Pos2::new(rect.right() - 92.0, rect.bottom() - 18.0));
    if app.doc.tasks.is_empty() {
        p.text(rect.center(), Align2::CENTER_CENTER, tr("Add tasks with dates to the timeline"), FontId::proportional(13.0), theme::TEXT_DIM);
        return;
    }
    let ps = app.doc.tasks.iter().map(|t| t.start).min().unwrap_or(app.doc.start).min(app.doc.start);
    let pf = app.doc.finish_date().max(ps + Duration::days(1));
    let span = ((pf - ps).num_days() + 1) as f32;
    let x = |d: NaiveDate| area.left() + (d - ps).num_days() as f32 / span * area.width();
    // the plan's first and last day as date chips at both ends of a rounded track
    let small = FontId::proportional(11.0);
    let chip = |x: f32, align: Align2, text: String, fill: Color32| {
        let g = p.layout_no_wrap(text, small.clone(), Color32::WHITE);
        let w = g.size().x + 14.0;
        let x0 = if align == Align2::LEFT_CENTER { x } else { x - w };
        let r = Rect::from_min_size(Pos2::new(x0, area.center().y - 10.0), Vec2::new(w, 20.0));
        p.rect_filled(r, 10.0, fill);
        p.galley(Pos2::new(r.left() + 7.0, r.center().y - g.size().y / 2.0), g, Color32::WHITE);
    };
    chip(lbl.right() + 6.0, Align2::LEFT_CENTER, app.fmt_date(ps), theme::GREEN);
    chip(rect.right() - 6.0, Align2::RIGHT_CENTER, app.fmt_finish(pf), theme::SUMMARY);
    p.rect_filled(area, 6.0, theme::TIMELINE_BAR);
    // month ticks above
    let mut d = NaiveDate::from_ymd_opt(ps.year(), ps.month(), 1).unwrap();
    let mut last_label_x = f32::NEG_INFINITY;
    while d <= pf {
        if d >= ps {
            let xx = x(d);
            p.line_segment([Pos2::new(xx, area.top() - 4.0), Pos2::new(xx, area.top())], Stroke::new(1.0_f32, theme::TEXT_DIM));
            if xx - last_label_x > 60.0 {
                p.text(Pos2::new(xx + 3.0, area.top() - 8.0), Align2::LEFT_CENTER, format!("{} {}", app.loc.month_abbr(d), app.loc.year(d)), small.clone(), theme::TEXT_DIM);
                last_label_x = xx;
            }
        }
        d = add_months(d, 1);
    }
    // today marker
    let today = chrono::Local::now().date_naive();
    if today >= ps && today <= pf {
        let tx = x(today);
        p.line_segment([Pos2::new(tx, area.top()), Pos2::new(tx, area.bottom())], Stroke::new(2.0_f32, TIMELINE_TODAY));
        p.circle_filled(Pos2::new(tx, area.bottom() + 2.0), 3.5, TIMELINE_TODAY);
        p.text(Pos2::new(tx + 6.0, area.bottom() + 2.0), Align2::LEFT_CENTER, tr("Today"), small.clone(), TIMELINE_TODAY);
    }
    // tasks on timeline
    let items: Vec<usize> = (0..app.doc.tasks.len()).filter(|&i| app.doc.tasks[i].on_timeline).collect();
    if items.is_empty() {
        p.text(area.center(), Align2::CENTER_CENTER, tr("Add tasks with dates to the timeline"), FontId::proportional(theme::UI_SIZE), theme::TEXT_DIM);
    }
    // two lanes only when bars overlap
    let bars: Vec<(f32, f32)> = items.iter().filter(|&&i| !app.doc.tasks[i].is_milestone()).map(|&i| (x(app.doc.tasks[i].start), x(app.doc.tasks[i].finish + Duration::days(1)))).collect();
    let overlap = bars.iter().enumerate().any(|(a, ra)| bars.iter().skip(a + 1).any(|rb| ra.0 < rb.1 - 1.0 && rb.0 < ra.1 - 1.0));
    let lanes = if overlap { 2.0f32 } else { 1.0 };
    let lane_h = (area.height() - 4.0) / lanes;
    let mut lane_end = [f32::NEG_INFINITY; 2];
    let mut bar_no = 0usize;
    for i in items {
        let t = &app.doc.tasks[i];
        let (x0, x1) = (x(t.start), x(t.finish + Duration::days(1)));
        if t.is_milestone() {
            let c = Pos2::new(x0, area.bottom() + 2.0);
            p.add(Shape::convex_polygon(vec![c + Vec2::new(0.0, -6.0), c + Vec2::new(6.0, 0.0), c + Vec2::new(0.0, 6.0), c + Vec2::new(-6.0, 0.0)], theme::rgb(crate::fmt_dialogs::MILESTONE_ORANGE), Stroke::NONE));
            p.text(c + Vec2::new(8.0, 0.0), Align2::LEFT_CENTER, crate::thai::shape(&t.name), small.clone(), theme::TEXT);
            continue;
        }
        let lane = if !overlap || lane_end[0] <= x0 + 1.0 { 0 } else { 1 };
        lane_end[lane] = x1;
        let r = Rect::from_min_max(Pos2::new(x0, area.top() + 2.0 + lane as f32 * lane_h), Pos2::new(x1.max(x0 + 3.0), area.top() + 2.0 + (lane as f32 + 1.0) * lane_h - 2.0));
        // rounded bars in turn of a few soft colours
        let fill = TIMELINE_COLORS[bar_no % TIMELINE_COLORS.len()];
        bar_no += 1;
        p.rect_filled(r, 5.0, fill);
        p.rect_stroke(r, 5.0, Stroke::new(1.0_f32, Color32::WHITE), egui::StrokeKind::Inside);
        let cp = p.with_clip_rect(r.shrink(2.0));
        let two_line = lane_h >= 28.0;
        cp.text(Pos2::new(r.left() + 4.0, r.center().y - if two_line { 6.0 } else { 0.0 }), Align2::LEFT_CENTER, crate::thai::shape(&t.name), FontId::proportional(11.0), theme::TEXT);
        if two_line {
            cp.text(
                Pos2::new(r.left() + 4.0, r.center().y + 6.0),
                Align2::LEFT_CENTER,
                format!("{} - {}", app.fmt_bar_date(t.start), app.fmt_bar_date(t.finish)),
                FontId::proportional(10.0),
                theme::TEXT_DIM,
            );
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::View;
    use crate::model::{Baseline, ConstraintType, DurUnit};

    fn fills(shape: &Shape, out: &mut Vec<(Rect, Color32)>) {
        match shape {
            Shape::Vec(v) => v.iter().for_each(|s| fills(s, out)),
            Shape::Rect(r) if r.fill != Color32::TRANSPARENT => out.push((r.rect, r.fill)),
            _ => {}
        }
    }

    /// Tracking Gantt (the standard bar styles): a task that slipped two days after its baseline
    /// is drawn in the top half of its row, its baseline in grey in the bottom half from the
    /// baseline dates, and its progress up to the actual duration.
    #[test]
    fn tracking_gantt_draws_the_baseline_under_the_current_bar() {
        let ctx = egui::Context::default();
        let mut app = App::with_ctx(&ctx, None);
        let mon = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
        let at = |d: u32, h: u32| NaiveDate::from_ymd_opt(2026, 10, d).unwrap().and_hms_opt(h, 0, 0).unwrap();
        app.doc = Project::new();
        app.doc.start = mon;
        let mut t = Task::new(1, "A", mon);
        t.set_duration_minutes(2400, DurUnit::Days, false, 480);
        t.baselines.push(Baseline { number: 0, start: Some(at(19, 8)), finish: Some(at(23, 17)), dur_min: Some(2400), ..Default::default() });
        t.constraint_type = ConstraintType::Snet;
        t.constraint_date = NaiveDate::from_ymd_opt(2026, 10, 21);
        app.doc.tasks = vec![t];
        app.doc.next_uid = 2;
        app.recalc();
        let cal = app.doc.calendar.clone();
        app.doc.tasks[0].set_percent(40, &cal);
        app.recalc();
        assert_eq!(app.doc.tasks[0].start_dt, Some(at(21, 8)));
        app.view = View::TrackingGantt;
        app.px_per_day = 40.0;
        app.scroll_x = 0.0;
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1400.0, 500.0));
        let mut shapes = Vec::new();
        // (a new Area fades in for ~0.1 s: draw a few frames first)
        for k in 0..12 {
            let input = egui::RawInput { screen_rect: Some(screen), time: Some(k as f64 * 0.03), ..Default::default() };
            let out = ctx.run_pass(input, |ctx| {
                egui::Area::new(egui::Id::unique("gantt_test")).fixed_pos(Pos2::ZERO).show(ctx, |ui| {
                    let mut cmds = Vec::new();
                    gantt(&mut app, ui, screen, &mut cmds);
                });
            });
            shapes = out.shapes;
        }
        let mut rects = Vec::new();
        for cs in &shapes {
            fills(&cs.shape, &mut rects);
        }
        let find = |c: [u8; 3]| rects.iter().filter(|(_, f)| *f == Color32::from_rgb(c[0], c[1], c[2])).map(|(r, _)| *r).max_by(|a, b| a.width().total_cmp(&b.width()));
        let task = find([90, 155, 234]).expect("task bar (top half)");
        let base = find([140, 150, 165]).expect("baseline bar (bottom half)");
        let prog = find([29, 79, 153]).expect("progress bar");
        let ppd = app.px_per_day;
        // baseline Mon 08:00 - Fri 17:00, task Wed 08:00 - next Tue 17:00 (two working days later)
        assert!((task.left() - base.left() - 2.0 * ppd).abs() < 1.5, "task {task:?} baseline {base:?}");
        assert!(base.top() >= task.center().y, "baseline below the task bar: {base:?} {task:?}");
        // 40 % of 5 days = 2 days: through Thu 17:00
        let through = task.left() + (1.0 + 9.0 / 24.0) * ppd;
        assert!((prog.right() - through).abs() < 1.5, "progress ends at {} not {}", prog.right(), through);
    }

    /// Two-task project in the Gantt Chart; returns the app, the context and the two bar rects.
    fn two_tasks() -> (App, egui::Context, Rect, Rect) {
        let ctx = egui::Context::default();
        let mut app = App::with_ctx(&ctx, None);
        let mon = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
        app.doc = Project::new();
        app.doc.start = mon;
        for uid in 1..=2 {
            let mut t = Task::new(uid, if uid == 1 { "A" } else { "B" }, mon);
            t.set_duration_minutes(960, DurUnit::Days, false, 480);
            app.doc.tasks.push(t);
        }
        app.doc.next_uid = 3;
        app.recalc();
        app.view = View::Gantt;
        app.px_per_day = 40.0;
        // scroll the chart to a day before the project start
        app.scroll_x = ((mon - app.gantt_origin()).num_days() - 1) as f32 * app.px_per_day;
        let shapes = frames(&mut app, &ctx, vec![Vec::new(); 12]);
        let mut rects = Vec::new();
        for cs in &shapes {
            fills(&cs.shape, &mut rects);
        }
        let mut bars: Vec<Rect> = rects.iter().filter(|(_, f)| *f == Color32::from_rgb(90, 155, 234)).map(|(r, _)| *r).collect();
        bars.sort_by(|a, b| a.top().total_cmp(&b.top()));
        assert_eq!(bars.len(), 2, "two task bars");
        (app, ctx, bars[0], bars[1])
    }

    fn frames(app: &mut App, ctx: &egui::Context, events: Vec<Vec<egui::Event>>) -> Vec<egui::epaint::ClippedShape> {
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1400.0, 500.0));
        let mut shapes = Vec::new();
        for (k, ev) in events.into_iter().enumerate() {
            let t = ctx.input(|i| i.time) + 0.03 * (k as f64 + 1.0);
            let input = egui::RawInput { screen_rect: Some(screen), time: Some(t), events: ev, ..Default::default() };
            let out = ctx.run_pass(input, |ctx| {
                egui::Area::new(egui::Id::unique("gantt_test")).fixed_pos(Pos2::ZERO).show(ctx, |ui| {
                    // cover the screen like the central panel does, so the pointer is over it
                    ui.set_min_size(screen.size());
                    let mut cmds = Vec::new();
                    gantt(app, ui, screen, &mut cmds);
                });
            });
            shapes = out.shapes;
        }
        shapes
    }

    /// Press, move in steps, release (a real drag).
    fn drag(from: Pos2, to: Pos2) -> Vec<Vec<egui::Event>> {
        let btn = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
        let mut v = vec![vec![egui::Event::PointerMoved(from)], vec![btn(from, true)]];
        for k in 1..=8 {
            v.push(vec![egui::Event::PointerMoved(from + (to - from) * (k as f32 / 8.0))]);
        }
        v.push(vec![btn(to, false)]);
        v.push(Vec::new());
        v
    }

    #[test]
    fn dragging_a_bar_onto_another_links_them() {
        let (mut app, ctx, a, b) = two_tasks();
        frames(&mut app, &ctx, drag(a.center(), b.center()));
        let preds = &app.doc.tasks[1].predecessors;
        assert_eq!(preds.len(), 1, "B gets a predecessor");
        assert_eq!((preds[0].pred_uid, preds[0].kind), (1, LinkType::FS));
        // and B now follows A: Wed 08:00
        assert_eq!(app.doc.tasks[1].start_dt, NaiveDate::from_ymd_opt(2026, 10, 21).unwrap().and_hms_opt(8, 0, 0));
    }

    #[test]
    fn dragging_a_bar_along_its_row_moves_it() {
        let (mut app, ctx, a, _) = two_tasks();
        let ppd = app.px_per_day;
        frames(&mut app, &ctx, drag(a.center(), a.center() + Vec2::new(2.0 * ppd, 0.0)));
        let t = &app.doc.tasks[0];
        // MPP sets Start No Earlier Than the new start: Wed 08:00
        assert_eq!(t.constraint_type, crate::model::ConstraintType::Snet);
        assert_eq!(t.start_dt, NaiveDate::from_ymd_opt(2026, 10, 21).unwrap().and_hms_opt(8, 0, 0));
        assert!(app.doc.tasks[1].predecessors.is_empty());
    }

    #[test]
    fn dragging_the_end_of_a_bar_changes_its_duration() {
        let (mut app, ctx, a, _) = two_tasks();
        let ppd = app.px_per_day;
        let end = Pos2::new(a.right() - 1.0, a.center().y);
        frames(&mut app, &ctx, drag(end, end + Vec2::new(ppd, 0.0)));
        assert_eq!(app.doc.tasks[0].dur_min, 1440, "2 days + 1 day");
    }

    #[test]
    fn split_task_pieces_can_be_moved_and_joined() {
        let (mut app, ctx, a, _) = two_tasks();
        let ppd = app.px_per_day;
        let at = |d: u32, h: u32| NaiveDate::from_ymd_opt(2026, 10, d).unwrap().and_hms_opt(h, 0, 0).unwrap();
        // Split Task in the middle of the 2-day task A (Mon-Tue): a 1-day gap, finish Wed
        assert!(app.split_task_at(0, at(20, 8), 480));
        assert_eq!(app.doc.tasks[0].splits, vec![(480, 480)]);
        assert_eq!(app.doc.tasks[0].finish_dt, Some(at(21, 17)));
        let segs = app.doc.tasks[0].segments(&app.doc.calendar);
        assert_eq!(segs, vec![(at(19, 8), at(19, 17)), (at(21, 8), at(21, 17))]);
        // drag the second piece one day right: a 2-day gap
        let piece = Pos2::new(a.left() + 2.15 * ppd, a.center().y);
        frames(&mut app, &ctx, drag(piece, piece + Vec2::new(ppd, 0.0)));
        assert_eq!(app.doc.tasks[0].splits, vec![(480, 960)]);
        assert_eq!(app.doc.tasks[0].finish_dt, Some(at(22, 17)));
        // drag it back onto the first piece: one piece again
        let piece = Pos2::new(a.left() + 3.15 * ppd, a.center().y);
        frames(&mut app, &ctx, drag(piece, piece - Vec2::new(2.0 * ppd, 0.0)));
        assert!(app.doc.tasks[0].splits.is_empty(), "{:?}", app.doc.tasks[0].splits);
        assert_eq!(app.doc.tasks[0].finish_dt, Some(at(20, 17)));
    }
}
