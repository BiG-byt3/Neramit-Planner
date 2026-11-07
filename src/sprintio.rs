//! Project > Manage Sprints and the Task Board in .mpp files (newer MPP14 files).
//!
//! - TBkndSprint: a row per sprint after the standard "No Sprint" (ID 0, UID 1). FixedMeta entries
//!   of 10 bytes; FixedData 38 bytes - u32 ID, u32 UID, GUID, u32 start and u32 finish (the standard
//!   timestamps, -1 none), u16 duration units (9 weeks; 39 for "No Sprint"), u32 duration (tenths
//!   of a minute of calendar time: two weeks = 201600); var 0x3E400003 the name.
//! - TBkndBoardColumn: a row per Task Board column - u32 ID, u32 UID, GUID; var 0x3C400003 the
//!   name (Not Started, Next up, In progress, Done and the user's own).
//! - Tasks: var 0x0B400585 the sprint's UID, 0x0B400584 the board column's UID (-1: the column
//!   its % complete gives), 0x0B400586 / 0x0B400587 its place in the board / sprint column (u64).
//! - Props 0x0100000D / 0x0001000D: the sprint rows and their var data size; 0x0100000C /
//!   0x0001000C the same for the board columns.

use crate::model::{Calendar, Sprint};
use chrono::{Duration, NaiveDate};

pub const V_SPRINT_NAME: u32 = 0x3E40_0003;
pub const V_COLUMN_NAME: u32 = 0x3C40_0003;
pub const T_BOARD_COLUMN: u32 = 0x0B40_0584;
pub const T_SPRINT: u32 = 0x0B40_0585;
pub const P_SPRINT_ROWS: u32 = 0x0100_000D;
pub const P_SPRINT_VSIZE: u32 = 0x0001_000D;
/// The standard "No Sprint" (the sprint of tasks in none)
pub const NO_SPRINT_UID: u32 = 1;
/// The standard default "Next up" column (the second; UID 2 in the standard files)
pub const NEXT_UP_UID: u32 = 2;

/// A sprint row's record (38 bytes).
pub fn sprint_record(id: u32, uid: u32, guid: [u8; 16], start: Option<chrono::NaiveDateTime>, finish: Option<chrono::NaiveDateTime>, units: u16, tenths: u32) -> Vec<u8> {
    let mut r = Vec::with_capacity(38);
    r.extend_from_slice(&id.to_le_bytes());
    r.extend_from_slice(&uid.to_le_bytes());
    r.extend_from_slice(&guid);
    r.extend_from_slice(&crate::mpp::timestamp(start).to_le_bytes());
    r.extend_from_slice(&crate::mpp::timestamp(finish).to_le_bytes());
    r.extend_from_slice(&units.to_le_bytes());
    r.extend_from_slice(&tenths.to_le_bytes());
    r
}

/// The calendar time of a sprint from its first to its last day (MPP keeps two weeks as
/// 14 days, tenths of a minute).
pub fn sprint_tenths(s: &Sprint) -> u32 {
    let days = (s.finish - s.start).num_days().max(0) + 1;
    (days * 1440 * 10) as u32
}

/// The sprints of a file: (UID, name, start, finish, duration in tenths) in ID order; a sprint
/// without dates follows the one before it (the first starts with the project).
pub fn read_sprints(cfb: &crate::mpp::Cfb<'_>, rt: &str, cal: &Calendar, pstart: NaiveDate) -> Vec<Sprint> {
    use crate::mpp::{datetime_at, u16_at, u32_at};
    let t = "TBkndSprint";
    let (m, d) = (cfb.stream(&[rt, t, "FixedMeta"]), cfb.stream(&[rt, t, "FixedData"]));
    let var = crate::mpp::parse_var(&cfb.stream(&[rt, t, "VarMeta"]), &cfb.stream(&[rt, t, "Var2Data"]));
    let n = u32_at(&m, 8).unwrap_or(0) as usize;
    let mut rows: Vec<(u32, u32, Option<chrono::NaiveDateTime>, Option<chrono::NaiveDateTime>, u32, String)> = Vec::new();
    for i in 0..n {
        let b = 16 + 10 * i;
        if m.get(b) != Some(&0) {
            continue;
        }
        let Some(off) = u32_at(&m, b + 4).map(|o| o as usize) else { continue };
        let (Some(id), Some(uid)) = (u32_at(&d, off), u32_at(&d, off + 4)) else { continue };
        if id == 0 || uid == NO_SPRINT_UID {
            continue;
        }
        let start = datetime_at(&d, off + 24);
        let finish = datetime_at(&d, off + 28);
        let _units = u16_at(&d, off + 32);
        let tenths = u32_at(&d, off + 34).unwrap_or(0);
        let name = var.get(&(uid, V_SPRINT_NAME)).map(|v| crate::mpp::utf16(v)).unwrap_or_default();
        rows.push((id, uid, start, finish, tenths, name));
    }
    rows.sort_by_key(|r| r.0);
    let mut out: Vec<Sprint> = Vec::new();
    let mut next = cal.next_working(pstart);
    for (_, uid, start, finish, tenths, name) in rows {
        let s = start.map(|x| x.date()).unwrap_or(next);
        let f = finish.map(|x| x.date()).unwrap_or_else(|| sprint_end(cal, s, tenths as i64 / 10));
        out.push(Sprint { uid, name, start: s, finish: f.max(s) });
        next = cal.next_working(f.max(s) + Duration::days(1));
    }
    out
}

/// The last day of a sprint from its first and its length: whole days of calendar time (the standard
/// own sprints: two weeks are 336 hours), else working time (two weeks of 40 hours: 80 hours).
pub fn sprint_end(cal: &Calendar, start: NaiveDate, minutes: i64) -> NaiveDate {
    if minutes <= 0 {
        return start;
    }
    if minutes % 1440 == 0 {
        start + Duration::days(minutes / 1440 - 1)
    } else {
        let days = (minutes as f64 / cal.minutes_per_day() as f64).ceil() as i32;
        cal.finish_for(start, days.max(1))
    }
}

/// The board columns of a file: (UID, ID, name).
pub fn read_columns(cfb: &crate::mpp::Cfb<'_>, rt: &str) -> Vec<(u32, u32, String)> {
    use crate::mpp::u32_at;
    let t = "TBkndBoardColumn";
    let (m, d) = (cfb.stream(&[rt, t, "FixedMeta"]), cfb.stream(&[rt, t, "FixedData"]));
    let var = crate::mpp::parse_var(&cfb.stream(&[rt, t, "VarMeta"]), &cfb.stream(&[rt, t, "Var2Data"]));
    let n = u32_at(&m, 8).unwrap_or(0) as usize;
    let mut out = Vec::new();
    for i in 0..n {
        let b = 16 + 10 * i;
        if m.get(b) != Some(&0) {
            continue;
        }
        let Some(off) = u32_at(&m, b + 4).map(|o| o as usize) else { continue };
        let (Some(id), Some(uid)) = (u32_at(&d, off), u32_at(&d, off + 4)) else { continue };
        let name = var.get(&(uid, V_COLUMN_NAME)).map(|v| crate::mpp::utf16(v)).unwrap_or_default();
        out.push((uid, id, name));
    }
    out.sort_by_key(|c| c.1);
    out
}

/// The UID of the "Next up" column (the second one).
pub fn next_up_uid(cols: &[(u32, u32, String)]) -> u32 {
    cols.iter().find(|c| c.1 == 1).map(|c| c.0).unwrap_or(NEXT_UP_UID)
}

/// File UIDs for the project's sprints: their own where they can be (2 and up, each once), else
/// new ones after the highest.
pub fn file_uids(sprints: &[Sprint]) -> Vec<u32> {
    let mut used: std::collections::HashSet<u32> = std::collections::HashSet::new();
    let mut out: Vec<Option<u32>> = sprints.iter().map(|s| (s.uid > NO_SPRINT_UID && used.insert(s.uid)).then_some(s.uid)).collect();
    let mut next = sprints.iter().map(|s| s.uid).max().unwrap_or(1).max(1) + 1;
    for u in out.iter_mut() {
        if u.is_none() {
            *u = Some(next);
            next += 1;
        }
    }
    out.into_iter().map(|u| u.unwrap()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprint_rows() {
        let s = Sprint { uid: 2, name: "Sprint 1".into(), start: NaiveDate::from_ymd_opt(2026, 10, 22).unwrap(), finish: NaiveDate::from_ymd_opt(2026, 11, 4).unwrap() };
        // the standard "Sprint 1": two weeks, 201600 tenths of a minute
        assert_eq!(sprint_tenths(&s), 201_600);
        let r = sprint_record(1, 2, [0; 16], None, None, 9, sprint_tenths(&s));
        assert_eq!(r.len(), 38);
        assert_eq!(file_uids(&[Sprint { uid: 1, ..s.clone() }, s.clone(), Sprint { uid: 2, ..s.clone() }]), vec![3, 2, 4]);
    }
}
