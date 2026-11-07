//! Thai display shaping for egui, which renders glyphs without OpenType GSUB/GPOS.
//!
//! `tools/thai_preshape.py` asked HarfBuzz how Noto Sans Thai positions every syllable cluster
//! (consonant + lower vowel + upper vowel + tone mark, SARA AM) and baked the shaped glyphs into
//! the bundled font on Private Use code points. `shape` swaps each cluster for that pre-shaped
//! text, so tone marks sit above upper vowels, ป/ฝ/ฟ use narrow marks, and ญ/ฐ lose their tails
//! before lower vowels — exactly as a shaping engine would draw them.
//!
//! Only used for drawing; stored data and text being edited keep the original characters.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::OnceLock;

static TABLE: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

fn table() -> &'static HashMap<&'static str, &'static str> {
    TABLE.get_or_init(|| {
        include_str!("../assets/fonts/thai_clusters.txt")
            .lines()
            .filter_map(|l| l.split_once('\t'))
            .collect()
    })
}

fn is_consonant(c: char) -> bool {
    ('\u{0E01}'..='\u{0E2E}').contains(&c)
}

fn is_mark(c: char) -> bool {
    matches!(c, '\u{0E31}' | '\u{0E34}'..='\u{0E3A}' | '\u{0E47}'..='\u{0E4E}')
}

/// Thai combining marks (upper / lower vowels, tone marks) and pre-shaped mark glyphs.
pub fn is_combining(c: char) -> bool {
    is_mark(c) || ('\u{E100}'..='\u{E8FF}').contains(&c) && !is_consonant(c)
}

/// Break `text` (already shaped) into lines no wider than `max_w`, measured by `width`.
/// Breaks at spaces first, then before Thai leading vowels or after a final consonant cluster,
/// and never between a base letter and its marks. Returns the text with '\n' inserted.
pub fn wrap(text: &str, max_w: f32, width: impl Fn(&str) -> f32) -> String {
    let chars: Vec<char> = text.chars().collect();
    // cluster boundaries: indices where a new cluster starts
    let mut starts: Vec<usize> = Vec::new();
    for (i, c) in chars.iter().enumerate() {
        if i == 0 || !is_combining(*c) {
            starts.push(i);
        }
    }
    starts.push(chars.len());
    let mut lines: Vec<String> = Vec::new();
    let mut line_start = 0usize; // index into chars
    let mut k = 1; // next cluster end (index into starts)
    while line_start < chars.len() {
        // extend while it fits
        let mut best_end = None;
        let mut j = k;
        while j < starts.len() {
            let end = starts[j];
            let s: String = chars[line_start..end].iter().collect();
            if width(s.trim_end()) <= max_w || best_end.is_none() {
                best_end = Some(j);
                j += 1;
            } else {
                break;
            }
        }
        let mut end_j = best_end.unwrap_or(k);
        if end_j < starts.len() - 1 {
            // prefer a nicer break point inside the fitting part
            let lo = (line_start..).take(1).next().unwrap();
            let mut chosen = None;
            for jj in (k..=end_j).rev() {
                let e = starts[jj];
                if e <= lo {
                    break;
                }
                let next = chars.get(e).copied();
                let prev = chars.get(e.wrapping_sub(1)).copied();
                if prev == Some(' ') || next == Some(' ') {
                    chosen = Some(jj);
                    break;
                }
            }
            if chosen.is_none() {
                for jj in (k..=end_j).rev() {
                    let e = starts[jj];
                    if matches!(chars.get(e), Some('\u{0E40}'..='\u{0E44}')) && jj > k {
                        chosen = Some(jj);
                        break;
                    }
                }
            }
            if let Some(c) = chosen {
                // keep at least a third of the line before a soft break
                let s: String = chars[line_start..starts[c]].iter().collect();
                if width(&s) >= max_w * 0.33 {
                    end_j = c;
                }
            }
        }
        let end = starts[end_j];
        let line: String = chars[line_start..end].iter().collect();
        lines.push(line.trim().to_string());
        line_start = end;
        while line_start < chars.len() && chars[line_start] == ' ' {
            line_start += 1;
        }
        k = starts.iter().position(|s| *s > line_start).unwrap_or(starts.len() - 1).max(1);
        if lines.len() > 64 {
            break;
        }
    }
    lines.join("\n")
}

/// Greedy wrapping at spaces only (words longer than the width stay on their own line).
pub fn wrap_words(text: &str, max_w: f32, width: impl Fn(&str) -> f32) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in text.split(' ').filter(|w| !w.is_empty()) {
        let candidate = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
        if cur.is_empty() || width(&candidate) <= max_w {
            cur = candidate;
        } else {
            lines.push(std::mem::take(&mut cur));
            cur = word.to_string();
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines.join("\n")
}

/// Text ready for display with correct Thai mark placement.
pub fn shape(s: &str) -> Cow<'_, str> {
    if !s.chars().any(is_mark) && !s.contains('\u{0E33}') {
        return Cow::Borrowed(s);
    }
    let t = table();
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len() + 8);
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if is_consonant(c) {
            let mut j = i + 1;
            while j < chars.len() && is_mark(chars[j]) {
                j += 1;
            }
            if j < chars.len() && chars[j] == '\u{0E33}' {
                j += 1;
            }
            if j > i + 1 {
                let cluster: String = chars[i..j].iter().collect();
                match t.get(cluster.as_str()) {
                    Some(rep) => out.push_str(rep),
                    None => out.push_str(&cluster),
                }
                i = j;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    Cow::Owned(out)
}

/// A Thai number in words: 121 → "หนึ่งร้อยยี่สิบเอ็ด". `edt`: a final 1 is read "เอ็ด" (the
/// number has more digits above it, as the Royal Institute writes 101 หนึ่งร้อยเอ็ด, 1,001 หนึ่งพันเอ็ด).
fn number_words(n: u64, edt: bool) -> String {
    const DIGIT: [&str; 10] = ["", "หนึ่ง", "สอง", "สาม", "สี่", "ห้า", "หก", "เจ็ด", "แปด", "เก้า"];
    const PLACE: [&str; 6] = ["", "สิบ", "ร้อย", "พัน", "หมื่น", "แสน"];
    if n >= 1_000_000 {
        let (hi, lo) = (n / 1_000_000, n % 1_000_000);
        let mut s = number_words(hi, false) + "ล้าน";
        if lo > 0 {
            s += &number_words(lo, true);
        }
        return s;
    }
    let digits: Vec<u64> = n.to_string().bytes().map(|b| (b - b'0') as u64).collect();
    let len = digits.len();
    let mut s = String::new();
    for (k, &d) in digits.iter().enumerate() {
        let place = len - 1 - k;
        if d == 0 {
            continue;
        }
        match (place, d) {
            (0, 1) if edt || len > 1 => s.push_str("เอ็ด"),
            (1, 1) => s.push_str("สิบ"),
            (1, 2) => s.push_str("ยี่สิบ"),
            _ => {
                s.push_str(DIGIT[d as usize]);
                s.push_str(PLACE[place]);
            }
        }
    }
    s
}

/// An amount of baht in words, as written on cheques and contracts: 1,250.50 →
/// "หนึ่งพันสองร้อยห้าสิบบาทห้าสิบสตางค์", 100 → "หนึ่งร้อยบาทถ้วน".
pub fn baht_text(v: f64) -> String {
    if !v.is_finite() {
        return String::new();
    }
    let neg = v < 0.0;
    let cents = (v.abs() * 100.0).round() as u64;
    let (baht, satang) = (cents / 100, cents % 100);
    let mut s = String::from(if neg { "ลบ" } else { "" });
    if baht == 0 && satang == 0 {
        return s + "ศูนย์บาทถ้วน";
    }
    if baht > 0 {
        s += &number_words(baht, false);
        s += "บาท";
    }
    if satang == 0 {
        s += "ถ้วน";
    } else {
        s += &number_words(satang, false);
        s += "สตางค์";
    }
    s
}

/// A key that sorts text in Thai dictionary order (Windows th-TH collation, case-insensitive):
/// leading vowels เ แ โ ใ ไ count after the consonant they precede, tone marks only break ties.
pub fn sort_key(s: &str) -> Vec<u8> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Globalization::{LCMapStringEx, LCMAP_SORTKEY, NORM_IGNORECASE};
        let name: Vec<u16> = "th-TH\0".encode_utf16().collect();
        let src: Vec<u16> = s.encode_utf16().collect();
        if src.is_empty() {
            return Vec::new();
        }
        unsafe {
            let flags = LCMAP_SORTKEY | NORM_IGNORECASE;
            let n = LCMapStringEx(name.as_ptr(), flags, src.as_ptr(), src.len() as i32, std::ptr::null_mut(), 0, std::ptr::null(), std::ptr::null(), 0);
            if n > 0 {
                let mut buf = vec![0u8; n as usize];
                let k = LCMapStringEx(name.as_ptr(), flags, src.as_ptr(), src.len() as i32, buf.as_mut_ptr() as *mut u16, n, std::ptr::null(), std::ptr::null(), 0);
                if k > 0 {
                    buf.truncate(k as usize);
                    return buf;
                }
            }
        }
    }
    fallback_sort_key(s)
}

/// Without the Windows collation: swap each leading vowel with its consonant, drop the tone marks
/// for the first comparison and append them as a tie-breaker.
fn fallback_sort_key(s: &str) -> Vec<u8> {
    let chars: Vec<char> = s.to_lowercase().chars().collect();
    let mut base = String::new();
    let mut tones = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if ('\u{0E40}'..='\u{0E44}').contains(&c) && chars.get(i + 1).is_some_and(|n| is_consonant(*n)) {
            base.push(chars[i + 1]);
            base.push(c);
            i += 2;
            continue;
        }
        if matches!(c, '\u{0E48}'..='\u{0E4C}') {
            tones.push(c);
        } else {
            base.push(c);
        }
        i += 1;
    }
    let mut key = base.into_bytes();
    key.push(0);
    key.extend(tones.into_bytes());
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_known_clusters() {
        // tone mark after an upper vowel uses a raised (pre-shaped) glyph
        let s = shape("ชั้น");
        assert_ne!(s, "ชั้น");
        assert!(s.chars().any(|c| ('\u{E100}'..='\u{E8FF}').contains(&c)));
        // ญ before a lower vowel loses its tail
        assert_ne!(shape("กตัญญู"), "กตัญญู");
        // SARA AM after a tone mark
        assert_ne!(shape("น้ำ"), "น้ำ");
        // wrapping never leaves a mark at the start of a line
        let w = wrap(&shape("งานเตรียมการสำนักงานสนามและสโตร์"), 60.0, |t| t.chars().filter(|c| !is_combining(*c)).count() as f32 * 6.0);
        for line in w.lines() {
            assert!(!line.chars().next().map(is_combining).unwrap_or(false), "{w:?}");
        }
        assert_eq!(wrap("Gantt Chart Format", 40.0, |t| t.len() as f32 * 6.0), "Gantt\nChart\nFormat");
        // Thai dictionary order: กา < ไก่ < ข, and เสา sorts under ส (not after ฮ)
        let mut v = vec!["ฮ", "เสา", "ข", "ไก่", "กา", "Zebra", "apple"];
        v.sort_by_key(|s| sort_key(s));
        let pos = |w: &str| v.iter().position(|x| *x == w).unwrap();
        assert!(pos("กา") < pos("ไก่") && pos("ไก่") < pos("ข"), "{v:?}");
        assert!(pos("เสา") < pos("ฮ"), "{v:?}");
        assert!(pos("apple") < pos("Zebra"), "{v:?}");
        let mut f = vec!["ฮ", "เสา", "ข", "ไก่", "กา"];
        f.sort_by_key(|s| fallback_sort_key(s));
        assert_eq!(f, vec!["กา", "ไก่", "ข", "เสา", "ฮ"]);
        // baht in words
        for (v, want) in [
            (0.0, "ศูนย์บาทถ้วน"),
            (1.0, "หนึ่งบาทถ้วน"),
            (11.0, "สิบเอ็ดบาทถ้วน"),
            (21.0, "ยี่สิบเอ็ดบาทถ้วน"),
            (101.0, "หนึ่งร้อยเอ็ดบาทถ้วน"),
            (1001.0, "หนึ่งพันเอ็ดบาทถ้วน"),
            (1_000_000.0, "หนึ่งล้านบาทถ้วน"),
            (1_000_001.0, "หนึ่งล้านเอ็ดบาทถ้วน"),
            (11_000_000.0, "สิบเอ็ดล้านบาทถ้วน"),
            (21_500_000.0, "ยี่สิบเอ็ดล้านห้าแสนบาทถ้วน"),
            (4_293_920.0, "สี่ล้านสองแสนเก้าหมื่นสามพันเก้าร้อยยี่สิบบาทถ้วน"),
            (120.5, "หนึ่งร้อยยี่สิบบาทห้าสิบสตางค์"),
            (0.25, "ยี่สิบห้าสตางค์"),
            (2.01, "สองบาทหนึ่งสตางค์"),
            (1e12, "หนึ่งล้านล้านบาทถ้วน"),
            (-75.0, "ลบเจ็ดสิบห้าบาทถ้วน"),
        ] {
            assert_eq!(baht_text(v), want, "{v}");
        }
        // plain Latin is untouched and not reallocated
        assert!(matches!(shape("Gantt Chart"), Cow::Borrowed(_)));
    }
}
