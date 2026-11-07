//! Reader for .mpp / .mpt project files (MPP12 and MPP14).
//!
//! Layout of the format:
//! - The file is an OLE compound document; project data lives under the "   114" (or "   112") storage.
//! - `Props` holds project properties and *field maps* (key 0x00020014 tasks, 0x00020015 resources)
//!   that tell where each field (identified by its field constant) lives in the fixed records.
//! - `TBkndTask`, `TBkndRsc`, `TBkndAssn`, `TBkndCons`, `TBkndCal` each hold FixedMeta/FixedData
//!   (fixed-size records), Fixed2Meta/Fixed2Data and VarMeta/Var2Data (variable data keyed by
//!   unique ID + field ID, e.g. names and notes).
//! - Timestamps are two u16 values: time in 1/10 minute and days since 1983-12-31.
//! - Durations and lags are in 1/10 minute.

use crate::model::*;
use chrono::{Duration, NaiveDate};
use std::collections::HashMap;

type R<T> = Result<T, String>;

pub(crate) fn u16_at(b: &[u8], o: usize) -> Option<u16> {
    b.get(o..o + 2).map(|s| u16::from_le_bytes([s[0], s[1]]))
}
pub(crate) fn u32_at(b: &[u8], o: usize) -> Option<u32> {
    b.get(o..o + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}
pub(crate) fn i32_at(b: &[u8], o: usize) -> Option<i32> {
    u32_at(b, o).map(|v| v as i32)
}
pub(crate) fn f64_at(b: &[u8], o: usize) -> Option<f64> {
    b.get(o..o + 8).map(|s| f64::from_le_bytes(s.try_into().unwrap()))
}

pub(crate) fn epoch() -> NaiveDate {
    NaiveDate::from_ymd_opt(1983, 12, 31).unwrap()
}

/// MPP timestamp -> date (time of day is not needed by Neramit Planner's day-based model).
pub(crate) fn date_at(b: &[u8], o: usize) -> Option<NaiveDate> {
    let days = u16_at(b, o + 2)?;
    if days == 0 || days == 0xFFFF {
        return None;
    }
    Some(epoch() + Duration::days(days as i64))
}

/// Date and time: u16 tenths of a minute after midnight, u16 days since the epoch.
pub(crate) fn datetime_at(b: &[u8], o: usize) -> Option<chrono::NaiveDateTime> {
    let d = date_at(b, o)?;
    let tenths = u16_at(b, o).filter(|t| *t != 0xFFFF).unwrap_or(0) as i64;
    Some(d.and_time(chrono::NaiveTime::MIN) + Duration::minutes(tenths / 10))
}

pub(crate) fn utf16(b: &[u8]) -> String {
    let units: Vec<u16> = b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    String::from_utf16_lossy(&units).trim_end_matches('\0').to_string()
}

// =========================================================================== compound file

struct Entry {
    /// the 128-byte directory entry as stored
    raw: Vec<u8>,
    name: String,
    kind: u8,
    left: u32,
    right: u32,
    child: u32,
    start: u32,
    size: u64,
}

pub(crate) struct Cfb<'a> {
    b: &'a [u8],
    ssz: usize,
    mssz: usize,
    fat: Vec<u32>,
    minifat: Vec<u32>,
    mini_cutoff: u64,
    ministream: Vec<u8>,
    entries: Vec<Entry>,
}

const END: u32 = 0xFFFF_FFFA;

impl<'a> Cfb<'a> {
    pub(crate) fn new(b: &'a [u8]) -> R<Self> {
        if b.len() < 512 || b[..8] != [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1] {
            return Err("Not a .mpp project file (missing compound-file signature).".into());
        }
        let ssz = 1usize << u16_at(b, 0x1E).unwrap_or(9).min(16);
        let mssz = 1usize << u16_at(b, 0x20).unwrap_or(6).min(16);
        let nfat = u32_at(b, 0x2C).unwrap_or(0) as usize;
        let dir_start = u32_at(b, 0x30).unwrap_or(END);
        let mini_cutoff = u32_at(b, 0x38).unwrap_or(4096) as u64;
        let minifat_start = u32_at(b, 0x3C).unwrap_or(END);
        let mut difat_sec = u32_at(b, 0x44).unwrap_or(END);
        let ndifat = u32_at(b, 0x48).unwrap_or(0);
        let mut difat: Vec<u32> = (0..109).filter_map(|i| u32_at(b, 0x4C + i * 4)).collect();
        let mut cfb = Cfb { b, ssz, mssz, fat: Vec::new(), minifat: Vec::new(), mini_cutoff, ministream: Vec::new(), entries: Vec::new() };
        for _ in 0..ndifat {
            if difat_sec >= END {
                break;
            }
            let sec = cfb.sector(difat_sec).to_vec();
            let n = ssz / 4;
            for i in 0..n - 1 {
                difat.push(u32_at(&sec, i * 4).unwrap_or(END));
            }
            difat_sec = u32_at(&sec, (n - 1) * 4).unwrap_or(END);
        }
        let mut fat = Vec::new();
        for &fs in difat.iter().take(nfat) {
            if fs >= END {
                continue;
            }
            let sec = cfb.sector(fs);
            for i in 0..ssz / 4 {
                fat.push(u32_at(sec, i * 4).unwrap_or(END));
            }
        }
        cfb.fat = fat;
        if minifat_start < END {
            let d = cfb.chain(minifat_start);
            cfb.minifat = (0..d.len() / 4).map(|i| u32_at(&d, i * 4).unwrap()).collect();
        }
        let dir = cfb.chain(dir_start);
        for e in dir.chunks_exact(128) {
            let nlen = (u16_at(e, 64).unwrap_or(0) as usize).min(64);
            let name = utf16(&e[..nlen.saturating_sub(2)]);
            cfb.entries.push(Entry {
                raw: e.to_vec(),
                name,
                kind: e[66],
                left: u32_at(e, 68).unwrap(),
                right: u32_at(e, 72).unwrap(),
                child: u32_at(e, 76).unwrap(),
                start: u32_at(e, 116).unwrap(),
                size: u32_at(e, 120).unwrap() as u64,
            });
        }
        if cfb.entries.is_empty() {
            return Err("Damaged file: no directory entries.".into());
        }
        let (rs, rsize) = (cfb.entries[0].start, cfb.entries[0].size);
        if rs < END {
            let mut ms = cfb.chain(rs);
            ms.truncate(rsize as usize);
            cfb.ministream = ms;
        }
        Ok(cfb)
    }

    pub(crate) fn sector_size(&self) -> usize {
        self.ssz
    }
    pub(crate) fn entry_count(&self) -> usize {
        self.entries.len()
    }
    /// 1 storage, 2 stream, 5 root
    pub(crate) fn entry_kind(&self, i: usize) -> u8 {
        self.entries[i].kind
    }
    pub(crate) fn entry_data(&self, i: usize) -> Vec<u8> {
        self.read(&self.entries[i])
    }
    pub(crate) fn entry_raw(&self, i: usize) -> &[u8] {
        &self.entries[i].raw
    }
    /// Directory entry index of a stream by its path.
    pub(crate) fn entry_index(&self, path: &[&str]) -> Option<usize> {
        self.find(path)
    }

    fn sector(&self, n: u32) -> &[u8] {
        let o = (n as usize + 1) * self.ssz;
        self.b.get(o..(o + self.ssz).min(self.b.len())).unwrap_or(&[])
    }

    fn chain(&self, start: u32) -> Vec<u8> {
        let mut out = Vec::new();
        let mut n = start;
        let mut guard = 0;
        while n < END && guard <= self.fat.len() {
            out.extend_from_slice(self.sector(n));
            n = *self.fat.get(n as usize).unwrap_or(&END);
            guard += 1;
        }
        out
    }

    fn read(&self, e: &Entry) -> Vec<u8> {
        if e.size < self.mini_cutoff {
            let mut out = Vec::new();
            let mut n = e.start;
            let mut guard = 0;
            while n < END && guard <= self.minifat.len() {
                let o = n as usize * self.mssz;
                out.extend_from_slice(self.ministream.get(o..(o + self.mssz).min(self.ministream.len())).unwrap_or(&[]));
                n = *self.minifat.get(n as usize).unwrap_or(&END);
                guard += 1;
            }
            out.truncate(e.size as usize);
            out
        } else {
            let mut d = self.chain(e.start);
            d.truncate(e.size as usize);
            d
        }
    }

    /// Children of a storage entry (in-order walk of its red-black tree).
    fn children(&self, idx: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut stack = Vec::new();
        let mut cur = self.entries.get(idx).map(|e| e.child).unwrap_or(u32::MAX);
        let mut guard = 0;
        while (cur != u32::MAX || !stack.is_empty()) && guard < self.entries.len() * 4 {
            guard += 1;
            while cur != u32::MAX && (cur as usize) < self.entries.len() {
                stack.push(cur as usize);
                cur = self.entries[cur as usize].left;
            }
            let Some(i) = stack.pop() else { break };
            out.push(i);
            cur = self.entries[i].right;
        }
        out
    }

    fn find(&self, path: &[&str]) -> Option<usize> {
        let mut idx = 0;
        for part in path {
            idx = self.children(idx).into_iter().find(|&c| self.entries[c].name == *part)?;
        }
        Some(idx)
    }

    pub(crate) fn stream(&self, path: &[&str]) -> Vec<u8> {
        self.find(path).filter(|&i| self.entries[i].kind == 2).map(|i| self.read(&self.entries[i])).unwrap_or_default()
    }
}

// =========================================================================== MPP structures

/// Props stream: 16-byte header, then items of [size u32][key u32][attr u32][data].
/// Neramit Planner's own Props item: the Gantt chart's drawings (JSON).
pub(crate) const P_NERAMIT_DRAWINGS: u32 = 0x7B42_0001;
/// Neramit Planner's own Props item: sprints in a file from an MPP without them (no TBkndSprint
/// table): {"sprints": [...], "tasks": [[file UID, sprint UID]], "next_up": [file UID]}.
pub(crate) const P_NERAMIT_SPRINTS: u32 = 0x7B42_0002;
/// Neramit Planner's own Props item: Project > Payment Schedule (JSON, tasks by file UID).
pub(crate) const P_NERAMIT_PAYMENTS: u32 = 0x7B42_0003;

pub(crate) fn parse_props(d: &[u8]) -> HashMap<u32, Vec<u8>> {
    let mut out = HashMap::new();
    let count = u32_at(d, 12).unwrap_or(0);
    let mut o = 16;
    for _ in 0..count {
        let (Some(size), Some(key)) = (u32_at(d, o), u32_at(d, o + 4)) else { break };
        let start = o + 12;
        let end = start + size as usize;
        if end > d.len() {
            break;
        }
        out.insert(key, d[start..end].to_vec());
        // (an item of odd length is padded to an even one)
        o = end.div_ceil(2) * 2;
    }
    out
}

/// Field map: field id -> (fixed block index, byte offset). Only fixed-data fields are recorded.
pub(crate) fn parse_field_map(d: &[u8]) -> HashMap<u32, (usize, usize)> {
    let mut out = HashMap::new();
    let mut block = 0;
    let mut last: i64 = -1;
    for e in d.chunks_exact(28) {
        let off = u16_at(e, 4).unwrap() as i64;
        let ft = u32_at(e, 12).unwrap();
        if off != 0xFFFF {
            if off < last {
                block += 1;
            }
            last = off;
            out.insert(ft, (block, off as usize));
        }
    }
    out
}

/// VarMeta + Var2Data -> (unique id, field id) -> bytes.
pub(crate) fn parse_var(meta: &[u8], data: &[u8]) -> HashMap<(u32, u32), Vec<u8>> {
    let mut out = HashMap::new();
    let n = u32_at(meta, 8).unwrap_or(0) as usize;
    for i in 0..n {
        let base = 0x18 + i * 12;
        let (Some(uid), Some(off), Some(ft)) = (u32_at(meta, base), u32_at(meta, base + 4), u32_at(meta, base + 8)) else { break };
        let off = off as usize;
        let Some(size) = u32_at(data, off) else { continue };
        if let Some(v) = data.get(off + 4..off + 4 + size as usize) {
            out.insert((uid, ft), v.to_vec());
        }
    }
    out
}

/// FixedMeta entries: (meta bytes, record offset, record length).
fn parse_fixed<'a>(meta: &'a [u8], data: &[u8], item: usize) -> Vec<(&'a [u8], usize, usize)> {
    let n = u32_at(meta, 8).unwrap_or(0) as usize;
    let mut offs: Vec<(&[u8], usize)> = Vec::new();
    for i in 0..n {
        let base = 16 + i * item;
        let Some(m) = meta.get(base..base + item) else { break };
        offs.push((m, u32_at(m, 4).unwrap_or(0) as usize));
    }
    let mut sorted: Vec<usize> = offs.iter().map(|o| o.1).collect();
    sorted.sort_unstable();
    sorted.dedup();
    offs.into_iter()
        .map(|(m, off)| {
            let next = sorted.iter().copied().find(|&o| o > off).unwrap_or(data.len());
            (m, off, next.saturating_sub(off))
        })
        .collect()
}

// =========================================================================== RTF notes

/// Plain text -> the RTF MPP keeps notes in (Unicode characters as \uN?, lines as \par).
pub(crate) fn text_to_rtf(text: &str) -> Vec<u8> {
    let mut s = String::from("{\\rtf1\\ansi\\ansicpg1252\\deff0{\\fonttbl{\\f0\\fnil Segoe UI;}}\\uc1\\pard\\f0\\fs20 ");
    for (k, line) in text.split('\n').enumerate() {
        if k > 0 {
            s.push_str("\\par ");
        }
        for c in line.trim_end_matches('\r').chars() {
            match c {
                '\\' | '{' | '}' => {
                    s.push('\\');
                    s.push(c);
                }
                '\t' => s.push_str("\\tab "),
                c if (c as u32) < 0x80 => s.push(c),
                c => {
                    // RTF \u takes a signed 16-bit value; characters beyond the BMP as surrogates
                    let mut buf = [0u16; 2];
                    for u in c.encode_utf16(&mut buf) {
                        s.push_str(&format!("\\u{}?", *u as i16));
                    }
                }
            }
        }
    }
    s.push_str("\\par}");
    s.push('\0');
    s.into_bytes()
}

/// Minimal RTF -> plain text: handles \par, \tab, \'hh (code page 874 / 1252) and \uN.
pub(crate) fn rtf_to_text(rtf: &[u8]) -> String {
    let s = String::from_utf8_lossy(rtf);
    if !s.starts_with("{\\rtf") {
        return s.trim_end_matches('\0').to_string();
    }
    let cp874 = s.contains("\\ansicpg874");
    let b: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let mut depth = 0i32;
    let mut skip_depth: Option<i32> = None;
    let mut uc = 1usize;
    let mut pending_skip = 0usize;
    let byte_char = |v: u8| -> char {
        if cp874 && v >= 0xA1 && v <= 0xFB {
            char::from_u32(0x0E01 + (v as u32 - 0xA1)).unwrap_or('?')
        } else {
            v as char
        }
    };
    while i < b.len() {
        let c = b[i];
        match c {
            '{' => {
                depth += 1;
                // destinations we don't want: {\* ...}, font/color tables, info
                let rest: String = b[i + 1..(i + 16).min(b.len())].iter().collect();
                if skip_depth.is_none()
                    && (rest.starts_with("\\*") || rest.starts_with("\\fonttbl") || rest.starts_with("\\colortbl") || rest.starts_with("\\stylesheet") || rest.starts_with("\\info"))
                {
                    skip_depth = Some(depth);
                }
                i += 1;
            }
            '}' => {
                if skip_depth == Some(depth) {
                    skip_depth = None;
                }
                depth -= 1;
                i += 1;
            }
            '\\' => {
                i += 1;
                if i >= b.len() {
                    break;
                }
                let n = b[i];
                if n == '\'' {
                    let hex: String = b[i + 1..(i + 3).min(b.len())].iter().collect();
                    i += 3;
                    if pending_skip > 0 {
                        pending_skip -= 1;
                    } else if skip_depth.is_none() {
                        if let Ok(v) = u8::from_str_radix(&hex, 16) {
                            out.push(byte_char(v));
                        }
                    }
                } else if n.is_ascii_alphabetic() {
                    let start = i;
                    while i < b.len() && b[i].is_ascii_alphabetic() {
                        i += 1;
                    }
                    let word: String = b[start..i].iter().collect();
                    let nstart = i;
                    if i < b.len() && (b[i] == '-' || b[i].is_ascii_digit()) {
                        i += 1;
                        while i < b.len() && b[i].is_ascii_digit() {
                            i += 1;
                        }
                    }
                    let num: Option<i32> = b[nstart..i].iter().collect::<String>().parse().ok();
                    if i < b.len() && b[i] == ' ' {
                        i += 1;
                    }
                    if skip_depth.is_some() {
                        continue;
                    }
                    match word.as_str() {
                        "par" | "line" => out.push('\n'),
                        "tab" => out.push('\t'),
                        "uc" => uc = num.unwrap_or(1).max(0) as usize,
                        "u" => {
                            let v = num.unwrap_or(63);
                            let v = if v < 0 { v + 65536 } else { v } as u32;
                            if let Some(ch) = char::from_u32(v) {
                                out.push(ch);
                            }
                            pending_skip = uc;
                        }
                        _ => {}
                    }
                } else {
                    // escaped literal like \{ \} \\
                    if skip_depth.is_none() && matches!(n, '{' | '}' | '\\') {
                        out.push(n);
                    }
                    i += 1;
                }
            }
            '\r' | '\n' => i += 1,
            _ => {
                if pending_skip > 0 {
                    pending_skip -= 1;
                } else if skip_depth.is_none() && depth >= 1 {
                    out.push(c);
                }
                i += 1;
            }
        }
    }
    out.trim().to_string()
}

// =========================================================================== reader

// Field identifiers used below.
pub(crate) const T_ID: u32 = 0x0B40_0017;
pub(crate) const T_UID: u32 = 0x0B40_0056;
pub(crate) const T_NAME: u32 = 0x0B40_000E;
pub(crate) const T_NOTES: u32 = 0x0B40_000F;
pub(crate) const T_START: u32 = 0x0B40_0023;
pub(crate) const T_FINISH: u32 = 0x0B40_0024;
pub(crate) const T_DURATION: u32 = 0x0B40_001D;
pub(crate) const T_PERCENT: u32 = 0x0B40_0020;
pub(crate) const T_CTYPE: u32 = 0x0B40_0011;
pub(crate) const T_CDATE: u32 = 0x0B40_0012;
pub(crate) const T_OUTLINE_LEVEL: u32 = 0x0B40_00F9;
pub(crate) const T_MANUAL_START: u32 = 0x0B40_0503;
pub(crate) const T_MANUAL_FINISH: u32 = 0x0B40_0504;
pub(crate) const T_PRIORITY: u32 = 0x0B40_0019;
pub(crate) const T_DEADLINE: u32 = 0x0B40_01B5;
pub(crate) const T_TYPE: u32 = 0x0B40_0080;
pub(crate) const T_WBS: u32 = 0x0B40_0010;
pub(crate) const T_CONTACT: u32 = 0x0B40_0060;
// tracking (checked on real files): actual start / finish are
// fixed-data timestamps, actual duration u32 tenths of a minute; the baseline start / finish /
// duration live in the var data
pub(crate) const T_ACTUAL_START: u32 = 0x0B40_0029;
pub(crate) const T_ACTUAL_FINISH: u32 = 0x0B40_002A;
pub(crate) const T_ACTUAL_DURATION: u32 = 0x0B40_001C;
pub(crate) const T_BASELINE_DURATION: u32 = 0x0B40_001B;
pub(crate) const T_BASELINE_START: u32 = 0x0B40_002B;
pub(crate) const T_BASELINE_FINISH: u32 = 0x0B40_002C;
// costs (the XML of a file it wrote): f64 in cents; accrual u16 1/2/3
pub(crate) const T_FIXED_COST: u32 = 0x0B40_0008;
pub(crate) const T_FIXED_COST_ACCRUAL: u32 = 0x0B40_00C8;
pub(crate) const R_ACCRUE_AT: u32 = 0x0C40_0013;
/// Assignment var data: planned work, timephased (a split task): u16 block count, blocks
/// of 28 bytes from offset 40 = u32 start (1/80 working minute from the assignment start), f64
/// cumulative work, f64 rate; then the end time. A block with no work between blocks with work
/// is a split.
pub(crate) const A_TIMEPHASED_WORK: u32 = 0x0F40_0031;

fn timephased_splits(d: &[u8]) -> Vec<(i64, i64)> {
    let Some(n) = u16_at(d, 0).map(|n| n as usize) else { return Vec::new() };
    if !(3..=500).contains(&n) || d.len() < 40 + 28 * n + 4 {
        return Vec::new();
    }
    let mut blocks = Vec::with_capacity(n);
    for k in 0..n {
        let o = 40 + 28 * k;
        let (Some(t), Some(rate)) = (u32_at(d, o), f64_at(d, o + 12)) else { return Vec::new() };
        blocks.push((t as i64 / 80, rate));
    }
    let end = u32_at(d, 40 + 28 * n).map(|t| t as i64 / 80).unwrap_or(0);
    let first_work = blocks.iter().position(|b| b.1 > 0.0);
    let last_work = blocks.iter().rposition(|b| b.1 > 0.0);
    let (Some(fw), Some(lw)) = (first_work, last_work) else { return Vec::new() };
    let mut out = Vec::new();
    let mut gaps_before = 0;
    for k in fw + 1..lw {
        if blocks[k].1 == 0.0 {
            let st = blocks[k].0;
            let en = if k + 1 < n { blocks[k + 1].0 } else { end };
            if en > st {
                out.push((st - blocks[fw].0 - gaps_before, en - st));
                gaps_before += en - st;
            }
        }
    }
    out
}
pub(crate) const R_NAME: u32 = 0x0C40_0001;
pub(crate) const R_INITIALS: u32 = 0x0C40_0002;
pub(crate) const R_GROUP: u32 = 0x0C40_0003;
pub(crate) const R_NOTES: u32 = 0x0C40_000F;
pub(crate) const R_EMAIL: u32 = 0x0C40_000B;
pub(crate) const R_MATERIAL: u32 = 0x0C40_012B;
pub(crate) const C_NAME: u32 = 0x0D40_0001;
pub(crate) const C_DATA: u32 = 0x0D40_0008;
pub(crate) const P_START: u32 = 0x0240_0002;
pub(crate) const P_TITLE: u32 = 0x0240_0008;
pub(crate) const P_CALENDAR: u32 = 0x0240_000E;
pub(crate) const P_MIN_PER_DAY: u32 = 0x0240_001D;
/// MPP14: File > Options > Schedule (keys found by comparing files with the XML)
pub(crate) const P_DEFAULT_TASK_TYPE: u32 = 0x0240_0031;
pub(crate) const P_NEW_TASKS_MANUAL: u32 = 0x0240_13C8;
/// Options > Schedule flags (the only properties that separate the files whose XML export
/// says Autolink 1 / SplitsInProgressTasks 0 from the others)
pub(crate) const P_SPLIT_IN_PROGRESS: u32 = 0x0240_001A;
pub(crate) const P_AUTOLINK: u32 = 0x0240_001B;

pub(crate) struct Version {
    pub(crate) root: &'static str,
    pub(crate) meta2: usize,
    pub(crate) milestone: (usize, u8),
    pub(crate) manual: Option<(usize, u8)>,
    /// offset of the duration-units word relative to the Duration field; bit 0x20 = estimated
    pub(crate) est_delta: usize,
    pub(crate) link_lag_offset: usize,
    pub(crate) link_format_offset: usize,
}

pub(crate) const V14: Version = Version { root: "   114", meta2: 96, milestone: (10, 1), manual: Some((8, 7)), est_delta: 80, link_lag_offset: 14, link_format_offset: 18 };
pub(crate) const V12: Version = Version { root: "   112", meta2: 86, milestone: (8, 5), manual: None, est_delta: 4, link_lag_offset: 16, link_format_offset: 14 };

pub(crate) fn version_of(cfb: &Cfb) -> R<&'static Version> {
    let roots: Vec<String> = cfb.children(0).iter().map(|&i| cfb.entries[i].name.clone()).collect();
    if roots.iter().any(|r| r == V14.root) {
        Ok(&V14)
    } else if roots.iter().any(|r| r == V12.root) {
        Ok(&V12)
    } else {
        Err("This file is not a supported .mpp file (MPP12 / MPP14).".into())
    }
}

pub fn load_mpp(bytes: &[u8]) -> R<Project> {
    let cfb = Cfb::new(bytes)?;
    let roots: Vec<String> = cfb.children(0).iter().map(|&i| cfb.entries[i].name.clone()).collect();
    let v = if roots.iter().any(|r| r == V14.root) {
        &V14
    } else if roots.iter().any(|r| r == V12.root) {
        &V12
    } else if roots.iter().any(|r| r == "   19" || r == "   18" || r == "   111") {
        return Err("This .mpp file uses an old format (MPP9 or earlier), which Neramit Planner cannot read.\n\
                    Save it again in a newer format, or save it as XML."
            .into());
    } else {
        return Err("This file is not a .mpp project file, or it is password protected.".into());
    };
    let rt = v.root;
    let props = parse_props(&cfb.stream(&[rt, "Props"]));
    let tmap = props.get(&0x0002_0014).map(|d| parse_field_map(d)).ok_or("The task field map is missing (file may be password protected or damaged).")?;
    let mpd = props.get(&P_MIN_PER_DAY).and_then(|d| u32_at(d, 0)).filter(|v| *v > 0).unwrap_or(480) as f64;

    let mut p = Project::new();
    p.file_format = Some(if std::ptr::eq(v, &V14) { 14 } else { 12 });
    p.calendar.hours_per_day = mpd / 60.0;
    if let Some(t) = props.get(&P_TITLE) {
        p.title = utf16(t);
    }
    if let Some(d) = props.get(&P_START).and_then(|d| date_at(d, 0)) {
        p.start = d;
        p.start_dt = props.get(&P_START).and_then(|d| datetime_at(d, 0));
    }
    if std::ptr::eq(v, &V14) {
        // "New tasks: Manually Scheduled" (the standard status bar) and the default task type
        if let Some(m) = props.get(&P_NEW_TASKS_MANUAL).and_then(|d| u16_at(d, 0)) {
            p.new_tasks_auto = m == 0;
        }
        if let Some(m) = props.get(&P_AUTOLINK).and_then(|d| u16_at(d, 0)) {
            p.options.autolink = m != 0;
        }
        if let Some(m) = props.get(&P_SPLIT_IN_PROGRESS).and_then(|d| u16_at(d, 0)) {
            p.options.split_in_progress = m != 0;
        }
        if let Some(t) = props.get(&P_DEFAULT_TASK_TYPE).and_then(|d| u16_at(d, 0)) {
            p.options.default_task_type = match t {
                1 => crate::model::TaskType::FixedDuration,
                2 => crate::model::TaskType::FixedWork,
                _ => crate::model::TaskType::FixedUnits,
            };
        }
    }

    // ---------------- Project > Custom Fields: renamed fields, formulas, lookup tables (cfdefs.rs)
    p.custom_defs = crate::cfdefs::read_defs(&cfb, rt, &props, std::ptr::eq(v, &V14));
    if p.custom_defs.iter().all(|d| d.alias.is_empty()) {
        // (older files: the names found by their field numbers)
        for (table, resource) in [("TBkndTask", false), ("TBkndRsc", true)] {
            for (f, alias) in crate::custom::aliases_in(&cfb.stream(&[rt, table, "Props"]), resource) {
                match p.custom_defs.iter_mut().find(|d| d.field == f && d.resource == resource) {
                    Some(d) => d.alias = alias,
                    None => {
                        let mut d = crate::custom::CustomFieldDef::new(f, resource);
                        d.alias = alias;
                        p.custom_defs.push(d);
                    }
                }
            }
        }
    }

    // ---------------- Project > WBS > Define Code (Props 0x0240138B - u32 levels; each level u16
    // type, u16 length, u16 separator, u16 0, u32 0; u32 flags 1 generate, 2 verify; the prefix)
    if let Some(d) = props.get(&0x0240_138B).filter(|d| d.len() >= 8) {
        let n = u32_at(d, 0).unwrap_or(0) as usize;
        if n > 0 && n <= 10 && d.len() >= 4 + 12 * n + 4 {
            let levels = (0..n)
                .map(|k| {
                    let o = 4 + 12 * k;
                    let sep = char::from_u32(u16_at(d, o + 4).unwrap_or(46) as u32).map(|c| c.to_string()).unwrap_or_else(|| ".".into());
                    crate::wbs::WbsLevel { seq: (u16_at(d, o).unwrap_or(0) as u8).min(3), len: u16_at(d, o + 2).unwrap_or(0) as u8, sep }
                })
                .collect();
            let flags = u32_at(d, 4 + 12 * n).unwrap_or(3);
            let prefix = utf16(&d[(8 + 12 * n).min(d.len())..]);
            p.wbs_mask = Some(crate::wbs::WbsMask { prefix, levels, generate: flags & 1 != 0, verify_unique: flags & 2 != 0 });
        }
    }

    // ---------------- calendars
    let cvar = parse_var(&cfb.stream(&[rt, "TBkndCal", "VarMeta"]), &cfb.stream(&[rt, "TBkndCal", "Var2Data"]));
    let wanted_cal = props.get(&P_CALENDAR).map(|d| utf16(d)).unwrap_or_default();
    let mut cals: Vec<(u32, String, Vec<u8>)> = cvar
        .iter()
        .filter(|((_, ft), _)| *ft == C_DATA)
        .map(|((u, _), d)| (*u, cvar.get(&(*u, C_NAME)).map(|n| utf16(n)).unwrap_or_default(), d.clone()))
        .collect();
    cals.sort_by_key(|c| c.0);
    if let Some((_, name, data)) = cals.iter().find(|c| c.1 == wanted_cal).or_else(|| cals.iter().find(|c| !c.1.is_empty())) {
        p.calendar.name = name.clone();
        apply_calendar(&mut p.calendar, data);
    }

    // ---------------- resources
    let rmap = props.get(&0x0002_0015).map(|d| parse_field_map(d)).unwrap_or_default();
    let rvar = parse_var(&cfb.stream(&[rt, "TBkndRsc", "VarMeta"]), &cfb.stream(&[rt, "TBkndRsc", "Var2Data"]));
    let rfd = cfb.stream(&[rt, "TBkndRsc", "FixedData"]);
    let rfm = cfb.stream(&[rt, "TBkndRsc", "FixedMeta"]);
    let r_uid_off = rmap.get(&0x0C40_001B).map(|x| x.1).unwrap_or(4);
    // Fixed2Meta rows (one per FixedMeta row): byte 8 bit 0x10 marks a cost resource
    let r2m = cfb.stream(&[rt, "TBkndRsc", "Fixed2Meta"]);
    let r2n = u32_at(&r2m, 8).unwrap_or(0) as usize;
    let r2item = if r2n > 0 { (r2m.len().saturating_sub(16)) / r2n } else { 0 };
    // (file ID, file unique ID, resource) sorted by ID, then renumbered for Neramit Planner
    let mut resources: Vec<(u32, u32, Resource)> = Vec::new();
    for (row, (meta, off, len)) in parse_fixed(&rfm, &rfd, 37).into_iter().enumerate() {
        if meta[0] != 0 || len < 32 {
            continue;
        }
        let rec = &rfd[off..off + len];
        let Some(uid) = u32_at(rec, r_uid_off) else { continue };
        let Some(name) = rvar.get(&(uid, R_NAME)).map(|n| utf16(n)) else { continue };
        if uid == 0 || name.is_empty() {
            continue;
        }
        let id = u32_at(rec, rmap.get(&0x0C40_0000).map(|x| x.1).unwrap_or(0)).unwrap_or(uid);
        let mut r = Resource::new(0, &name);
        r.custom = crate::custom::read_mpp(true, &|ft| rvar.get(&(uid, ft)).cloned(), meta);
        // type (resources made in MPP one type at a time): material = FixedMeta byte 14
        // bit 0, cost = Fixed2Meta byte 8 bit 0x10
        let cost = r2item > 8 && r2m.get(16 + row * r2item + 8).map(|b| b & 0x10 != 0).unwrap_or(false);
        r.rtype = if meta.get(14).map(|b| b & 1 == 1).unwrap_or(false) {
            ResourceType::Material
        } else if cost {
            ResourceType::Cost
        } else {
            ResourceType::Work
        };
        if let Some(a) = rmap.get(&R_ACCRUE_AT).and_then(|l| u16_at(rec, l.1)) {
            r.accrue_at = Accrue::from_code(a as i64);
        }
        if let Some(s) = rvar.get(&(uid, R_INITIALS)).map(|n| utf16(n)) {
            r.initials = s;
        }
        r.group = rvar.get(&(uid, R_GROUP)).map(|n| utf16(n)).unwrap_or_default();
        r.notes = rvar.get(&(uid, R_NOTES)).map(|n| rtf_to_text(n)).unwrap_or_default();
        r.email = rvar.get(&(uid, R_EMAIL)).map(|n| utf16(n)).unwrap_or_default();
        r.material_label = rvar.get(&(uid, R_MATERIAL)).map(|n| utf16(n)).unwrap_or_default();
        let f = |ft: u32, dflt: usize| f64_at(rec, rmap.get(&ft).map(|x| x.1).unwrap_or(dflt)).filter(|v| v.is_finite()).unwrap_or(0.0);
        r.max_units = (f(0x0C40_0004, 8) / 100.0).max(0.0);
        if r.max_units == 0.0 {
            r.max_units = 100.0;
        }
        r.std_rate = f(0x0C40_0006, 16);
        r.ovt_rate = f(0x0C40_0007, 24);
        // Cost/Use is stored in cents (rates per hour are not)
        r.cost_per_use = f(0x0C40_0012, 72) / 100.0;
        resources.push((id, uid, r));
    }
    resources.sort_by_key(|r| r.0);
    let mut res_new_uid: HashMap<u32, u32> = HashMap::new();
    for (_, file_uid, mut r) in resources {
        r.uid = p.alloc_res_uid();
        r.src_uid = Some(file_uid);
        res_new_uid.insert(file_uid, r.uid);
        p.resources.push(r);
    }

    // ---------------- Resource > Resource Pool (extlinks.rs): the pool this project shares
    if let Some(path) = props.get(&crate::extlinks::P_POOL_MONIKER).and_then(|m| crate::extlinks::moniker_path(m)) {
        let mode = props.get(&crate::extlinks::P_POOL_MODE).and_then(|d| u16_at(d, 0)).unwrap_or(3);
        if mode == 1 || mode == 2 {
            p.resource_pool = Some(crate::model::ResourcePool { path, sharer_first: mode == 1 });
        }
    }

    // ---------------- Format > Drawing (Neramit Planner's own item)
    if let Some(d) = props.get(&P_NERAMIT_DRAWINGS) {
        p.drawings = serde_json::from_slice(d).unwrap_or_default();
    }
    // Project > Payment Schedule (Neramit Planner's own item; its tasks are mapped below)
    let own_payments: Option<crate::model::PaymentPlan> = props.get(&P_NERAMIT_PAYMENTS).and_then(|d| serde_json::from_slice(d).ok());

    // ---------------- Project > Manage Sprints, the Task Board's columns (sprintio.rs)
    p.sprints = crate::sprintio::read_sprints(&cfb, rt, &p.calendar, p.start);
    let next_up = crate::sprintio::next_up_uid(&crate::sprintio::read_columns(&cfb, rt));

    // ---------------- links between projects (extlinks.rs): which file each external task is from
    let ext_links = props.get(&crate::extlinks::P_LINKS).map(|b| crate::extlinks::parse_links(b)).unwrap_or_default();

    // (a file without sprint tables: Neramit Planner's own item)
    let mut own_sprint_of: HashMap<u32, u32> = HashMap::new();
    let mut own_next_up: Vec<u32> = Vec::new();
    if cfb.stream(&[rt, "TBkndSprint", "FixedMeta"]).is_empty() {
        if let Some(v) = props.get(&P_NERAMIT_SPRINTS).and_then(|d| serde_json::from_slice::<serde_json::Value>(d).ok()) {
            p.sprints = serde_json::from_value(v["sprints"].clone()).unwrap_or_default();
            own_sprint_of = serde_json::from_value::<Vec<(u32, u32)>>(v["tasks"].clone()).unwrap_or_default().into_iter().collect();
            own_next_up = serde_json::from_value(v["next_up"].clone()).unwrap_or_default();
        }
    }

    // ---------------- tasks
    let tvar = parse_var(&cfb.stream(&[rt, "TBkndTask", "VarMeta"]), &cfb.stream(&[rt, "TBkndTask", "Var2Data"]));
    let tfd = cfb.stream(&[rt, "TBkndTask", "FixedData"]);
    let tfm = cfb.stream(&[rt, "TBkndTask", "FixedMeta"]);
    let t2d = cfb.stream(&[rt, "TBkndTask", "Fixed2Data"]);
    let t2m = cfb.stream(&[rt, "TBkndTask", "Fixed2Meta"]);
    let recs = parse_fixed(&tfm, &tfd, 47);
    let recs2 = parse_fixed(&t2m, &t2d, v.meta2);
    let loc = |ft: u32| tmap.get(&ft).copied();
    let uid_loc = loc(T_UID).ok_or("Task unique ID field not found.")?;
    struct Raw {
        id: u32,
        uid: u32,
        task: Task,
        ctype: u16,
        cdate: Option<NaiveDate>,
        cdt: Option<chrono::NaiveDateTime>,
    }
    let mut raws: Vec<Raw> = Vec::new();
    for (i, (meta, off, len)) in recs.iter().enumerate() {
        if meta[0] != 0 || *len < 100 {
            continue;
        }
        let rec = &tfd[*off..*off + *len];
        let rec2: &[u8] = recs2.get(i).map(|(_, o, l)| t2d.get(*o..*o + *l).unwrap_or(&[])).unwrap_or(&[]);
        let meta2: &[u8] = recs2.get(i).map(|(m, _, _)| *m).unwrap_or(&[]);
        let blob = |l: (usize, usize)| if l.0 == 0 { rec } else { rec2 };
        let Some(uid) = u32_at(blob(uid_loc), uid_loc.1) else { continue };
        let Some(name) = tvar.get(&(uid, T_NAME)).map(|n| utf16(n)) else { continue };
        if uid == 0 {
            continue; // project summary task
        }
        let get_u32 = |ft: u32| loc(ft).and_then(|l| u32_at(blob(l), l.1));
        let get_u16 = |ft: u32| loc(ft).and_then(|l| u16_at(blob(l), l.1));
        let get_date = |ft: u32| loc(ft).and_then(|l| date_at(blob(l), l.1));
        let get_dt = |ft: u32| loc(ft).and_then(|l| datetime_at(blob(l), l.1));
        let id = get_u32(T_ID).unwrap_or(uid);
        let level = get_u16(T_OUTLINE_LEVEL).unwrap_or(1).clamp(1, 250) as u8;
        let custom = if std::ptr::eq(v, &V14) { crate::custom::read_mpp(false, &|ft| tvar.get(&(uid, ft)).cloned(), meta) } else { Vec::new() };
        let manual = v.manual.map(|(b, bit)| meta2.get(b).map(|x| (x >> bit) & 1 == 1).unwrap_or(false)).unwrap_or(false);
        let mut start = get_date(T_START);
        let mut finish = get_date(T_FINISH);
        let mut placeholder = false;
        let mut no_dates = false;
        let (mut manual_start, mut manual_finish) = (None, None);
        if manual {
            (manual_start, manual_finish) = (get_dt(T_MANUAL_START), get_dt(T_MANUAL_FINISH));
            let (ms, mf) = (get_date(T_MANUAL_START), get_date(T_MANUAL_FINISH));
            // MPP keeps the text of a manual task's Start / Finish (0x0505 / 0x0506); an empty
            // text means no date was entered: one of the standard placeholder tasks
            // (any text: "TBD" typed is no date either)
            let blank = |ft: u32| tvar.contains_key(&(uid, ft));
            placeholder = ms.is_none() || mf.is_none() || blank(0x0B40_0505) || blank(0x0B40_0506);
            no_dates = (ms.is_none() || blank(0x0B40_0505)) && (mf.is_none() || blank(0x0B40_0506));
            if let Some(d) = ms {
                start = Some(d);
            }
            if let Some(d) = mf {
                finish = Some(d);
            }
        }
        let dur_tenths = get_u32(T_DURATION).filter(|v| *v != u32::MAX).unwrap_or(0) as i64;
        // duration format next to the duration (low 5 bits; 0x20 = estimated)
        let dur_word = loc(T_DURATION).and_then(|l| u16_at(blob(l), l.1 + v.est_delta)).unwrap_or(7);
        let dur_fmt = dur_word as i64 & 0x1F;
        let (dur_unit, dur_elapsed) = DurUnit::from_code(dur_fmt);
        let dur_minutes = dur_tenths / 10;
        let milestone = meta.get(v.milestone.0).map(|x| (x >> v.milestone.1) & 1 == 1).unwrap_or(false);
        let start = start.unwrap_or(p.start);
        let mut t = Task::new(0, &name, start);
        t.outline_level = level;
        t.manual = manual;
        t.set_duration_minutes(dur_minutes, dur_unit, dur_elapsed, mpd as i64);
        t.milestone = milestone && dur_minutes == 0;
        // a manual task keeps its entered start (with time) and, for a summary, its entered finish
        t.start_dt = manual_start;
        t.finish_dt = manual_finish;
        t.placeholder = placeholder;
        t.no_dates = no_dates;
        if manual {
            t.manual_text = [0x0B40_0505, 0x0B40_0506, 0x0B40_0507].map(|ft| tvar.get(&(uid, ft)).map(|v| utf16(v).trim_matches(char::from(0)).to_string()));
        }
        t.custom = custom;
        if let Some(s) = tvar.get(&(uid, crate::sprintio::T_SPRINT)).and_then(|v| u32_at(v, 0)) {
            if p.sprints.iter().any(|x| x.uid == s) {
                t.sprint = Some(s);
            }
        }
        if tvar.get(&(uid, crate::sprintio::T_BOARD_COLUMN)).and_then(|v| u32_at(v, 0)) == Some(next_up) {
            t.board_status = 1;
        }
        if let Some(&s) = own_sprint_of.get(&uid) {
            t.sprint = Some(s);
        }
        if own_next_up.contains(&uid) {
            t.board_status = 1;
        }
        // an external task: its own dates (MPP keeps them as last seen in its file)
        if let Some(x) = tvar.get(&(uid, crate::extlinks::T_EXT_UID)).and_then(|v| u32_at(v, 0)) {
            let path = ext_links.iter().find(|l| l.kind == crate::extlinks::KIND_TASKS && l.uids.contains(&uid)).map(|l| l.path()).unwrap_or_default();
            let id = tvar.get(&(uid, crate::extlinks::T_EXT_ID)).and_then(|v| u32_at(v, 0)).unwrap_or(0);
            t.external = Some(crate::model::ExternalTask { path, uid: x & 0xFFFF, id });
            t.manual = false;
            t.start_dt = get_dt(T_START);
            t.finish_dt = get_dt(T_FINISH);
        }
        // outline state saved with the file (MPP14): FixedMeta byte 15 bit 0x02 is
        // "collapsed" (bit 0x01 = shown, i.e. every ancestor expanded); only used on summary tasks
        if std::ptr::eq(v, &V14) {
            t.collapsed = meta.get(15).map(|b| b & 0x02 != 0).unwrap_or(false);
        }
        // FixedMeta byte 12: 0x04 Rollup (on every summary in the standard files), 0x02 Marked,
        // 0x80 Hide Bar (checked on real files)
        if let Some(b) = meta.get(12).filter(|_| std::ptr::eq(v, &V14)) {
            t.rollup = b & 0x04 != 0;
            t.marked = b & 0x02 != 0;
            t.hide_bar = b & 0x80 != 0;
        }
        t.finish = finish.unwrap_or(start);
        t.percent_complete = get_u16(T_PERCENT).unwrap_or(0).min(100) as u8;
        if let Some(n) = tvar.get(&(uid, T_NOTES)) {
            t.notes = rtf_to_text(n);
        }
        t.estimated = loc(T_DURATION).and_then(|l| u16_at(blob(l), l.1 + v.est_delta)).map(|u| u & 0x20 != 0).unwrap_or(false);
        t.summary_fmt = Some((dur_word & 0x3F) as u8);
        t.priority = get_u16(T_PRIORITY).filter(|p| *p <= 1000).unwrap_or(500);
        t.deadline = get_date(T_DEADLINE);
        t.fixed_cost = loc(T_FIXED_COST).and_then(|l| f64_at(blob(l), l.1)).filter(|v| v.is_finite()).map(|v| v / 100.0).unwrap_or(0.0);
        t.fixed_cost_accrual = Accrue::from_code(get_u16(T_FIXED_COST_ACCRUAL).unwrap_or(3) as i64);
        t.actual_start = get_dt(T_ACTUAL_START);
        t.actual_finish = get_dt(T_ACTUAL_FINISH).filter(|_| t.actual_start.is_some());
        t.actual_dur_min = loc(T_ACTUAL_DURATION).and_then(|l| u32_at(blob(l), l.1)).filter(|v| *v > 0 && *v != u32::MAX).map(|v| (v / 10) as i64);
        {
            let bdt = |ft: u32| tvar.get(&(uid, ft)).and_then(|d| datetime_at(d, 0));
            let (bs, bf) = (bdt(T_BASELINE_START), bdt(T_BASELINE_FINISH));
            if bs.is_some() || bf.is_some() {
                // (the XML leaves a zero baseline duration out)
                let bd = tvar.get(&(uid, T_BASELINE_DURATION)).and_then(|d| u32_at(d, 0)).filter(|v| *v > 0 && *v != u32::MAX).map(|v| (v / 10) as i64);
                // baseline work (1/1000 minute) and cost (cents) are f64s next to them (zero left
                // out, as in the XML)
                let bw = tvar.get(&(uid, 0x0B40_0001)).and_then(|d| f64_at(d, 0)).filter(|v| v.is_finite() && *v != 0.0).map(|v| (v / 1000.0).round() as i64);
                let bc = tvar.get(&(uid, 0x0B40_0006)).and_then(|d| f64_at(d, 0)).filter(|v| v.is_finite() && *v != 0.0).map(|v| v / 100.0);
                t.baselines.push(crate::model::Baseline { number: 0, start: bs, finish: bf, dur_min: bd, work_min: bw, cost: bc, ..Default::default() });
            }
        }
        t.task_type = TaskType::from_code(get_u16(T_TYPE).unwrap_or(0) as i64);
        // a code typed in (0x0B400010), or one the WBS mask generated (0x0B40018D, after the prefix)
        if let Some(w) = tvar.get(&(uid, 0x0B40_018D)).filter(|_| tvar.get(&(uid, T_WBS)).is_none()) {
            // (its own level's part: put together with the parents' codes below)
            t.wbs = format!("\u{1}{}", utf16(w));
        }
        if let Some(w) = tvar.get(&(uid, T_WBS)) {
            t.wbs = utf16(w);
        }
        if let Some(c) = tvar.get(&(uid, T_CONTACT)) {
            t.contact = utf16(c);
        }
        raws.push(Raw { id, uid, task: t, ctype: get_u16(T_CTYPE).unwrap_or(0), cdate: get_date(T_CDATE), cdt: get_dt(T_CDATE) });
    }
    raws.sort_by_key(|r| r.id);
    let mut uid_map: HashMap<u32, u32> = HashMap::new();
    for r in raws.iter_mut() {
        r.task.src_uid = Some(r.uid);
        r.task.uid = p.alloc_uid();
        uid_map.insert(r.uid, r.task.uid);
    }
    if let Some(mut plan) = own_payments {
        for it in plan.items.iter_mut() {
            it.task_uid = it.task_uid.and_then(|u| uid_map.get(&u).copied());
        }
        p.payments = plan;
    }


    // ---------------- links
    let cfd = cfb.stream(&[rt, "TBkndCons", "FixedData"]);
    let cfm = cfb.stream(&[rt, "TBkndCons", "FixedMeta"]);
    let mut preds: HashMap<u32, Vec<Link>> = HashMap::new();
    let n = u32_at(&cfm, 8).unwrap_or(0) as usize;
    for i in 0..n {
        let Some(meta) = cfm.get(16 + i * 10..16 + i * 10 + 10) else { break };
        if meta[0] != 0 {
            continue;
        }
        let off = u32_at(meta, 4).unwrap_or(0) as usize;
        let Some(rec) = cfd.get(off..off + 20) else { continue };
        let (pu, su, ty) = (u32_at(rec, 4).unwrap(), u32_at(rec, 8).unwrap(), u16_at(rec, 12).unwrap());
        let lag = i32_at(rec, v.link_lag_offset).unwrap_or(0);
        let (Some(&pn), Some(&sn)) = (uid_map.get(&pu), uid_map.get(&su)) else { continue };
        let kind = match ty {
            0 => LinkType::FF,
            2 => LinkType::SF,
            3 => LinkType::SS,
            _ => LinkType::FS,
        };
        let fmt = u16_at(rec, v.link_format_offset).unwrap_or(7) as i64;
        let mut link = Link::new(pn, kind);
        if matches!(fmt, 19 | 20) {
            // percent of the predecessor's duration
            link.lag_pct = Some(lag as f64);
        } else {
            // tenths of a minute; elapsed formats (em, eh, ed, ew, emo) count calendar time
            let (unit, elapsed) = DurUnit::from_code(fmt);
            link.set_lag_minutes(lag as i64 / 10, unit, elapsed, mpd as i64);
        }
        preds.entry(sn).or_default().push(link);
    }

    // ---------------- assignments
    let afd = cfb.stream(&[rt, "TBkndAssn", "FixedData"]);
    let afm = cfb.stream(&[rt, "TBkndAssn", "FixedMeta"]);
    let mut task_res: HashMap<u32, Vec<u32>> = HashMap::new();
    // assignment records: units f64 x10000 at 12, work f64 in 1/1000 minute at 20, cost f64
    // in cents at 68
    let mut task_asg: HashMap<u32, Vec<Assignment>> = HashMap::new();
    // splits live in the assignments' timephased work (the unassigned placeholder too)
    let avar = parse_var(&cfb.stream(&[rt, "TBkndAssn", "VarMeta"]), &cfb.stream(&[rt, "TBkndAssn", "Var2Data"]));
    let mut task_splits: HashMap<u32, Vec<(i64, i64)>> = HashMap::new();
    for (meta, off, len) in parse_fixed(&afm, &afd, 34) {
        if meta[0] != 0 || len < 12 {
            continue;
        }
        let rec = &afd[off..off + len];
        let (tu, ru) = (u32_at(rec, 4).unwrap(), u32_at(rec, 8).unwrap());
        if let (Some(&tn), Some(d)) = (uid_map.get(&tu), u32_at(rec, 0).and_then(|au| avar.get(&(au, A_TIMEPHASED_WORK)))) {
            let sp = timephased_splits(d);
            if !sp.is_empty() {
                task_splits.entry(tn).or_insert(sp);
            }
        }
        if let (Some(&tn), Some(&rn)) = (uid_map.get(&tu), res_new_uid.get(&ru)) {
            let units = f64_at(rec, 12).filter(|v| v.is_finite() && *v >= 0.0).map(|v| v / 10000.0).unwrap_or(1.0);
            let work = f64_at(rec, 20).filter(|v| v.is_finite() && *v >= 0.0).map(|v| (v / 1000.0).round() as i64).unwrap_or(-1);
            let cost = f64_at(rec, 68).filter(|v| v.is_finite()).map(|v| v / 100.0).unwrap_or(0.0);
            let v = task_asg.entry(tn).or_default();
            if v.iter().all(|a| a.res_uid != rn) {
                v.push(Assignment { res_uid: rn, units, work_min: work, cost });
            }
        }
        if let (Some(&tn), Some(&rn)) = (uid_map.get(&tu), res_new_uid.get(&ru)) {
            let v = task_res.entry(tn).or_default();
            if !v.contains(&rn) {
                v.push(rn);
            }
        }
    }

    for mut r in raws {
        let uid = r.task.uid;
        r.task.predecessors = preds.remove(&uid).unwrap_or_default();
        r.task.resources = task_res.remove(&uid).unwrap_or_default();
        r.task.assignments = task_asg.remove(&uid).unwrap_or_default();
        r.task.splits = task_splits.remove(&uid).unwrap_or_default();
        // the file's work was worked out for its duration
        r.task.work_ref_dur = r.task.dur_minutes(p.calendar.minutes_per_day());
        if !r.task.manual && r.ctype != 0 {
            r.task.constraint_type = ConstraintType::from_code(r.ctype as i64);
            r.task.constraint_date = r.cdate;
            r.task.constraint_dt = r.cdt;
            r.task.normalize_constraint();
        }
        p.tasks.push(r.task);
    }
    // (a project without tasks is a valid .mpp: MPP saves one too)
    crate::wbs::compose(&mut p);
    if std::ptr::eq(v, &V14) {
        p.saved_view = read_saved_view(&cfb, &uid_map).map(Box::new);
    }
    Ok(p)
}


// ====================================================================== saved view (MPP14)
//
// The "   214" storage:
// - `214/Props` key 0x03400007: name of the active view ("&Gantt Chart"); key 0x03400000: font
//   list, entries of 68 bytes [u16 flags][u16 size pt][name UTF-16].
// - `CV_iew`: FixedData records (id at 0, name at 4); Var2Data (view id, 0x20400006) is a property
//   list (entries 2-byte aligned): 0x2240000a table name, 0x22400008 settings, 0x2240000c cell formats.
// - 0x22400007: last u16 = sheet width (px) left of the divider.
// - settings 0x22400008: text styles = 19 records of 32 bytes from 16 (font index +10, flags +13,
//   colour +14..16 with "automatic" flag +17); gridlines = 14 records of 30 bytes from 680; timescale
//   tiers at 1152 (middle unit, bottom unit, counts, labels), Size % at 1180; bar styles = records of 195
//   bytes: shapes block (middle shape/pattern/colour, start type/shape/colour at +14, end at +27),
//   then 91 bytes later from/to fields, show-for and "not" flag masks, five text fields, the name.
// - `CTable`: Var2Data (table id, 0x20400006): u16 column count-1 at 4, columns of 115 bytes from 12
//   (field id, width in characters, title/data alignment, custom title).
// - cell formats 0x2240000c: records of 44 bytes (task uid, field, font index, flags, colour,
//   background at +24 with "none" flag +27).

use crate::app::TCol;
use crate::format::{BarAppearance, BarField, BarStyle, BarText, CellFormat, EndShape, EndStyle, EndType, MidShape, MidStyle, Pattern, ShowFor, TextItem, TextStyle, TsUnit};

#[derive(Clone, Debug, Default)]
pub struct SavedView {
    #[allow(dead_code)]
    pub name: String,
    pub columns: Vec<SavedColumn>,
    pub bar_styles: Vec<BarStyle>,
    pub text_styles: Vec<(TextItem, TextStyle)>,
    /// middle unit, middle count, middle label code, bottom unit, bottom count, bottom label code
    pub tiers: Option<(TsUnit, u8, u16, TsUnit, u8, u16)>,
    /// Format > Gridlines of the view: line, type, colour
    pub gridlines: Vec<(crate::format::GridKind, crate::format::LineType, crate::format::Rgb)>,
    /// timescale Size (%)
    pub timescale_size: Option<u16>,
    /// sheet width left of the divider, px from the left of the view (including the view bar)
    pub sheet_width: Option<u16>,
    /// ID column width in characters
    pub id_width: Option<u8>,
    /// (task uid in the project, column key or "*" for the whole row, format)
    pub cell_formats: Vec<(u32, String, CellFormat)>,
    pub show_timeline: bool,
    /// fonts used by the view (loaded before the view is applied)
    pub fonts: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct SavedColumn {
    pub col: TCol,
    /// width in the standard character units
    pub width_chars: u8,
    /// 0 left, 1 center, 2 right
    pub align: u8,
    pub title_align: u8,
    pub title: Option<String>,
}

/// Property list whose entries are 2-byte aligned (view storage).
fn parse_props2(d: &[u8]) -> HashMap<u32, Vec<u8>> {
    let mut out = HashMap::new();
    let count = u32_at(d, 12).unwrap_or(0);
    let mut o = 16;
    for _ in 0..count {
        let (Some(size), Some(key)) = (u32_at(d, o), u32_at(d, o + 4)) else { break };
        let start = o + 12;
        let end = start + size as usize;
        if end > d.len() {
            break;
        }
        out.insert(key, d[start..end].to_vec());
        o = end.div_ceil(2) * 2;
    }
    out
}

fn cstr16(b: &[u8]) -> String {
    utf16(b).split('\0').next().unwrap_or("").replace('&', "").trim().to_string()
}

fn task_col(field: u32) -> Option<TCol> {
    if field >> 16 != 0x0B40 {
        return None;
    }
    Some(match field & 0xFFFF {
        0x000E => TCol::Name,
        0x00CD => TCol::Indicators,
        0x0500 => TCol::Mode,
        0x0507 | 0x001D => TCol::Duration,
        0x0505 | 0x0023 => TCol::Start,
        0x0506 | 0x0024 => TCol::Finish,
        0x002F => TCol::Predecessors,
        0x0030 => TCol::Successors,
        0x0031 => TCol::Resources,
        0x0020 => TCol::Percent,
        0x0000 => TCol::Work,
        0x0005 => TCol::Cost,
        0x0008 => TCol::FixedCost,
        0x00C8 => TCol::FixedCostAccrual,
        0x0056 => TCol::Uid,
        0x0029 => TCol::ActualStart,
        0x002A => TCol::ActualFinish,
        0x001C => TCol::ActualDuration,
        0x001F => TCol::RemainingDuration,
        0x002B => TCol::BaselineStart,
        0x002C => TCol::BaselineFinish,
        0x001B => TCol::BaselineDuration,
        0x0025 => TCol::EarlyStart,
        0x0026 => TCol::EarlyFinish,
        0x0027 => TCol::LateStart,
        0x0028 => TCol::LateFinish,
        0x0019 => TCol::Priority,
        0x01B5 => TCol::Deadline,
        0x0011 => TCol::ConstraintType,
        0x0012 => TCol::ConstraintDate,
        0x0010 => TCol::Wbs,
        0x000F => TCol::Notes,
        _ => return None,
    })
}

fn bar_field(field: u32) -> Option<BarField> {
    if field >> 16 != 0x0B40 {
        return None;
    }
    Some(match field & 0xFFFF {
        0x0503 | 0x0505 | 0x0023 => BarField::Start,
        0x0504 | 0x0506 | 0x0024 => BarField::Finish,
        // Complete Through (0x0174: the summaries' "Summary Progress" bar)
        0x0077 | 0x0174 => BarField::CompleteThrough,
        0x01B5 => BarField::Deadline,
        0x0029 => BarField::ActualStart,
        0x002A => BarField::ActualFinish,
        // Baseline Start / Finish (0x0513 / 0x0514: the "estimated" pair MPP14 uses in bar styles)
        0x002B | 0x0513 => BarField::BaselineStart,
        0x002C | 0x0514 => BarField::BaselineFinish,
        0x0025 => BarField::EarlyStart,
        0x0026 => BarField::EarlyFinish,
        0x0027 => BarField::LateStart,
        0x0028 => BarField::LateFinish,
        _ => return None,
    })
}

fn bar_text(field: u32) -> BarText {
    if field >> 16 != 0x0B40 {
        return BarText::None;
    }
    match field & 0xFFFF {
        0x000E => BarText::Name,
        0x0503 | 0x0505 | 0x0023 => BarText::Start,
        0x0504 | 0x0506 | 0x0024 => BarText::Finish,
        0x0507 | 0x001D => BarText::Duration,
        0x0020 => BarText::PercentComplete,
        0x0031 => BarText::ResourceNames,
        0x01B5 => BarText::Deadline,
        0x002F => BarText::Predecessors,
        _ => BarText::None,
    }
}

fn end_shape(code: u8) -> EndShape {
    match code {
        1 | 10 => EndShape::HouseUp,
        2 | 9 => EndShape::HouseDown,
        3 | 13 | 23 => EndShape::Diamond,
        4 | 8 | 15 => EndShape::ArrowUp,
        5 | 14 | 16 => EndShape::ArrowDown,
        6 => EndShape::ArrowRight,
        7 => EndShape::ArrowLeft,
        11 => EndShape::Line,
        12 => EndShape::Square,
        17 => EndShape::Circle,
        18 => EndShape::Star,
        19 => EndShape::BracketLeft,
        20 => EndShape::BracketRight,
        _ => EndShape::None,
    }
}

fn end_style(shape: u8, kind: u8, c: &[u8]) -> EndStyle {
    EndStyle {
        shape: end_shape(shape),
        kind: match kind {
            1 => EndType::Framed,
            2 => EndType::Dashed,
            _ => EndType::Solid,
        },
        color: [c[0], c[1], c[2]],
    }
}

/// Show-for flag bit -> category (bits found by comparing the standard built-in styles).
fn flag_cat(bit: u32) -> Option<Option<crate::format::Category>> {
    use crate::format::Category as C;
    Some(Some(match bit {
        0 => C::Normal,
        1 => C::Milestone,
        2 => C::Summary,
        3 => C::Critical,
        4 => C::Noncritical,
        5 => C::Marked,
        6 => C::Finished,
        7 => C::InProgress,
        8 => C::NotFinished,
        9 => C::NotStarted,
        26 => C::RolledUp,
        27 => C::ProjectSummary,
        28 => C::Split,
        29 => C::External,
        40 => C::GroupBySummary,
        41 => C::Deliverable,
        43 => C::Active,
        44 => C::ManuallyScheduled,
        46..=48 => C::Placeholder,
        // "warning" variants of the manual styles look the same as the plain ones
        45 | 49 => return Some(None),
        _ => return None,
    }))
}

fn show_for(flags: u64, not: u64) -> Option<Vec<ShowFor>> {
    let mut v = Vec::new();
    for (mask, negate) in [(flags, false), (not, true)] {
        for bit in 0..64u32 {
            if mask >> bit & 1 == 1 {
                match flag_cat(bit) {
                    Some(Some(cat)) => v.push(ShowFor { cat, not: negate }),
                    Some(None) => {}
                    // a condition we cannot evaluate: leave the style out rather than misapply it
                    None if !negate => return None,
                    None => {}
                }
            }
        }
    }
    Some(v)
}

fn read_bar_styles(d: &[u8]) -> Vec<BarStyle> {
    let valid = |n: usize| -> bool {
        n >= 91 + 52 && n + 4 < d.len() && d.get(n - 48..n - 46) == Some(&[0x40, 0x0B][..]) && d.get(n - 44..n - 42) == Some(&[0x40, 0x0B][..])
    };
    // first record: the first position whose from / to fields are task fields
    let Some(first) = (143..d.len().saturating_sub(200)).find(|&n| valid(n) && d[n] != 0 && d[n + 1] == 0) else { return Vec::new() };
    let mut out = Vec::new();
    let mut n = first;
    while valid(n) && n + 100 <= d.len() {
        let name = cstr16(&d[n..n + 100]);
        let b = n - 91;
        let (Some(from), Some(to)) = (u32_at(d, n - 50).and_then(bar_field), u32_at(d, n - 46).and_then(bar_field)) else {
            n += 195;
            continue;
        };
        let flags = u64::from_le_bytes(d[n - 42..n - 34].try_into().unwrap());
        let not = u64::from_le_bytes(d[n - 34..n - 26].try_into().unwrap());
        if let (false, Some(sf)) = (name.is_empty(), show_for(flags, not)) {
            let mut text = [BarText::None; 5];
            for (k, t) in text.iter_mut().enumerate() {
                *t = u32_at(d, n - 24 + k * 4).map(bar_text).unwrap_or(BarText::None);
            }
            let mid_shape = MidShape::ALL.get(d[b] as usize).copied().unwrap_or(MidShape::Full);
            let pattern = Pattern::ALL.get(d[b + 1] as usize).copied().unwrap_or(Pattern::Solid);
            out.push(BarStyle {
                name,
                show_for: sf,
                row: 1,
                from,
                to,
                bar: BarAppearance {
                    middle: MidStyle { shape: mid_shape, pattern, color: [d[b + 2], d[b + 3], d[b + 4]] },
                    start: end_style(d[b + 15], d[b + 14], &d[b + 16..b + 19]),
                    end: end_style(d[b + 28], d[b + 27], &d[b + 29..b + 32]),
                },
                text,
            });
        }
        n += 195;
    }
    out
}

const TEXT_ITEMS: [TextItem; 19] = [
    TextItem::Highlighted,
    TextItem::RowColumnTitles,
    TextItem::Noncritical,
    TextItem::Critical,
    TextItem::Summary,
    TextItem::Milestone,
    TextItem::MiddleTier,
    TextItem::BottomTier,
    TextItem::BarTextLeft,
    TextItem::BarTextRight,
    TextItem::BarTextTop,
    TextItem::BarTextBottom,
    TextItem::BarTextInside,
    TextItem::Marked,
    TextItem::ProjectSummary,
    TextItem::External,
    TextItem::TopTier,
    TextItem::ChangedCells,
    TextItem::Inactive,
];

fn tsunit(code: u16) -> Option<TsUnit> {
    Some(match code {
        1 => TsUnit::Hours,
        2 => TsUnit::Days,
        3 => TsUnit::Weeks,
        4 => TsUnit::ThirdsOfMonths,
        5 => TsUnit::Months,
        6 => TsUnit::Quarters,
        7 => TsUnit::HalfYears,
        8 => TsUnit::Years,
        _ => return None,
    })
}

fn line_type(code: u8) -> crate::format::LineType {
    use crate::format::LineType as L;
    match code {
        0 => L::None,
        1 => L::Solid,
        2 => L::DenseDots,
        3 => L::Dotted,
        4 => L::Dashed,
        _ => L::Solid,
    }
}

fn read_saved_view(cfb: &Cfb, uid_map: &HashMap<u32, u32>) -> Option<SavedView> {
    let root = "   214";
    let wprops = parse_props2(&cfb.stream(&[root, "Props"]));
    let fonts: Vec<(String, f32)> = wprops
        .get(&0x0340_0000)
        .map(|v| {
            let n = u16_at(v, 0).unwrap_or(0) as usize;
            (0..n)
                .map(|i| {
                    let b = 2 + i * 68;
                    v.get(b..b + 68).map(|e| (cstr16(&e[4..]), u16_at(e, 2).unwrap_or(10) as f32)).unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default();
    let font_of = |i: u8| fonts.get(i as usize).filter(|f| !f.0.is_empty()).cloned();
    let active = wprops.get(&0x0340_0007).map(|v| cstr16(v)).filter(|s| !s.is_empty()).unwrap_or_else(|| "Gantt Chart".into());

    // the view record and its property list
    let vfd = cfb.stream(&[root, "CV_iew", "FixedData"]);
    let vfm = cfb.stream(&[root, "CV_iew", "FixedMeta"]);
    let view_id = parse_fixed(&vfm, &vfd, 10).into_iter().find_map(|(_, off, len)| {
        let r = vfd.get(off..off + len)?;
        (len >= 8 && cstr16(&r[4..len.min(4 + 100)]) == active).then(|| u32_at(r, 0)).flatten()
    })?;
    let vvar = parse_var(&cfb.stream(&[root, "CV_iew", "VarMeta"]), &cfb.stream(&[root, "CV_iew", "Var2Data"]));
    let props = parse_props2(vvar.get(&(view_id, 0x2040_0006))?);

    let mut sv = SavedView { name: active.clone(), ..Default::default() };
    // sheet / chart divider: last u16 of 0x22400007 (719 = the 719 px MPP showed)
    sv.sheet_width = props.get(&0x2240_0007).filter(|v| v.len() == 37).and_then(|v| u16_at(v, 35)).filter(|w| (40..=4000).contains(w));
    let lower = active.to_lowercase();
    sv.show_timeline = lower.contains("timeline") || active.contains("เส้นเวลา");

    // table columns
    if let Some(table_name) = props.get(&0x2240_000a).map(|v| cstr16(v.get(2..).unwrap_or(&[]))) {
        let tfd = cfb.stream(&[root, "CTable", "FixedData"]);
        let tfm = cfb.stream(&[root, "CTable", "FixedMeta"]);
        let table_id = parse_fixed(&tfm, &tfd, 10).into_iter().find_map(|(_, off, len)| {
            let r = tfd.get(off..off + len)?;
            (len >= 8 && cstr16(&r[4..len.min(4 + 100)]) == table_name).then(|| u32_at(r, 0)).flatten()
        });
        let tvar = parse_var(&cfb.stream(&[root, "CTable", "VarMeta"]), &cfb.stream(&[root, "CTable", "Var2Data"]));
        if let Some(d) = table_id.and_then(|id| tvar.get(&(id, 0x2040_0006))) {
            let n = u16_at(d, 4).unwrap_or(0) as usize + 1;
            for i in 0..n {
                let o = 12 + i * 115;
                let Some(r) = d.get(o..o + 115) else { break };
                if u32_at(r, 0) == Some(0x0B40_0017) {
                    sv.id_width = Some(r[4]);
                }
                let Some(col) = u32_at(r, 0).and_then(task_col) else { continue };
                let title = if r[9] & 0x02 != 0 { Some(cstr16(&r[13..])).filter(|t| !t.is_empty()) } else { None };
                sv.columns.push(SavedColumn { col, width_chars: r[4], align: r[7].min(2), title_align: (r[5] & 0x0F).min(2), title });
            }
        }
    }

    if let Some(s) = props.get(&0x2240_0008) {
        // text styles
        if s.len() > 16 + 32 * 19 {
            for (k, item) in TEXT_ITEMS.iter().enumerate() {
                let t = 16 + 32 * k + 10;
                let Some((font, size)) = font_of(s[t]) else { continue };
                let flags = s[t + 3];
                let color = (s[t + 7] != 0xFF).then(|| [s[t + 4], s[t + 5], s[t + 6]]);
                sv.text_styles.push((
                    *item,
                    TextStyle { font, size, bold: flags & 1 != 0, italic: flags & 2 != 0, underline: flags & 4 != 0, strike: flags & 8 != 0, color, background: None },
                ));
            }
        }
        // timescale: middle / bottom tiers
        if let (Some(mu), Some(bu), Some(mc), Some(bc), Some(ml), Some(bl)) =
            (u16_at(s, 1152).and_then(tsunit), u16_at(s, 1154).and_then(tsunit), u16_at(s, 1156), u16_at(s, 1158), u16_at(s, 1160), u16_at(s, 1162))
        {
            if (1..=99).contains(&mc) && (1..=99).contains(&bc) {
                sv.tiers = Some((mu, mc as u8, ml, bu, bc as u8, bl));
            }
        }
        // gridlines: 14 records of 30 bytes from 680 in MPXJ's order (current date = 9th at 920):
        // normal type, 2 bytes, colour, "automatic" flag (automatic = black); the interval part
        // follows at +15
        if s.len() > 680 + 30 * 14 && s[920] <= 4 {
            use crate::format::GridKind as G;
            const ORDER: [G; 14] = [
                G::SheetRows,
                G::SheetColumns,
                G::TitleVertical,
                G::TitleHorizontal,
                G::MiddleTierColumn,
                G::BottomTierColumn,
                G::GanttRows,
                G::BarRows,
                G::CurrentDate,
                G::PageBreaks,
                G::ProjectStart,
                G::ProjectFinish,
                G::StatusDate,
                G::TopTierColumn,
            ];
            for (k, kind) in ORDER.iter().enumerate() {
                let r = &s[680 + 30 * k..680 + 30 * k + 7];
                if r[0] <= 4 {
                    let color = if r[6] == 0xFF { [0, 0, 0] } else { [r[3], r[4], r[5]] };
                    sv.gridlines.push((*kind, line_type(r[0]), color));
                }
            }
        }
        // timescale Size (%): 134 in a file MPP showed at 30 px per month
        sv.timescale_size = u16_at(s, 1180).filter(|z| (10..=1000).contains(z));
        sv.bar_styles = read_bar_styles(s);
    }

    // formats of single cells / rows (Font dialog, background colour)
    if let Some(c) = props.get(&0x2240_000c) {
        for r in c.chunks_exact(44) {
            let (Some(file_uid), Some(field)) = (u32_at(r, 0), u32_at(r, 4)) else { continue };
            let Some(&uid) = uid_map.get(&file_uid) else { continue };
            let key = if field == u32::MAX {
                "*".to_string()
            } else if let Some(col) = task_col(field) {
                col.title().to_string()
            } else {
                continue;
            };
            // u24 at 40: the attributes the format sets (1 font style, 2 underline, 4 strikethrough,
            // 8 colour, 0x10 font and size, 0x40 background); the others come from the text style.
            // (A file of cells set to "TH SarabunPSK 14" has 0x10 only; 0x20 / 0x20000 are unknown.) Font index at 8 (0x3F: none), style flags at 11 (1 bold, 2 italic, 4 underline,
            // 8 strike), colour at 12 and background at 24, each followed by an "automatic" byte
            // (0xFF: automatic colour / no fill).
            let mask = u32_at(r, 40).unwrap_or(0) & 0x00FF_FFFF;
            let has = |bit: u32| mask & bit != 0;
            let flags = r[11];
            let font = if r[8] == 0x3F { None } else { font_of(r[8]) };
            let f = CellFormat {
                bold: has(1) && flags & 1 != 0,
                italic: has(1) && flags & 2 != 0,
                underline: has(2) && flags & 4 != 0,
                strike: has(4) && flags & 8 != 0,
                color: (has(8) && r[15] != 0xFF).then(|| [r[12], r[13], r[14]]),
                // a white background is left out so the reader's own highlight still shows
                background: (has(0x40) && r[27] != 0xFF && r[24..27] != [0xFF, 0xFF, 0xFF]).then(|| [r[24], r[25], r[26]]),
                size: font.as_ref().filter(|_| has(0x10)).map(|f| f.1),
                font: font.filter(|_| has(0x10)).map(|f| f.0),
                exact: if has(1) { 1 | 2 } else { 0 } | if has(2) { 4 } else { 0 } | if has(4) { 8 } else { 0 },
                whole: true,
            };
            sv.cell_formats.push((uid, key, f));
        }
    }

    let mut used: Vec<String> = sv.text_styles.iter().map(|(_, t)| t.font.clone()).chain(sv.cell_formats.iter().filter_map(|(_, _, f)| f.font.clone())).collect();
    used.sort();
    used.dedup();
    sv.fonts = used;
    Some(sv)
}

/// Calendar data: 7 day blocks of 60 bytes (Sunday first), then u32 exception count at 420
/// and variable-length exception records (92 bytes + name, 4-byte aligned).
/// The data MPP keeps for a base calendar (var 0x0D400008), the inverse of apply_calendar
/// (calendars changed one setting at a time in MPP). Seven 60-byte days from Sunday: u16
/// flag (1 = the standard default day), u16 number of working periods, u32 working time, five u16
/// starts, five u32 lengths and five u32 running totals, all in tenths of a minute. Then u32 count
/// of exceptions, each a 92-byte record (u16 first and last day since 1983-12-31, u32 days, ...,
/// u32 1 at +72, u32 id at +80, u32 name bytes at +88) and its UTF-16 name padded to 4 bytes,
/// and a u32 0. None when every day is the standard default and there are no exceptions (MPP
/// keeps no data then).
pub(crate) fn calendar_data(cal: &Calendar) -> Option<Vec<u8>> {
    let default_working = [false, true, true, true, true, true, false]; // Sunday..Saturday
    let mut out = vec![0u8; 420];
    let mut all_default = true;
    for day in 0..7 {
        let idx = (day + 6) % 7;
        let working = cal.working_days[idx];
        let times: Vec<(u16, u16)> = match cal.work_times.get(idx) {
            Some(v) if !v.is_empty() => v.clone(),
            _ => crate::model::STANDARD_DAY.to_vec(),
        };
        let o = day * 60;
        if working == default_working[day] && (!working || times == crate::model::STANDARD_DAY.to_vec()) {
            put_u16(&mut out, o, 1);
            continue;
        }
        all_default = false;
        if working {
            let n = times.len().min(5);
            put_u16(&mut out, o + 2, n as u16);
            let mut total = 0u32;
            for (k, &(a, b)) in times.iter().take(n).enumerate() {
                let len = (b.saturating_sub(a)) as u32 * 10;
                put_u16(&mut out, o + 8 + 2 * k, a * 10);
                put_u32(&mut out, o + 20 + 4 * k, len);
                total += len;
                put_u32(&mut out, o + 40 + 4 * k, total);
            }
            put_u32(&mut out, o + 4, total);
        }
    }
    if all_default && cal.holidays.is_empty() {
        return None;
    }
    let mut hs: Vec<&Holiday> = cal.holidays.iter().collect();
    hs.sort_by_key(|h| (h.start, h.finish));
    out.extend_from_slice(&(hs.len() as u32).to_le_bytes());
    for (k, h) in hs.iter().enumerate() {
        let mut rec = vec![0u8; 92];
        let from = (h.start - epoch()).num_days().clamp(1, 0xFFFE) as u16;
        let to = (h.finish - epoch()).num_days().clamp(from as i64, 0xFFFE) as u16;
        put_u16(&mut rec, 0, from);
        put_u16(&mut rec, 2, to);
        put_u32(&mut rec, 4, (to - from) as u32 + 1);
        put_u32(&mut rec, 72, 1);
        put_u32(&mut rec, 80, k as u32 + 1);
        let mut name: Vec<u8> = h.name.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        name.extend_from_slice(&[0, 0]);
        put_u32(&mut rec, 88, name.len() as u32);
        out.extend_from_slice(&rec);
        let pad = (4 - name.len() % 4) % 4;
        out.extend_from_slice(&name);
        out.extend(std::iter::repeat(0u8).take(pad));
    }
    out.extend_from_slice(&0u32.to_le_bytes());
    Some(out)
}

/// Do two calendars give the same working time (days, hours and exceptions)?
pub(crate) fn same_calendar(a: &Calendar, b: &Calendar) -> bool {
    let times = |c: &Calendar, k: usize| -> Vec<(u16, u16)> {
        if !c.working_days[k] {
            return Vec::new();
        }
        match c.work_times.get(k) {
            Some(v) if !v.is_empty() => v.clone(),
            _ => crate::model::STANDARD_DAY.to_vec(),
        }
    };
    let hol = |c: &Calendar| -> Vec<(NaiveDate, NaiveDate, String)> {
        let mut v: Vec<_> = c.holidays.iter().map(|h| (h.start, h.finish, h.name.clone())).collect();
        v.sort();
        v
    };
    a.working_days == b.working_days && (0..7).all(|k| times(a, k) == times(b, k)) && hol(a) == hol(b)
}

pub(crate) fn apply_calendar(cal: &mut Calendar, d: &[u8]) {
    // MPP default week: Monday-Friday working
    let default_working = [false, true, true, true, true, true, false]; // Sunday..Saturday
    for day in 0..7 {
        let (Some(flag), Some(count)) = (u16_at(d, day * 60), u16_at(d, day * 60 + 2)) else { return };
        let working = if flag == 1 { default_working[day] } else { count > 0 };
        // our index 0 = Monday
        let idx = (day + 6) % 7;
        cal.working_days[idx] = working;
        // working periods: starts u16 at +8, lengths u32 at +20, both in tenths of a minute
        if cal.work_times.len() != 7 {
            cal.work_times = vec![Vec::new(); 7];
        }
        if flag != 1 && working {
            let mut iv = Vec::new();
            for k in 0..(count as usize).min(5) {
                let (Some(st), Some(len)) = (u16_at(d, day * 60 + 8 + 2 * k), u32_at(d, day * 60 + 20 + 4 * k)) else { break };
                let a = st as u32 / 10;
                let b = (a + len / 10).min(24 * 60);
                if b > a {
                    iv.push((a as u16, b as u16));
                }
            }
            cal.work_times[idx] = iv;
        }
    }
    let count = u32_at(d, 420).unwrap_or(0) as usize;
    let mut o = 424;
    for _ in 0..count.min(10_000) {
        let (Some(f), Some(t), Some(nlen)) = (u16_at(d, o), u16_at(d, o + 2), u32_at(d, o + 88)) else { break };
        let nlen = nlen as usize;
        let name = d.get(o + 92..o + 92 + nlen).map(utf16).unwrap_or_default();
        if f > 0 && t >= f {
            cal.holidays.push(Holiday {
                name: if name.is_empty() { "Exception".into() } else { name },
                start: epoch() + Duration::days(f as i64),
                finish: epoch() + Duration::days(t as i64),
            });
        }
        o += 92 + ((nlen + 3) & !3);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rtf_thai() {
        let rtf = br"{\rtf1\ansi\ansicpg874{\fonttbl{\f0 Cordia New;}}\pard\f0\'cb\'c1\'d2\'c2 test\par line2}";
        assert_eq!(rtf_to_text(rtf), "\u{0e2b}\u{0e21}\u{0e32}\u{0e22} test\nline2");
        assert_eq!(rtf_to_text(b"{\\rtf1\\uc1\\u3627?\\u3617?}"), "\u{0e2b}\u{0e21}");
    }

    /// Calendars made one setting at a time (NERAMIT_CALDIR = folder of .mpp files): the
    /// data Neramit Planner writes for what it read is the standard, byte for byte.
    #[test]
    fn calendar_data_roundtrip() {
        let Ok(dir) = std::env::var("NERAMIT_CALDIR") else { return };
        let mut checked = 0;
        for e in std::fs::read_dir(&dir).unwrap() {
            let path = e.unwrap().path();
            if path.extension().map(|x| x != "mpp").unwrap_or(true) {
                continue;
            }
            let bytes = std::fs::read(&path).unwrap();
            let cfb = Cfb::new(&bytes).unwrap();
            let rt = "   114";
            let cvar = parse_var(&cfb.stream(&[rt, "TBkndCal", "VarMeta"]), &cfb.stream(&[rt, "TBkndCal", "Var2Data"]));
            // (exception ids are the standard running counter: numbered from 1 here)
            let orig = cvar.get(&(1, C_DATA)).cloned().map(|mut d| {
                let n = u32_at(&d, 420).unwrap_or(0) as usize;
                let mut o = 424;
                for k in 0..n {
                    put_u32(&mut d, o + 80, k as u32 + 1);
                    let nlen = u32_at(&d, o + 88).unwrap_or(0) as usize;
                    o += 92 + ((nlen + 3) & !3);
                }
                d
            });
            let mut cal = Calendar::default();
            if let Some(d) = &orig {
                apply_calendar(&mut cal, d);
            }
            assert_eq!(calendar_data(&cal), orig, "{}", path.display());
            checked += 1;
        }
        assert!(checked > 0);
    }

    /// Validates the reader against real files when NERAMIT_MPP_TESTDATA points at a folder of
    /// `name.mpp` + `name.xml` pairs (the XML of the same file).
    #[test]
    fn mpp_matches_project_xml() {
        let Ok(dir) = std::env::var("NERAMIT_MPP_TESTDATA") else { return };
        let mut checked = 0;
        for e in std::fs::read_dir(&dir).unwrap() {
            let path = e.unwrap().path();
            if path.extension().map(|x| x != "mpp").unwrap_or(true) {
                continue;
            }
            let xml = path.with_extension("xml");
            if !xml.exists() {
                continue;
            }
            let a = load_mpp(&std::fs::read(&path).unwrap()).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let b = crate::fileio::load(&xml).unwrap();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let bt: Vec<&Task> = b.tasks.iter().filter(|t| !t.name.is_empty()).collect();
            assert_eq!(a.tasks.len(), bt.len(), "{name}: task count");
            for (x, y) in a.tasks.iter().zip(bt.iter()) {
                // the XML reader trims surrounding whitespace; the .mpp keeps the name verbatim
                assert_eq!(x.name.trim(), y.name.trim(), "{name}");
                assert_eq!(x.outline_level, y.outline_level, "{name}: {}", x.name);
                assert_eq!(x.duration, y.duration, "{name}: {}", x.name);
                assert_eq!(x.start, y.start, "{name}: {}", x.name);
                assert_eq!(x.finish, y.finish, "{name}: {}", x.name);
                assert_eq!(x.manual, y.manual, "{name}: {}", x.name);
                assert_eq!(x.percent_complete, y.percent_complete, "{name}: {}", x.name);
                assert_eq!(x.actual_start, y.actual_start, "{name}: actual start of {}", x.name);
                assert_eq!(x.actual_finish, y.actual_finish, "{name}: actual finish of {}", x.name);
                assert_eq!(x.baselines, y.baselines, "{name}: baselines of {}", x.name);
                assert_eq!(x.splits, y.splits, "{name}: splits of {}", x.name);
                assert_eq!(x.estimated, y.estimated, "{name}: estimated of {}", x.name);
                assert_eq!(x.priority, y.priority, "{name}: priority of {}", x.name);
                assert_eq!(x.deadline, y.deadline, "{name}: deadline of {}", x.name);
                assert_eq!(x.task_type, y.task_type, "{name}: task type of {}", x.name);
                if x.percent_complete == 0 && !x.is_summary {
                    assert_eq!(x.constraint_type, y.constraint_type, "{name}: constraint of {}", x.name);
                }
                assert_eq!(x.predecessors.len(), y.predecessors.len(), "{name}: links of {}", x.name);
                assert_eq!(x.resources.len(), y.resources.len(), "{name}: resources of {}", x.name);
            }
            assert_eq!(a.calendar.working_days, b.calendar.working_days, "{name}: work week");
            assert_eq!(a.new_tasks_auto, b.new_tasks_auto, "{name}: new tasks manual / auto");
            assert_eq!(a.options.default_task_type, b.options.default_task_type, "{name}: default task type");
            assert_eq!(a.options.autolink, b.options.autolink, "{name}: autolink");
            assert_eq!(a.options.split_in_progress, b.options.split_in_progress, "{name}: split in-progress tasks");
            assert_eq!(a.calendar.holidays.len(), b.calendar.holidays.len(), "{name}: holidays");
            checked += 1;
        }
        assert!(checked > 0, "no .mpp/.xml pairs found in {dir}");
    }
}


// =========================================================================== writing (MPP14)

pub(crate) fn put_u16(b: &mut [u8], o: usize, v: u16) {
    if let Some(x) = b.get_mut(o..o + 2) {
        x.copy_from_slice(&v.to_le_bytes());
    }
}
pub(crate) fn put_u32(b: &mut [u8], o: usize, v: u32) {
    if let Some(x) = b.get_mut(o..o + 4) {
        x.copy_from_slice(&v.to_le_bytes());
    }
}
pub(crate) fn put_f64(b: &mut [u8], o: usize, v: f64) {
    if let Some(x) = b.get_mut(o..o + 8) {
        x.copy_from_slice(&v.to_le_bytes());
    }
}

/// The standard timestamp: u16 tenths of a minute, u16 days since 1983-12-31; NA = all ones.
pub(crate) fn timestamp(t: Option<chrono::NaiveDateTime>) -> u32 {
    use chrono::Timelike;
    match t {
        Some(t) => {
            let days = (t.date() - epoch()).num_days().clamp(0, 0xFFFE) as u32;
            let tenths = (t.hour() * 60 + t.minute()) * 10;
            (days << 16) | tenths
        }
        None => u32::MAX,
    }
}

/// Write a task field into its record (FixedData or Fixed2Data, as the field map says).
pub(crate) fn put_field(tmap: &HashMap<u32, (usize, usize)>, a: &mut [u8], b: &mut [u8], (off, len): (usize, usize), (off2, len2): (usize, usize), ft: u32, bytes: &[u8]) {
    if let Some(&(blob, o)) = tmap.get(&ft) {
        let (buf, start, size) = if blob == 0 { (a, off, len) } else { (b, off2, len2) };
        if o + bytes.len() <= size && start + o + bytes.len() <= buf.len() {
            buf[start + o..start + o + bytes.len()].copy_from_slice(bytes);
        }
    }
}

/// Fixed records of a FixedMeta / FixedData pair: (meta position, data offset, length).
pub(crate) fn fixed_index(meta: &[u8], data_len: usize, item: usize) -> Vec<(usize, usize, usize)> {
    let n = u32_at(meta, 8).unwrap_or(0) as usize;
    let mut v: Vec<(usize, usize)> = (0..n).filter_map(|i| {
        let base = 16 + i * item;
        (base + item <= meta.len()).then(|| (base, u32_at(meta, base + 4).unwrap_or(0) as usize))
    }).collect();
    let mut offs: Vec<usize> = v.iter().map(|x| x.1).collect();
    offs.sort_unstable();
    offs.dedup();
    v.drain(..).map(|(base, off)| {
        let next = offs.iter().copied().find(|&o| o > off).unwrap_or(data_len);
        (base, off, next.saturating_sub(off))
    }).collect()
}

/// Set (or add) a var-data item: same-size values are written in place, others appended and
/// pointed to (the VarMeta header keeps the Var2Data size at 0x14).
fn var_set(meta: &mut Vec<u8>, data: &mut Vec<u8>, uid: u32, ft: u32, value: &[u8]) {
    let n = u32_at(meta, 8).unwrap_or(0) as usize;
    for i in 0..n {
        let base = 0x18 + i * 12;
        if u32_at(meta, base) == Some(uid) && u32_at(meta, base + 8) == Some(ft) {
            let off = u32_at(meta, base + 4).unwrap_or(0) as usize;
            if u32_at(data, off) == Some(value.len() as u32) && off + 4 + value.len() <= data.len() {
                data[off + 4..off + 4 + value.len()].copy_from_slice(value);
                return;
            }
            let new_off = data.len() as u32;
            data.extend_from_slice(&(value.len() as u32).to_le_bytes());
            data.extend_from_slice(value);
            put_u32(meta, base + 4, new_off);
            put_u32(meta, 0x14, data.len() as u32);
            return;
        }
    }
    // a new item goes after the last one (the trailer moves down)
    let at = 0x18 + n * 12;
    let mut item = Vec::with_capacity(12);
    item.extend_from_slice(&uid.to_le_bytes());
    item.extend_from_slice(&(data.len() as u32).to_le_bytes());
    item.extend_from_slice(&ft.to_le_bytes());
    meta.splice(at..at, item);
    put_u32(meta, 8, (n + 1) as u32);
    data.extend_from_slice(&(value.len() as u32).to_le_bytes());
    data.extend_from_slice(value);
    put_u32(meta, 0x14, data.len() as u32);
}

/// Remove a var-data item (its bytes stay unreferenced in Var2Data).
fn var_remove(meta: &mut Vec<u8>, uid: u32, ft: u32) -> bool {
    let n = u32_at(meta, 8).unwrap_or(0) as usize;
    for i in 0..n {
        let base = 0x18 + i * 12;
        if u32_at(meta, base) == Some(uid) && u32_at(meta, base + 8) == Some(ft) {
            meta.drain(base..base + 12);
            put_u32(meta, 8, (n - 1) as u32);
            return true;
        }
    }
    false
}

pub(crate) fn utf16z(s: &str) -> Vec<u8> {
    s.encode_utf16().chain(std::iter::once(0)).flat_map(|u| u.to_le_bytes()).collect()
}

/// Save `doc` into a copy of the .mpp it was read from (`orig`). The values edited in
/// Neramit Planner replace the file's (tasks: dates, durations, progress, constraints, names, flags,
/// work and cost; links: type and lag; resources and assignments: their numbers); the standard other
/// data stays as it was. Tasks, links, resources or assignments added or removed cannot be
/// written into an existing .mpp yet: the caller is told so (save as Project XML instead).
#[allow(dead_code)]
pub fn save_mpp_patch(orig: &[u8], doc: &Project) -> R<Vec<u8>> {
    let cfb = Cfb::new(orig)?;
    let v = version_of(&cfb)?;
    if !std::ptr::eq(v, &V14) {
        return Err("Saving back to .mpp is supported for MPP14 files. Save as XML instead.".into());
    }
    let rt = v.root;
    let props_raw = cfb.stream(&[rt, "Props"]);
    let props = parse_props(&props_raw);
    let tmap = props.get(&0x0002_0014).map(|d| parse_field_map(d)).ok_or("The task field map is missing.")?;
    let rmap = props.get(&0x0002_0015).map(|d| parse_field_map(d)).unwrap_or_default();
    let mpd = doc.calendar.minutes_per_day();
    let cal = &doc.calendar;
    let path = |a: &str, b: &str| cfb.entry_index(&[rt, a, b]).ok_or_else(|| format!("The file has no {a}/{b} stream."));
    let mut replace: HashMap<usize, Vec<u8>> = HashMap::new();

    // ---------------- tasks
    let mut tfm = cfb.stream(&[rt, "TBkndTask", "FixedMeta"]);
    let mut tfd = cfb.stream(&[rt, "TBkndTask", "FixedData"]);
    let mut t2m = cfb.stream(&[rt, "TBkndTask", "Fixed2Meta"]);
    let mut t2d = cfb.stream(&[rt, "TBkndTask", "Fixed2Data"]);
    let mut tvm = cfb.stream(&[rt, "TBkndTask", "VarMeta"]);
    let mut tvd = cfb.stream(&[rt, "TBkndTask", "Var2Data"]);
    let recs = fixed_index(&tfm, tfd.len(), 47);
    let recs2 = fixed_index(&t2m, t2d.len(), v.meta2);
    let uid_loc = tmap.get(&T_UID).copied().ok_or("Task unique ID field not found.")?;
    // file uid -> record index
    let mut by_uid: HashMap<u32, usize> = HashMap::new();
    for (k, &(base, off, len)) in recs.iter().enumerate() {
        if tfm.get(base) != Some(&0) || len < 100 {
            continue;
        }
        let blob = if uid_loc.0 == 0 { &tfd[off..off + len] } else { recs2.get(k).map(|r| &t2d[r.1..r.1 + r.2]).unwrap_or(&[]) };
        if let Some(u) = u32_at(blob, uid_loc.1) {
            if u != 0 && tvm.len() > 0 {
                by_uid.insert(u, k);
            }
        }
    }
    let file_tasks: std::collections::HashSet<u32> = parse_var(&tvm, &tvd).keys().filter(|(_, ft)| *ft == T_NAME).map(|(u, _)| *u).filter(|u| *u != 0 && by_uid.contains_key(u)).collect();
    let doc_tasks: std::collections::HashSet<u32> = doc.tasks.iter().filter_map(|t| t.src_uid).collect();
    if doc.tasks.iter().any(|t| t.src_uid.is_none()) || file_tasks.iter().any(|u| !doc_tasks.contains(u)) {
        return Err("Tasks were added or deleted. Neramit Planner can save into an existing .mpp only when the tasks are the same; save as XML (.xml) instead.".into());
    }
    // the file's order of tasks (its IDs; blank rows keep theirs) must still be the order
    let id_loc = tmap.get(&T_ID).copied();
    let file_id = |fu: u32| -> Option<u32> {
        let ri = *by_uid.get(&fu)?;
        let (_, off, len) = recs[ri];
        let (_, off2, len2) = recs2.get(ri).copied().unwrap_or((0, 0, 0));
        let (blob, o) = id_loc?;
        if blob == 0 { u32_at(&tfd[off..off + len], o) } else { u32_at(t2d.get(off2..off2 + len2)?, o) }
    };
    let ids: Vec<u32> = doc.tasks.iter().filter_map(|t| t.src_uid.and_then(file_id)).collect();
    if ids.windows(2).any(|w| w[1] <= w[0]) {
        return Err("Tasks were moved. Neramit Planner can save into an existing .mpp only when the tasks keep their order; save as XML (.xml) instead.".into());
    }
    let names0 = parse_var(&tvm, &tvd);
    for t in doc.tasks.iter() {
        let Some(fu) = t.src_uid else { continue };
        let Some(&ri) = by_uid.get(&fu) else { continue };
        let (base, off, len) = recs[ri];
        let (base2, off2, len2) = recs2.get(ri).copied().unwrap_or((0, 0, 0));
        // write a field wherever the field map puts it
        macro_rules! put {
            ($ft:expr, $bytes:expr) => {
                put_field(&tmap, &mut tfd, &mut t2d, (off, len), (off2, len2), $ft, &$bytes)
            };
        }
        let ts = |x: Option<chrono::NaiveDateTime>| timestamp(x).to_le_bytes();
        let (st, ft) = (t.start_dt, t.finish_dt);
        put!(T_OUTLINE_LEVEL, (t.outline_level as u16).to_le_bytes());
        put!(T_START, ts(st));
        put!(T_FINISH, ts(ft));
        if !t.placeholder {
            put!(T_MANUAL_START, ts(st));
            put!(T_MANUAL_FINISH, ts(ft));
        }
        put!(0x0B40_0025, ts(st));
        put!(0x0B40_0026, ts(ft));
        // late dates from the total slack
        let shift = |x: chrono::NaiveDateTime, start: bool| -> chrono::NaiveDateTime {
            let k = t.slack_min;
            if k > 0 {
                let y = cal.add_work_minutes(x, k);
                if start { cal.next_work_instant(y) } else { y }
            } else if k < 0 {
                cal.sub_work_minutes(x, -k)
            } else {
                x
            }
        };
        put!(0x0B40_0027, ts(st.map(|x| shift(x, true))));
        put!(0x0B40_0028, ts(ft.map(|x| shift(x, false))));
        let d = t.dur_minutes(mpd);
        let a = t.actual_minutes(mpd).min(d);
        // the file's duration and its format word, to keep a format MPP chose ("null" ...)
        let (old_dur, old_fmt) = match tmap.get(&T_DURATION) {
            Some(&(blob, o)) => {
                let (buf, start) = if blob == 0 { (&tfd, off) } else { (&t2d, off2) };
                (u32_at(buf, start + o), u16_at(buf, start + o + v.est_delta))
            }
            None => (None, None),
        };
        let same_duration = old_dur == Some((d * 10) as u32) && old_fmt.map(|f| (f & 0x20 != 0) == t.estimated).unwrap_or(false);
        put!(T_DURATION, ((d * 10) as u32).to_le_bytes());
        put!(T_ACTUAL_DURATION, ((a * 10) as u32).to_le_bytes());
        put!(0x0B40_001F, (((d - a).max(0) * 10) as u32).to_le_bytes());
        if let Some(&(blob, o)) = tmap.get(&T_DURATION) {
            let (buf, start, size) = if blob == 0 { (&mut tfd, off, len) } else { (&mut t2d, off2, len2) };
            let at = start + o + v.est_delta;
            // a summary's format stays as stored ("null" / estimated days)
            if o + v.est_delta + 2 <= size && !t.is_summary && !same_duration {
                let old = u16_at(buf.as_slice(), at).unwrap_or(7);
                let unit = if t.is_summary { 7 } else { t.dur_unit.code(t.dur_elapsed) as u16 };
                let code = (old & !0x3F) | (unit & 0x1F) | if t.estimated { 0x20 } else { 0 };
                put_u16(buf, at, code);
            }
        }
        put!(T_PERCENT, (t.percent_complete as u16).to_le_bytes());
        put!(T_CTYPE, (t.constraint_type.code() as u16).to_le_bytes());
        put!(T_CDATE, ts(if t.constraint_type.needs_date() { t.constraint_instant(cal) } else { None }));
        put!(T_PRIORITY, t.priority.to_le_bytes());
        put!(T_DEADLINE, ts(t.deadline.map(|d| cal.day_end(d))));
        put!(T_ACTUAL_START, ts(t.actual_start));
        put!(T_ACTUAL_FINISH, ts(t.actual_finish));
        // work: total, actual, remaining (the standard Work is actual + remaining); cost and remaining cost
        let actual_work = (t.work_min as f64 * t.percent_complete as f64 / 100.0).round();
        put!(0x0B40_0000, (t.work_min as f64 * 1000.0).to_le_bytes());
        put!(0x0B40_0002, (actual_work * 1000.0).to_le_bytes());
        put!(0x0B40_0004, ((t.work_min as f64 - actual_work).max(0.0) * 1000.0).to_le_bytes());
        put!(0x0B40_0005, (t.cost * 100.0).round().to_le_bytes());
        put!(0x0B40_000A, ((t.cost - t.actual_cost).max(0.0) * 100.0).round().to_le_bytes());
        put!(T_FIXED_COST, (t.fixed_cost * 100.0).round().to_le_bytes());
        put!(T_FIXED_COST_ACCRUAL, (t.fixed_cost_accrual.code() as u16).to_le_bytes());
        // flags in the metadata: milestone (FixedMeta), manually scheduled (Fixed2Meta)
        let (mb, mbit) = v.milestone;
        if let Some(x) = tfm.get_mut(base + mb) {
            *x = if t.milestone { *x | (1 << mbit) } else { *x & !(1 << mbit) };
        }
        if let (Some((b, bit)), true) = (v.manual, base2 > 0) {
            if let Some(x) = t2m.get_mut(base2 + b) {
                *x = if t.manual { *x | (1 << bit) } else { *x & !(1 << bit) };
            }
        }
        // names and the baseline live in the var data
        let cur = &names0;
        if cur.get(&(fu, T_NAME)).map(|n| utf16(n)) != Some(t.name.clone()) {
            var_set(&mut tvm, &mut tvd, fu, T_NAME, &utf16z(&t.name));
        }
        if let Some(b) = t.baseline(0) {
            if b.start.is_some() || cur.contains_key(&(fu, T_BASELINE_START)) {
                var_set(&mut tvm, &mut tvd, fu, T_BASELINE_START, &timestamp(b.start).to_le_bytes());
                var_set(&mut tvm, &mut tvd, fu, T_BASELINE_FINISH, &timestamp(b.finish).to_le_bytes());
                var_set(&mut tvm, &mut tvd, fu, T_BASELINE_DURATION, &((b.dur_min.unwrap_or(0) * 10) as u32).to_le_bytes());
            }
        }
    }
    replace.insert(path("TBkndTask", "FixedMeta")?, tfm);
    replace.insert(path("TBkndTask", "FixedData")?, tfd);
    replace.insert(path("TBkndTask", "Fixed2Meta")?, t2m);
    replace.insert(path("TBkndTask", "Fixed2Data")?, t2d);
    replace.insert(path("TBkndTask", "VarMeta")?, tvm);
    replace.insert(path("TBkndTask", "Var2Data")?, tvd);

    // ---------------- links: type and lag of every link the file has
    let to_file: HashMap<u32, u32> = doc.tasks.iter().filter_map(|t| t.src_uid.map(|f| (t.uid, f))).collect();
    let mut want: HashMap<(u32, u32), &Link> = HashMap::new();
    for t in &doc.tasks {
        let Some(su) = t.src_uid else { continue };
        for l in &t.predecessors {
            if let Some(&pu) = to_file.get(&l.pred_uid) {
                want.insert((pu, su), l);
            }
        }
    }
    let cfm = cfb.stream(&[rt, "TBkndCons", "FixedMeta"]);
    let mut cfd = cfb.stream(&[rt, "TBkndCons", "FixedData"]);
    let n = u32_at(&cfm, 8).unwrap_or(0) as usize;
    let mut seen = 0usize;
    for i in 0..n {
        let Some(meta) = cfm.get(16 + i * 10..16 + i * 10 + 10) else { break };
        if meta[0] != 0 {
            continue;
        }
        let off = u32_at(meta, 4).unwrap_or(0) as usize;
        if off + 20 > cfd.len() {
            continue;
        }
        let (pu, su) = (u32_at(&cfd, off + 4).unwrap_or(0), u32_at(&cfd, off + 8).unwrap_or(0));
        let Some(l) = want.get(&(pu, su)) else {
            return Err("Links were removed. Neramit Planner can save into an existing .mpp only when the links are the same; save as XML (.xml) instead.".into());
        };
        seen += 1;
        let ty: u16 = match l.kind {
            LinkType::FF => 0,
            LinkType::FS => 1,
            LinkType::SF => 2,
            LinkType::SS => 3,
        };
        put_u16(&mut cfd, off + 12, ty);
        let (lag, fmt) = match l.lag_pct {
            Some(pct) => ((pct.round() as i32), if l.elapsed { 20u16 } else { 19 }),
            None => ((l.lag_minutes(mpd, 0) * 10) as i32, l.lag_unit.code(l.elapsed) as u16),
        };
        put_u32(&mut cfd, off + v.link_lag_offset, lag as u32);
        put_u16(&mut cfd, off + v.link_format_offset, fmt);
    }
    if seen != want.len() {
        return Err("Links were added. Neramit Planner can save into an existing .mpp only when the links are the same; save as XML (.xml) instead.".into());
    }
    replace.insert(path("TBkndCons", "FixedData")?, cfd);

    // ---------------- resources: names and numbers
    let rfm = cfb.stream(&[rt, "TBkndRsc", "FixedMeta"]);
    let mut rfd = cfb.stream(&[rt, "TBkndRsc", "FixedData"]);
    let mut rvm = cfb.stream(&[rt, "TBkndRsc", "VarMeta"]);
    let mut rvd = cfb.stream(&[rt, "TBkndRsc", "Var2Data"]);
    let r_uid_off = rmap.get(&0x0C40_001B).map(|x| x.1).unwrap_or(4);
    let rvar = parse_var(&rvm, &rvd);
    let file_res: Vec<(usize, usize, u32)> = fixed_index(&rfm, rfd.len(), 37)
        .into_iter()
        .filter(|&(base, _, len)| rfm.get(base) == Some(&0) && len >= 32)
        .filter_map(|(_, off, len)| u32_at(&rfd[off..off + len], r_uid_off).map(|u| (off, len, u)))
        .filter(|(_, _, u)| *u != 0 && rvar.contains_key(&(*u, R_NAME)))
        .collect();
    let res_by_file: HashMap<u32, &Resource> = doc.resources.iter().filter_map(|r| r.src_uid.map(|f| (f, r))).collect();
    if doc.resources.iter().any(|r| r.src_uid.is_none()) || file_res.iter().any(|(_, _, u)| !res_by_file.contains_key(u)) {
        return Err("Resources were added or deleted. Neramit Planner can save into an existing .mpp only when the resources are the same; save as XML (.xml) instead.".into());
    }
    for &(off, len, fu) in &file_res {
        let r = res_by_file[&fu];
        let mut putf = |ft: u32, dflt: usize, val: f64| {
            let o = rmap.get(&ft).map(|x| x.1).unwrap_or(dflt);
            if o + 8 <= len {
                put_f64(&mut rfd, off + o, val);
            }
        };
        putf(0x0C40_0004, 8, r.max_units * 100.0);
        putf(0x0C40_0006, 16, r.std_rate);
        putf(0x0C40_0007, 24, r.ovt_rate);
        putf(0x0C40_0012, 72, r.cost_per_use * 100.0);
        // the resource's work totals (work, regular work, remaining work)
        if r.rtype == ResourceType::Work {
            let w = r.work_min as f64 * 1000.0;
            for ft in [0x0C40_000D, 0x0C40_0016, 0x0C40_0026] {
                if let Some(o) = rmap.get(&ft).map(|x| x.1) {
                    if o + 8 <= len {
                        put_f64(&mut rfd, off + o, w);
                    }
                }
            }
        }
        if let Some(o) = rmap.get(&R_ACCRUE_AT).map(|x| x.1) {
            if o + 2 <= len {
                put_u16(&mut rfd, off + o, r.accrue_at.code() as u16);
            }
        }
        if rvar.get(&(fu, R_NAME)).map(|n| utf16(n)) != Some(r.name.clone()) {
            var_set(&mut rvm, &mut rvd, fu, R_NAME, &utf16z(&r.name));
        }
    }
    replace.insert(path("TBkndRsc", "FixedData")?, rfd);
    replace.insert(path("TBkndRsc", "VarMeta")?, rvm);
    replace.insert(path("TBkndRsc", "Var2Data")?, rvd);

    // ---------------- assignments: units, work, cost
    let afm = cfb.stream(&[rt, "TBkndAssn", "FixedMeta"]);
    let mut afd = cfb.stream(&[rt, "TBkndAssn", "FixedData"]);
    let mut avm = cfb.stream(&[rt, "TBkndAssn", "VarMeta"]);
    let res_doc_uid: HashMap<u32, u32> = doc.resources.iter().filter_map(|r| r.src_uid.map(|f| (f, r.uid))).collect();
    let task_by_file: HashMap<u32, &Task> = doc.tasks.iter().filter_map(|t| t.src_uid.map(|f| (f, t))).collect();
    let mut file_asg = 0usize;
    for (base, off, len) in fixed_index(&afm, afd.len(), 34) {
        if afm.get(base) != Some(&0) || len < 76 {
            continue;
        }
        let (tu, ru) = (u32_at(&afd, off + 4).unwrap_or(0), u32_at(&afd, off + 8).unwrap_or(0));
        let (Some(t), Some(&rdoc)) = (task_by_file.get(&tu), res_doc_uid.get(&ru)) else { continue };
        let Some(a) = t.assignments.iter().find(|a| a.res_uid == rdoc) else {
            return Err("Assignments were removed. Neramit Planner can save into an existing .mpp only when the assignments are the same; save as XML (.xml) instead.".into());
        };
        file_asg += 1;
        // new units or work: MPP rebuilds the timephased work from them
        let (old_units, old_work) = (f64_at(&afd, off + 12).unwrap_or(0.0) / 10000.0, f64_at(&afd, off + 20).unwrap_or(0.0) / 1000.0);
        if (old_units - a.units).abs() > 1e-9 || (old_work - a.work_min.max(0) as f64).abs() > 0.5 {
            if let Some(au) = u32_at(&afd, off) {
                var_remove(&mut avm, au, A_TIMEPHASED_WORK);
            }
        }
        put_f64(&mut afd, off + 12, a.units * 10000.0);
        put_f64(&mut afd, off + 20, a.work_min.max(0) as f64 * 1000.0);
        put_u32(&mut afd, off + 52, timestamp(t.start_dt));
        put_u32(&mut afd, off + 56, timestamp(t.finish_dt));
        if let Some(r) = doc.resource_by_uid(rdoc) {
            let cost = match r.rtype {
                ResourceType::Work => a.work_min.max(0) as f64 / 60.0 * r.std_rate + r.cost_per_use * a.units,
                ResourceType::Material => a.units * r.std_rate + r.cost_per_use,
                ResourceType::Cost => a.cost,
            };
            put_f64(&mut afd, off + 68, (cost * 100.0).round());
        }
    }
    let doc_asg: usize = doc.tasks.iter().filter(|t| t.src_uid.is_some()).map(|t| t.assignments.len()).sum();
    if doc_asg != file_asg {
        return Err("Assignments were added. Neramit Planner can save into an existing .mpp only when the assignments are the same; save as XML (.xml) instead.".into());
    }
    replace.insert(path("TBkndAssn", "FixedData")?, afd);
    replace.insert(path("TBkndAssn", "VarMeta")?, avm);

    // ---------------- project: start and title (Props)
    let mut items: Vec<(u32, u32, Vec<u8>)> = Vec::new();
    let count = u32_at(&props_raw, 12).unwrap_or(0);
    let mut o = 16;
    for _ in 0..count {
        let (Some(size), Some(key), Some(attr)) = (u32_at(&props_raw, o), u32_at(&props_raw, o + 4), u32_at(&props_raw, o + 8)) else { break };
        let st = o + 12;
        let en = st + size as usize;
        if en > props_raw.len() {
            break;
        }
        items.push((key, attr, props_raw[st..en].to_vec()));
        o = en;
    }
    let tail = props_raw[o.min(props_raw.len())..].to_vec();
    for (key, _, val) in items.iter_mut() {
        if *key == P_START && val.len() >= 4 {
            let start = doc.start_dt.filter(|t| t.date() == doc.start).unwrap_or_else(|| cal.day_start(doc.start));
            val[..4].copy_from_slice(&timestamp(Some(start)).to_le_bytes());
        }
        if *key == P_TITLE && utf16(val) != doc.title {
            *val = utf16z(&doc.title);
        }
    }
    let mut props_new = props_raw[..16.min(props_raw.len())].to_vec();
    for (key, attr, val) in &items {
        props_new.extend_from_slice(&(val.len() as u32).to_le_bytes());
        props_new.extend_from_slice(&key.to_le_bytes());
        props_new.extend_from_slice(&attr.to_le_bytes());
        props_new.extend_from_slice(val);
    }
    props_new.extend_from_slice(&tail);
    // the header keeps the stream's size (less 4) twice
    if props_new.len() >= 8 {
        let n = (props_new.len() - 4) as u32;
        put_u32(&mut props_new, 0, n);
        put_u32(&mut props_new, 4, n);
    }
    replace.insert(cfb.entry_index(&[rt, "Props"]).ok_or("The file has no Props stream.")?, props_new);

    crate::mppw::rewrite_cfb(orig, &replace)
}
