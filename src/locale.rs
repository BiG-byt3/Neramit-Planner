//! Regional settings and date formats.
//!
//! MPP formats every date with the Windows user locale: day/month order and separator from the
//! short date pattern, day and month names from the locale, and the Thai Buddhist year when the
//! locale calendar is Thai Buddhist (so a Thai PC shows `พ 1/3/61` where a US PC shows `Thu 3/1/18`).
//! The 21 date formats of Options > General > Date format.

use chrono::{Datelike, NaiveDate};
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Order {
    Dmy,
    Mdy,
    Ymd,
}

#[derive(Clone, Debug)]
pub struct Locale {
    /// Mon..Sun as MPP shows them in "ddd" formats
    pub day_abbr: [String; 7],
    pub day_long: [String; 7],
    /// Jan..Dec
    pub month_abbr: [String; 12],
    pub month_long: [String; 12],
    pub date_sep: String,
    pub time_sep: String,
    pub order: Order,
    /// added to the Gregorian year (543 for the Thai Buddhist calendar)
    pub year_offset: i32,
    pub hour24: bool,
    pub am: String,
    pub pm: String,
    /// BCP-47 name, e.g. "th-TH"
    pub name: String,
}

static SYSTEM: OnceLock<Locale> = OnceLock::new();

/// The Windows user locale (English/Gregorian on other platforms or if the lookup fails).
pub fn system() -> &'static Locale {
    SYSTEM.get_or_init(|| windows_locale().unwrap_or_else(Locale::english))
}

impl Locale {
    pub fn english() -> Locale {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        Locale {
            day_abbr: s(&["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]).try_into().unwrap(),
            day_long: s(&["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"]).try_into().unwrap(),
            month_abbr: s(&["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]).try_into().unwrap(),
            month_long: s(&["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"]).try_into().unwrap(),
            date_sep: "/".into(),
            time_sep: ":".into(),
            order: Order::Mdy,
            year_offset: 0,
            hour24: false,
            am: "AM".into(),
            pm: "PM".into(),
            name: "en-US".into(),
        }
    }

    /// Thai Buddhist calendar in use (year shown as พ.ศ.)
    pub fn buddhist(&self) -> bool {
        self.year_offset == 543
    }

    pub fn year(&self, d: NaiveDate) -> i32 {
        d.year() + self.year_offset
    }

    pub fn day_abbr(&self, d: NaiveDate) -> &str {
        &self.day_abbr[d.weekday().num_days_from_monday() as usize]
    }


    pub fn month_abbr(&self, d: NaiveDate) -> &str {
        &self.month_abbr[d.month0() as usize]
    }

    /// `d/M/yy` or `M/d/yy` (two-digit year, no leading zeros) like the standard "1/28/09".
    pub fn numeric(&self, d: NaiveDate, full_year: bool) -> String {
        let y = self.year(d);
        let ys = if full_year { y.to_string() } else { format!("{:02}", y.rem_euclid(100)) };
        let (a, b) = match self.order {
            Order::Mdy => (d.month(), d.day()),
            _ => (d.day(), d.month()),
        };
        if self.order == Order::Ymd {
            format!("{ys}{}{}{}{}", self.date_sep, d.month(), self.date_sep, d.day())
        } else {
            format!("{a}{}{b}{}{ys}", self.date_sep, self.date_sep)
        }
    }

    /// `d/M` or `M/d`
    pub fn month_day(&self, d: NaiveDate) -> String {
        match self.order {
            Order::Mdy => format!("{}{}{}", d.month(), self.date_sep, d.day()),
            _ => format!("{}{}{}", d.day(), self.date_sep, d.month()),
        }
    }

    /// "Jan 28" / "28 Jan"
    pub fn mmm_dd(&self, d: NaiveDate) -> String {
        match self.order {
            Order::Mdy => format!("{} {}", self.month_abbr(d), d.day()),
            _ => format!("{} {}", d.day(), self.month_abbr(d)),
        }
    }

    /// The full date as written in a letter: "5 พฤศจิกายน 2569", "November 5, 2026", "5 November 2026".
    pub fn long_date(&self, d: NaiveDate) -> String {
        let m = &self.month_long[d.month0() as usize];
        match self.order {
            Order::Mdy => format!("{m} {}, {}", d.day(), self.year(d)),
            _ => format!("{} {m} {}", d.day(), self.year(d)),
        }
    }

    /// "January 28" / "28 January"
    pub fn mmmm_dd(&self, d: NaiveDate) -> String {
        match self.order {
            Order::Mdy => format!("{} {}", self.month_long[d.month0() as usize], d.day()),
            _ => format!("{} {}", d.day(), self.month_long[d.month0() as usize]),
        }
    }

    pub fn time(&self, h: u32, m: u32) -> String {
        if self.hour24 {
            format!("{h}{}{m:02}", self.time_sep)
        } else {
            let h12 = if h % 12 == 0 { 12 } else { h % 12 };
            format!("{h12}{}{m:02} {}", self.time_sep, if h < 12 { &self.am } else { &self.pm })
        }
    }
}

/// DateFormat (0..=20). The time part is the task's start or finish time of day.
pub fn format(fmt: u8, d: NaiveDate, time: (u32, u32), loc: &Locale) -> String {
    let (h, m) = time;
    let yy = format!("'{:02}", loc.year(d).rem_euclid(100));
    let week = format!("W{:02}", d.iso_week().week());
    match fmt {
        0 => format!("{} {}", loc.numeric(d, false), loc.time(h, m)),
        1 => loc.numeric(d, false),
        2 => format!("{}, {} {}", loc.mmmm_dd(d), loc.year(d), loc.time(h, m)),
        3 => format!("{}, {}", loc.mmmm_dd(d), loc.year(d)),
        4 => format!("{} {}", loc.mmm_dd(d), loc.time(h, m)),
        5 => format!("{}, {}", loc.mmm_dd(d), yy),
        6 => loc.mmmm_dd(d),
        7 => loc.mmm_dd(d),
        8 => format!("{} {} {}", loc.day_abbr(d), loc.numeric(d, false), loc.time(h, m)),
        9 => format!("{} {}", loc.day_abbr(d), loc.numeric(d, false)),
        10 => format!("{} {}, {}", loc.day_abbr(d), loc.mmm_dd(d), yy),
        11 => format!("{} {}", loc.day_abbr(d), loc.time(h, m)),
        12 => loc.month_day(d),
        13 => d.day().to_string(),
        14 => loc.time(h, m),
        15 => format!("{} {}", loc.day_abbr(d), loc.mmm_dd(d)),
        16 => format!("{} {}", loc.day_abbr(d), loc.month_day(d)),
        17 => format!("{} {}", loc.day_abbr(d), d.day()),
        18 => format!("{week}/{}", d.weekday().number_from_monday()),
        19 => format!("{week}/{}/{:02} {}", d.weekday().number_from_monday(), loc.year(d).rem_euclid(100), loc.time(h, m)),
        _ => loc.numeric(d, true),
    }
}

/// The labels shown in Options > General > Date format, rendered for a sample date like MPP does.
pub fn format_names(loc: &Locale) -> Vec<String> {
    let sample = NaiveDate::from_ymd_opt(2009, 1, 28).unwrap();
    (0..=20u8).map(|f| format(f, sample, (12, 33), loc)).collect()
}

/// Default format: `pjDate_ddd_mm_dd_yy` ("Wed 1/28/09").
pub const DEFAULT_FORMAT: u8 = 9;

/// Parse a date typed by the user in the locale's day/month order. Accepts an optional leading weekday,
/// 2- or 4-digit years, Buddhist years (2561 -> 2018) and ISO `2018-03-01`.
pub fn parse(s: &str, loc: &Locale) -> Option<NaiveDate> {
    let s = s.trim();
    let s = match s.split_once(' ') {
        Some((a, b)) if a.chars().all(|c| c.is_alphabetic() || c == '.') && !b.trim().is_empty() => b.trim(),
        _ => s,
    };
    // strip a time of day
    let s = s.split(' ').next().unwrap_or(s);
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(d);
    }
    let parts: Vec<&str> = s.split(['/', '-', '.']).filter(|p| !p.trim().is_empty()).collect();
    let nums: Vec<i32> = parts.iter().map(|p| p.trim().parse::<i32>()).collect::<Result<_, _>>().ok()?;
    let today = chrono::Local::now().date_naive();
    let (d, m, y) = match nums.len() {
        2 => match loc.order {
            Order::Mdy => (nums[1], nums[0], today.year()),
            _ => (nums[0], nums[1], today.year()),
        },
        3 => match loc.order {
            Order::Mdy => (nums[1], nums[0], fix_year(nums[2], loc)),
            Order::Ymd => (nums[2], nums[1], fix_year(nums[0], loc)),
            Order::Dmy => (nums[0], nums[1], fix_year(nums[2], loc)),
        },
        _ => return None,
    };
    NaiveDate::from_ymd_opt(y, m as u32, d as u32)
}

fn fix_year(y: i32, loc: &Locale) -> i32 {
    let mut y = y;
    if y < 100 {
        // two-digit years are in the locale's calendar: '61 is 2561 on a Thai PC
        y += if loc.buddhist() { 2500 } else { 2000 };
    }
    if y > 2400 {
        y -= 543;
    }
    y
}

/// The Windows user locale with day and month names in the program's display language and the
/// year from `calendar_type` (0 = automatic: Buddhist with Thai, Gregorian with English; 1 =
/// Gregorian; 2 = Thai Buddhist). Date order and separators stay the user's regional format.
pub fn for_display(thai: bool, calendar_type: u8) -> Locale {
    let mut l = system().clone();
    if l.name.starts_with("th") != thai {
        let names = windows_locale_named(Some(if thai { "th-TH" } else { "en-US" })).unwrap_or_else(|| if thai { thai_names() } else { Locale::english() });
        l.day_abbr = names.day_abbr;
        l.day_long = names.day_long;
        l.month_abbr = names.month_abbr;
        l.month_long = names.month_long;
        if !l.hour24 {
            l.am = names.am;
            l.pm = names.pm;
        }
    }
    l.year_offset = match calendar_type {
        1 => 0,
        2 => 543,
        _ => {
            if thai {
                543
            } else {
                0
            }
        }
    };
    l
}

/// Thai names without Windows (other platforms).
fn thai_names() -> Locale {
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    let mut l = Locale::english();
    l.day_abbr = s(&["จ", "อ", "พ", "พ", "ศ", "ส", "อ"]).try_into().unwrap();
    l.day_long = s(&["จันทร์", "อังคาร", "พุธ", "พฤหัสบดี", "ศุกร์", "เสาร์", "อาทิตย์"]).try_into().unwrap();
    l.month_abbr = s(&["ม.ค.", "ก.พ.", "มี.ค.", "เม.ย.", "พ.ค.", "มิ.ย.", "ก.ค.", "ส.ค.", "ก.ย.", "ต.ค.", "พ.ย.", "ธ.ค."]).try_into().unwrap();
    l.month_long = s(&["มกราคม", "กุมภาพันธ์", "มีนาคม", "เมษายน", "พฤษภาคม", "มิถุนายน", "กรกฎาคม", "สิงหาคม", "กันยายน", "ตุลาคม", "พฤศจิกายน", "ธันวาคม"]).try_into().unwrap();
    l.name = "th-TH".into();
    l
}

#[cfg(windows)]
fn windows_locale() -> Option<Locale> {
    windows_locale_named(None)
}

/// A Windows locale by name (None: the user's own).
#[cfg(windows)]
fn windows_locale_named(want: Option<&str>) -> Option<Locale> {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetUserDefaultLocaleName(name: *mut u16, len: i32) -> i32;
        fn GetLocaleInfoEx(name: *const u16, lctype: u32, data: *mut u16, len: i32) -> i32;
    }
    let name_w: Vec<u16> = match want {
        Some(w) => w.encode_utf16().chain(std::iter::once(0)).collect(),
        None => {
            let mut buf = [0u16; 85];
            let n = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), 85) };
            if n <= 0 {
                return None;
            }
            buf[..n as usize].to_vec() // includes the terminating NUL
        }
    };
    let name = String::from_utf16_lossy(&name_w[..name_w.len().saturating_sub(1)]);
    let info = |t: u32| -> String {
        let mut out = [0u16; 128];
        let k = unsafe { GetLocaleInfoEx(name_w.as_ptr(), t, out.as_mut_ptr(), 128) };
        if k <= 0 {
            String::new()
        } else {
            String::from_utf16_lossy(&out[..(k as usize).saturating_sub(1)])
        }
    };
    let mut loc = Locale::english();
    loc.name = name.clone();
    let thai = name.starts_with("th");
    for i in 0..7u32 {
        let abbr = info(0x31 + i); // LOCALE_SABBREVDAYNAME1.. (Monday first)
        let long = info(0x2A + i);
        if !abbr.is_empty() {
            let a = abbr.trim_end_matches('.').to_string();
            // MPP shows a single letter per weekday on Thai systems (พ 1/3/61)
            loc.day_abbr[i as usize] = if thai { a.chars().take(1).collect() } else { a };
        }
        if !long.is_empty() {
            loc.day_long[i as usize] = long;
        }
    }
    for i in 0..12u32 {
        let abbr = info(0x44 + i); // LOCALE_SABBREVMONTHNAME1..
        let long = info(0x38 + i);
        if !abbr.is_empty() {
            loc.month_abbr[i as usize] = abbr;
        }
        if !long.is_empty() {
            loc.month_long[i as usize] = long;
        }
    }
    let sep = info(0x1D);
    if !sep.is_empty() {
        loc.date_sep = sep;
    }
    let tsep = info(0x1E);
    if !tsep.is_empty() {
        loc.time_sep = tsep;
    }
    let short = info(0x1F).to_ascii_lowercase();
    let pos = |c: char| short.find(c).unwrap_or(usize::MAX);
    loc.order = if pos('y') < pos('m') && pos('y') < pos('d') {
        Order::Ymd
    } else if pos('m') < pos('d') {
        Order::Mdy
    } else {
        Order::Dmy
    };
    loc.hour24 = info(0x23) == "1";
    let am = info(0x28);
    let pm = info(0x29);
    if !am.is_empty() {
        loc.am = am;
    }
    if !pm.is_empty() {
        loc.pm = pm;
    }
    // LOCALE_ICALENDARTYPE: 7 = Thai Buddhist
    if info(0x1009) == "7" {
        loc.year_offset = 543;
    }
    Some(loc)
}

#[cfg(not(windows))]
fn windows_locale() -> Option<Locale> {
    None
}

#[cfg(not(windows))]
fn windows_locale_named(_want: Option<&str>) -> Option<Locale> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thai() -> Locale {
        let mut l = Locale::english();
        l.order = Order::Dmy;
        l.year_offset = 543;
        l.day_abbr = ["จ", "อ", "พ", "พ", "ศ", "ส", "อ"].map(String::from);
        l
    }

    #[test]
    fn long_dates() {
        let d = NaiveDate::from_ymd_opt(2026, 11, 5).unwrap();
        let mut th = thai();
        th.month_long[10] = "พฤศจิกายน".into();
        assert_eq!(th.long_date(d), "5 พฤศจิกายน 2569");
        assert_eq!(Locale::english().long_date(d), "November 5, 2026");
    }

    #[test]
    fn formats_like_project() {
        let d = NaiveDate::from_ymd_opt(2018, 3, 1).unwrap();
        assert_eq!(format(9, d, (8, 0), &thai()), "พ 1/3/61");
        assert_eq!(format(12, d, (8, 0), &thai()), "1/3");
        assert_eq!(format(9, d, (8, 0), &Locale::english()), "Thu 3/1/18");
        assert_eq!(format(0, d, (17, 0), &Locale::english()), "3/1/18 5:00 PM");
        assert_eq!(format(10, d, (8, 0), &Locale::english()), "Thu Mar 1, '18");
    }

    #[test]
    fn parses_locale_dates() {
        let d = NaiveDate::from_ymd_opt(2018, 3, 1).unwrap();
        assert_eq!(parse("พ 1/3/61", &thai()), Some(d));
        assert_eq!(parse("1/3/2561", &thai()), Some(d));
        assert_eq!(parse("Thu 3/1/18", &Locale::english()), Some(d));
        assert_eq!(parse("2018-03-01", &thai()), Some(d));
    }
}
