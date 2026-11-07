//! Parsing and formatting helpers for dates, durations and predecessor strings.

use crate::model::{DurUnit, Link, LinkType};
use std::collections::HashMap;

/// The project's duration conversions (Options > Schedule): minutes per day and per week, days
/// per month.
#[derive(Clone, Copy, Debug)]
pub struct DurUnits {
    pub mpd: i64,
    pub mpw: i64,
    pub dpm: f64,
}

impl DurUnits {
    pub fn of(p: &crate::model::Project) -> DurUnits {
        DurUnits { mpd: p.calendar.minutes_per_day(), mpw: (p.options.hours_per_week * 60.0).round().max(1.0) as i64, dpm: p.options.days_per_month.max(1.0) }
    }

    /// Minutes in one unit (elapsed units count calendar time: a day is 24 hours, a month 30 days).
    pub fn unit_minutes(&self, u: DurUnit, elapsed: bool) -> f64 {
        match (u, elapsed) {
            (DurUnit::Minutes, _) => 1.0,
            (DurUnit::Hours, _) => 60.0,
            (DurUnit::Days, false) => self.mpd as f64,
            (DurUnit::Days, true) => 1440.0,
            (DurUnit::Weeks, false) => self.mpw as f64,
            (DurUnit::Weeks, true) => 10080.0,
            (DurUnit::Months, false) => self.dpm * self.mpd as f64,
            (DurUnit::Months, true) => 43200.0,
        }
    }
}

/// A unit word: "d", "days", "hrs", "wk", "mo", "นาที", ... (elapsed with a leading "e").
fn unit_word(w: &str) -> Option<(DurUnit, bool)> {
    let w = w.trim().trim_end_matches('.');
    let plain = |w: &str| -> Option<DurUnit> {
        Some(match w {
            "m" | "min" | "mins" | "minute" | "minutes" | "นาที" => DurUnit::Minutes,
            "h" | "hr" | "hrs" | "hour" | "hours" | "ชม" | "ชั่วโมง" => DurUnit::Hours,
            "d" | "dy" | "dys" | "day" | "days" | "วัน" => DurUnit::Days,
            "w" | "wk" | "wks" | "week" | "weeks" | "สัปดาห์" => DurUnit::Weeks,
            "mo" | "mon" | "mons" | "month" | "months" | "เดือน" => DurUnit::Months,
            _ => return None,
        })
    };
    if let Some(u) = plain(w) {
        return Some((u, false));
    }
    w.strip_prefix('e').and_then(plain).map(|u| (u, true))
}

/// Parse a duration such as "5", "5d", "0.5 days", "4h", "30 min", "2w", "1 mo", "3ed", "5 days?".
/// Returns (minutes, unit, elapsed); a bare number uses `default`.
pub fn parse_dur(s: &str, du: DurUnits, default: DurUnit) -> Option<(i64, DurUnit, bool)> {
    let s = s.trim().to_lowercase().replace('?', "");
    if s.is_empty() {
        return None;
    }
    let num_end = s.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == ',')).unwrap_or(s.len());
    let num: f64 = s[..num_end].replace(',', ".").parse().ok()?;
    let rest = s[num_end..].trim();
    let (unit, elapsed) = if rest.is_empty() { (default, false) } else { unit_word(rest)? };
    let m = num * du.unit_minutes(unit, elapsed);
    if !(0.0..=36500.0 * 1440.0).contains(&m) {
        return None;
    }
    Some((m.round() as i64, unit, elapsed))
}

/// A duration with the standard labels: "1 day", "0.5 days", "4 hrs", "3 edays", "2 wks?".
pub fn fmt_dur(m: i64, unit: DurUnit, elapsed: bool, du: DurUnits, o: &crate::model::ProjectOptions, thai: bool) -> String {
    let v = m as f64 / du.unit_minutes(unit, elapsed);
    let num = fmt_num(v);
    let k = match unit {
        DurUnit::Minutes => 0,
        DurUnit::Hours => 1,
        DurUnit::Days => 2,
        DurUnit::Weeks => 3,
        DurUnit::Months => 4,
    };
    let label = if thai && !elapsed {
        ["นาที", "ชม.", "วัน", "สัปดาห์", "เดือน"][k].to_string()
    } else {
        let base = o.duration_labels.get(k).cloned().unwrap_or_else(|| ["min", "hr", "day", "wk", "mon"][k].into());
        let plural = (v - 1.0).abs() > 1e-9 && matches!(base.as_str(), "min" | "hr" | "day" | "dy" | "wk" | "mon" | "minute" | "hour" | "week" | "month");
        format!("{}{base}{}", if elapsed { "e" } else { "" }, if plural { "s" } else { "" })
    };
    format!("{num}{}{label}", if o.space_before_label { " " } else { "" })
}

/// A number as MPP shows it in durations: up to two decimals, no trailing zeros.
pub fn fmt_num(v: f64) -> String {
    let r = (v * 100.0).round() / 100.0;
    if (r - r.round()).abs() < 1e-9 {
        format!("{}", r.round() as i64)
    } else {
        let s = format!("{r:.2}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// Lag text of a link: "+4 hrs", "-1 day", "+50%", "+3 edays" (empty for no lag).
pub fn fmt_lag(l: &Link, du: DurUnits, o: &crate::model::ProjectOptions, thai: bool) -> String {
    if let Some(p) = l.lag_pct {
        if p == 0.0 {
            return String::new();
        }
        return format!("{}{}%", if p > 0.0 { "+" } else { "-" }, fmt_num(p.abs()));
    }
    let m = l.lag_minutes(du.mpd, 0);
    if m == 0 {
        return String::new();
    }
    let unit = if l.lag_min.is_some() { l.lag_unit } else { DurUnit::Days };
    format!("{}{}", if m > 0 { "+" } else { "-" }, fmt_dur(m.abs(), unit, l.elapsed, du, o, thai))
}

/// Parse a lag: "+2", "-1d", "+3 days", "+4h", "+27ed", "+50%". Fills the link's lag fields.
pub fn parse_lag_into(s: &str, l: &mut Link, du: DurUnits) -> Option<()> {
    let s = s.trim();
    let (sign, rest) = if let Some(r) = s.strip_prefix('-') { (-1.0, r.trim()) } else { (1.0, s.trim_start_matches('+').trim()) };
    if let Some(p) = rest.strip_suffix('%') {
        let v: f64 = p.trim().parse().ok()?;
        l.lag_pct = Some(sign * v);
        l.lag_min = None;
        l.lag = 0;
        l.elapsed = false;
        return Some(());
    }
    let (m, unit, elapsed) = parse_dur(rest, du, DurUnit::Days)?;
    l.set_lag_minutes((sign * m as f64) as i64, unit, elapsed, du.mpd);
    Some(())
}

/// Parse a predecessor list such as "2,3FS+2 days,4SS-1d,5FF+4h,6FS+50%".
/// `id_to_uid` maps displayed row IDs to task UIDs.
pub fn parse_predecessors(s: &str, id_to_uid: &HashMap<usize, u32>, du: DurUnits) -> Result<Vec<Link>, String> {
    let mut out = Vec::new();
    for raw in s.split([',', ';']) {
        let item = raw.trim().to_uppercase();
        if item.is_empty() {
            continue;
        }
        let id_end = item.find(|c: char| !c.is_ascii_digit()).unwrap_or(item.len());
        let id: usize = item[..id_end].parse().map_err(|_| format!("'{raw}' is not a valid predecessor"))?;
        let uid = *id_to_uid.get(&id).ok_or_else(|| format!("Task ID {id} does not exist"))?;
        let mut rest = item[id_end..].trim();
        let mut kind = LinkType::FS;
        for (code, k) in [("FS", LinkType::FS), ("SS", LinkType::SS), ("FF", LinkType::FF), ("SF", LinkType::SF)] {
            if let Some(r) = rest.strip_prefix(code) {
                kind = k;
                rest = r.trim();
                break;
            }
        }
        let mut link = Link::new(uid, kind);
        if !rest.is_empty() {
            parse_lag_into(&rest.to_lowercase(), &mut link, du).ok_or_else(|| format!("Invalid lag in '{raw}'"))?;
        }
        out.push(link);
    }
    Ok(out)
}

/// How money is shown: the current project's currency options (symbol, position 0-3 as in
/// "$1", "1$", "$ 1", "1 $", decimal digits). Set by the window every frame and by the MCP server
/// for the document it works on.
static CURRENCY: std::sync::RwLock<(String, u8, u8)> = std::sync::RwLock::new((String::new(), 0, 2));

pub fn set_currency(symbol: &str, position: u8, digits: u8) {
    if let Ok(mut c) = CURRENCY.write() {
        if c.0 != symbol || c.1 != position || c.2 != digits {
            *c = (symbol.to_string(), position, digits);
        }
    }
}

/// The currency symbol money is shown with now.
pub fn currency_symbol() -> String {
    let sym = CURRENCY.read().map(|c| c.0.clone()).unwrap_or_default();
    if sym.is_empty() { "฿".to_string() } else { sym }
}

pub fn fmt_money(v: f64) -> String {
    let (_, pos, dig) = CURRENCY.read().map(|c| c.clone()).unwrap_or_default();
    format_money(v, &currency_symbol(), pos, dig)
}

pub fn format_money(v: f64, sym: &str, pos: u8, dig: u8) -> String {
    let dig = dig.min(4) as u32;
    let neg = v < 0.0;
    let scale = 10i64.pow(dig);
    let units = (v.abs() * scale as f64).round() as i64;
    let whole = units / scale;
    let frac = units % scale;
    let digits = whole.to_string();
    let mut grouped = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    let num = if dig > 0 { format!("{grouped}.{frac:0w$}", w = dig as usize) } else { grouped };
    let sign = if neg { "-" } else { "" };
    match pos {
        1 => format!("{sign}{num}{sym}"),
        2 => format!("{sign}{sym} {num}"),
        3 => format!("{sign}{num} {sym}"),
        _ => format!("{sign}{sym}{num}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn preds() {
        let m: HashMap<usize, u32> = [(1, 10), (2, 20)].into_iter().collect();
        let du = DurUnits { mpd: 480, mpw: 2400, dpm: 20.0 };
        let l = parse_predecessors("1, 2SS+3 days", &m, du).unwrap();
        assert_eq!(l.len(), 2);
        assert_eq!(l[1].kind, LinkType::SS);
        assert_eq!(l[1].lag, 3);
        assert!(parse_predecessors("9", &m, du).is_err());
        let l = parse_predecessors("1FS+4h,2FF-50%,1SS+3ed", &m, du).unwrap();
        assert_eq!(l[0].lag_minutes(480, 0), 240);
        assert_eq!(l[1].lag_pct, Some(-50.0));
        assert!(l[2].elapsed && l[2].lag_minutes(480, 0) == 3 * 1440);
    }

    #[test]
    fn durations_with_units() {
        let du = DurUnits { mpd: 480, mpw: 2400, dpm: 20.0 };
        assert_eq!(parse_dur("0.5d", du, DurUnit::Days), Some((240, DurUnit::Days, false)));
        assert_eq!(parse_dur("4 hrs", du, DurUnit::Days), Some((240, DurUnit::Hours, false)));
        assert_eq!(parse_dur("1 wk", du, DurUnit::Days), Some((2400, DurUnit::Weeks, false)));
        assert_eq!(parse_dur("1mo", du, DurUnit::Days), Some((9600, DurUnit::Months, false)));
        assert_eq!(parse_dur("3 edays", du, DurUnit::Days), Some((4320, DurUnit::Days, true)));
        assert_eq!(parse_dur("2", du, DurUnit::Days), Some((960, DurUnit::Days, false)));
        let o = crate::model::ProjectOptions::default();
        assert_eq!(fmt_dur(240, DurUnit::Days, false, du, &o, false), "0.5 days");
        assert_eq!(fmt_dur(480, DurUnit::Days, false, du, &o, false), "1 day");
        assert_eq!(fmt_dur(240, DurUnit::Hours, false, du, &o, false), "4 hrs");
        assert_eq!(fmt_dur(4320, DurUnit::Days, true, du, &o, false), "3 edays");
        assert_eq!(fmt_dur(2400, DurUnit::Weeks, false, du, &o, false), "1 wk");
    }

    #[test]
    fn money() {
        assert_eq!(format_money(1234567.5, "฿", 0, 2), "฿1,234,567.50");
        assert_eq!(format_money(-1234.5, "$", 3, 0), "-1,235 $");
        assert_eq!(format_money(0.5, "€", 1, 2), "0.50€");
    }
}
