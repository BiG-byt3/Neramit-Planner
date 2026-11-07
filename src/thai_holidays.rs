//! Thai public holidays for Change Working Time > Add Thai holidays.
//!
//! Fixed-date holidays come from a rule list; the Buddhist (lunar) holidays, the Royal Ploughing
//! Ceremony and the cabinet's special days change every year and come from a per-year table.
//! A holiday that falls on a Saturday or Sunday gets a substitution day on the next weekday that is
//! not already a holiday (the cabinet rule: Songkran 13–15 Apr 2025 Sun–Tue → Wed 16 Apr).
//!
//! Years missing from the table can be added without a new release: put lines such as
//! `2028-02-10 วันมาฆบูชา` in `thai-holidays.txt` in the settings folder
//! (`%APPDATA%\Neramit Planner`). A year listed there counts as known.

use chrono::{Datelike, Duration, NaiveDate, Weekday};

#[derive(Clone, Debug, PartialEq)]
pub struct ThaiHoliday {
    pub date: NaiveDate,
    pub name_th: String,
    pub name_en: String,
    /// observed by government offices only (Royal Ploughing Ceremony, Buddhist Lent Day)
    pub gov_only: bool,
}

impl ThaiHoliday {
    pub fn name(&self, thai: bool) -> &str {
        if thai { &self.name_th } else { &self.name_en }
    }
}

/// Result of `holidays`: the days plus the years the caller should warn about.
#[derive(Clone, Debug, Default)]
pub struct HolidaySet {
    pub days: Vec<ThaiHoliday>,
    /// years without Buddhist holiday dates (only the fixed-date holidays were added)
    pub missing: Vec<i32>,
    /// years whose dates are not yet officially announced
    pub provisional: Vec<i32>,
}

const FIXED: &[(u32, u32, &str, &str)] = &[
    (1, 1, "วันขึ้นปีใหม่", "New Year's Day"),
    (4, 6, "วันจักรี", "Chakri Memorial Day"),
    (4, 13, "วันสงกรานต์", "Songkran Festival"),
    (4, 14, "วันสงกรานต์", "Songkran Festival"),
    (4, 15, "วันสงกรานต์", "Songkran Festival"),
    (5, 1, "วันแรงงานแห่งชาติ", "National Labour Day"),
    (5, 4, "วันฉัตรมงคล", "Coronation Day"),
    (6, 3, "วันเฉลิมพระชนมพรรษาสมเด็จพระราชินี", "H.M. the Queen's Birthday"),
    (7, 28, "วันเฉลิมพระชนมพรรษาพระบาทสมเด็จพระเจ้าอยู่หัว", "H.M. the King's Birthday"),
    (8, 12, "วันแม่แห่งชาติ", "Mother's Day"),
    (10, 13, "วันนวมินทรมหาราช", "King Bhumibol Memorial Day"),
    (10, 23, "วันปิยมหาราช", "Chulalongkorn Day"),
    (12, 5, "วันพ่อแห่งชาติ", "Father's Day"),
    (12, 10, "วันรัฐธรรมนูญ", "Constitution Day"),
    (12, 31, "วันสิ้นปี", "New Year's Eve"),
];

/// Per-year dates that do not follow a fixed day of the Gregorian calendar.
struct Year {
    year: i32,
    /// Makha Bucha, Visakha Bucha, Asarnha Bucha, Buddhist Lent Day (month, day)
    lunar: [(u32, u32); 4],
    /// Royal Ploughing Ceremony (government offices only), when announced
    plough: Option<(u32, u32)>,
    /// cabinet special holidays
    special: &'static [(u32, u32)],
    /// dates not yet announced by the cabinet
    provisional: bool,
}

const YEARS: &[Year] = &[
    // ครม. 12 Nov 2024
    Year { year: 2025, lunar: [(2, 12), (5, 11), (7, 10), (7, 11)], plough: Some((5, 9)), special: &[(6, 2), (8, 11)], provisional: false },
    // ครม. / ธปท. announcements for 2569
    Year { year: 2026, lunar: [(3, 3), (5, 31), (7, 29), (7, 30)], plough: Some((5, 13)), special: &[(1, 2)], provisional: false },
    // published calendars, not yet announced by the cabinet
    Year { year: 2027, lunar: [(2, 21), (5, 20), (7, 18), (7, 19)], plough: None, special: &[], provisional: true },
];

const LUNAR: [(&str, &str); 4] = [
    ("วันมาฆบูชา", "Makha Bucha Day"),
    ("วันวิสาขบูชา", "Visakha Bucha Day"),
    ("วันอาสาฬหบูชา", "Asarnha Bucha Day"),
    ("วันเข้าพรรษา", "Buddhist Lent Day"),
];

/// Extra dates from `thai-holidays.txt`: `YYYY-MM-DD name` (or a Buddhist year, `2571-02-10`).
/// A trailing `*` marks a government-only day. `#` starts a comment.
pub fn parse_user_file(text: &str) -> Vec<ThaiHoliday> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let (date, rest) = line.split_once(|c: char| c.is_whitespace()).unwrap_or((line, ""));
        let mut parts = date.split(['-', '/']);
        let (Some(y), Some(m), Some(d)) = (parts.next(), parts.next(), parts.next()) else { continue };
        let (Ok(mut y), Ok(m), Ok(d)) = (y.parse::<i32>(), m.parse::<u32>(), d.parse::<u32>()) else { continue };
        if y > 2400 {
            y -= 543;
        }
        let Some(date) = NaiveDate::from_ymd_opt(y, m, d) else { continue };
        let mut name = rest.trim();
        let gov_only = name.ends_with('*');
        name = name.trim_end_matches('*').trim();
        let name = if name.is_empty() { "วันหยุด" } else { name };
        out.push(ThaiHoliday { date, name_th: name.to_string(), name_en: name.to_string(), gov_only });
    }
    out
}

fn user_file() -> Vec<ThaiHoliday> {
    crate::app::settings_dir()
        .and_then(|d| std::fs::read_to_string(d.join("thai-holidays.txt")).ok())
        .map(|t| parse_user_file(&t))
        .unwrap_or_default()
}

/// Holidays from `first` to `last` year (inclusive), with substitution days.
pub fn holidays(first: i32, last: i32, include_gov_only: bool) -> HolidaySet {
    holidays_with(first, last, include_gov_only, &user_file())
}

pub fn holidays_with(first: i32, last: i32, include_gov_only: bool, extra: &[ThaiHoliday]) -> HolidaySet {
    let mut set = HolidaySet::default();
    let (first, last) = (first.min(last), first.max(last).min(first.min(last) + 20));
    for y in first..=last {
        let mut base: Vec<(ThaiHoliday, bool)> = Vec::new(); // (day, gets a substitute)
        let mk = |m: u32, d: u32, th: &str, en: &str, gov: bool| {
            NaiveDate::from_ymd_opt(y, m, d).map(|date| ThaiHoliday { date, name_th: th.into(), name_en: en.into(), gov_only: gov })
        };
        for &(m, d, th, en) in FIXED {
            base.extend(mk(m, d, th, en, false).map(|h| (h, true)));
        }
        let user: Vec<&ThaiHoliday> = extra.iter().filter(|h| h.date.year() == y).collect();
        match YEARS.iter().find(|t| t.year == y) {
            Some(t) => {
                for (k, &(m, d)) in t.lunar.iter().enumerate() {
                    base.extend(mk(m, d, LUNAR[k].0, LUNAR[k].1, k == 3).map(|h| (h, true)));
                }
                if let Some((m, d)) = t.plough {
                    base.extend(mk(m, d, "วันพืชมงคล", "Royal Ploughing Ceremony", true).map(|h| (h, false)));
                }
                for &(m, d) in t.special {
                    base.extend(mk(m, d, "วันหยุดพิเศษ", "Special holiday", false).map(|h| (h, false)));
                }
                if t.provisional && user.is_empty() {
                    set.provisional.push(y);
                }
            }
            None if user.is_empty() => set.missing.push(y),
            None => {}
        }
        for h in user {
            base.retain(|(b, _)| b.date != h.date);
            base.push((h.clone(), true));
        }
        base.retain(|(h, _)| include_gov_only || !h.gov_only);
        base.sort_by_key(|(h, _)| h.date);
        let mut taken: Vec<NaiveDate> = base.iter().map(|(h, _)| h.date).collect();
        let mut subs = Vec::new();
        for (h, sub) in &base {
            if !sub || !matches!(h.date.weekday(), Weekday::Sat | Weekday::Sun) {
                continue;
            }
            let mut d = h.date + Duration::days(1);
            while matches!(d.weekday(), Weekday::Sat | Weekday::Sun) || taken.contains(&d) {
                d += Duration::days(1);
            }
            taken.push(d);
            subs.push(ThaiHoliday {
                date: d,
                name_th: format!("วันหยุดชดเชย{}", h.name_th),
                name_en: format!("Substitution for {}", h.name_en),
                gov_only: h.gov_only,
            });
        }
        set.days.extend(base.into_iter().map(|(h, _)| h));
        set.days.extend(subs);
    }
    set.days.sort_by_key(|h| h.date);
    set
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dates(set: &HolidaySet, y: i32) -> Vec<String> {
        set.days.iter().filter(|h| h.date.year() == y).map(|h| h.date.format("%m-%d").to_string()).collect()
    }

    #[test]
    fn year_2569_matches_the_cabinet_list() {
        let set = holidays_with(2026, 2026, true, &[]);
        let d = dates(&set, 2026);
        for want in ["01-01", "01-02", "03-03", "04-06", "04-13", "04-14", "04-15", "05-01", "05-04", "05-13", "06-01", "06-03", "07-28", "07-29", "07-30", "08-12", "10-13", "10-23", "12-07", "12-10", "12-31"] {
            assert!(d.contains(&want.to_string()), "missing {want}: {d:?}");
        }
        let sub = set.days.iter().find(|h| h.date == NaiveDate::from_ymd_opt(2026, 6, 1).unwrap()).unwrap();
        assert_eq!(sub.name_th, "วันหยุดชดเชยวันวิสาขบูชา");
        assert!(set.missing.is_empty() && set.provisional.is_empty());
        // without government-only days
        let d = dates(&holidays_with(2026, 2026, false, &[]), 2026);
        assert!(!d.contains(&"05-13".to_string()) && !d.contains(&"07-30".to_string()));
    }

    #[test]
    fn substitution_days() {
        // 2568: Songkran Sun–Tue → Wed 16 Apr; Chakri Sun → Mon 7 Apr; Coronation Sun → Mon 5 May
        let d = dates(&holidays_with(2025, 2025, true, &[]), 2025);
        for want in ["04-07", "04-16", "05-05", "05-12"] {
            assert!(d.contains(&want.to_string()), "missing {want}: {d:?}");
        }
        // 2570: Asarnha Bucha Sun 18 Jul, Lent Mon 19 Jul → substitute Tue 20 Jul
        let set = holidays_with(2027, 2027, true, &[]);
        let d = dates(&set, 2027);
        for want in ["02-22", "05-03", "07-19", "07-20", "10-25", "12-06"] {
            assert!(d.contains(&want.to_string()), "missing {want}: {d:?}");
        }
        assert_eq!(set.provisional, vec![2027]);
    }

    #[test]
    fn unknown_years_and_user_file() {
        let set = holidays_with(2028, 2029, true, &[]);
        assert_eq!(set.missing, vec![2028, 2029]);
        assert!(dates(&set, 2028).contains(&"04-13".to_string()));
        let extra = parse_user_file("# test\n2571-02-10 วันมาฆบูชา\n2028-07-07 วันเข้าพรรษา *\nbad line\n");
        assert_eq!(extra.len(), 2);
        assert_eq!(extra[0].date, NaiveDate::from_ymd_opt(2028, 2, 10).unwrap());
        assert!(extra[1].gov_only);
        let set = holidays_with(2028, 2028, false, &extra);
        assert!(set.missing.is_empty());
        let d = dates(&set, 2028);
        assert!(d.contains(&"02-10".to_string()) && !d.contains(&"07-07".to_string()));
    }
}
