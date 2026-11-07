//! Save As .mpp (MPP14): the task, resource, assignment, link and calendar tables are
//! rebuilt from the project while every record the file already had is kept (with Neramit Planner's
//! values written into it), so tasks, links, resources and assignments can be added, deleted or
//! moved and the standard data stays as it was.
//!
//! The tables:
//! - `FixedMeta`: header `ba ad df fa`, u32 4 (6 while it holds deleted rows), u32 row count, u32
//!   FixedData size; per row: u8 state (0 used, 2 deleted, 4 spare), u8, u16 number of var items
//!   of the row, u32 offset of the row's FixedData record, then flag bits.
//! - `Fixed2Meta` / `Fixed2Data`: one row per FixedMeta row. Fixed2Data starts with the row's
//!   GUID; a task adds a sort key (f64, rising with the ID) and its outline parent's GUID, a link
//!   its predecessor's and successor's task GUIDs, an assignment its task's and resource's, a
//!   resource calendar its resource's and its base calendar's.
//! - `VarMeta`: header `ba ad df fa`, u32 0, u32 item count, 8 zero bytes, u32 Var2Data size; items
//!   (unique ID, offset, field) sorted by unique ID and field; Var2Data items are u32 size + bytes.
//! - `Props` keeps each table's row count (key 0x0100000N) and Var2Data size (0x0001000N): N = 1
//!   tasks, 2 resources, 3 calendars, 4 assignments, 5 links. Records are stored in the order they
//!   were made; a task's position in the outline is its ID field.
//! - Every task that is not a summary has an assignment: an unassigned task has a placeholder
//!   (resource 0xFFFF0001, a fixed GUID) whose work is the task's duration. Every resource has its
//!   own calendar row (based on the project calendar).
//! - MPP stores scheduling results it can work out again: flags (critical, summary, ...),
//!   slack, the outline parent, the date the links ask for and what drives the start, and lists
//!   of predecessors, of children without predecessors and of resources (var data 0x0B4004E3..E5).
//!   They are written the way MPP writes them.

use crate::model::*;
use crate::mpp::{f64_at, parse_field_map, parse_props, put_u16, put_u32, timestamp, u16_at, u32_at, utf16, utf16z, Cfb};
use crate::mpp::{
    A_TIMEPHASED_WORK, R_ACCRUE_AT, R_GROUP, R_INITIALS, R_MATERIAL, R_NAME, T_ACTUAL_DURATION, T_ACTUAL_FINISH, T_ACTUAL_START, T_BASELINE_DURATION, T_BASELINE_FINISH,
    T_BASELINE_START, T_CDATE, T_CTYPE, T_DEADLINE, T_DURATION, T_FINISH, T_FIXED_COST, T_FIXED_COST_ACCRUAL, T_ID, T_MANUAL_FINISH, T_MANUAL_START, T_NAME, T_OUTLINE_LEVEL,
    T_PERCENT, T_PRIORITY, T_START, T_TYPE, T_UID,
};
use chrono::NaiveDateTime;
use std::collections::{HashMap, HashSet};

type R<T> = Result<T, String>;
type FieldMap = HashMap<u32, (usize, usize)>;

// ----------------------------------------------------------------- task fields (field)
const T_WORK: u32 = 0x0B40_0000;
const T_ACTUAL_WORK: u32 = 0x0B40_0002;
const T_REMAINING_WORK: u32 = 0x0B40_0004;
const T_COST: u32 = 0x0B40_0005;
const T_ACTUAL_COST: u32 = 0x0B40_0007;
const T_REMAINING_COST: u32 = 0x0B40_000A;
const T_LEVEL_DELAY: u32 = 0x0B40_0014;
const T_FREE_SLACK: u32 = 0x0B40_0015;
const T_REMAINING_DURATION: u32 = 0x0B40_001F;
const T_PCT_WORK: u32 = 0x0B40_0021;
const T_EARLY_START: u32 = 0x0B40_0025;
const T_EARLY_FINISH: u32 = 0x0B40_0026;
const T_LATE_START: u32 = 0x0B40_0027;
const T_LATE_FINISH: u32 = 0x0B40_0028;
const T_CREATED: u32 = 0x0B40_005D;
const T_RESUME: u32 = 0x0B40_0063;
const T_STOP: u32 = 0x0B40_0064;
const T_PARENT_UID: u32 = 0x0B40_00A0;
/// a copy of the duration's format word (units, 0x20 = estimated) in Fixed2Data
const T_DUR_FORMAT2: u32 = 0x0B40_0509;
/// the latest start the task's links ask for
const T_LINK_DATE: u32 = 0x0B40_0183;
const T_START_SLACK: u32 = 0x0B40_01B6;
const T_FINISH_SLACK: u32 = 0x0B40_01B7;
const T_GUID: u32 = 0x0B40_0477;
const T_SORT_KEY: u32 = 0x0B40_0479;
const T_PARENT_GUID: u32 = 0x0B40_047F;
/// what drives the start (0x01 actual start, 0x04 constraint, 0x08 links, 0x10 project start,
/// 0x40 summary; 0x20: the driving instant ends a working interval) and that instant
const T_START_DRIVER: u32 = 0x0B40_04E2;
const T_DRIVER_DATE: u32 = 0x0B40_04E7;
/// var data: predecessors, children without predecessors, resources of a linked task
const T_PRED_LIST: u32 = 0x0B40_04E3;
const T_START_KIDS: u32 = 0x0B40_04E4;
const T_LINKED_RES: u32 = 0x0B40_04E5;
/// var items a new task takes over from the task it is copied from (MPP gives every task
/// these, with default values)
const NEW_TASK_VARS: [u32; 8] = [0x0B40_00A4, 0x0B40_00A5, 0x0B40_00A8, 0x0B40_00A9, 0x0B40_00AA, 0x0B40_00F4, 0x0B40_0563, 0x0B40_0564];

// ----------------------------------------------------------------- resource fields
const R_ID: u32 = 0x0C40_0000;
const R_UID: u32 = 0x0C40_001B;
const R_MAX_UNITS: u32 = 0x0C40_0004;
const R_STD_RATE: u32 = 0x0C40_0006;
const R_OVT_RATE: u32 = 0x0C40_0007;
const R_WORK: u32 = 0x0C40_000D;
const R_ACTUAL_WORK: u32 = 0x0C40_000E;
const R_COST_PER_USE: u32 = 0x0C40_0012;
const R_REMAINING_WORK: u32 = 0x0C40_0016;
const R_REGULAR_WORK: u32 = 0x0C40_0026;
const R_CALENDAR: u32 = 0x0C40_0038;
const R_GUID: u32 = 0x0C40_02D8;
const R_GUID2: u32 = 0x0C40_02D9;
const R_SORT_KEY: u32 = 0x0C40_02DA;
const R_CREATED: u32 = 0x0C40_02D6;
/// var items a new resource takes over from the resource it is copied from
const NEW_RES_VARS: [u32; 2] = [0x0C40_0055, 0x0C40_02E3];

// ----------------------------------------------------------------- assignment fields
const A_UID: u32 = 0x0F40_0000;
const A_TASK: u32 = 0x0F40_0001;
const A_RES: u32 = 0x0F40_0002;
const A_UNITS: u32 = 0x0F40_0007;
const A_WORK: u32 = 0x0F40_0008;
const A_ACTUAL_WORK: u32 = 0x0F40_000A;
const A_REGULAR_WORK: u32 = 0x0F40_000B;
const A_REMAINING_WORK: u32 = 0x0F40_000C;
const A_START: u32 = 0x0F40_0014;
const A_FINISH: u32 = 0x0F40_0015;
const A_RESUME: u32 = 0x0F40_0018;
const A_COST: u32 = 0x0F40_001A;
const A_ACTUAL_COST: u32 = 0x0F40_001C;
const A_REMAINING_COST: u32 = 0x0F40_001D;
const A_STOP: u32 = 0x0F40_0108;
const A_CREATED: u32 = 0x0F40_027A;
const A_GUID: u32 = 0x0F40_027C;
const A_TASK_GUID: u32 = 0x0F40_027D;
const A_RES_GUID: u32 = 0x0F40_027E;
/// The standard "unassigned" resource of a task's placeholder assignment
const UNASSIGNED: u32 = 0xFFFF_0001;
const UNASSIGNED_GUID: [u8; 16] = [0x78, 0x8b, 0xcb, 0xa0, 0x8c, 0x2a, 0x6d, 0x43, 0, 0, 0, 0, 0, 0, 0, 0xff];

// ----------------------------------------------------------------- link fields
const L_UID: u32 = 0x0E40_0000;
const L_PRED: u32 = 0x0E40_0002;
const L_SUCC: u32 = 0x0E40_0005;
const L_TYPE: u32 = 0x0E40_0007;
const L_LAG: u32 = 0x0E40_0009;
const L_FMT: u32 = 0x0E40_000A;
const L_GUID: u32 = 0x0E40_0015;
const L_PRED_GUID: u32 = 0x0E40_0016;
const L_SUCC_GUID: u32 = 0x0E40_0017;

// ----------------------------------------------------------------- calendar fields
const C_BASE: u32 = 0x0D40_0006;
const C_RES: u32 = 0x0D40_0007;
const C_BASE_GUID: u32 = 0x0D40_001D;
const C_UID: u32 = 0x0D40_0009;
const C_GUID: u32 = 0x0D40_001B;
const C_RES_GUID: u32 = 0x0D40_001C;

// =========================================================================== tables

/// One row of a table: its FixedMeta entry and record, its Fixed2Meta entry and record, and its
/// var items (field, value).
#[derive(Clone, Debug, Default)]
pub(crate) struct Row {
    pub meta: Vec<u8>,
    pub data: Vec<u8>,
    pub meta2: Vec<u8>,
    pub data2: Vec<u8>,
    pub uid: Option<u32>,
    pub vars: Vec<(u32, Vec<u8>)>,
}

impl Row {
    fn var(&self, ft: u32) -> Option<&[u8]> {
        self.vars.iter().find(|v| v.0 == ft).map(|v| v.1.as_slice())
    }
    fn set_var(&mut self, ft: u32, val: Vec<u8>) {
        match self.vars.iter_mut().find(|v| v.0 == ft) {
            Some(v) => v.1 = val,
            None => self.vars.push((ft, val)),
        }
    }
    fn remove_var(&mut self, ft: u32) {
        self.vars.retain(|v| v.0 != ft);
    }
    fn used(&self) -> bool {
        self.meta.first() == Some(&0)
    }
    /// Write a field where the field map puts it (FixedData or Fixed2Data).
    fn put(&mut self, map: &FieldMap, ft: u32, bytes: &[u8]) {
        if skip_field(ft) {
            return;
        }
        if let Some(&(blob, o)) = map.get(&ft) {
            let buf = if blob == 0 { &mut self.data } else { &mut self.data2 };
            if let Some(x) = buf.get_mut(o..o + bytes.len()) {
                x.copy_from_slice(bytes);
            }
        }
    }
    fn get(&self, map: &FieldMap, ft: u32, len: usize) -> Option<&[u8]> {
        let &(blob, o) = map.get(&ft)?;
        let buf = if blob == 0 { &self.data } else { &self.data2 };
        buf.get(o..o + len)
    }
    fn get_u32(&self, map: &FieldMap, ft: u32) -> Option<u32> {
        self.get(map, ft, 4).and_then(|b| u32_at(b, 0))
    }
    fn get_u16(&self, map: &FieldMap, ft: u32) -> Option<u16> {
        self.get(map, ft, 2).and_then(|b| u16_at(b, 0))
    }
    fn guid(&self, map: &FieldMap, ft: u32) -> [u8; 16] {
        self.get(map, ft, 16).and_then(|b| b.try_into().ok()).unwrap_or([0; 16])
    }
    fn flag(&mut self, byte: usize, mask: u8, on: bool) {
        set_bit(&mut self.meta, byte, mask, on);
    }
    fn flag2(&mut self, byte: usize, mask: u8, on: bool) {
        set_bit(&mut self.meta2, byte, mask, on);
    }
}

fn set_bit(buf: &mut [u8], byte: usize, mask: u8, on: bool) {
    if let Some(x) = buf.get_mut(byte) {
        *x = if on { *x | mask } else { *x & !mask };
    }
}

pub(crate) struct Tbl {
    hdr: Vec<u8>,
    item: usize,
    hdr2: Option<Vec<u8>>,
    item2: usize,
    vhdr: Vec<u8>,
    uid_loc: (usize, usize),
    pub rows: Vec<Row>,
    /// var items of unique IDs that have no row (written back unchanged)
    orphans: Vec<(u32, u32, Vec<u8>)>,
    /// MPP leaves spare entries after the counted ones (old rows it may use again): they and
    /// the data they point to are written back after the rows, uncounted
    spare: Vec<Row>,
    meta_tail: Vec<u8>,
    meta2_tail: Vec<u8>,
    /// var items after the counted ones (the same for VarMeta), and bytes after those
    vspare: Vec<(u32, u32, Vec<u8>)>,
    vtail: Vec<u8>,
}

type MetaRows = (Vec<(Vec<u8>, Vec<u8>)>, Vec<(Vec<u8>, Vec<u8>)>, Vec<u8>);

/// (meta entry, record) of the counted rows of a FixedMeta / FixedData pair, of the spare entries
/// after them, and the bytes after those.
fn rows_of(meta: &[u8], data: &[u8], item: usize) -> MetaRows {
    let n = u32_at(meta, 8).unwrap_or(0) as usize;
    let cap = if item > 0 { meta.len().saturating_sub(16) / item } else { 0 };
    let ents: Vec<(Vec<u8>, usize)> = (0..cap.max(n)).filter_map(|i| meta.get(16 + i * item..16 + (i + 1) * item).map(|m| (m.to_vec(), u32_at(m, 4).unwrap_or(0) as usize))).collect();
    let n = n.min(ents.len());
    // a counted row's record runs to the next counted row's; a spare one's to the next of any
    let mut counted: Vec<usize> = ents[..n].iter().map(|e| e.1).collect();
    counted.sort_unstable();
    counted.dedup();
    let mut all: Vec<usize> = ents.iter().map(|e| e.1).collect();
    all.sort_unstable();
    all.dedup();
    let cut = |offs: &[usize], off: usize| -> Vec<u8> {
        let next = offs.iter().copied().find(|&o| o > off).unwrap_or(data.len());
        data.get(off..next.max(off)).unwrap_or(&[]).to_vec()
    };
    let rows = ents[..n].iter().map(|(m, off)| (m.clone(), cut(&counted, *off))).collect();
    let spare = ents[n..].iter().map(|(m, off)| (m.clone(), cut(&all, *off))).collect();
    let tail = meta.get(16 + ents.len() * item..).unwrap_or(&[]).to_vec();
    (rows, spare, tail)
}

/// A Fixed2Meta entry size when the caller does not know it: its length over its entries (counted
/// and spare ones).
fn guess_item(meta: &[u8], spare: usize) -> usize {
    let n = u32_at(meta, 8).unwrap_or(0) as usize + spare;
    if n > 0 {
        meta.len().saturating_sub(16) / n
    } else {
        0
    }
}

impl Tbl {
    pub(crate) fn read(cfb: &Cfb, rt: &str, name: &str, item: usize, item2: Option<usize>, uid_loc: (usize, usize)) -> R<Tbl> {
        let fm = cfb.stream(&[rt, name, "FixedMeta"]);
        let fd = cfb.stream(&[rt, name, "FixedData"]);
        if fm.len() < 16 {
            return Err(format!("The file's {name} table is missing."));
        }
        let (r1, s1, meta_tail) = rows_of(&fm, &fd, item);
        let f2m = cfb.stream(&[rt, name, "Fixed2Meta"]);
        let f2d = cfb.stream(&[rt, name, "Fixed2Data"]);
        let (hdr2, item2, (r2, s2, meta2_tail)) = if f2m.len() >= 16 {
            let it2 = item2.unwrap_or_else(|| guess_item(&f2m, s1.len()));
            (Some(f2m[..16].to_vec()), it2, rows_of(&f2m, &f2d, it2))
        } else {
            (None, 0, (Vec::new(), Vec::new(), Vec::new()))
        };
        if hdr2.is_some() && (r2.len() != r1.len() || s2.len() != s1.len()) {
            return Err(format!("The file's {name} table is not in a form Neramit Planner can rewrite."));
        }
        let vm = cfb.stream(&[rt, name, "VarMeta"]);
        let vd = cfb.stream(&[rt, name, "Var2Data"]);
        let mut vars: HashMap<u32, Vec<(u32, Vec<u8>)>> = HashMap::new();
        let n = u32_at(&vm, 8).unwrap_or(0) as usize;
        let vcap = vm.len().saturating_sub(0x18) / 12;
        let value = |off: usize| -> Option<Vec<u8>> {
            let size = u32_at(&vd, off)? as usize;
            vd.get(off + 4..off + 4 + size).map(|v| v.to_vec())
        };
        let mut vspare = Vec::new();
        for i in 0..vcap.max(n) {
            let b = 0x18 + i * 12;
            let (Some(uid), Some(off), Some(ft)) = (u32_at(&vm, b), u32_at(&vm, b + 4), u32_at(&vm, b + 8)) else { break };
            if i < n {
                if let Some(v) = value(off as usize) {
                    vars.entry(uid).or_default().push((ft, v));
                }
            } else {
                vspare.push((uid, ft, value(off as usize).unwrap_or_default()));
            }
        }
        let vtail = vm.get(0x18 + vcap.max(n) * 12..).unwrap_or(&[]).to_vec();
        let mut rows = Vec::with_capacity(r1.len());
        for (k, (m, d)) in r1.into_iter().enumerate() {
            let (m2, d2) = r2.get(k).cloned().unwrap_or_default();
            let used = m.first() == Some(&0);
            let uid = if used { u32_at(if uid_loc.0 == 0 { &d } else { &d2 }, uid_loc.1) } else { None };
            let v = uid.and_then(|u| vars.remove(&u)).unwrap_or_default();
            rows.push(Row { meta: m, data: d, meta2: m2, data2: d2, uid, vars: v });
        }
        let mut orphans: Vec<(u32, u32, Vec<u8>)> = vars.into_iter().flat_map(|(u, v)| v.into_iter().map(move |(ft, x)| (u, ft, x))).collect();
        orphans.sort_by_key(|o| (o.0, o.1));
        let spare: Vec<Row> = s1
            .into_iter()
            .enumerate()
            .map(|(k, (m, d))| {
                let (m2, d2) = s2.get(k).cloned().unwrap_or_default();
                Row { meta: m, data: d, meta2: m2, data2: d2, uid: None, vars: Vec::new() }
            })
            .collect();
        let vhdr = match vm.get(..0x18) {
            Some(h) => h.to_vec(),
            None => {
                let mut h = vec![0u8; 0x18];
                h[..4].copy_from_slice(&[0xba, 0xad, 0xdf, 0xfa]);
                h
            }
        };
        Ok(Tbl { hdr: fm[..16].to_vec(), item, hdr2, item2, vhdr, uid_loc, rows, orphans, spare, meta_tail, meta2_tail, vspare, vtail })
    }

    /// The table's streams: (FixedMeta, FixedData, Fixed2Meta + Fixed2Data, VarMeta, Var2Data).
    pub(crate) fn write(&self) -> (Vec<u8>, Vec<u8>, Option<(Vec<u8>, Vec<u8>)>, Vec<u8>, Vec<u8>) {
        let mut items: Vec<(u32, u32, &[u8])> = Vec::new();
        let mut seen: HashSet<(u32, u32)> = HashSet::new();
        for r in &self.rows {
            if let Some(u) = r.uid.filter(|_| r.used()) {
                for (ft, v) in &r.vars {
                    if seen.insert((u, *ft)) {
                        items.push((u, *ft, v));
                    }
                }
            }
        }
        for (u, ft, v) in &self.orphans {
            if seen.insert((*u, *ft)) {
                items.push((*u, *ft, v));
            }
        }
        items.sort_by_key(|x| (x.0, x.1));
        let mut nvars: HashMap<u32, u16> = HashMap::new();
        for (u, _, _) in &items {
            *nvars.entry(*u).or_default() += 1;
        }
        let mut fm = self.hdr.clone();
        let mut fd = Vec::new();
        let mut f2m = self.hdr2.clone();
        let mut f2d = Vec::new();
        for (k, r) in self.rows.iter().chain(self.spare.iter()).enumerate() {
            let counted = k < self.rows.len();
            let mut m = r.meta.clone();
            m.resize(self.item, 0);
            put_u32(&mut m, 4, fd.len() as u32);
            if let Some(u) = r.uid.filter(|_| r.used() && counted) {
                put_u16(&mut m, 2, nvars.get(&u).copied().unwrap_or(0));
            }
            fm.extend_from_slice(&m);
            fd.extend_from_slice(&r.data);
            if let Some(h2) = f2m.as_mut() {
                let mut m2 = r.meta2.clone();
                m2.resize(self.item2, 0);
                put_u32(&mut m2, 4, f2d.len() as u32);
                h2.extend_from_slice(&m2);
                f2d.extend_from_slice(&r.data2);
            }
        }
        fm.extend_from_slice(&self.meta_tail);
        if let Some(h2) = f2m.as_mut() {
            h2.extend_from_slice(&self.meta2_tail);
        }
        // the header says whether deleted rows are left in the table
        let deleted = self.rows.iter().any(|r| r.meta.first() == Some(&2));
        put_u32(&mut fm, 4, if deleted { 6 } else { 4 });
        put_u32(&mut fm, 8, self.rows.len() as u32);
        put_u32(&mut fm, 12, fd.len() as u32);
        if let Some(h2) = f2m.as_mut() {
            put_u32(h2, 8, self.rows.len() as u32);
            put_u32(h2, 12, f2d.len() as u32);
        }
        let mut vm = self.vhdr.clone();
        let mut vd = Vec::new();
        for (u, ft, v) in items.iter().copied().chain(self.vspare.iter().map(|(u, ft, v)| (*u, *ft, v.as_slice()))) {
            vm.extend_from_slice(&u.to_le_bytes());
            vm.extend_from_slice(&(vd.len() as u32).to_le_bytes());
            vm.extend_from_slice(&ft.to_le_bytes());
            vd.extend_from_slice(&(v.len() as u32).to_le_bytes());
            vd.extend_from_slice(v);
        }
        vm.extend_from_slice(&self.vtail);
        put_u32(&mut vm, 8, items.len() as u32);
        put_u32(&mut vm, 0x14, vd.len() as u32);
        (fm, fd, f2m.map(|h| (h, f2d)), vm, vd)
    }

    /// The highest unique ID the table has used (deleted and spare rows and their var items
    /// included, so none is reused).
    fn max_uid(&self) -> u32 {
        let (b, o) = self.uid_loc;
        let rows = self
            .rows
            .iter()
            .chain(self.spare.iter())
            .filter(|r| r.meta.first() != Some(&4))
            .filter_map(|r| u32_at(if b == 0 { &r.data } else { &r.data2 }, o));
        let vars = self.vspare.iter().map(|v| v.0).chain(self.orphans.iter().map(|v| v.0));
        rows.chain(vars).filter(|u| *u < 0x0100_0000).max().unwrap_or(0)
    }

    fn index(&self) -> HashMap<u32, usize> {
        self.rows.iter().enumerate().filter(|(_, r)| r.used()).filter_map(|(k, r)| r.uid.map(|u| (u, k))).collect()
    }
}

// =========================================================================== GUIDs

/// Random (version 4) GUIDs in the standard byte order; seeded from the clock and the process.
struct GuidGen(u64);

impl GuidGen {
    fn new() -> Self {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0);
        GuidGen(t ^ ((std::process::id() as u64) << 32) ^ 0x9E37_79B9_7F4A_7C15)
    }
    fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn guid(&mut self) -> [u8; 16] {
        let mut g = [0u8; 16];
        g[..8].copy_from_slice(&self.next_u64().to_le_bytes());
        g[8..].copy_from_slice(&self.next_u64().to_le_bytes());
        g[7] = (g[7] & 0x0F) | 0x40;
        g[8] = (g[8] & 0x3F) | 0x80;
        g
    }
}

// =========================================================================== saving

fn field_map(props: &HashMap<u32, Vec<u8>>, key: u32) -> FieldMap {
    props.get(&key).map(|d| parse_field_map(d)).unwrap_or_default()
}

fn put_streams(cfb: &Cfb, rt: &str, name: &str, t: &Tbl, replace: &mut HashMap<usize, Vec<u8>>) -> R<(u32, u32)> {
    let (fm, fd, f2, vm, vd) = t.write();
    let vsize = vd.len() as u32;
    // a stream MPP left out because it was empty stays out while it is empty
    let mut put = |s: &str, data: Vec<u8>| -> R<()> {
        match cfb.entry_index(&[rt, name, s]) {
            Some(i) => {
                replace.insert(i, data);
                Ok(())
            }
            None if data.is_empty() => Ok(()),
            None => Err(format!("The file has no {name}/{s} stream for the new data. Save as XML (.xml).")),
        }
    };
    put("FixedMeta", fm)?;
    put("FixedData", fd)?;
    if let Some((f2m, f2d)) = f2 {
        put("Fixed2Meta", f2m)?;
        put("Fixed2Data", f2d)?;
    }
    put("VarMeta", vm)?;
    put("Var2Data", vd)?;
    Ok((t.rows.len() as u32, vsize))
}

fn ts(x: Option<NaiveDateTime>) -> [u8; 4] {
    timestamp(x).to_le_bytes()
}

/// Which rows keep their sort key: the longest run of keys rising in display order (above the
/// project summary task's), so that as few rows as possible get new ones.
fn rising_keys(keys: &[Option<f64>], floor: f64) -> Vec<bool> {
    // longest increasing subsequence over the rows that have a key (patience sorting)
    let idx: Vec<usize> = (0..keys.len()).filter(|&i| keys[i].map(|k| k > floor).unwrap_or(false)).collect();
    let mut tails: Vec<usize> = Vec::new(); // positions in idx
    let mut prev: Vec<Option<usize>> = vec![None; idx.len()];
    for (p, &i) in idx.iter().enumerate() {
        let k = keys[i].unwrap();
        let at = tails.partition_point(|&t| keys[idx[t]].unwrap() < k);
        prev[p] = if at > 0 { Some(tails[at - 1]) } else { None };
        if at == tails.len() {
            tails.push(p);
        } else {
            tails[at] = p;
        }
    }
    let mut keep = vec![false; keys.len()];
    let mut cur = tails.last().copied();
    while let Some(p) = cur {
        keep[idx[p]] = true;
        cur = prev[p];
    }
    keep
}

/// Tests: NERAMIT_MPPT_SKIPFT="0b4004e7,0b400015" leaves those fixed fields as read.
fn skip_field(ft: u32) -> bool {
    cfg!(test)
        && std::env::var("NERAMIT_MPPT_SKIPFT")
            .map(|s| s.split(',').any(|x| u32::from_str_radix(x.trim(), 16).ok() == Some(ft)))
            .unwrap_or(false)
}

/// Tests: NERAMIT_MPPT_SKIP="fields,lists,ids,project" leaves those parts of the task table as read.
fn skip(part: &str) -> bool {
    cfg!(test) && std::env::var("NERAMIT_MPPT_SKIP").map(|s| s.split(',').any(|x| x == part)).unwrap_or(false)
}

/// Save `doc` into a copy of the .mpp it was read from (`orig`): every table is rebuilt from the
/// project, keeping the file's records (and the standard other data) for what is still there.
pub fn save_mpp(orig: &[u8], doc: &Project) -> R<Vec<u8>> {
    save_mpp_at(orig, doc, None)
}

/// save_mpp for a file to be written at `target` (links to other files are kept relative to it).
pub fn save_mpp_at(orig: &[u8], doc: &Project, target: Option<&std::path::Path>) -> R<Vec<u8>> {
    let this_path = target;
    let cfb = Cfb::new(orig)?;
    let v = crate::mpp::version_of(&cfb)?;
    if !std::ptr::eq(v, &crate::mpp::V14) {
        // an MPP12 file is saved in the MPP14 format, as the standard Save As does
        return save_new_mpp(doc);
    }
    let rt = v.root;
    let props_raw = cfb.stream(&[rt, "Props"]);
    let props = parse_props(&props_raw);
    let tmap = field_map(&props, 0x0002_0014);
    let rmap = field_map(&props, 0x0002_0015);
    let kmap = field_map(&props, 0x0002_0016);
    let amap = field_map(&props, 0x0002_0017);
    let cmap = field_map(&props, 0x0002_0018);
    if tmap.is_empty() {
        return Err("The task field map is missing.".into());
    }
    let cal = &doc.calendar;
    let mpd = cal.minutes_per_day();
    let mut gen = GuidGen::new();
    let now = chrono::Local::now().naive_local();
    let mut replace: HashMap<usize, Vec<u8>> = HashMap::new();

    // ================================================================ tasks: which record is whose
    let uid_loc = tmap.get(&T_UID).copied().ok_or("Task unique ID field not found.")?;
    let mut tt = Tbl::read(&cfb, rt, "TBkndTask", 47, Some(v.meta2), uid_loc)?;
    let ebits = EBits::of(&tt);
    let named: HashSet<u32> = tt.rows.iter().filter(|r| r.used() && r.var(T_NAME).is_some()).filter_map(|r| r.uid).filter(|u| *u != 0).collect();
    let mut next_uid = tt.max_uid() + 1;
    // doc task -> file unique ID (a task copied in Neramit Planner keeps its source's: only the first
    // one owns the record)
    let mut fuid: Vec<u32> = Vec::with_capacity(doc.tasks.len());
    let mut owned: HashSet<u32> = HashSet::new();
    let mut new_tasks: Vec<usize> = Vec::new();
    for (i, t) in doc.tasks.iter().enumerate() {
        match t.src_uid.filter(|u| named.contains(u) && !owned.contains(u)) {
            Some(u) => {
                owned.insert(u);
                fuid.push(u);
            }
            None => {
                fuid.push(next_uid);
                new_tasks.push(i);
                next_uid += 1;
            }
        }
    }
    let uid_to_fuid: HashMap<u32, u32> = doc.tasks.iter().zip(&fuid).map(|(t, u)| (t.uid, *u)).collect();
    let id_loc = tmap.get(&T_ID).copied();
    let file_id = |r: &Row| -> u32 { id_loc.and_then(|(b, o)| u32_at(if b == 0 { &r.data } else { &r.data2 }, o)).unwrap_or(u32::MAX) };
    // the file's rows in ID order (without the project summary task)
    let mut by_id: Vec<usize> = (0..tt.rows.len()).filter(|&k| tt.rows[k].used() && tt.rows[k].uid.map(|u| u != 0).unwrap_or(false)).collect();
    by_id.sort_by_key(|&k| file_id(&tt.rows[k]));
    // blank rows (the standard null tasks) stay after the task they followed
    let mut nulls_after: HashMap<Option<u32>, Vec<u32>> = HashMap::new();
    let mut last_kept: Option<u32> = None;
    let mut file_order: Vec<u32> = Vec::new();
    for &k in &by_id {
        let u = tt.rows[k].uid.unwrap_or(0);
        if named.contains(&u) {
            if owned.contains(&u) {
                last_kept = Some(u);
                file_order.push(u);
            }
        } else {
            nulls_after.entry(last_kept).or_default().push(u);
        }
    }
    // the file's outline before the edits (named tasks in ID order)
    let mut orig_parent: HashMap<u32, u32> = HashMap::new();
    let mut orig_kids: HashMap<u32, Vec<u32>> = HashMap::new();
    {
        let mut stack: Vec<(u16, u32)> = Vec::new();
        for &k in &by_id {
            let r = &tt.rows[k];
            let Some(u) = r.uid.filter(|u| named.contains(u)) else { continue };
            let lv = r.get_u16(&tmap, T_OUTLINE_LEVEL).unwrap_or(1);
            while stack.last().map(|s| s.0 >= lv).unwrap_or(false) {
                stack.pop();
            }
            let p = stack.last().map(|s| s.1).unwrap_or(0);
            orig_parent.insert(u, p);
            orig_kids.entry(p).or_default().push(u);
            stack.push((lv, u));
        }
    }
    let orig_zero: HashSet<u32> = tt.rows.iter().filter(|r| r.used() && r.get_u32(&tmap, T_DURATION) == Some(0)).filter_map(|r| r.uid).collect();
    // summaries the file keeps without a start-children list (older files - f4 - have none on
    // most summaries, and MPP failed to open a copy where Neramit Planner had added them)
    let orig_no_e4: HashSet<u32> = tt.rows.iter().filter(|r| r.used() && r.var(T_START_KIDS).is_none()).filter_map(|r| r.uid).filter(|u| orig_kids.contains_key(u)).collect();
    let deleted_tasks = named.iter().filter(|u| !owned.contains(u)).count();
    let doc_order: Vec<u32> = fuid.iter().copied().filter(|u| owned.contains(u)).collect();
    let reordered = !new_tasks.is_empty() || deleted_tasks > 0 || doc_order != file_order;

    // ================================================================ links wanted
    let c_uid_loc = cmap.get(&L_UID).copied().unwrap_or((0, 0));
    let mut ct = Tbl::read(&cfb, rt, "TBkndCons", 10, None, c_uid_loc)?;
    let c_pred = cmap.get(&L_PRED).map(|x| x.1).unwrap_or(4);
    let c_succ = cmap.get(&L_SUCC).map(|x| x.1).unwrap_or(8);
    let mut orig_preds: HashMap<u32, Vec<u32>> = HashMap::new();
    for r in ct.rows.iter().filter(|r| r.used() && r.data.len() >= 12) {
        let (pu, su) = (u32_at(&r.data, c_pred).unwrap_or(0), u32_at(&r.data, c_succ).unwrap_or(0));
        orig_preds.entry(su).or_default().push(pu);
    }
    for v in orig_preds.values_mut() {
        v.sort_unstable();
        v.dedup();
    }
    let mut want_links: Vec<(u32, u32, &Link)> = Vec::new();
    for (t, &su) in doc.tasks.iter().zip(&fuid) {
        for l in &t.predecessors {
            if let Some(&pu) = uid_to_fuid.get(&l.pred_uid) {
                if !want_links.iter().any(|x| x.0 == pu && x.1 == su) {
                    want_links.push((pu, su, l));
                }
            }
        }
    }

    // ================================================================ resources: which record is whose
    let r_uid_loc = rmap.get(&R_UID).copied().unwrap_or((0, 4));
    let mut rtb = Tbl::read(&cfb, rt, "TBkndRsc", 37, None, r_uid_loc)?;
    let res_named: HashSet<u32> = rtb.rows.iter().filter(|r| r.used() && r.var(R_NAME).is_some()).filter_map(|r| r.uid).filter(|u| *u != 0).collect();
    let mut next_ruid = rtb.max_uid() + 1;
    let mut res_fuid: HashMap<u32, u32> = HashMap::new(); // doc resource uid -> file uid
    let mut res_owned: HashSet<u32> = HashSet::new();
    let mut new_res: Vec<usize> = Vec::new();
    for (i, r) in doc.resources.iter().enumerate() {
        match r.src_uid.filter(|u| res_named.contains(u) && !res_owned.contains(u)) {
            Some(u) => {
                res_owned.insert(u);
                res_fuid.insert(r.uid, u);
            }
            None => {
                res_fuid.insert(r.uid, next_ruid);
                new_res.push(i);
                next_ruid += 1;
            }
        }
    }

    // the resources of every task in the file (assignment table, placeholders left out)
    let mut orig_res: HashMap<u32, Vec<u32>> = HashMap::new();
    {
        let a_uid_loc = amap.get(&A_UID).copied().unwrap_or((0, 0));
        let at0 = Tbl::read(&cfb, rt, "TBkndAssn", 34, None, a_uid_loc)?;
        let a_task = amap.get(&A_TASK).map(|x| x.1).unwrap_or(4);
        let a_res = amap.get(&A_RES).map(|x| x.1).unwrap_or(8);
        for r in at0.rows.iter().filter(|r| r.used() && r.data.len() >= 12) {
            let (tu, ru) = (u32_at(&r.data, a_task).unwrap_or(0), u32_at(&r.data, a_res).unwrap_or(0));
            orig_res.entry(tu).or_default().push(ru);
        }
        for v in orig_res.values_mut() {
            v.sort_unstable();
            v.dedup();
        }
    }

    // ================================================================ Project > Manage Sprints
    // (sprintio.rs): the standard "No Sprint" stays, the project's sprints follow in order
    let next_up = crate::sprintio::next_up_uid(&crate::sprintio::read_columns(&cfb, rt));
    let mut sprint_fuid: HashMap<u32, u32> = HashMap::new();
    let mut sprint_counts: Option<(u32, u32)> = None;
    let sprints_ok = cfb.entry_index(&[rt, "TBkndSprint", "FixedData"]).is_some();
    if sprints_ok && !skip("sprints") {
        let fu = crate::sprintio::file_uids(&doc.sprints);
        for (s, u) in doc.sprints.iter().zip(&fu) {
            sprint_fuid.insert(s.uid, *u);
        }
        let file_sprints = crate::sprintio::read_sprints(&cfb, rt, cal, doc.start);
        let same = file_sprints.len() == doc.sprints.len()
            && file_sprints.iter().zip(doc.sprints.iter().zip(&fu)).all(|(a, (b, u))| a.uid == *u && a.name == b.name && a.start == b.start && a.finish == b.finish);
        if !same {
            let mut st = Tbl::read(&cfb, rt, "TBkndSprint", 10, None, (0, 4))?;
            // the rows of "No Sprint" (ID 0) and of sprints kept give their GUIDs back
            let guid_of: HashMap<u32, [u8; 16]> = st.rows.iter().filter(|r| r.used()).filter_map(|r| Some((u32_at(&r.data, 4)?, r.data.get(8..24)?.try_into().ok()?))).collect();
            // (a sprint's own row: FixedMeta bytes 8-9 0x03B7; "No Sprint" has 0x0237)
            let proto = st.rows.iter().find(|r| r.used() && u32_at(&r.data, 0).map(|id| id > 0).unwrap_or(false)).cloned();
            st.rows.retain(|r| !r.used() || u32_at(&r.data, 0) == Some(0));
            if let Some(no) = st.rows.iter_mut().find(|r| r.used()) {
                if no.var(crate::sprintio::V_SPRINT_NAME).is_none() {
                    no.set_var(crate::sprintio::V_SPRINT_NAME, utf16z("No Sprint"));
                }
            }
            for (k, (s, &u)) in doc.sprints.iter().zip(&fu).enumerate() {
                let guid = guid_of.get(&u).copied().unwrap_or_else(|| gen.guid());
                let mut meta = proto.as_ref().map(|r| r.meta.clone()).unwrap_or_else(|| vec![0, 0, 1, 0, 0, 0, 0, 0, 0xb7, 0x03]);
                meta.resize(10, 0);
                let data = crate::sprintio::sprint_record(k as u32 + 1, u, guid, Some(cal.day_start(s.start)), Some(cal.day_end(s.finish)), 9, crate::sprintio::sprint_tenths(s));
                st.rows.push(Row { meta, data, meta2: Vec::new(), data2: Vec::new(), uid: Some(u), vars: vec![(crate::sprintio::V_SPRINT_NAME, utf16z(&s.name))] });
            }
            sprint_counts = Some(put_streams(&cfb, rt, "TBkndSprint", &st, &mut replace)?);
        }
    }

    // ================================================================ task rows
    if !new_tasks.is_empty() {
        let plain = task_template(&tt, &tmap, v, &by_id, &named, RowKind::Plain)
            .ok_or("The .mpp file has no task Neramit Planner can copy for the new tasks. Save as XML (.xml) instead.")?;
        // a new row is a copy of one of the same kind where the file has one (a summary
        // copied from a plain task kept bits MPP reads as the task's own, e.g. its estimate)
        let mut tmpl_of: HashMap<RowKind, usize> = HashMap::new();
        for &i in &new_tasks {
            let kind = RowKind::of(&doc.tasks[i]);
            let tmpl = *tmpl_of.entry(kind).or_insert_with(|| task_template(&tt, &tmap, v, &by_id, &named, kind).unwrap_or(plain));
            let mut r = tt.rows[tmpl].clone();
            let u = fuid[i];
            r.uid = Some(u);
            r.vars.retain(|(ft, _)| NEW_TASK_VARS.contains(ft));
            r.put(&tmap, T_UID, &u.to_le_bytes());
            r.put(&tmap, T_GUID, &gen.guid());
            r.put(&tmap, T_CREATED, &ts(Some(now)));
            r.put(&tmap, T_LEVEL_DELAY, &((doc.tasks[i].level_delay_min * 10) as u32).to_le_bytes());
            tt.rows.push(r);
        }
    }
    // deleted tasks go (their links and assignments below)
    tt.rows.retain(|r| match r.uid {
        Some(u) if r.used() && named.contains(&u) => owned.contains(&u),
        _ => true,
    });
    let row_of = tt.index();
    let task_guid: HashMap<u32, [u8; 16]> = tt.rows.iter().filter(|r| r.used()).filter_map(|r| r.uid.map(|u| (u, r.guid(&tmap, T_GUID)))).collect();
    let project_guid = task_guid.get(&0).copied().unwrap_or([0; 16]);
    // IDs and sort keys: the project's order, blank rows after the task they followed
    if reordered && !skip("ids") {
        let mut display: Vec<u32> = nulls_after.get(&None).cloned().unwrap_or_default();
        for &u in &fuid {
            display.push(u);
            if owned.contains(&u) {
                if let Some(n) = nulls_after.get(&Some(u)) {
                    display.extend(n.iter().copied());
                }
            }
        }
        // sort keys: a row keeps its own while the order allows it (MPP keeps them, and
        // re-keying every row made MPP move a manually scheduled task to the project start);
        // new and moved rows get one between their neighbours'
        let new_set: HashSet<u32> = new_tasks.iter().map(|&i| fuid[i]).collect();
        let old_key: Vec<Option<f64>> = display
            .iter()
            .map(|u| {
                row_of.get(u).filter(|_| !new_set.contains(u)).and_then(|&k| {
                    let b = tt.rows[k].get(&tmap, T_SORT_KEY, 8)?;
                    let x = f64::from_le_bytes(b.get(..8)?.try_into().ok()?);
                    x.is_finite().then_some(x)
                })
            })
            .collect();
        let project_key = row_of
            .get(&0)
            .and_then(|&k| tt.rows[k].get(&tmap, T_SORT_KEY, 8))
            .and_then(|b| b.get(..8).map(|b| f64::from_le_bytes(b.try_into().unwrap())))
            .filter(|x| x.is_finite())
            .unwrap_or(1.0);
        // keep the longest run of rising keys (a moved row loses its key)
        let keep = rising_keys(&old_key, project_key);
        let mut prev = project_key;
        for (pos, &u) in display.iter().enumerate() {
            let key = if keep[pos] {
                old_key[pos].unwrap()
            } else {
                let next = (pos + 1..display.len()).find(|&j| keep[j]).and_then(|j| old_key[j]);
                match next {
                    Some(n) => prev + (n - prev) / 2.0,
                    None => prev.floor() + 1.0,
                }
            };
            prev = key;
            if let Some(&k) = row_of.get(&u) {
                let id = pos as u32 + 1;
                tt.rows[k].put(&tmap, T_ID, &id.to_le_bytes());
                tt.rows[k].put(&tmap, T_SORT_KEY, &key.to_le_bytes());
            }
        }
    }
    // outline: parents and the summaries' children
    let mut parent: Vec<Option<usize>> = vec![None; doc.tasks.len()];
    {
        let mut stack: Vec<usize> = Vec::new();
        for (i, t) in doc.tasks.iter().enumerate() {
            while stack.last().map(|&s| doc.tasks[s].outline_level >= t.outline_level).unwrap_or(false) {
                stack.pop();
            }
            parent[i] = stack.last().copied();
            stack.push(i);
        }
    }
    // the project's outline and links in file unique IDs
    let new_parent: HashMap<u32, u32> = (0..doc.tasks.len()).map(|i| (fuid[i], parent[i].map(|p| fuid[p]).unwrap_or(0))).collect();
    let mut new_kids: HashMap<u32, Vec<u32>> = HashMap::new();
    for i in 0..doc.tasks.len() {
        new_kids.entry(new_parent[&fuid[i]]).or_default().push(fuid[i]);
    }
    let new_preds: HashMap<u32, Vec<u32>> = doc
        .tasks
        .iter()
        .zip(&fuid)
        .map(|(t, &u)| {
            let mut v: Vec<u32> = t.predecessors.iter().filter_map(|l| uid_to_fuid.get(&l.pred_uid).copied()).collect();
            v.sort_unstable();
            v.dedup();
            (u, v)
        })
        .collect();
    let empty: Vec<u32> = Vec::new();
    let preds_changed = |u: u32| new_preds.get(&u).unwrap_or(&empty) != orig_preds.get(&u).unwrap_or(&empty);
    let parent_changed = |u: u32| new_parent.get(&u) != orig_parent.get(&u);
    fn descendants(kids: &HashMap<u32, Vec<u32>>, u: u32, out: &mut Vec<u32>) {
        if let Some(ks) = kids.get(&u) {
            for &k in ks {
                if out.len() < 100_000 {
                    out.push(k);
                    descendants(kids, k, out);
                }
            }
        }
    }
    // a summary's list: the children that start when the summary starts (the standard
    // files; a list naming a child that starts later makes MPP fail on opening). Never empty:
    // then its first child.
    let start_at: HashMap<u32, NaiveDateTime> = doc
        .tasks
        .iter()
        .zip(&fuid)
        .map(|(t, &u)| (u, t.start_dt.unwrap_or_else(|| cal.day_start(t.start))))
        .collect();
    let summary_start = |s: u32| -> Option<NaiveDateTime> {
        let ks = new_kids.get(&s)?;
        let own = if s == 0 { None } else { start_at.get(&s).copied() };
        own.or_else(|| ks.iter().filter_map(|k| start_at.get(k)).min().copied())
    };
    // a list written is kept while every child it names is still a child starting with the
    // summary (MPP lists only some of them)
    let start_kids_hold = |s: u32, list: Option<&[u8]>| -> bool {
        let Some(b) = list.filter(|b| b.len() >= 8) else { return false };
        let ks = new_kids.get(&s).map(|v| v.as_slice()).unwrap_or(&[]);
        let n = (u32_at(b, 0).unwrap_or(0) as usize).min(b.len() / 4 - 1);
        let at = summary_start(s);
        n > 0 && (0..n).all(|i| u32_at(b, 4 + i * 4).map(|k| ks.contains(&k) && start_at.get(&k).copied() == at).unwrap_or(false))
    };
    // does a summary's list need working out again (its children or their links changed)?
    let kids_changed = |s: u32| -> bool {
        if new_kids.get(&s) != orig_kids.get(&s) {
            return true;
        }
        let mut sub = Vec::new();
        descendants(&new_kids, s, &mut sub);
        sub.iter().any(|&d| preds_changed(d) || parent_changed(d) || !owned.contains(&d))
    };
    let pstart = doc.start_dt.filter(|t| t.date() == doc.start).unwrap_or_else(|| cal.day_start(doc.start));
    // WBS codes the mask generated: their own level's part (MPP adds the parents')
    let wbs_own = crate::wbs::own_parts(doc);
    // tasks whose % complete is not the file's (their assignments' actual work is worked out again)
    let mut pct_changed: HashSet<u32> = HashSet::new();
    for (i, t) in doc.tasks.iter().enumerate() {
        let u = fuid[i];
        let Some(&k) = row_of.get(&u) else { continue };
        let r = &mut tt.rows[k];
        let is_new = !owned.contains(&u);
        if is_new || r.get_u16(&tmap, T_PERCENT) != Some(t.percent_complete as u16) {
            pct_changed.insert(u);
        }
        if t.external.is_some() && !is_new && !skip("fields") {
            // a task of another project keeps the standard row: only dates brought in from its file
            write_external_dates(r, &tmap, t, mpd);
        } else if !skip("fields") {
            write_task(r, &tmap, v, t, cal, mpd, is_new, pstart, ebits, wbs_own[i].as_deref());
            if let Some(x) = t.external.as_ref().filter(|_| is_new) {
                make_external(r, &tmap, x);
            }
            // Project > Manage Sprints / Task Board (sprintio.rs)
            if sprints_ok && !skip("sprints") {
                // (the standard rows keep both items, -1 for none, with their bits in sync_var_bits)
                let used = t.sprint.is_some() || t.board_status == 1;
                match t.sprint.and_then(|u| sprint_fuid.get(&u)) {
                    Some(&u) => r.set_var(crate::sprintio::T_SPRINT, u.to_le_bytes().to_vec()),
                    None if used || r.var(crate::sprintio::T_SPRINT).is_some() => r.set_var(crate::sprintio::T_SPRINT, u32::MAX.to_le_bytes().to_vec()),
                    None => {}
                }
                let col = r.var(crate::sprintio::T_BOARD_COLUMN).and_then(|v| u32_at(v, 0));
                if t.board_status == 1 && col != Some(next_up) {
                    r.set_var(crate::sprintio::T_BOARD_COLUMN, next_up.to_le_bytes().to_vec());
                } else if (t.board_status != 1 && col == Some(next_up)) || (used && col.is_none()) {
                    r.set_var(crate::sprintio::T_BOARD_COLUMN, u32::MAX.to_le_bytes().to_vec());
                }
                sync_var_bits(r, ebits);
            }
        }
        if skip("lists") {
            continue;
        }
        // outline parent
        let (pu, pg) = match parent[i] {
            Some(p) => (fuid[p], task_guid.get(&fuid[p]).copied().unwrap_or(project_guid)),
            None => (0, project_guid),
        };
        if !skip("parent") {
            r.put(&tmap, T_PARENT_UID, &pu.to_le_bytes());
            r.put(&tmap, T_PARENT_GUID, &pg);
        }
        // scheduling lists: kept as saved unless what they depend on changed
        let preds: Vec<u32> = new_preds.get(&u).cloned().unwrap_or_default();
        let e3_changed = is_new || preds_changed(u) || parent_changed(u) || preds.iter().any(|&p| parent_changed(p));
        if skip("e3") || !e3_changed {
        } else {
            // (a list Neramit Planner worked out made MPP fail on f2 converted whole; tasks
            // without one open and schedule the same - MPP works it out)
            r.remove_var(T_PRED_LIST);
        }
        let mut res_now: Vec<u32> = t.assignments.iter().filter_map(|a| res_fuid.get(&a.res_uid).copied()).collect();
        res_now.sort_unstable();
        let e5_changed = e3_changed
            || preds.is_empty() != orig_preds.get(&u).map(|v| v.is_empty()).unwrap_or(true)
            || orig_res.get(&u).map(|v| v.iter().copied().filter(|x| *x != UNASSIGNED).collect::<Vec<_>>()).unwrap_or_default() != res_now
            || (t.dur_minutes(mpd) == 0) != orig_zero.contains(&u);
        if skip("e5") || !e5_changed {
        } else {
            // (the resources of a linked task: left to MPP as the lists above)
            r.remove_var(T_LINKED_RES);
        }
        if !skip("e4") {
            if new_kids.contains_key(&u) {
                if orig_no_e4.contains(&u) && !is_new {
                    // the file's summary had none: it stays without
                } else if is_new || kids_changed(u) || !start_kids_hold(u, r.var(T_START_KIDS)) {
                    // (a list Neramit Planner worked out made MPP fail on some files - f4
                    // converted whole, f8 - while summaries without one, which the standard files
                    // have too, open and schedule the same: it is left to MPP)
                    r.remove_var(T_START_KIDS);
                }
            } else {
                r.remove_var(T_START_KIDS);
            }
        }
        // Fixed2Meta byte 95: 0x08 summary; the list bits follow the lists
        if !skip("flags") {
            sync_var_bits(r, ebits);
        }
        // FixedMeta byte 16 bits 0x10 / 0x80: the task has real resources (without them MPP
        // counts its placeholder's work as nobody's)
        if !t.is_summary && !skip("flags") {
            let real = t.assignments.iter().any(|a| res_fuid.contains_key(&a.res_uid));
            r.flag(16, 0x10, real);
            r.flag(16, 0x80, real);
        } else if !skip("flags") {
            // (never on a summary in the user's files; 0x10 on nearly all)
            r.flag(16, 0x80, false);
            if is_new {
                r.flag(16, 0x10, true);
            }
        }
    }
    // the project summary task: dates, duration, work, cost and the top-level list
    if let Some(&k) = row_of.get(&0).filter(|_| !skip("project")) {
        let r = &mut tt.rows[k];
        write_project_task(r, &tmap, doc, cal, mpd, pstart);
        if !skip("e4") && new_kids.contains_key(&0) && !orig_no_e4.contains(&0) && (kids_changed(0) || !start_kids_hold(0, r.var(T_START_KIDS))) {
            r.remove_var(T_START_KIDS);
        }
    }
    let (task_rows, task_vsize) = put_streams(&cfb, rt, "TBkndTask", &tt, &mut replace)?;

    // ================================================================ resources and their calendars
    let k_uid_loc = kmap.get(&C_UID).copied().unwrap_or((0, 8));
    let mut kt = Tbl::read(&cfb, rt, "TBkndCal", 10, None, k_uid_loc)?;
    let k_res = kmap.get(&C_RES).map(|x| x.1).unwrap_or(4);
    let mut next_kuid = kt.max_uid() + 1;
    // the project calendar: written again when its working time is not what the file has
    // (FixedMeta byte 8 bit 0x40 says the row has its calendar data)
    {
        let pname = props.get(&crate::mpp::P_CALENDAR).map(|d| utf16(d)).unwrap_or_default();
        let is_base = |r: &Row| r.used() && r.data.len() >= 12 && u32_at(&r.data, k_res).map(|x| x == u32::MAX).unwrap_or(false);
        let k = kt
            .rows
            .iter()
            .position(|r| is_base(r) && r.var(crate::mpp::C_NAME).map(utf16).as_deref() == Some(pname.as_str()))
            .or_else(|| kt.rows.iter().position(|r| is_base(r) && r.var(crate::mpp::C_NAME).is_some()));
        if let Some(k) = k.filter(|_| !skip("calendar")) {
            let mut have = Calendar::default();
            if let Some(d) = kt.rows[k].var(crate::mpp::C_DATA) {
                crate::mpp::apply_calendar(&mut have, d);
            }
            if !crate::mpp::same_calendar(&have, &doc.calendar) {
                match crate::mpp::calendar_data(&doc.calendar) {
                    Some(b) => kt.rows[k].set_var(crate::mpp::C_DATA, b),
                    None => kt.rows[k].remove_var(crate::mpp::C_DATA),
                }
                let has = kt.rows[k].var(crate::mpp::C_DATA).is_some();
                kt.rows[k].flag(8, 0x40, has);
            }
        }
    }
    if !new_res.is_empty() {
        let candidates: Vec<usize> = (0..rtb.rows.len()).filter(|&k| rtb.rows[k].uid.map(|u| res_named.contains(&u)).unwrap_or(false)).collect();
        // a resource's type in the file: material (FixedMeta byte 14 bit 0), cost (Fixed2Meta
        // byte 8 bit 0x10) or work
        let kind_of = |k: usize| -> ResourceType {
            let r = &rtb.rows[k];
            if r.meta.get(14).map(|b| b & 1 == 1).unwrap_or(false) {
                ResourceType::Material
            } else if r.meta2.get(8).map(|b| b & 0x10 != 0).unwrap_or(false) {
                ResourceType::Cost
            } else {
                ResourceType::Work
            }
        };
        let fallback = candidates
            .iter()
            .copied()
            .filter(|&k| kind_of(k) == ResourceType::Work)
            .min_by_key(|&k| rtb.rows[k].vars.len())
            .or_else(|| candidates.first().copied())
            .or_else(|| (0..rtb.rows.len()).find(|&k| rtb.rows[k].used() && rtb.rows[k].uid == Some(0)))
            .ok_or("The .mpp file has no resource Neramit Planner can copy for the new resources. Save as XML (.xml).")?;
        // a copy of a resource of the same type when the file has one
        let tmpl: Vec<(ResourceType, usize)> = [ResourceType::Work, ResourceType::Material, ResourceType::Cost]
            .into_iter()
            .map(|ty| (ty, candidates.iter().copied().filter(|&k| kind_of(k) == ty).min_by_key(|&k| rtb.rows[k].vars.len()).unwrap_or(fallback)))
            .collect();
        let tmpl_for = |ty: ResourceType| tmpl.iter().find(|x| x.0 == ty).map(|x| x.1).unwrap_or(fallback);
        // a resource calendar to copy, and the project calendar new resources are based on (as in
        // MPP; a calendar based on another one made MPP schedule on that one)
        let is_res_cal = |r: &Row| r.used() && r.data.len() >= 12 && u32_at(&r.data, k_res).map(|x| x != u32::MAX).unwrap_or(false);
        let cal_tmpl = kt.rows.iter().position(|r| is_res_cal(r) && u32_at(&r.data, k_res) != Some(0)).or_else(|| kt.rows.iter().position(is_res_cal));
        let pcal_name = props.get(&crate::mpp::P_CALENDAR).map(|d| utf16(d)).unwrap_or_default();
        let pcal = kt
            .rows
            .iter()
            .filter(|r| r.used() && !is_res_cal(r))
            .find(|r| r.var(crate::mpp::C_NAME).map(utf16).as_deref() == Some(pcal_name.as_str()))
            .map(|r| (r.get_u32(&kmap, C_UID).unwrap_or(1), r.guid(&kmap, C_GUID)));
        for &i in &new_res {
            let mut row = rtb.rows[tmpl_for(doc.resources[i].rtype)].clone();
            let u = res_fuid[&doc.resources[i].uid];
            row.uid = Some(u);
            row.vars.retain(|(ft, _)| NEW_RES_VARS.contains(ft));
            row.set_var(R_CREATED, ts(Some(now)).to_vec());
            row.put(&rmap, R_UID, &u.to_le_bytes());
            let g = gen.guid();
            row.put(&rmap, R_GUID, &g);
            row.put(&rmap, R_GUID2, &g);
            if let Some(ck) = cal_tmpl {
                let mut c = kt.rows[ck].clone();
                let cu = next_kuid;
                next_kuid += 1;
                c.uid = Some(cu);
                c.vars.clear();
                c.put(&kmap, C_UID, &cu.to_le_bytes());
                put_u32(&mut c.data, k_res, u);
                c.put(&kmap, C_GUID, &g);
                c.put(&kmap, C_RES_GUID, &g);
                if let Some((pu, pg)) = pcal {
                    c.put(&kmap, C_BASE, &pu.to_le_bytes());
                    c.put(&kmap, C_BASE_GUID, &pg);
                }
                kt.rows.push(c);
                row.put(&rmap, R_CALENDAR, &cu.to_le_bytes());
            }
            rtb.rows.push(row);
        }
    }
    let deleted_res: HashSet<u32> = res_named.iter().copied().filter(|u| !res_owned.contains(u)).collect();
    rtb.rows.retain(|r| match r.uid {
        Some(u) if r.used() => !deleted_res.contains(&u),
        _ => true,
    });
    kt.rows.retain(|r| !(r.used() && r.data.len() >= 12 && u32_at(&r.data, k_res).map(|x| deleted_res.contains(&x)).unwrap_or(false)));
    let rrow_of = rtb.index();
    // work and actual work of every resource (a material's work is its quantity, an hour a unit)
    let mut res_work: HashMap<u32, (f64, f64)> = HashMap::new();
    for t in doc.tasks.iter().filter(|t| !t.is_summary) {
        for a in &t.assignments {
            let Some(r) = doc.resource_by_uid(a.res_uid) else { continue };
            let w = match r.rtype {
                ResourceType::Work => a.work_min.max(0) as f64,
                ResourceType::Material => a.units * 60.0,
                ResourceType::Cost => 0.0,
            };
            let e = res_work.entry(a.res_uid).or_default();
            e.0 += w;
            e.1 += w * t.percent_complete as f64 / 100.0;
        }
    }
    let res_changed = !new_res.is_empty() || !deleted_res.is_empty() || doc.resources.iter().map(|r| res_fuid[&r.uid]).collect::<Vec<_>>() != {
        let mut v: Vec<(u32, u32)> = rtb.rows.iter().filter(|r| r.used()).filter_map(|r| r.uid.filter(|u| res_named.contains(u)).map(|u| (r.get_u32(&rmap, R_ID).unwrap_or(0), u))).collect();
        v.sort_unstable();
        v.into_iter().map(|x| x.1).collect::<Vec<_>>()
    };
    for (pos, r) in doc.resources.iter().enumerate() {
        let u = res_fuid[&r.uid];
        let Some(&k) = rrow_of.get(&u) else { continue };
        let row = &mut rtb.rows[k];
        if res_changed {
            let id = pos as u32 + 1;
            row.put(&rmap, R_ID, &id.to_le_bytes());
            row.put(&rmap, R_SORT_KEY, &(id as f64 + 1.0).to_le_bytes());
        }
        let (w, aw) = res_work.get(&r.uid).copied().unwrap_or((0.0, 0.0));
        let w = if r.rtype == ResourceType::Work { r.work_min as f64 } else { w };
        write_resource(row, &rmap, r, w, aw, !res_owned.contains(&u));
    }
    let res_guid: HashMap<u32, [u8; 16]> = rtb.rows.iter().filter(|r| r.used()).filter_map(|r| r.uid.map(|u| (u, r.guid(&rmap, R_GUID)))).collect();
    let (res_rows, res_vsize) = put_streams(&cfb, rt, "TBkndRsc", &rtb, &mut replace)?;
    let (cal_rows, cal_vsize) = put_streams(&cfb, rt, "TBkndCal", &kt, &mut replace)?;

    // ================================================================ assignments
    let a_uid_loc = amap.get(&A_UID).copied().unwrap_or((0, 0));
    let mut at = Tbl::read(&cfb, rt, "TBkndAssn", 34, None, a_uid_loc)?;
    let a_task = amap.get(&A_TASK).map(|x| x.1).unwrap_or(4);
    let a_res = amap.get(&A_RES).map(|x| x.1).unwrap_or(8);
    let mut next_auid = at.max_uid() + 1;
    let task_by_fuid: HashMap<u32, &Task> = doc.tasks.iter().zip(&fuid).map(|(t, u)| (*u, t)).collect();
    // wanted: (task file uid, resource file uid) -> assignment; placeholders for tasks without any
    let mut want_asg: Vec<(u32, u32, Option<&Assignment>)> = Vec::new();
    for (t, &u) in doc.tasks.iter().zip(&fuid) {
        // a summary has no assignments (Neramit Planner rolls its subtasks up, as its XML says)
        if t.is_summary {
            continue;
        }
        let real: Vec<&Assignment> = t.assignments.iter().filter(|a| res_fuid.contains_key(&a.res_uid)).collect();
        if real.is_empty() {
            want_asg.push((u, UNASSIGNED, None));
        }
        for a in real {
            want_asg.push((u, res_fuid[&a.res_uid], Some(a)));
        }
    }
    let want_keys: HashSet<(u32, u32)> = want_asg.iter().map(|x| (x.0, x.1)).collect();
    let asg_key = |r: &Row| (u32_at(&r.data, a_task).unwrap_or(0), u32_at(&r.data, a_res).unwrap_or(0));
    let real_tmpl = at.rows.iter().find(|r| r.used() && r.data.len() >= 12 && asg_key(r).1 != UNASSIGNED).cloned();
    let ph_tmpl = at.rows.iter().find(|r| r.used() && r.data.len() >= 12 && asg_key(r).1 == UNASSIGNED).cloned();
    let mut have_keys: HashSet<(u32, u32)> = HashSet::new();
    at.rows.retain(|r| {
        if !r.used() || r.data.len() < 12 {
            return true;
        }
        let key = asg_key(r);
        want_keys.contains(&key) && have_keys.insert(key)
    });
    let mut new_asg: HashSet<u32> = HashSet::new();
    for &(tu, ru, _) in &want_asg {
        if have_keys.contains(&(tu, ru)) {
            continue;
        }
        let base = if ru == UNASSIGNED { ph_tmpl.clone().or_else(|| real_tmpl.clone()) } else { real_tmpl.clone().or_else(|| ph_tmpl.clone()) };
        let Some(mut row) = base else {
            return Err("The .mpp file has no assignment Neramit Planner can copy for the new tasks. Save as XML (.xml).".into());
        };
        let au = next_auid;
        next_auid += 1;
        new_asg.insert(au);
        row.uid = Some(au);
        // MPP rebuilds the timephased work; the other var items are copied
        row.remove_var(A_TIMEPHASED_WORK);
        row.remove_var(0x0F40_0032);
        row.set_var(A_CREATED, ts(Some(now)).to_vec());
        row.put(&amap, A_UID, &au.to_le_bytes());
        put_u32(&mut row.data, a_task, tu);
        put_u32(&mut row.data, a_res, ru);
        row.put(&amap, A_GUID, &gen.guid());
        have_keys.insert((tu, ru));
        at.rows.push(row);
    }
    let want_map: HashMap<(u32, u32), Option<&Assignment>> = want_asg.iter().map(|x| ((x.0, x.1), x.2)).collect();
    for row in at.rows.iter_mut() {
        if !row.used() || row.data.len() < 12 {
            continue;
        }
        let key = asg_key(row);
        let Some(&a) = want_map.get(&key) else { continue };
        let Some(t) = task_by_fuid.get(&key.0) else { continue };
        let res = a.and_then(|a| doc.resource_by_uid(a.res_uid));
        let keep_actuals = !pct_changed.contains(&key.0) && !row.uid.map(|u| new_asg.contains(&u)).unwrap_or(true);
        write_assignment(row, &amap, t, a, res, cal, mpd, keep_actuals);
        // Fixed2Meta: byte 9 bit 0x08 = a real resource (a placeholder's work does not count),
        // byte 8 bits 0x30 = a cost resource
        row.flag2(9, 0x08, key.1 != UNASSIGNED);
        row.flag2(8, 0x30, res.map(|r| r.rtype == ResourceType::Cost).unwrap_or(false));
        row.put(&amap, A_TASK_GUID, &task_guid.get(&key.0).copied().unwrap_or([0; 16]));
        row.put(&amap, A_RES_GUID, &if key.1 == UNASSIGNED { UNASSIGNED_GUID } else { res_guid.get(&key.1).copied().unwrap_or([0; 16]) });
    }
    let (asg_rows, asg_vsize) = put_streams(&cfb, rt, "TBkndAssn", &at, &mut replace)?;

    // ================================================================ links
    let external_fuids: HashSet<u32> = doc.tasks.iter().zip(&fuid).filter(|(t, _)| t.external.is_some()).map(|(_, &u)| u).collect();
    let want_set: HashSet<(u32, u32)> = want_links.iter().map(|x| (x.0, x.1)).collect();
    let link_tmpl = ct.rows.iter().find(|r| r.used() && r.data.len() >= 20).cloned();
    let mut have_links: HashSet<(u32, u32)> = HashSet::new();
    ct.rows.retain(|r| {
        if !r.used() || r.data.len() < 12 {
            return true;
        }
        let key = (u32_at(&r.data, c_pred).unwrap_or(0), u32_at(&r.data, c_succ).unwrap_or(0));
        want_set.contains(&key) && have_links.insert(key)
    });
    let mut next_luid = ct.max_uid() + 1;
    for &(pu, su, _) in &want_links {
        if have_links.contains(&(pu, su)) {
            continue;
        }
        let Some(mut row) = link_tmpl.clone() else {
            return Err("Links were added to a .mpp file that has none. Neramit Planner can't write the first link of such a file yet; save as XML (.xml).".into());
        };
        let lu = next_luid;
        next_luid += 1;
        row.uid = Some(lu);
        row.vars.clear();
        row.put(&cmap, L_UID, &lu.to_le_bytes());
        put_u32(&mut row.data, c_pred, pu);
        put_u32(&mut row.data, c_succ, su);
        row.put(&cmap, L_GUID, &gen.guid());
        have_links.insert((pu, su));
        ct.rows.push(row);
    }
    let link_of: HashMap<(u32, u32), &Link> = want_links.iter().map(|x| ((x.0, x.1), x.2)).collect();
    let t_off = cmap.get(&L_TYPE).map(|x| x.1).unwrap_or(12);
    let lag_off = cmap.get(&L_LAG).map(|x| x.1).unwrap_or(v.link_lag_offset);
    let fmt_off = cmap.get(&L_FMT).map(|x| x.1).unwrap_or(v.link_format_offset);
    for row in ct.rows.iter_mut() {
        if !row.used() || row.data.len() < 20 {
            continue;
        }
        let key = (u32_at(&row.data, c_pred).unwrap_or(0), u32_at(&row.data, c_succ).unwrap_or(0));
        let Some(l) = link_of.get(&key) else { continue };
        let ty: u16 = match l.kind {
            LinkType::FF => 0,
            LinkType::FS => 1,
            LinkType::SF => 2,
            LinkType::SS => 3,
        };
        let (lag, fmt) = match l.lag_pct {
            Some(pct) => (pct.round() as i32, if l.elapsed { 20u16 } else { 19 }),
            None => ((l.lag_minutes(mpd, 0) * 10) as i32, l.lag_unit.code(l.elapsed) as u16),
        };
        put_u16(&mut row.data, t_off, ty);
        put_u32(&mut row.data, lag_off, lag as u32);
        put_u16(&mut row.data, fmt_off, fmt);
        row.put(&cmap, L_PRED_GUID, &task_guid.get(&key.0).copied().unwrap_or([0; 16]));
        row.put(&cmap, L_SUCC_GUID, &task_guid.get(&key.1).copied().unwrap_or([0; 16]));
        // FixedMeta byte 8 bit 0x20: a link to or from a task of another project (without
        // it MPP takes the link for one inside the project)
        let cross = external_fuids.contains(&key.0) || external_fuids.contains(&key.1);
        set_bit(&mut row.meta, 8, 0x20, cross);
    }
    let (link_rows, link_vsize) = put_streams(&cfb, rt, "TBkndCons", &ct, &mut replace)?;

    // ================================================================ project properties
    let counts: HashMap<u32, u32> = [
        (0x0100_0001, task_rows),
        (0x0001_0001, task_vsize),
        (0x0100_0002, res_rows),
        (0x0001_0002, res_vsize),
        (0x0100_0003, cal_rows),
        (0x0001_0003, cal_vsize),
        (0x0100_0004, asg_rows),
        (0x0001_0004, asg_vsize),
        (0x0100_0005, link_rows),
        (0x0001_0005, link_vsize),
    ]
    .into_iter()
    .collect();
    let finish = doc.tasks.iter().filter(|t| t.active).filter_map(|t| t.finish_dt).max().unwrap_or(pstart);
    // ================================================================ Project > Custom Fields
    // renamed fields and lookup tables: the task / resource table's Props, TBkndOutlCode and the
    // project's Props (cfdefs.rs)
    let du = crate::util::DurUnits::of(doc);
    let file_lists = crate::cfdefs::lookup_values(&cfb, rt);
    let mut oc = Tbl::read(&cfb, rt, "TBkndOutlCode", 10, Some(10), (0, 0)).ok();
    let mut oc_changed = false;
    let mut masks_add: Vec<([u8; 16], crate::custom::CfKind)> = Vec::new();
    let mut tables_gone: Vec<[u8; 16]> = Vec::new();
    let mut lookups_gone: Vec<u32> = Vec::new();
    for (table, resource) in [("TBkndTask", false), ("TBkndRsc", true)] {
        if skip("cfdefs") {
            break;
        }
        let Some(idx) = cfb.entry_index(&[rt, table, "Props"]) else { continue };
        let raw = cfb.stream(&[rt, table, "Props"]);
        let tp = crate::mpp::parse_props(&raw);
        let Some(have) = tp.get(&crate::cfdefs::DEFS_ITEM).and_then(|b| crate::cfdefs::parse_item(b, true)) else { continue };
        let mut item = have.clone();
        item.aliases = doc.custom_defs.iter().filter(|d| d.resource == resource && !d.alias.trim().is_empty()).map(|d| (d.field.field_id(resource), d.alias.trim().to_string())).collect();
        item.aliases.sort_by_key(|a| a.0);
        let lists: Vec<&crate::custom::CustomFieldDef> = doc.custom_defs.iter().filter(|d| d.resource == resource && !d.values.is_empty() && crate::cfdefs::type_codes(d.field.kind).is_some()).collect();
        if let Some(oc) = oc.as_mut() {
            // a list the project no longer has: its table goes
            for e in item.attrs.iter_mut() {
                let Some(id) = u32_at(e, 0) else { continue };
                let Some(t) = crate::cfdefs::attr_table(e) else { continue };
                if !lists.iter().any(|d| d.field.field_id(resource) == id) {
                    tables_gone.push(t);
                    lookups_gone.push(id);
                    e[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
                    e[36..68].fill(0);
                }
            }
            for d in &lists {
                let id = d.field.field_id(resource);
                let pos = item.attrs.iter().position(|e| u32_at(e, 0) == Some(id));
                let tguid = match pos.and_then(|p| crate::cfdefs::attr_table(&item.attrs[p])) {
                    Some(t) => t,
                    None => {
                        let t = gen.guid();
                        match pos {
                            Some(p) => item.attrs[p][36..52].copy_from_slice(&t),
                            None => match crate::cfdefs::new_attr(id, d.field.kind, t) {
                                Some(e) => item.attrs.push(e),
                                None => continue,
                            },
                        }
                        masks_add.push((t, d.field.kind));
                        t
                    }
                };
                if file_lists.get(&tguid) != Some(&d.values) {
                    let kept = set_lookup_rows(oc, &mut gen, tguid, d.field.kind, &d.values, du);
                    oc_changed = true;
                    // a default value that went: none
                    if let Some(e) = item.attrs.iter_mut().find(|e| u32_at(e, 0) == Some(id)) {
                        let def: [u8; 16] = e[52..68].try_into().unwrap();
                        if def != [0; 16] && !kept.contains(&def) {
                            e[52..68].fill(0);
                        }
                    }
                }
            }
            item.attrs.sort_by_key(|e| u32_at(e, 0).unwrap_or(0));
        }
        let mut have_aliases = have.aliases.clone();
        have_aliases.sort_by_key(|a| a.0);
        if have_aliases == item.aliases && have.attrs == item.attrs {
            continue;
        }
        let bytes = crate::cfdefs::build_item(&item);
        replace.insert(idx, rewrite_props(&raw, |key, val| {
            if key == crate::cfdefs::DEFS_ITEM {
                *val = bytes.clone();
            }
        }));
    }
    let mut oc_counts: Option<(u32, u32)> = None;
    if let Some(oc) = oc.as_mut() {
        if !tables_gone.is_empty() {
            oc.rows.retain(|r| !(r.used() && r.data2.get(18..34).map(|g| tables_gone.iter().any(|t| t[..] == *g)).unwrap_or(false)));
            oc_changed = true;
        }
        if oc_changed {
            oc_counts = Some(put_streams(&cfb, rt, "TBkndOutlCode", oc, &mut replace)?);
        }
    }
    // formulas (the project's Props), keeping the standard flags (rollups) for a field that keeps one
    let formulas_have = props.get(&crate::cfdefs::P_FORMULAS).map(|b| crate::cfdefs::parse_formulas(b)).unwrap_or_default();
    let formulas_want: Vec<(u32, u32, String)> = doc
        .custom_defs
        .iter()
        .filter(|d| !d.formula.trim().is_empty())
        .map(|d| {
            let id = d.field.field_id(d.resource);
            let flags = formulas_have.iter().find(|f| f.0 == id).map(|f| f.1).unwrap_or(0x0D);
            (id, flags, crate::cfdefs::formula_to_file(d.formula.trim(), d.resource, &doc.custom_defs))
        })
        .collect();
    let formulas_changed = {
        let mut a = formulas_have.clone();
        let mut b = formulas_want.clone();
        a.sort();
        b.sort();
        a != b
    };
    let any_defs = doc.custom_defs.iter().any(|d| !d.alias.trim().is_empty() || !d.formula.trim().is_empty() || !d.values.is_empty());
    let save_guid = gen.guid();
    let props_new = rewrite_props(&props_raw, |key, val| {
        if let Some((rows, vsize)) = sprint_counts {
            if key == crate::sprintio::P_SPRINT_ROWS && val.len() == 4 {
                val.copy_from_slice(&rows.to_le_bytes());
            }
            if key == crate::sprintio::P_SPRINT_VSIZE && val.len() == 4 {
                val.copy_from_slice(&vsize.to_le_bytes());
            }
        }
        if let Some((rows, vsize)) = oc_counts {
            if key == 0x0100_0008 && val.len() == 4 {
                val.copy_from_slice(&rows.to_le_bytes());
            }
            if key == 0x0001_0008 && val.len() == 4 {
                val.copy_from_slice(&vsize.to_le_bytes());
            }
        }
        // lookup tables: their code masks (one level of characters, numbers ... and ".")
        if key == crate::cfdefs::P_LOOKUP_MASKS && (!masks_add.is_empty() || !tables_gone.is_empty()) {
            *val = crate::cfdefs::rewrite_masks(val, &masks_add, &tables_gone);
        }
        if key == crate::cfdefs::P_LOOKUP_FIELDS && !lookups_gone.is_empty() {
            *val = crate::cfdefs::without_lookup_fields(val, &lookups_gone);
        }
        if key == crate::cfdefs::P_FORMULAS && formulas_changed && !skip("cfdefs") {
            *val = crate::cfdefs::build_formulas(&formulas_want);
        }
        if key == crate::cfdefs::P_HAS_DEFS && any_defs && val.len() >= 2 && !skip("cfdefs") {
            val[..2].copy_from_slice(&1u16.to_le_bytes());
        }
        if let Some(n) = counts.get(&key) {
            if val.len() == 4 {
                val.copy_from_slice(&n.to_le_bytes());
            }
        }
        if key == crate::mpp::P_START && val.len() >= 4 {
            val[..4].copy_from_slice(&ts(Some(pstart)));
        }
        if key == P_FINISH && val.len() >= 4 {
            val[..4].copy_from_slice(&ts(Some(finish)));
        }
        // a new save GUID, as MPP gives the file at every save (links from other projects
        // name the file by it; files made from the template would otherwise share one)
        if key == crate::extlinks::P_PROJECT_GUID && val.len() == 16 && !skip("guid") {
            val.copy_from_slice(&save_guid);
        }
        if key == crate::mpp::P_TITLE && utf16(val) != doc.title {
            *val = utf16z(&doc.title);
        }
        // Project > WBS > Define Code (the mask and its prefix)
        if key == 0x0240_138B {
            let mut m = Vec::new();
            match &doc.wbs_mask {
                Some(w) if !w.levels.is_empty() => {
                    m.extend_from_slice(&(w.levels.len() as u32).to_le_bytes());
                    for l in &w.levels {
                        m.extend_from_slice(&(l.seq as u16).to_le_bytes());
                        m.extend_from_slice(&(l.len as u16).to_le_bytes());
                        m.extend_from_slice(&(l.sep.encode_utf16().next().unwrap_or(46)).to_le_bytes());
                        m.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
                    }
                    let flags = w.generate as u32 | (w.verify_unique as u32) << 1 | 4;
                    m.extend_from_slice(&flags.to_le_bytes());
                    m.extend_from_slice(&utf16z(&w.prefix));
                }
                _ => m.extend_from_slice(&[0, 0, 0, 0]),
            }
            *val = m;
        }
        if key == 0x0240_13BE {
            let mut m = utf16z(doc.wbs_mask.as_ref().map(|w| w.prefix.as_str()).unwrap_or(""));
            m.extend_from_slice(&[0, 0]);
            *val = m;
        }
        // File > Options > Schedule
        let put16 = |val: &mut Vec<u8>, x: u16| {
            if val.len() >= 2 {
                val[..2].copy_from_slice(&x.to_le_bytes());
            }
        };
        if key == crate::mpp::P_MIN_PER_DAY && val.len() >= 4 {
            val[..4].copy_from_slice(&(mpd as u32).to_le_bytes());
        }
        if key == crate::mpp::P_NEW_TASKS_MANUAL {
            put16(val, (!doc.new_tasks_auto) as u16);
        }
        if key == crate::mpp::P_AUTOLINK {
            put16(val, doc.options.autolink as u16);
        }
        if key == crate::mpp::P_SPLIT_IN_PROGRESS {
            put16(val, doc.options.split_in_progress as u16);
        }
        if key == crate::mpp::P_DEFAULT_TASK_TYPE {
            put16(
                val,
                match doc.options.default_task_type {
                    TaskType::FixedDuration => 1,
                    TaskType::FixedWork => 2,
                    _ => 0,
                },
            );
        }
    });
    // links to other files (extlinks.rs): the projects of the external tasks (UIDs by file, in the
    // project's order), the resource pool; links of other kinds stay as they are
    let links_have = props.get(&crate::extlinks::P_LINKS).map(|b| crate::extlinks::parse_links(b)).unwrap_or_default();
    let mut links_want: Vec<crate::extlinks::FileLink> = Vec::new();
    {
        let mut by_path: Vec<(String, Vec<u32>)> = Vec::new();
        for (t, &u) in doc.tasks.iter().zip(&fuid) {
            if let Some(x) = &t.external {
                match by_path.iter_mut().find(|e| e.0.eq_ignore_ascii_case(&x.path)) {
                    Some(e) => e.1.push(u),
                    None => by_path.push((x.path.clone(), vec![u])),
                }
            }
        }
        for (path, uids) in by_path {
            let old = links_have.iter().find(|l| l.kind == crate::extlinks::KIND_TASKS && l.path().eq_ignore_ascii_case(&path));
            let mut l = match old {
                Some(o) => o.clone(),
                None => crate::extlinks::FileLink::new(crate::extlinks::project_guid(&path).unwrap_or_else(|| gen.guid()), crate::extlinks::KIND_TASKS, &path, this_path),
            };
            l.uids = uids;
            links_want.push(l);
        }
    }
    let n_task_links = links_want.len() as u32;
    if let Some(rp) = &doc.resource_pool {
        let old = links_have.iter().find(|l| l.kind == crate::extlinks::KIND_POOL);
        links_want.push(match old {
            Some(o) if o.path().eq_ignore_ascii_case(&rp.path) => o.clone(),
            Some(_) | None => crate::extlinks::FileLink::new(crate::extlinks::project_guid(&rp.path).unwrap_or_else(|| gen.guid()), crate::extlinks::KIND_POOL, &rp.path, this_path),
        });
    }
    links_want.extend(links_have.iter().filter(|l| l.kind != crate::extlinks::KIND_TASKS && l.kind != crate::extlinks::KIND_POOL).cloned());
    let links_bytes = crate::extlinks::build_links(&links_want);
    let links_changed = !skip("links") && props.get(&crate::extlinks::P_LINKS).map(|b| *b != links_bytes).unwrap_or(!links_want.is_empty());
    let props_new = if links_changed {
        rewrite_props(&props_new, |key, val| {
            if key == crate::extlinks::P_LINKS {
                *val = links_bytes.clone();
            }
            if key == crate::extlinks::P_EXT_PROJECTS && val.len() >= 4 {
                val[..4].copy_from_slice(&n_task_links.to_le_bytes());
            }
        })
    } else {
        props_new
    };
    // Resource > Resource Pool (extlinks.rs): the pool's moniker, its link and the sharing mode
    let pool_have = props.get(&crate::extlinks::P_POOL_MONIKER).and_then(|m| crate::extlinks::moniker_path(m)).filter(|_| props.get(&crate::extlinks::P_POOL_MODE).and_then(|d| u16_at(d, 0)).map(|m| m == 1 || m == 2).unwrap_or(false));
    let pool_mode_have = props.get(&crate::extlinks::P_POOL_MODE).and_then(|d| u16_at(d, 0)).unwrap_or(3);
    let pool_mode_want: u16 = match &doc.resource_pool {
        Some(rp) if rp.sharer_first => 1,
        Some(_) => 2,
        None => 3,
    };
    let props_new = if (pool_have.as_deref() != doc.resource_pool.as_ref().map(|r| r.path.as_str()) || pool_mode_have != pool_mode_want) && !skip("pool") {
        let on = doc.resource_pool.is_some();
        let p1 = rewrite_props(&props_new, |key, val| {
            let put = |val: &mut Vec<u8>, x: u32| {
                let n = val.len().min(4);
                val[..n].copy_from_slice(&x.to_le_bytes()[..n]);
            };
            match key {
                crate::extlinks::P_POOL_MODE => put(val, pool_mode_want as u32),
                crate::extlinks::P_POOL_ON | crate::extlinks::P_POOL_COUNT => put(val, on as u32),
                _ => {}
            }
        });
        props_with(&p1, crate::extlinks::P_POOL_MONIKER, doc.resource_pool.as_ref().map(|rp| crate::extlinks::moniker(&rp.path, 0)))
    } else {
        props_new
    };
    // Format > Drawing: the Gantt chart's drawings in an item of Neramit Planner's own (MPP keeps
    // drawings only in a visible window, so its own storage could not be seen; MPP leaves
    // items it does not know alone)
    // sprints in a file without sprint tables (from an MPP before them): Neramit Planner's own item
    let props_new = if !sprints_ok && !skip("sprints") {
        let tasks: Vec<(u32, u32)> = doc.tasks.iter().zip(&fuid).filter_map(|(t, &u)| t.sprint.map(|s| (u, s))).collect();
        let next: Vec<u32> = doc.tasks.iter().zip(&fuid).filter(|(t, _)| t.board_status == 1).map(|(_, &u)| u).collect();
        let want = (!doc.sprints.is_empty() || !next.is_empty()).then(|| serde_json::to_vec(&serde_json::json!({ "sprints": doc.sprints, "tasks": tasks, "next_up": next })).unwrap_or_default());
        if want.is_some() || props.contains_key(&crate::mpp::P_NERAMIT_SPRINTS) {
            props_with(&props_new, crate::mpp::P_NERAMIT_SPRINTS, want)
        } else {
            props_new
        }
    } else {
        props_new
    };
    // Project > Payment Schedule: Neramit Planner's own item, its tasks by file UID
    let payments = (!doc.payments.is_empty()).then(|| {
        let mut plan = doc.payments.clone();
        for it in plan.items.iter_mut() {
            it.task_uid = it.task_uid.and_then(|u| uid_to_fuid.get(&u).copied());
        }
        serde_json::to_vec(&plan).unwrap_or_default()
    });
    let props_new = if payments.is_some() || props.contains_key(&crate::mpp::P_NERAMIT_PAYMENTS) {
        props_with(&props_new, crate::mpp::P_NERAMIT_PAYMENTS, payments)
    } else {
        props_new
    };
    let drawings = if doc.drawings.is_empty() { None } else { serde_json::to_vec(&doc.drawings).ok() };
    let props_new = if drawings.is_some() || props.contains_key(&crate::mpp::P_NERAMIT_DRAWINGS) {
        props_with(&props_new, crate::mpp::P_NERAMIT_DRAWINGS, drawings)
    } else {
        props_new
    };
    replace.insert(cfb.entry_index(&[rt, "Props"]).ok_or("The file has no Props stream.")?, props_new);
    // tests: NERAMIT_MPPT_ONLY="TBkndTask,Props,..." replaces only those streams' storages
    #[cfg(test)]
    if let Ok(only) = std::env::var("NERAMIT_MPPT_ONLY") {
        let keep: Vec<usize> = only.split(',').flat_map(|t| {
            if t == "Props" {
                vec![cfb.entry_index(&[rt, "Props"])]
            } else {
                ["FixedMeta", "FixedData", "Fixed2Meta", "Fixed2Data", "VarMeta", "Var2Data"].iter().map(|s| cfb.entry_index(&[rt, t, s])).collect()
            }
        }).flatten().collect();
        replace.retain(|k, _| keep.contains(k));
    }
    crate::mppw::rewrite_cfb(orig, &replace)
}

/// A project made in Neramit Planner (no .mpp of its own) as a new .mpp: written into a small template
/// project (tasks, a link, work / material / cost resources and an assignment, for
/// their record layouts), whose own items all go.
pub fn save_new_mpp(doc: &Project) -> R<Vec<u8>> {
    const TEMPLATE: &[u8] = include_bytes!("../assets/mpp-template.mpp");
    let mut d = doc.clone();
    for t in d.tasks.iter_mut() {
        t.src_uid = None;
    }
    for r in d.resources.iter_mut() {
        r.src_uid = None;
    }
    save_mpp(TEMPLATE, &d)
}

/// MPP property: the project's finish date.
const P_FINISH: u32 = 0x0240_0003;

/// The kinds of task row new rows are copied from.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum RowKind {
    Plain,
    Summary,
    Milestone,
    Manual,
    /// a manual task with a Start / Finish / Duration shown as text (nothing entered)
    Texts,
    /// a task of another project
    External,
}

impl RowKind {
    fn of(t: &Task) -> RowKind {
        if t.external.is_some() {
            RowKind::External
        } else if t.is_summary {
            RowKind::Summary
        } else if t.manual && t.manual_text.iter().any(|x| x.is_some()) {
            RowKind::Texts
        } else if t.manual {
            RowKind::Manual
        } else if t.milestone {
            RowKind::Milestone
        } else {
            RowKind::Plain
        }
    }
}

/// The plainest task of the file of a kind (not started or constrained, with the fewest var
/// items): new tasks are copies of it. A plain one: any task when the file has no plain one.
fn task_template(tt: &Tbl, tmap: &FieldMap, v: &crate::mpp::Version, by_id: &[usize], named: &HashSet<u32>, kind: RowKind) -> Option<usize> {
    let level = |k: usize| tt.rows[k].get_u16(tmap, T_OUTLINE_LEVEL).unwrap_or(1);
    let pos: HashMap<usize, usize> = by_id.iter().enumerate().map(|(p, &k)| (k, p)).collect();
    let is_summary_row = |k: usize| -> bool { pos.get(&k).and_then(|&p| by_id.get(p + 1)).map(|&n| level(n) > level(k)).unwrap_or(false) };
    let fits = |k: usize| -> bool {
        let r = &tt.rows[k];
        let (mb, mbit) = v.milestone;
        let milestone = r.meta.get(mb).map(|x| (x >> mbit) & 1 == 1).unwrap_or(false);
        let manual = v.manual.map(|(b, bit)| r.meta2.get(b).map(|x| (x >> bit) & 1 == 1).unwrap_or(false)).unwrap_or(false);
        let started = r.get_u32(tmap, T_ACTUAL_START).map(|x| x != u32::MAX).unwrap_or(false);
        let ctype = r.get_u16(tmap, T_CTYPE).unwrap_or(0);
        let texts = [T_START_TEXT, T_FINISH_TEXT, T_DURATION_TEXT].iter().any(|ft| r.var(*ft).is_some());
        let summary = is_summary_row(k);
        if (started || ctype != 0) && kind != RowKind::External {
            return false;
        }
        match kind {
            RowKind::Plain => !milestone && !manual && !summary,
            RowKind::Summary => summary && !manual,
            RowKind::Milestone => milestone && !manual && !summary,
            RowKind::Manual => manual && !milestone && !summary && !texts,
            RowKind::Texts => manual && !milestone && !summary && texts,
            RowKind::External => r.var(crate::extlinks::T_EXT_UID).is_some(),
        }
    };
    let candidates: Vec<usize> = by_id.iter().copied().filter(|&k| tt.rows[k].uid.map(|u| named.contains(&u)).unwrap_or(false)).collect();
    let best = candidates.iter().copied().filter(|&k| fits(k)).min_by_key(|&k| tt.rows[k].vars.len());
    if kind != RowKind::Plain {
        return best;
    }
    best.or_else(|| candidates.iter().copied().find(|&k| !is_summary_row(k))).or_else(|| candidates.first().copied())
}

/// An external task's row as stored, changed only where its file's dates came in
/// (Project > Links Between Projects: Accept).
fn write_external_dates(r: &mut Row, tmap: &FieldMap, t: &Task, mpd: i64) {
    let (st, ft) = (t.start_dt, t.finish_dt);
    if r.get_u32(tmap, T_START) == Some(crate::mpp::timestamp(st)) && r.get_u32(tmap, T_FINISH) == Some(crate::mpp::timestamp(ft)) && r.get_u16(tmap, T_PERCENT) == Some(t.percent_complete as u16) {
        return;
    }
    for (f, x) in [(T_START, st), (T_FINISH, ft), (T_EARLY_START, st), (T_EARLY_FINISH, ft), (T_LATE_START, st), (T_LATE_FINISH, ft)] {
        r.put(tmap, f, &ts(x));
    }
    let d = t.dur_minutes(mpd);
    let a = t.actual_minutes(mpd).min(d);
    r.put(tmap, T_DURATION, &((d * 10) as u32).to_le_bytes());
    r.put(tmap, T_REMAINING_DURATION, &(((d - a).max(0) * 10) as u32).to_le_bytes());
    r.put(tmap, T_PERCENT, &(t.percent_complete as u16).to_le_bytes());
}

/// A new row made an external task (var 0x0B4000F2 its UID there with
/// 0x0080 in the high word, 0x0B4000FF its ID, 0x0B400188 / 189 -1; FixedMeta bits 15 0x40,
/// 17 0x01, 18 0x20, 19 0xE0, 28 0x60 set and 17 0x04, 18 0x40 clear; field 0x0B4000C9 gains
/// 0x00044000. Without byte 28 bit 0x40 MPP shows the task as ID 0 of its project).
fn make_external(r: &mut Row, tmap: &FieldMap, x: &crate::model::ExternalTask) {
    r.set_var(crate::extlinks::T_EXT_UID, ((x.uid & 0xFFFF) | 0x0080_0000).to_le_bytes().to_vec());
    r.set_var(crate::extlinks::T_EXT_ID, x.id.to_le_bytes().to_vec());
    r.set_var(0x0B40_0188, u32::MAX.to_le_bytes().to_vec());
    r.set_var(0x0B40_0189, u32::MAX.to_le_bytes().to_vec());
    if r.var(0x0B40_00F4).is_none() {
        r.set_var(0x0B40_00F4, 1u32.to_le_bytes().to_vec());
    }
    r.flag(15, 0x40, true);
    r.flag(17, 0x01, true);
    r.flag(17, 0x04, false);
    r.flag(18, 0x40, false);
    r.flag(18, 0x20, true);
    r.flag(19, 0xE0, true);
    r.flag(28, 0x60, true);
    if let Some(c) = r.get_u32(tmap, 0x0B40_00C9) {
        r.put(tmap, 0x0B40_00C9, &(c | 0x0004_4000).to_le_bytes());
    }
}

/// The rows of lookup table `table` (TBkndOutlCode) for `values`: a value the table has keeps its
/// row (MPP may refer to it), others are added, those gone deleted. Returns the GUIDs of the
/// table's values.
fn set_lookup_rows(oc: &mut Tbl, gen: &mut GuidGen, table: [u8; 16], kind: crate::custom::CfKind, values: &[(String, String)], du: crate::util::DurUnits) -> Vec<[u8; 16]> {
    let Some((_, _, code, mbits)) = crate::cfdefs::type_codes(kind) else { return Vec::new() };
    let of_table = |r: &Row| r.used() && r.data2.get(18..34) == Some(&table[..]);
    let mut taken: HashSet<usize> = HashSet::new();
    let mut next = oc.max_uid() + 1;
    let mut guids = Vec::new();
    let mut new_rows = Vec::new();
    for (k, (value, desc)) in values.iter().enumerate() {
        let order = ((k + 1) as f64).to_le_bytes().to_vec();
        let found = (0..oc.rows.len()).find(|&i| {
            let r = &oc.rows[i];
            !taken.contains(&i) && of_table(r) && r.var(crate::cfdefs::V_VALUE).map(|v| crate::cfdefs::decode_value(u16_at(&r.data2, 16).unwrap_or(0x15), v)).as_deref() == Some(value.as_str())
        });
        match found {
            Some(i) => {
                taken.insert(i);
                let r = &mut oc.rows[i];
                r.set_var(crate::cfdefs::V_ORDER, order);
                r.set_var(crate::cfdefs::V_DESCRIPTION, utf16z(desc));
                guids.push(r.data2.get(..16).and_then(|g| g.try_into().ok()).unwrap_or([0; 16]));
            }
            None => {
                let uid = next;
                next += 1;
                let mut meta = vec![0u8; 10];
                meta[8..10].copy_from_slice(&mbits.to_le_bytes());
                let mut data = Vec::with_capacity(14);
                data.extend_from_slice(&uid.to_le_bytes());
                data.extend_from_slice(&uid.to_le_bytes());
                data.extend_from_slice(&1u32.to_le_bytes());
                data.extend_from_slice(&[0, 0]);
                let mut meta2 = vec![0u8; 10];
                meta2[8] = 0x1f;
                let g = gen.guid();
                let mut data2 = g.to_vec();
                data2.extend_from_slice(&code.to_le_bytes());
                data2.extend_from_slice(&table);
                let vars = vec![(crate::cfdefs::V_ORDER, order), (crate::cfdefs::V_DESCRIPTION, utf16z(desc)), (crate::cfdefs::V_VALUE, crate::cfdefs::encode_value(kind, value, du))];
                new_rows.push(Row { meta, data, meta2, data2, uid: Some(uid), vars });
                guids.push(g);
            }
        }
    }
    let mut i = 0;
    oc.rows.retain(|r| {
        let keep = !of_table(r) || taken.contains(&i);
        i += 1;
        keep
    });
    oc.rows.extend(new_rows);
    guids
}

/// Props with item `key` set to `value` (at the end) or removed (None); the count and sizes follow.
fn props_with(raw: &[u8], key: u32, value: Option<Vec<u8>>) -> Vec<u8> {
    let count = u32_at(raw, 12).unwrap_or(0);
    let mut o = 16;
    let mut out = raw[..16.min(raw.len())].to_vec();
    let mut n = 0u32;
    for _ in 0..count {
        let (Some(size), Some(k)) = (u32_at(raw, o), u32_at(raw, o + 4)) else { break };
        let en = o + 12 + size as usize;
        if en > raw.len() {
            break;
        }
        if k != key {
            out.extend_from_slice(&raw[o..en]);
            if size % 2 == 1 {
                out.push(0);
            }
            n += 1;
        }
        o = en.div_ceil(2) * 2;
    }
    if let Some(v) = value {
        out.extend_from_slice(&(v.len() as u32).to_le_bytes());
        out.extend_from_slice(&key.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&v);
        if v.len() % 2 == 1 {
            out.push(0);
        }
        n += 1;
    }
    out.extend_from_slice(raw.get(o..).unwrap_or(&[]));
    if out.len() >= 16 {
        put_u32(&mut out, 12, n);
        let s = (out.len() - 4) as u32;
        put_u32(&mut out, 0, s);
        put_u32(&mut out, 4, s);
    }
    out
}

/// Props items rewritten by `f(key, value)`; the header keeps the stream's size (less 4) twice.
fn rewrite_props(raw: &[u8], mut f: impl FnMut(u32, &mut Vec<u8>)) -> Vec<u8> {
    let count = u32_at(raw, 12).unwrap_or(0);
    let mut o = 16;
    let mut out = raw[..16.min(raw.len())].to_vec();
    for _ in 0..count {
        let (Some(size), Some(key), Some(attr)) = (u32_at(raw, o), u32_at(raw, o + 4), u32_at(raw, o + 8)) else { break };
        let st = o + 12;
        let en = st + size as usize;
        if en > raw.len() {
            break;
        }
        let mut val = raw[st..en].to_vec();
        f(key, &mut val);
        out.extend_from_slice(&(val.len() as u32).to_le_bytes());
        out.extend_from_slice(&key.to_le_bytes());
        out.extend_from_slice(&attr.to_le_bytes());
        out.extend_from_slice(&val);
        // (an item of odd length is padded to an even one, e.g. the links to other projects)
        if val.len() % 2 == 1 {
            out.push(0);
        }
        o = en.div_ceil(2) * 2;
    }
    out.extend_from_slice(&raw[o.min(raw.len())..]);
    if out.len() >= 8 {
        let n = (out.len() - 4) as u32;
        put_u32(&mut out, 0, n);
        put_u32(&mut out, 4, n);
    }
    out
}

/// Stop and resume of a started, unfinished task: the end of its actual work and the next
/// working instant (MPP: 50 % of a five-day task from Wed 08:00 stops Fri 12:00, resumes 13:00).
fn stop_resume(t: &Task, cal: &Calendar, mpd: i64) -> (Option<NaiveDateTime>, Option<NaiveDateTime>) {
    let Some(st) = t.actual_start else { return (None, None) };
    if t.percent_complete >= 100 {
        let f = t.actual_finish.or(t.finish_dt);
        return (f, f);
    }
    if t.is_summary {
        return (Some(st), Some(st));
    }
    let a = t.actual_minutes(mpd);
    let stop = cal.add_work_minutes(st, a);
    (Some(stop), Some(cal.next_work_instant(stop)))
}

/// The task's values in its record: everything Neramit Planner keeps, and the flags and scheduling
/// results MPP stores with them.
#[allow(clippy::too_many_arguments)]
fn write_task(r: &mut Row, tmap: &FieldMap, v: &crate::mpp::Version, t: &Task, cal: &Calendar, mpd: i64, is_new: bool, pstart: NaiveDateTime, eb: EBits, wbs_own: Option<&str>) {
    let (st, ft) = (t.start_dt, t.finish_dt);
    // what the file had (fields Neramit Planner does not keep are left alone while these hold)
    let old_pct = if is_new { None } else { r.get_u16(tmap, T_PERCENT) };
    let old_type = if is_new { None } else { r.get_u16(tmap, T_TYPE) };
    let was_manual = v.manual.map(|(b, bit)| r.meta2.get(b).map(|x| (x >> bit) & 1 == 1).unwrap_or(false)).unwrap_or(false);
    r.put(tmap, T_OUTLINE_LEVEL, &(t.outline_level as u16).to_le_bytes());
    r.put(tmap, T_START, &ts(st));
    r.put(tmap, T_FINISH, &ts(ft));
    if t.manual {
        // a date shown as text (nothing entered) keeps what MPP recorded
        if t.manual_text[0].is_none() || is_new {
            r.put(tmap, T_MANUAL_START, &ts(st));
        }
        if t.manual_text[1].is_none() || is_new {
            r.put(tmap, T_MANUAL_FINISH, &ts(ft));
        }
    }
    // Start / Finish / Duration texts of a manual task (MPP keeps "" for nothing entered;
    // without them a new row takes today's date; Fixed2Meta byte 88 has a bit for each, set in
    // sync_var_bits)
    for (k, ft) in [T_START_TEXT, T_FINISH_TEXT, T_DURATION_TEXT].into_iter().enumerate() {
        match t.manual_text[k].as_deref().filter(|_| t.manual) {
            Some(s) => {
                if r.var(ft).map(utf16).as_deref() != Some(s) {
                    r.set_var(ft, utf16z(s));
                }
            }
            None => r.remove_var(ft),
        }
    }
    if t.manual {
    } else if is_new || was_manual {
        // an automatically scheduled task has no manual dates (a new task with them made
        // MPP schedule it from them); a task the file has keeps what MPP recorded
        r.put(tmap, T_MANUAL_START, &ts(None));
        r.put(tmap, T_MANUAL_FINISH, &ts(None));
    }
    r.put(tmap, T_EARLY_START, &ts(st));
    r.put(tmap, T_EARLY_FINISH, &ts(ft));
    // late dates from the total slack
    let shift = |x: NaiveDateTime, start: bool| -> NaiveDateTime {
        let k = t.slack_min;
        if k > 0 {
            let y = cal.add_work_minutes(x, k);
            if start {
                cal.next_work_instant(y)
            } else {
                y
            }
        } else if k < 0 {
            cal.sub_work_minutes(x, -k)
        } else {
            x
        }
    };
    r.put(tmap, T_LATE_START, &ts(st.map(|x| shift(x, true))));
    r.put(tmap, T_LATE_FINISH, &ts(ft.map(|x| shift(x, false))));
    // slack in tenths of a minute
    let tenths = |m: i64| ((m * 10) as i32).to_le_bytes();
    r.put(tmap, T_FREE_SLACK, &tenths(t.free_slack_min));
    r.put(tmap, T_START_SLACK, &tenths(t.slack_min));
    r.put(tmap, T_FINISH_SLACK, &tenths(t.slack_min));
    // duration and its format word (units; 0x20 estimated)
    let d = t.dur_minutes(mpd);
    let a = t.actual_minutes(mpd).min(d);
    let old_dur = r.get_u32(tmap, T_DURATION);
    let old_fmt = tmap.get(&T_DURATION).and_then(|&(blob, o)| u16_at(if blob == 0 { &r.data } else { &r.data2 }, o + v.est_delta));
    r.put(tmap, T_DURATION, &((d * 10) as u32).to_le_bytes());
    r.put(tmap, T_ACTUAL_DURATION, &((a * 10) as u32).to_le_bytes());
    r.put(tmap, T_REMAINING_DURATION, &(((d - a).max(0) * 10) as u32).to_le_bytes());
    if let (Some(&(blob, o)), Some(old)) = (tmap.get(&T_DURATION), old_fmt) {
        let same_duration = !is_new && old_dur == Some((d * 10) as u32);
        let buf = if blob == 0 { &mut r.data } else { &mut r.data2 };
        let at = o + v.est_delta;
        // a summary keeps the units MPP gave it ("null" / days); its estimate follows the
        // subtasks'. A changed task gets its own units.
        // (a summary Neramit Planner made gets the standard 21, "none")
        let units = if is_new {
            // a new row: the format the task came with (the row it was copied from has its own),
            // else the standard for a new task (21 "none" for a summary)
            t.summary_fmt.map(|f| f as u16 & 0x1F).filter(|f| *f > 0).unwrap_or(if t.is_summary { 21 } else { t.dur_unit.code(t.dur_elapsed) as u16 & 0x1F })
        } else if t.is_summary || same_duration || (t.manual && old & 0x1F == 21) {
            // (a manual task whose duration was never entered keeps the standard "none")
            old & 0x1F
        } else {
            t.dur_unit.code(t.dur_elapsed) as u16 & 0x1F
        };
        let code = (old & !0x3F) | units | if t.estimated { 0x20 } else { 0 };
        if at + 2 <= buf.len() {
            put_u16(buf, at, code);
        }
    }
    if let Some(f2) = r.get_u16(tmap, T_DUR_FORMAT2) {
        r.put(tmap, T_DUR_FORMAT2, &(if t.estimated { f2 | 0x20 } else { f2 & !0x20 }).to_le_bytes());
    }
    // progress
    r.put(tmap, T_PERCENT, &(t.percent_complete as u16).to_le_bytes());
    // % work complete: the standard while % complete is unchanged (a task 2 % complete
    // with 0 % of its work done grew by an hour and a quarter when both were 2 %)
    if old_pct != Some(t.percent_complete as u16) {
        r.put(tmap, T_PCT_WORK, &(t.percent_complete as u16).to_le_bytes());
    }
    r.put(tmap, T_ACTUAL_START, &ts(t.actual_start));
    r.put(tmap, T_ACTUAL_FINISH, &ts(t.actual_finish));
    let (stop, resume) = stop_resume(t, cal, mpd);
    r.put(tmap, T_STOP, &ts(stop));
    r.put(tmap, T_RESUME, &ts(resume));
    // constraint, priority, deadline, type
    r.put(tmap, T_CTYPE, &(t.constraint_type.code() as u16).to_le_bytes());
    r.put(tmap, T_CDATE, &ts(if t.constraint_type.needs_date() { t.constraint_instant(cal) } else { None }));
    r.put(tmap, T_PRIORITY, &t.priority.to_le_bytes());
    r.put(tmap, T_DEADLINE, &ts(t.deadline.map(|d| cal.day_end(d))));
    let ttype = if t.is_summary { 1 } else { t.task_type.code() as u16 };
    r.put(tmap, T_TYPE, &ttype.to_le_bytes());
    // work and cost (the standard Work is actual + remaining)
    let actual_work = (t.work_min as f64 * t.percent_complete as f64 / 100.0).round();
    r.put(tmap, T_WORK, &(t.work_min as f64 * 1000.0).to_le_bytes());
    r.put(tmap, T_ACTUAL_WORK, &(actual_work * 1000.0).to_le_bytes());
    r.put(tmap, T_REMAINING_WORK, &((t.work_min as f64 - actual_work).max(0.0) * 1000.0).to_le_bytes());
    r.put(tmap, T_COST, &(t.cost * 100.0).round().to_le_bytes());
    r.put(tmap, T_ACTUAL_COST, &(t.actual_cost * 100.0).round().to_le_bytes());
    r.put(tmap, T_REMAINING_COST, &((t.cost - t.actual_cost).max(0.0) * 100.0).round().to_le_bytes());
    r.put(tmap, T_FIXED_COST, &(t.fixed_cost * 100.0).round().to_le_bytes());
    r.put(tmap, T_FIXED_COST_ACCRUAL, &(t.fixed_cost_accrual.code() as u16).to_le_bytes());
    // what drives the start, the date the links ask for
    let link = t.link_req;
    let (driver, ddate, ldate): (u32, Option<NaiveDateTime>, Option<NaiveDateTime>) = if t.is_summary {
        (0x40, None, None)
    } else if let Some(a0) = t.actual_start {
        if t.percent_complete >= 100 {
            (0x01, None, None)
        } else {
            (0x01, Some(link.map(|l| l.0).unwrap_or(pstart)), Some(a0))
        }
    } else if t.manual {
        (0x10, st, link.map(|l| l.0))
    } else {
        let start = st.unwrap_or(pstart);
        let by_link = link.filter(|(at, eod)| *at == start || (*eod && cal.next_work_instant(*at) == start));
        let by_constraint = matches!(t.constraint_type, ConstraintType::Snet | ConstraintType::Mso | ConstraintType::Fnet | ConstraintType::Mfo | ConstraintType::Snlt | ConstraintType::Fnlt)
            && t.constraint_instant(cal).map(|c| c == start || cal.next_work_instant(c) == start).unwrap_or(false);
        match by_link {
            Some((at, _)) if !by_constraint => (0x08 | if at != start { 0x20 } else { 0 }, Some(at), Some(at)),
            _ if by_constraint => (0x04, Some(start), link.map(|l| l.0)),
            _ => (0x10, Some(start), link.map(|l| l.0)),
        }
    };
    r.put(tmap, T_START_DRIVER, &driver.to_le_bytes());
    r.put(tmap, T_DRIVER_DATE, &ts(ddate));
    r.put(tmap, T_LINK_DATE, &ts(ldate));
    // flags in the metadata (one field toggled at a time in MPP)
    let (mb, mbit) = v.milestone;
    r.flag(mb, 1 << mbit, t.milestone);
    // critical: not for inactive or finished tasks
    r.flag(9, 0x40, t.critical && t.active && t.percent_complete < 100);
    // FixedMeta byte 12: 0x08 summary (on every summary, also those of older files that
    // keep no start-children list), 0x04 rollup, 0x02 marked, 0x80 hide bar
    r.flag(12, 0x08, t.is_summary);
    r.flag(12, 0x04, t.rollup);
    r.flag(12, 0x02, t.marked);
    r.flag(12, 0x80, t.hide_bar);
    r.flag(13, 0x08, t.effort_driven || t.task_type == TaskType::FixedWork);
    // the task type's bits, when the type changed (byte 16 bit 0x04 is not only the type's)
    let ttype_now = if t.is_summary { 1 } else { t.task_type.code() as u16 };
    if !t.is_summary && old_type != Some(ttype_now) {
        let fixed_duration = t.task_type == TaskType::FixedDuration;
        r.flag(11, 0x01, fixed_duration);
        r.flag(16, 0x04, !fixed_duration);
    }
    r.flag(15, 0x02, t.collapsed && t.is_summary);
    if let Some((b, bit)) = v.manual {
        r.flag2(b, 1 << bit, t.manual);
    }
    r.flag2(8, 0x40, t.active);
    // name and baseline in the var data
    if r.var(crate::mpp::T_NOTES).map(crate::mpp::rtf_to_text).unwrap_or_default().trim_end() != t.notes.trim_end() {
        if t.notes.trim().is_empty() {
            r.remove_var(crate::mpp::T_NOTES);
        } else {
            r.set_var(crate::mpp::T_NOTES, crate::mpp::text_to_rtf(&t.notes));
        }
    }
    // custom fields: written where they differ from what the file has
    write_custom(r, false, &t.custom);
    // WBS: a code the mask generated (0x0B40018D, without the prefix; FixedMeta byte 20 bit 0x80)
    // or one typed in (0x0B400010; bit 0x20). None: the outline number, kept by MPP itself.
    {
        let have = (r.var(0x0B40_0010).map(utf16), r.var(0x0B40_018D).map(utf16));
        let want = match wbs_own {
            Some(own) => (None, Some(own.to_string())),
            None if !t.wbs.is_empty() => (Some(t.wbs.clone()), None),
            None => (None, None),
        };
        if have != want {
            r.remove_var(0x0B40_0010);
            r.remove_var(0x0B40_018D);
            if let Some(w) = &want.0 {
                r.set_var(0x0B40_0010, utf16z(w));
            }
            if let Some(w) = &want.1 {
                r.set_var(0x0B40_018D, utf16z(w));
            }
            let (generated, typed) = (r.var(0x0B40_018D).is_some(), r.var(0x0B40_0010).is_some());
            r.flag(20, 0x80, generated);
            r.flag(20, 0x20, typed);
        }
    }
    if r.var(T_NAME).map(utf16) != Some(t.name.clone()) {
        r.set_var(T_NAME, utf16z(&t.name));
    }
    match t.baseline(0) {
        Some(b) if b.start.is_some() || r.var(T_BASELINE_START).is_some() => {
            // MPP keeps a baseline as six var items: work, cost, duration, start, finish and
            // the duration's format
            r.set_var(T_BASELINE_START, timestamp(b.start).to_le_bytes().to_vec());
            r.set_var(T_BASELINE_FINISH, timestamp(b.finish).to_le_bytes().to_vec());
            r.set_var(T_BASELINE_DURATION, ((b.dur_min.unwrap_or(0) * 10) as u32).to_le_bytes().to_vec());
            r.set_var(T_BASELINE_WORK, (b.work_min.unwrap_or(0) as f64 * 1000.0).to_le_bytes().to_vec());
            r.set_var(T_BASELINE_COST, (b.cost.unwrap_or(0.0) * 100.0).round().to_le_bytes().to_vec());
            if r.var(T_BASELINE_FORMAT).is_none() {
                let fmt: u16 = if t.is_summary { 0x15 } else { t.dur_unit.code(t.dur_elapsed) as u16 & 0x1F };
                r.set_var(T_BASELINE_FORMAT, fmt.to_le_bytes().to_vec());
            }
        }
        _ => {}
    }
    sync_var_bits(r, eb);
}

const T_BASELINE_WORK: u32 = 0x0B40_0001;
const T_START_TEXT: u32 = 0x0B40_0505;
const T_FINISH_TEXT: u32 = 0x0B40_0506;
const T_DURATION_TEXT: u32 = 0x0B40_0507;
const T_BASELINE_COST: u32 = 0x0B40_0006;
const T_BASELINE_FORMAT: u32 = 0x0B40_00B3;

/// FixedMeta bits that say a task has a var item (a bit set on exactly the rows that have
/// the item, on the real files): MPP looks there before it reads the var data.
/// A row's custom fields (tables in custom_tables.rs): each field whose value is not the
/// file's gets its var item (and a duration its format item) and its bits.
fn write_custom(r: &mut Row, resource: bool, values: &crate::custom::Values) {
    let have = crate::custom::read_mpp(resource, &|ft| r.var(ft).map(|v| v.to_vec()), &r.meta);
    for f in crate::custom::all() {
        let want = crate::custom::get(values, f);
        if crate::custom::get(&have, f) == want {
            continue;
        }
        let Some(s) = crate::custom_tables::spec(resource, f.kind, f.n) else { continue };
        let on = match want {
            Some(crate::custom::CfValue::Flag(b)) => *b,
            Some(_) => true,
            None => false,
        };
        if s.var != 0 {
            match want.and_then(crate::custom::mpp_bytes) {
                Some(b) => r.set_var(s.var, b),
                None => r.remove_var(s.var),
            }
        }
        if s.fmt_var != 0 {
            if on {
                // (days, the standard default unit for a custom duration)
                r.set_var(s.fmt_var, 7u16.to_le_bytes().to_vec());
            } else {
                r.remove_var(s.fmt_var);
            }
        }
        for &(b, m) in s.bits {
            if b >= 1000 {
                r.flag2((b - 1000) as usize, m, on);
            } else {
                r.flag(b as usize, m, on);
            }
        }
    }
}

/// Fixed2Meta byte 95 bits saying a task has its scheduling lists (E3 predecessors, E4 summary
/// start-children, E5 linked resources). Files from older MPP versions keep them one bit lower
/// (0x02/0x04/0x08 in the user's files, 0x04/0x08/0x10 in files the current MPP saves).
#[derive(Clone, Copy)]
struct EBits {
    e3: u8,
    e4: u8,
    e5: u8,
}

impl EBits {
    /// The layout the file's rows use (the newer one when nothing tells).
    fn of(tt: &Tbl) -> EBits {
        let score = |shift: u32| -> usize {
            tt.rows
                .iter()
                .filter(|r| r.used() && r.meta2.len() > 95 && r.var(T_NAME).is_some())
                .map(|r| {
                    let b = r.meta2[95];
                    [(T_PRED_LIST, 0), (T_START_KIDS, 1), (T_LINKED_RES, 2)]
                        .iter()
                        .filter(|(ft, k)| ((b >> (shift + k)) & 1 == 1) == r.var(*ft).is_some())
                        .count()
                })
                .sum()
        };
        let shift = if score(1) > score(2) { 1 } else { 2 };
        EBits { e3: 1 << shift, e4: 2 << shift, e5: 4 << shift }
    }
}

fn sync_var_bits(r: &mut Row, eb: EBits) {
    let has = |r: &Row, ft: u32| r.var(ft).is_some();
    let notes = has(r, 0x0B40_000F);
    r.flag(14, 0x10, notes);
    r.flag(44, 0x10, notes);
    let text1 = has(r, 0x0B40_0033);
    r.flag(21, 0x20, text1);
    let f4 = has(r, 0x0B40_00F4);
    r.flag(28, 0x20, f4);
    let progress = has(r, 0x0B40_015E);
    r.flag(44, 0x08, progress);
    let old_dur = has(r, 0x0B40_007C);
    r.flag(26, 0x80, old_dur);
    let baseline = [T_BASELINE_WORK, T_BASELINE_COST, T_BASELINE_DURATION, T_BASELINE_START, T_BASELINE_FINISH, T_BASELINE_FORMAT].iter().all(|ft| has(r, *ft));
    r.flag(21, 0x1F, baseline);
    r.flag(27, 0x10, baseline);
    r.flag2(95, eb.e3, has(r, T_PRED_LIST));
    r.flag2(95, eb.e4, has(r, T_START_KIDS));
    r.flag2(95, eb.e5, has(r, T_LINKED_RES));
    // Fixed2Meta byte 88: a manual task's Start / Finish / Duration text (the same three bits,
    // one lower in older files; a placeholder without them made MPP count its estimate in
    // its summary's)
    r.flag2(88, eb.e3, has(r, T_START_TEXT));
    r.flag2(88, eb.e4, has(r, T_FINISH_TEXT));
    r.flag2(88, eb.e5, has(r, T_DURATION_TEXT));
    // Fixed2Meta bytes 93-94 (16 bits): the sprint / Task Board items 0x0B400584-9 (0x0008
    // << k, one lower in older files)
    for k in 0..6u32 {
        let bit = ((eb.e3 as u32) << 1) << k;
        let on = has(r, crate::sprintio::T_BOARD_COLUMN + k);
        r.flag2(93, (bit & 0xFF) as u8, on && bit & 0xFF != 0);
        r.flag2(94, (bit >> 8) as u8, on && bit >> 8 != 0);
        if !on {
            r.flag2(93, (bit & 0xFF) as u8, false);
            r.flag2(94, (bit >> 8) as u8, false);
        }
    }
}

/// The project summary task (ID 0): the project's dates, duration, progress, work and cost.
fn write_project_task(r: &mut Row, tmap: &FieldMap, doc: &Project, cal: &Calendar, mpd: i64, pstart: NaiveDateTime) {
    let active: Vec<&Task> = doc.tasks.iter().filter(|t| t.active && t.external.is_none()).collect();
    let start = active.iter().filter_map(|t| t.start_dt).min().unwrap_or(pstart);
    let finish = active.iter().filter_map(|t| t.finish_dt).max().unwrap_or(start);
    let top: Vec<&Task> = active.iter().copied().filter(|t| t.outline_level == 1).collect();
    let work: i64 = top.iter().map(|t| t.work_min).sum();
    let cost: f64 = top.iter().map(|t| t.cost).sum();
    let actual_cost: f64 = top.iter().map(|t| t.actual_cost).sum();
    let leaves: Vec<&Task> = active.iter().copied().filter(|t| !t.is_summary).collect();
    let actual_work: f64 = leaves.iter().map(|t| (t.work_min as f64 * t.percent_complete as f64 / 100.0).round()).sum();
    let (sum_dur, sum_act): (i64, i64) = leaves.iter().map(|t| (t.dur_minutes(mpd), t.actual_minutes(mpd).min(t.dur_minutes(mpd)))).fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1));
    let pct = if sum_dur > 0 { ((sum_act as f64 / sum_dur as f64) * 100.0).floor() as u16 } else { 0 };
    let pct_work = if work > 0 { ((actual_work / work as f64) * 100.0).floor() as u16 } else { 0 };
    let first_actual = active.iter().filter_map(|t| t.actual_start).min();
    r.put(tmap, T_START, &ts(Some(start)));
    r.put(tmap, T_FINISH, &ts(Some(finish)));
    r.put(tmap, T_EARLY_START, &ts(Some(start)));
    r.put(tmap, T_EARLY_FINISH, &ts(Some(finish)));
    r.put(tmap, T_LATE_START, &ts(Some(start)));
    r.put(tmap, T_LATE_FINISH, &ts(Some(finish)));
    let d = cal.work_minutes(start, finish).max(0);
    let da = (d as f64 * pct as f64 / 100.0).round() as i64;
    r.put(tmap, T_DURATION, &((d * 10) as u32).to_le_bytes());
    r.put(tmap, T_ACTUAL_DURATION, &((da * 10) as u32).to_le_bytes());
    r.put(tmap, T_REMAINING_DURATION, &(((d - da).max(0) * 10) as u32).to_le_bytes());
    r.put(tmap, T_PERCENT, &pct.to_le_bytes());
    r.put(tmap, T_PCT_WORK, &pct_work.to_le_bytes());
    r.put(tmap, T_ACTUAL_START, &ts(first_actual));
    r.put(tmap, T_STOP, &ts(first_actual));
    r.put(tmap, T_RESUME, &ts(first_actual));
    r.put(tmap, T_WORK, &(work as f64 * 1000.0).to_le_bytes());
    r.put(tmap, T_ACTUAL_WORK, &(actual_work * 1000.0).to_le_bytes());
    r.put(tmap, T_REMAINING_WORK, &((work as f64 - actual_work).max(0.0) * 1000.0).to_le_bytes());
    r.put(tmap, T_COST, &(cost * 100.0).round().to_le_bytes());
    r.put(tmap, T_ACTUAL_COST, &(actual_cost * 100.0).round().to_le_bytes());
    r.put(tmap, T_REMAINING_COST, &((cost - actual_cost).max(0.0) * 100.0).round().to_le_bytes());
}

/// A resource's numbers and type. `work` and `actual_work` are in minutes (a material's in hours
/// of quantity); a new resource also gets the standard type fields.
fn write_resource(row: &mut Row, rmap: &FieldMap, r: &Resource, work: f64, actual_work: f64, is_new: bool) {
    let put_f = |row: &mut Row, ft: u32, val: f64| row.put(rmap, ft, &val.to_le_bytes());
    put_f(row, R_MAX_UNITS, r.max_units * 100.0);
    put_f(row, R_STD_RATE, r.std_rate);
    put_f(row, R_OVT_RATE, r.ovt_rate);
    put_f(row, R_COST_PER_USE, r.cost_per_use * 100.0);
    let aw = actual_work.round();
    put_f(row, R_WORK, work * 1000.0);
    put_f(row, R_REGULAR_WORK, work * 1000.0);
    put_f(row, R_ACTUAL_WORK, aw * 1000.0);
    put_f(row, R_REMAINING_WORK, (work - aw).max(0.0) * 1000.0);
    if is_new {
        // what MPP gives a resource it makes of that type (0x0C400046, 0x0C400110)
        row.put(rmap, 0x0C40_0046, &(if r.rtype == ResourceType::Material { 8u16 } else { 2 }).to_le_bytes());
        row.put(rmap, 0x0C40_0110, &(if r.rtype == ResourceType::Work { 0u16 } else { 1 }).to_le_bytes());
    }
    row.put(rmap, R_ACCRUE_AT, &(r.accrue_at.code() as u16).to_le_bytes());
    // type: material = FixedMeta byte 14 bit 0, cost = Fixed2Meta byte 8 bit 0x10; a work
    // resource also has FixedMeta byte 12 bits 0x14
    row.flag(14, 0x01, r.rtype == ResourceType::Material);
    row.flag(12, 0x14, r.rtype == ResourceType::Work);
    row.flag2(8, 0x10, r.rtype == ResourceType::Cost);
    if row.var(crate::mpp::R_NOTES).map(crate::mpp::rtf_to_text).unwrap_or_default().trim_end() != r.notes.trim_end() {
        if r.notes.trim().is_empty() {
            row.remove_var(crate::mpp::R_NOTES);
        } else {
            row.set_var(crate::mpp::R_NOTES, crate::mpp::text_to_rtf(&r.notes));
        }
    }
    if row.var(crate::mpp::R_EMAIL).map(utf16).unwrap_or_default() != r.email {
        if r.email.is_empty() {
            row.remove_var(crate::mpp::R_EMAIL);
        } else {
            row.set_var(crate::mpp::R_EMAIL, utf16z(&r.email));
        }
    }
    write_custom(row, true, &r.custom);
    if row.var(R_NAME).map(utf16) != Some(r.name.clone()) {
        row.set_var(R_NAME, utf16z(&r.name));
    }
    let text = |row: &mut Row, ft: u32, s: &str| {
        if s.is_empty() {
            row.remove_var(ft);
        } else if row.var(ft).map(utf16).as_deref() != Some(s) {
            row.set_var(ft, utf16z(s));
        }
    };
    text(row, R_INITIALS, &r.initials);
    text(row, R_GROUP, &r.group);
    text(row, R_MATERIAL, if r.rtype == ResourceType::Material { &r.material_label } else { "" });
}

/// Timephased data of an assignment without its own work curve (placeholder, material
/// and cost assignments MPP makes): a 40-byte header (u16 kind 0 work / 1 cost, u16 0x20,
/// u32 44, f64 rate = total x 160 / duration in minutes, f64 total, u32 end in 1/80 working
/// minute, u32 0, u32 1 for work / 0 for cost, u32 0), one 28-byte block (u32 start, f64 total so
/// far, f64 rate, 8 zero bytes; all zero when there is no work spread over time) and u32 end.
fn timephased_flat(cost: bool, total: f64, dur_min: i64) -> Vec<u8> {
    let end = (dur_min.max(0) * 80) as u32;
    let rate = if dur_min > 0 { total * 160.0 / dur_min as f64 } else { 0.0 };
    let mut v = Vec::with_capacity(72);
    v.extend_from_slice(&(cost as u16).to_le_bytes());
    v.extend_from_slice(&0x20u16.to_le_bytes());
    v.extend_from_slice(&44u32.to_le_bytes());
    v.extend_from_slice(&rate.to_le_bytes());
    v.extend_from_slice(&total.to_le_bytes());
    v.extend_from_slice(&end.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    v.extend_from_slice(&(!cost as u32).to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    // the block: a cost is spread over the task; a placeholder's or a material's is empty
    v.extend_from_slice(&0u32.to_le_bytes());
    if cost {
        v.extend_from_slice(&total.to_le_bytes());
        v.extend_from_slice(&rate.to_le_bytes());
    } else {
        v.extend_from_slice(&[0u8; 16]);
    }
    v.extend_from_slice(&[0u8; 8]);
    v.extend_from_slice(&(if cost { end } else { 0 }).to_le_bytes());
    v
}

/// Timephased work of a split task (f4 "Sanitary" as stored): the 40-byte header
/// with the number of blocks, the rate (work per 1/80 minute, x1000), the total work (1/1000
/// minute) and the end (1/80 working minute, gaps included); then per piece and per gap a 28-byte
/// block - u32 start, f64 work done by its end, f64 rate (0 in a gap), 8 zero bytes - and the end
/// again.
fn timephased_pieces(total: f64, dur_min: i64, splits: &[(i64, i64)]) -> Vec<u8> {
    let d = dur_min.max(1);
    let rate = total * 160.0 / d as f64;
    // (start, length, working?) in the task's working minutes, gaps included
    let mut parts: Vec<(i64, i64, bool)> = Vec::new();
    let (mut at, mut done) = (0i64, 0i64);
    for &(w, g) in splits {
        let w = (w - done).max(0).min(d - done);
        parts.push((at, w, true));
        at += w;
        done += w;
        if g > 0 {
            parts.push((at, g, false));
            at += g;
        }
    }
    parts.push((at, d - done, true));
    let end = at + (d - done);
    let mut v = Vec::with_capacity(44 + 28 * parts.len());
    v.extend_from_slice(&(parts.len() as u16).to_le_bytes());
    v.extend_from_slice(&0x20u16.to_le_bytes());
    v.extend_from_slice(&44u32.to_le_bytes());
    v.extend_from_slice(&rate.to_le_bytes());
    v.extend_from_slice(&total.to_le_bytes());
    v.extend_from_slice(&((end * 80) as u32).to_le_bytes());
    v.extend_from_slice(&[0u8; 12]);
    let mut work_done = 0i64;
    for (st, len, working) in parts {
        if working {
            work_done += len;
        }
        v.extend_from_slice(&((st * 80) as u32).to_le_bytes());
        v.extend_from_slice(&(total * work_done as f64 / d as f64).round().to_le_bytes());
        v.extend_from_slice(&(if working { rate } else { 0.0 }).to_le_bytes());
        v.extend_from_slice(&[0u8; 8]);
    }
    v.extend_from_slice(&((end * 80) as u32).to_le_bytes());
    v
}

const A_TIMEPHASED_COST: u32 = 0x0F40_028C;

/// How a cost assignment's cost is spread over its task (cost assignments made on
/// tasks of 0.5 to 10 days: everything is fixed but one u32, (duration in minutes + 1440) x 10;
/// a milestone has a shorter form).
fn cost_spread(dur_min: i64) -> Vec<u8> {
    let words: Vec<u32> = if dur_min <= 0 {
        vec![0x0004_0001, 0x10, 0, 0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0, 0x340A_75F7]
    } else {
        let mut w = vec![0x0004_0002, 0x10, 0, 0, 0, 0x3FF0_0000, 2, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0, 0x340A_75F7, 0, 0];
        w[14] = ((dur_min + 1440) * 10) as u32;
        w
    };
    words.iter().flat_map(|w| w.to_le_bytes()).collect()
}

/// An assignment's numbers: units, work (planned, actual, remaining), dates and cost. A
/// placeholder (no resource) has 100 % units and the task's duration as its work.
/// `keep_actuals`: the task's progress is the file's - its actual and remaining work and cost stay
/// as MPP recorded them while the assignment's units and work are unchanged (the standard
/// actual work is not always % complete of the work).
#[allow(clippy::too_many_arguments)]
fn write_assignment(row: &mut Row, amap: &FieldMap, t: &Task, a: Option<&Assignment>, res: Option<&Resource>, cal: &Calendar, mpd: i64, keep_actuals: bool) {
    let (units, work) = match (a, res.map(|r| r.rtype)) {
        // a material's quantity is also its work, an hour a unit (MPP: 1 m3 = PT1H)
        (Some(a), Some(ResourceType::Material)) => (a.units, (a.units * 60.0).round() as i64),
        (Some(_), Some(ResourceType::Cost)) => (0.0, 0),
        (Some(a), _) => (a.units, a.work_min.max(0)),
        (None, _) => (1.0, t.dur_minutes(mpd)),
    };
    let f = |row: &Row, ft: u32| row.get(amap, ft, 8).and_then(|b| f64_at(b, 0)).unwrap_or(0.0);
    let (old_units, old_work) = (f(row, A_UNITS) / 10000.0, f(row, A_WORK) / 1000.0);
    let changed = (old_units - units).abs() > 1e-9 || (old_work - work as f64).abs() > 0.5;
    if changed {
        // MPP rebuilds the timephased work from the new numbers
        row.remove_var(A_TIMEPHASED_WORK);
        row.remove_var(0x0F40_0032);
    }
    let keep = keep_actuals && !changed;
    let pct = t.percent_complete as f64 / 100.0;
    let actual = (work as f64 * pct).round();
    row.put(amap, A_UNITS, &(units * 10000.0).to_le_bytes());
    if !keep {
        row.put(amap, A_WORK, &(work as f64 * 1000.0).to_le_bytes());
        row.put(amap, A_REGULAR_WORK, &(work as f64 * 1000.0).to_le_bytes());
        row.put(amap, A_ACTUAL_WORK, &(actual * 1000.0).to_le_bytes());
        row.put(amap, A_REMAINING_WORK, &((work as f64 - actual).max(0.0) * 1000.0).to_le_bytes());
    }
    row.put(amap, A_START, &ts(t.start_dt));
    // a work assignment ends when its work is done at its units (MPP: 24 h at 300 % on a
    // three-day Fixed Duration task ends after the first day), never after the task
    let finish = match (a, res.map(|r| r.rtype), t.start_dt, t.finish_dt) {
        (Some(a), Some(ResourceType::Work), Some(s), Some(f)) if a.units > 0.0 && t.splits.is_empty() && !t.dur_elapsed => {
            let m = (a.work_min.max(0) as f64 / a.units).round() as i64;
            Some(cal.add_work_minutes(s, m).min(f))
        }
        _ => t.finish_dt,
    };
    row.put(amap, A_FINISH, &ts(finish));
    // stop / resume: the start until work is done
    let (stop, resume) = stop_resume(t, cal, mpd);
    row.put(amap, A_STOP, &ts(stop.or(t.start_dt)));
    row.put(amap, A_RESUME, &ts(resume.or(t.start_dt)));
    let cost = match (a, res) {
        (Some(a), Some(r)) => match r.rtype {
            ResourceType::Work => a.work_min.max(0) as f64 / 60.0 * r.std_rate + r.cost_per_use * a.units,
            ResourceType::Material => a.units * r.std_rate + r.cost_per_use,
            ResourceType::Cost => a.cost,
        },
        _ => 0.0,
    };
    let cost_c = (cost * 100.0).round();
    let actual_cost = (cost_c * pct).round();
    let kind = match (a, res.map(|r| r.rtype)) {
        (None, _) => 0, // placeholder
        (Some(_), Some(ResourceType::Work)) => 1,
        (Some(_), Some(ResourceType::Material)) => 2,
        _ => 3, // cost
    };
    if kind == 3 {
        // MPP keeps a cost assignment as units 6 and work of 0.06 per cent of its cost
        row.put(amap, A_UNITS, &6.0f64.to_le_bytes());
        for ft in [A_WORK, A_REGULAR_WORK, A_REMAINING_WORK] {
            row.put(amap, ft, &(cost_c * 0.06).to_le_bytes());
        }
        row.put(amap, A_ACTUAL_WORK, &0.0f64.to_le_bytes());
    }
    if kind != 1 {
        // what MPP writes for a placeholder, material or cost assignment (a work
        // assignment keeps the values of the one it was copied from)
        row.put(amap, 0x0F40_0037, &7u16.to_le_bytes());
        row.put(amap, 0x0F40_0051, &0u32.to_le_bytes());
        // their timephased data: MPP would rebuild missing data as work of a real resource.
        // A split task's pieces stay as the file has them.
        // (kept as the file has it while nothing about the assignment changed - it holds the
        // split between actual and remaining work, which MPP reads the durations from)
        if row.var(A_TIMEPHASED_WORK).is_none() || !keep {
            let total = match kind {
                0 => work as f64 * 1000.0,
                2 => (units * 60.0 * 1000.0).round(),
                _ => (cost_c * 0.06).round(),
            };
            let tp = if kind == 0 && !t.splits.is_empty() { timephased_pieces(total, t.dur_minutes(mpd), &t.splits) } else { timephased_flat(kind == 3, total, t.dur_minutes(mpd)) };
            row.set_var(A_TIMEPHASED_WORK, tp);
        }
    } else if !t.splits.is_empty() && (row.var(A_TIMEPHASED_WORK).is_none() || !keep) {
        // a work resource on a split task: its work in the task's pieces (without the data
        // MPP would spread it evenly over the gaps)
        row.set_var(A_TIMEPHASED_WORK, timephased_pieces(work as f64 * 1000.0, t.dur_minutes(mpd), &t.splits));
    }
    if kind == 3 {
        // a cost assignment's spread over the task (Fixed2Meta byte 41 bit 0x20 says it is there)
        if !keep || row.var(A_TIMEPHASED_COST).is_none() {
            row.set_var(A_TIMEPHASED_COST, cost_spread(t.dur_minutes(mpd)));
        }
        row.flag2(41, 0x20, true);
    }
    let old_cost = row.get(amap, A_COST, 8).and_then(|b| f64_at(b, 0)).unwrap_or(f64::NAN);
    if !(keep && (old_cost - cost_c).abs() < 0.5) {
        row.put(amap, A_COST, &cost_c.to_le_bytes());
        row.put(amap, A_ACTUAL_COST, &actual_cost.to_le_bytes());
        row.put(amap, A_REMAINING_COST, &(cost_c - actual_cost).max(0.0).to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_task(p: &mut Project, at: usize, name: &str, level: u8, days: i64) {
        let mpd = p.calendar.minutes_per_day();
        let uid = p.alloc_uid();
        let mut t = Task::new(uid, name, p.start);
        t.outline_level = level;
        t.set_duration_minutes(days * mpd, DurUnit::Days, false, mpd);
        p.tasks.insert(at, t);
    }

    fn delete_task(p: &mut Project, i: usize) {
        let uid = p.tasks[i].uid;
        p.tasks.remove(i);
        for t in p.tasks.iter_mut() {
            t.predecessors.retain(|l| l.pred_uid != uid);
        }
    }

    /// Custom field definitions of a saved file (the user's f2: Text1 "Item No. (Excel)"
    /// with a lookup table; NERAMIT_MPP_TESTDATA).
    #[test]
    fn custom_definitions_from_project() {
        let Ok(dir) = std::env::var("NERAMIT_MPP_TESTDATA") else { return };
        let Ok(bytes) = std::fs::read(std::path::Path::new(&dir).join("f2.mpp")) else { return };
        let p = crate::mpp::load_mpp(&bytes).unwrap();
        let d = p.custom_defs.iter().find(|d| !d.resource && d.field == crate::custom::CustomField::new(crate::custom::CfKind::Text, 1)).expect("Text1 defined");
        assert_eq!(d.alias, "Item No. (Excel)");
        println!("f2 Text1 lookup: {:?}", d.values);
    }

    /// Sprints and the Task Board of a saved file (sprints.mpp: Sprint A / B, T1 and T2 in
    /// A, T3 in B, T2 in "Next up"; NERAMIT_SPRINT_MPP).
    #[test]
    fn sprints_from_project() {
        let Ok(path) = std::env::var("NERAMIT_SPRINT_MPP") else { return };
        let p = crate::mpp::load_mpp(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(p.sprints.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), vec!["Sprint A", "Sprint B"]);
        let of = |n: &str| p.tasks.iter().find(|t| t.name == n).unwrap();
        let sprint = |n: &str| of(n).sprint.and_then(|u| p.sprints.iter().find(|s| s.uid == u)).map(|s| s.name.clone());
        assert_eq!((sprint("T1"), sprint("T2"), sprint("T3"), sprint("T4")), (Some("Sprint A".into()), Some("Sprint A".into()), Some("Sprint B".into()), None));
        assert_eq!(of("T2").board_status, 1);
        println!("sprints: {:?}", p.sprints);
    }

    /// Links between projects as saved (main2.mpp: X3 of ext.mpp before Y1, Z1 of
    /// ext2.mpp before Y2 with a day's lag, X1 of ext.mpp after Y3; NERAMIT_LINK_SAMPLES).
    #[test]
    fn external_tasks_from_project() {
        let Ok(dir) = std::env::var("NERAMIT_LINK_SAMPLES") else { return };
        let mut p = crate::mpp::load_mpp(&std::fs::read(std::path::Path::new(&dir).join("main2.mpp")).unwrap()).unwrap();
        crate::schedule::schedule(&mut p).unwrap();
        let ext: Vec<(String, String, u32, u32)> = p.tasks.iter().filter_map(|t| t.external.as_ref().map(|x| (t.name.clone(), std::path::Path::new(&x.path).file_name().unwrap().to_string_lossy().to_string(), x.uid, x.id))).collect();
        assert_eq!(ext, vec![("X3".into(), "ext.mpp".into(), 3, 3), ("Z1".into(), "ext2.mpp".into(), 1, 1), ("X1".into(), "ext.mpp".into(), 1, 2)]);
        let uid = |n: &str| p.tasks.iter().find(|t| t.name == n).unwrap().uid;
        let of = |n: &str| p.tasks.iter().find(|t| t.name == n).unwrap();
        assert!(of("Y1").predecessors.iter().any(|l| l.pred_uid == uid("X3")));
        assert!(of("Y2").predecessors.iter().any(|l| l.pred_uid == uid("Z1") && l.lag_min == Some(480)));
        assert!(of("X1").predecessors.iter().any(|l| l.pred_uid == uid("Y3")));
        // the standard dates: X3 Nov 2 - 3, Y1 Nov 4; Z1 Nov 9, Y2 Nov 11
        assert_eq!(of("Y1").start, chrono::NaiveDate::from_ymd_opt(2026, 11, 4).unwrap());
        assert_eq!(of("Y2").start, chrono::NaiveDate::from_ymd_opt(2026, 11, 11).unwrap());
        // saved again: the external tasks and the link table stay as saved
        let orig = std::fs::read(std::path::Path::new(&dir).join("main2.mpp")).unwrap();
        let bytes = save_mpp(&orig, &p).unwrap();
        let q = crate::mpp::load_mpp(&bytes).unwrap();
        assert_eq!(q.tasks.iter().filter(|t| t.external.is_some()).count(), 3);
        let a = crate::mpp::parse_props(&Cfb::new(&orig).unwrap().stream(&["   114", "Props"]));
        let b = crate::mpp::parse_props(&Cfb::new(&bytes).unwrap().stream(&["   114", "Props"]));
        assert_eq!(a.get(&crate::extlinks::P_LINKS), b.get(&crate::extlinks::P_LINKS));
    }

    /// Debug: NERAMIT_DEBUG="file.mpp;uid,uid" prints those tasks as read and after a reschedule.
    #[test]
    fn debug_tasks() {
        let Ok(spec) = std::env::var("NERAMIT_DEBUG") else { return };
        let parts: Vec<&str> = spec.split(';').collect();
        let mut p = crate::mpp::load_mpp(&std::fs::read(parts[0]).unwrap()).unwrap();
        let uids: Vec<u32> = parts[1].split(',').filter_map(|s| s.parse().ok()).collect();
        let show = |p: &Project, when: &str| {
            for t in p.tasks.iter().filter(|t| t.src_uid.map(|u| uids.contains(&u)).unwrap_or(false)) {
                println!("{when} uid {:?} manual {} placeholder {} start {:?} finish {:?} start_d {} dur {} actual {:?} cons {:?} {:?}", t.src_uid, t.manual, t.placeholder, t.start_dt, t.finish_dt, t.start, t.dur_min, t.actual_start, t.constraint_type, t.constraint_date);
            }
        };
        show(&p, "read");
        crate::schedule::schedule(&mut p).unwrap();
        show(&p, "sched");
        if parts.len() > 2 {
            let k = p.tasks.iter().position(|t| t.src_uid == parts[2].parse().ok()).unwrap();
            delete_task(&mut p, k);
            crate::schedule::schedule(&mut p).unwrap();
            show(&p, "after delete");
        }
    }

    /// Custom fields made one at a time (NERAMIT_CFDIR = folder of t_Text1.mpp ...
    /// r_Flag20.mpp, each with that field set on task 1 / resource 1): the reader finds each.
    #[test]
    fn custom_fields_from_project() {
        let Ok(dir) = std::env::var("NERAMIT_CFDIR") else { return };
        let mut checked = 0;
        for f in crate::custom::all() {
            for (res, pre) in [(false, "t"), (true, "r")] {
                let path = std::path::Path::new(&dir).join(format!("{pre}_{}.mpp", f.name()));
                let Ok(bytes) = std::fs::read(&path) else { continue };
                let p = crate::mpp::load_mpp(&bytes).unwrap();
                let vals = if res { &p.resources.iter().find(|r| r.src_uid == Some(1)).unwrap().custom } else { &p.tasks.iter().find(|t| t.src_uid == Some(1)).unwrap().custom };
                assert_eq!(vals.len(), 1, "{}: {:?}", path.display(), vals);
                assert_eq!(vals[0].0, f, "{}", path.display());
                checked += 1;
            }
        }
        assert!(checked >= 240, "{checked}");
    }

    /// Every task, link, resource and assignment of a real file written as new rows of the template
    /// (NERAMIT_MPPT_CONVERT = "input.mpp;output folder"): the path an MPP12 file takes.
    #[test]
    fn convert_to_template() {
        let Ok(spec) = std::env::var("NERAMIT_MPPT_CONVERT") else { return };
        let parts: Vec<&str> = spec.split(';').collect();
        let mut p = crate::mpp::load_mpp(&std::fs::read(parts[0]).unwrap()).unwrap();
        crate::schedule::schedule(&mut p).unwrap();
        let bytes = save_new_mpp(&p).unwrap();
        let dir = std::path::Path::new(parts[1]);
        let stem = std::path::Path::new(parts[0]).file_stem().unwrap().to_string_lossy().to_string();
        std::fs::write(dir.join(format!("{stem}.mpp")), &bytes).unwrap();
        crate::fileio::save(&dir.join(format!("{stem}_ours.xml")), &p).unwrap();
        let q = crate::mpp::load_mpp(&bytes).unwrap();
        assert_eq!(q.tasks.len(), p.tasks.len());
        assert_eq!(q.resources.len(), p.resources.len());
    }

    /// A project made in Neramit Planner saved as a new .mpp (NERAMIT_MPPT_NEW = output folder):
    /// summary, tasks, links with lag, a milestone, work / material / cost resources and their
    /// assignments, and a calendar with Saturday mornings and a holiday. Writes new.mpp and
    /// new_ours.xml.
    #[test]
    fn write_new_project() {
        let Ok(dir) = std::env::var("NERAMIT_MPPT_NEW") else { return };
        let dir = std::path::Path::new(&dir);
        let start = chrono::NaiveDate::from_ymd_opt(2026, 11, 2).unwrap();
        let mut p = Project::default();
        p.title = "New Neramit Planner".into();
        p.start = start;
        p.calendar.working_days[5] = true;
        p.calendar.work_times = vec![Vec::new(); 7];
        p.calendar.work_times[5] = vec![(8 * 60, 12 * 60)];
        p.calendar.holidays.push(Holiday { name: "Holiday".into(), start: start + chrono::Duration::days(9), finish: start + chrono::Duration::days(9) });
        let mpd = p.calendar.minutes_per_day();
        let add = |p: &mut Project, name: &str, level: u8, days: i64| -> u32 {
            let uid = p.alloc_uid();
            let mut t = Task::new(uid, name, start);
            t.outline_level = level;
            t.set_duration_minutes(days * mpd, DurUnit::Days, false, mpd);
            p.tasks.push(t);
            uid
        };
        add(&mut p, "Phase 1", 1, 1);
        let a = add(&mut p, "Site survey", 2, 2);
        let b = add(&mut p, "Foundation", 2, 5);
        let c = add(&mut p, "Inspection", 2, 0);
        add(&mut p, "Phase 2", 1, 1);
        let d = add(&mut p, "Structure", 2, 10);
        let e = add(&mut p, "Roof", 2, 4);
        let k = p.tasks.len() - 1;
        p.tasks[k].manual = false;
        let idx = |p: &Project, u: u32| p.index_of_uid(u).unwrap();
        let (ib, ic, id, ie) = (idx(&p, b), idx(&p, c), idx(&p, d), idx(&p, e));
        p.tasks[ib].predecessors.push(Link::new(a, LinkType::FS));
        p.tasks[ic].predecessors.push(Link::new(b, LinkType::FS));
        p.tasks[ic].milestone = true;
        let mut l = Link::new(c, LinkType::FS);
        l.lag_min = Some(2 * mpd);
        p.tasks[id].predecessors.push(l);
        p.tasks[ie].predecessors.push(Link::new(d, LinkType::SS));
        for t in p.tasks.iter_mut() {
            t.manual = false;
        }
        let worker = p.alloc_res_uid();
        let mut r = Resource::new(worker, "Worker");
        r.std_rate = 50.0;
        p.resources.push(r);
        let sand = p.alloc_res_uid();
        let mut r = Resource::new(sand, "Sand");
        r.rtype = ResourceType::Material;
        r.material_label = "m3".into();
        r.std_rate = 20.0;
        p.resources.push(r);
        let permit = p.alloc_res_uid();
        let mut r = Resource::new(permit, "Permit");
        r.rtype = ResourceType::Cost;
        p.resources.push(r);
        let dur = p.tasks[ib].dur_minutes(mpd);
        p.tasks[ib].assignments.push(Assignment { res_uid: worker, units: 1.0, work_min: dur, cost: 0.0 });
        p.tasks[ib].assignments.push(Assignment { res_uid: sand, units: 12.0, work_min: 0, cost: 0.0 });
        p.tasks[ib].resources = vec![worker, sand];
        p.tasks[ib].work_ref_dur = dur;
        p.tasks[ie].assignments.push(Assignment { res_uid: permit, units: 0.0, work_min: 0, cost: 900.0 });
        p.tasks[ie].resources = vec![permit];
        crate::schedule::schedule(&mut p).unwrap();
        crate::schedule::sync_assignments(&mut p);
        crate::schedule::schedule(&mut p).unwrap();
        let bytes = save_new_mpp(&p).unwrap();
        std::fs::write(dir.join("new.mpp"), &bytes).unwrap();
        crate::fileio::save(&dir.join("new_ours.xml"), &p).unwrap();
        let q = crate::mpp::load_mpp(&bytes).unwrap();
        assert_eq!(q.tasks.len(), p.tasks.len());
        assert_eq!(q.resources.len(), 3);
        assert!(crate::mpp::same_calendar(&q.calendar, &p.calendar));
    }

    /// Structural edits saved as .mpp for MPP to check (NERAMIT_MPPT_EDIT =
    /// "input.mpp;output folder;scenario,scenario,..."): every scenario writes <name>.mpp and our
    /// own <name>_ours.xml.
    #[test]
    fn write_structural_copies() {
        let Ok(spec) = std::env::var("NERAMIT_MPPT_EDIT") else { return };
        let parts: Vec<&str> = spec.split(';').collect();
        let orig = std::fs::read(parts[0]).unwrap();
        let dir = std::path::Path::new(parts[1]);
        let all = "none,add_end,add_mid,del,link_add,link_del,res_add,res_mat,res_cost,res_del,asg_del,indent,outdent,combo";
        let scenarios = parts.get(2).copied().unwrap_or(all);
        for sc in scenarios.split(',') {
            let mut p = crate::mpp::load_mpp(&orig).unwrap();
            crate::schedule::schedule(&mut p).unwrap();
            let mpd = p.calendar.minutes_per_day();
            let leaves: Vec<usize> = (0..p.tasks.len()).filter(|&i| !p.tasks[i].is_summary).collect();
            let on = |m: &str| sc == m || (sc == "combo" && ["add_mid", "del", "link_add", "res_add", "indent"].contains(&m));
            if on("add_end") {
                let last = p.tasks.len() - 1;
                let lvl = p.tasks[last].outline_level;
                new_task(&mut p, last + 1, "Added by Neramit Planner", lvl, 3);
                let pred = p.tasks[last].uid;
                p.tasks[last + 1].predecessors.push(Link::new(pred, LinkType::FS));
            }
            if on("add_mid") {
                let at = leaves[1];
                let lvl = p.tasks[at].outline_level;
                new_task(&mut p, at, "Inserted by Neramit Planner", lvl, 2);
            }
            if on("del") {
                // a task with a predecessor and a successor if there is one
                let k = (0..p.tasks.len())
                    .find(|&i| !p.tasks[i].is_summary && !p.tasks[i].predecessors.is_empty() && p.tasks.iter().any(|s| s.predecessors.iter().any(|l| l.pred_uid == p.tasks[i].uid)))
                    .unwrap_or(leaves[2]);
                delete_task(&mut p, k);
            }
            if on("link_add") {
                // link two neighbouring leaves that are not linked yet
                let ls: Vec<usize> = (0..p.tasks.len()).filter(|&i| !p.tasks[i].is_summary).collect();
                if let Some(w) = ls.windows(2).find(|w| p.tasks[w[1]].predecessors.is_empty() && !crate::schedule::creates_cycle(&p, p.tasks[w[0]].uid, p.tasks[w[1]].uid)) {
                    let pred = p.tasks[w[0]].uid;
                    p.tasks[w[1]].predecessors.push(Link::new(pred, LinkType::FS));
                }
            }
            if on("link_del") {
                if let Some(t) = p.tasks.iter_mut().find(|t| !t.predecessors.is_empty()) {
                    t.predecessors.remove(0);
                }
            }
            if on("res_cost") {
                let uid = p.alloc_res_uid();
                let mut r = Resource::new(uid, "Permit Neramit Planner");
                r.rtype = ResourceType::Cost;
                p.resources.push(r);
                let i = leaves[0];
                p.tasks[i].assignments.push(Assignment { res_uid: uid, units: 0.0, work_min: 0, cost: 250.0 });
                p.tasks[i].resources.push(uid);
            }
            if on("res_add") || on("res_mat") {
                let mat = on("res_mat");
                let uid = p.alloc_res_uid();
                let mut r = Resource::new(uid, if mat { "Sand Neramit Planner" } else { "Crane Neramit Planner" });
                if mat {
                    r.rtype = ResourceType::Material;
                    r.material_label = "m3".into();
                    r.std_rate = 12.0;
                } else {
                    r.std_rate = 30.0;
                }
                p.resources.push(r);
                let i = *leaves.last().unwrap();
                let i = i.min(p.tasks.len() - 1);
                let d = p.tasks[i].dur_minutes(mpd);
                p.tasks[i].assignments.push(Assignment { res_uid: uid, units: if mat { 5.0 } else { 1.0 }, work_min: if mat { 0 } else { d }, cost: 0.0 });
                p.tasks[i].resources.push(uid);
                p.tasks[i].work_ref_dur = d;
            }
            if on("res_del") {
                if let Some(r) = p.resources.iter().position(|r| r.rtype == ResourceType::Work && p.tasks.iter().any(|t| t.assignments.iter().any(|a| a.res_uid == r.uid))) {
                    let uid = p.resources[r].uid;
                    p.resources.remove(r);
                    for t in p.tasks.iter_mut() {
                        t.assignments.retain(|a| a.res_uid != uid);
                        t.resources.retain(|x| *x != uid);
                    }
                }
            }
            if on("asg_del") {
                if let Some(t) = p.tasks.iter_mut().find(|t| t.assignments.len() >= 2) {
                    let gone = t.assignments.remove(0);
                    t.resources.retain(|x| *x != gone.res_uid);
                }
            }
            if on("indent") {
                // a leaf under the leaf above it, with no link between the two
                let k = (1..p.tasks.len())
                    .find(|&i| {
                        let (a, b) = (&p.tasks[i - 1], &p.tasks[i]);
                        !b.is_summary && !a.is_summary && a.outline_level == b.outline_level && !b.predecessors.iter().any(|l| l.pred_uid == a.uid) && !a.predecessors.iter().any(|l| l.pred_uid == b.uid)
                    })
                    .unwrap();
                p.tasks[k].outline_level += 1;
            }
            if on("custom") {
                // a value in each kind of custom field on a task (and on a resource)
                use crate::custom::{CfKind as K, CfValue as V, CustomField as F};
                let i = leaves[0];
                let s = p.tasks[i].start_dt.unwrap_or_else(|| p.calendar.day_start(p.start));
                for (f, v) in [
                    (F::new(K::Text, 1), V::Text("Zone A".into())),
                    (F::new(K::Text, 17), V::Text("Phase 2".into())),
                    (F::new(K::Number, 3), V::Number(42.5)),
                    (F::new(K::Number, 12), V::Number(-7.0)),
                    (F::new(K::Flag, 1), V::Flag(true)),
                    (F::new(K::Flag, 14), V::Flag(true)),
                    (F::new(K::Date, 2), V::Date(s)),
                    (F::new(K::Cost, 5), V::Cost(1234.5)),
                    (F::new(K::Duration, 4), V::Duration(960)),
                    (F::new(K::Start, 7), V::Date(s)),
                    (F::new(K::Finish, 9), V::Date(s)),
                ] {
                    crate::custom::set(&mut p.tasks[i].custom, f, Some(v));
                }
                if let Some(r) = p.resources.first_mut() {
                    crate::custom::set(&mut r.custom, F::new(K::Text, 3), Some(V::Text("Crew 1".into())));
                    crate::custom::set(&mut r.custom, F::new(K::Number, 7), Some(V::Number(3.0)));
                    crate::custom::set(&mut r.custom, F::new(K::Flag, 10), Some(V::Flag(true)));
                }
            }
            if on("notes") {
                // notes on a task (two lines, Thai and English) and on a resource
                let i = leaves[0];
                p.tasks[i].notes = "Note from Neramit Planner
บรรทัดที่สอง: ตรวจสอบหน้างาน".into();
                if let Some(r) = p.resources.first_mut() {
                    r.notes = "Resource note".into();
                }
            }
            if on("wbs") {
                p.wbs_mask = Some(crate::wbs::WbsMask {
                    prefix: "BX-".into(),
                    levels: vec![crate::wbs::WbsLevel { seq: 0, len: 0, sep: ".".into() }, crate::wbs::WbsLevel { seq: 1, len: 0, sep: "-".into() }],
                    generate: true,
                    verify_unique: true,
                });
                crate::wbs::renumber(&mut p, None);
            }
            if on("kinds") {
                // new rows of every kind at the end: a summary with two subtasks, a milestone, a
                // manual task, a manual task with nothing entered and one with "TBD" as its start
                let n = p.tasks.len();
                new_task(&mut p, n, "New summary", 1, 1);
                new_task(&mut p, n + 1, "Sub one", 2, 2);
                new_task(&mut p, n + 2, "Sub two", 2, 3);
                let u1 = p.tasks[n + 1].uid;
                p.tasks[n + 2].predecessors.push(Link::new(u1, LinkType::FS));
                new_task(&mut p, n + 3, "New milestone", 1, 0);
                p.tasks[n + 3].milestone = true;
                let u2 = p.tasks[n + 2].uid;
                p.tasks[n + 3].predecessors.push(Link::new(u2, LinkType::FS));
                new_task(&mut p, n + 4, "Manual two days", 1, 2);
                new_task(&mut p, n + 5, "Nothing entered", 1, 1);
                new_task(&mut p, n + 6, "Start TBD", 1, 1);
                for k in n..n + 4 {
                    p.tasks[k].manual = false;
                }
                for k in n + 4..n + 7 {
                    p.tasks[k].manual = true;
                }
                p.tasks[n + 4].start_dt = Some(p.calendar.day_start(p.start));
                p.tasks[n + 5].set_nothing_entered();
                p.tasks[n + 6].manual_text[0] = Some("TBD".into());
                p.tasks[n + 6].sync_manual_text();
                // an existing manual task's start entered again (when the file has one)
                if let Some(k) = (0..n).find(|&k| p.tasks[k].manual && !p.tasks[k].is_summary && p.tasks[k].manual_text.iter().all(|x| x.is_none())) {
                    let d = p.tasks[k].start_dt.unwrap_or_else(|| p.calendar.day_start(p.tasks[k].start));
                    p.tasks[k].start_dt = Some(p.calendar.day_start(d.date() + chrono::Duration::days(7)));
                }
            }
            if on("cfdefs") {
                // Project > Custom Fields: renamed fields, formulas and lookup tables of several types
                use crate::custom::{CfKind, CustomField, CustomFieldDef};
                let mut set = |f: CustomField, resource: bool, alias: &str, formula: &str, values: &[(&str, &str)]| {
                    let k = match p.custom_defs.iter().position(|d| d.field == f && d.resource == resource) {
                        Some(k) => k,
                        None => {
                            p.custom_defs.push(CustomFieldDef::new(f, resource));
                            p.custom_defs.len() - 1
                        }
                    };
                    let d = &mut p.custom_defs[k];
                    d.alias = alias.into();
                    d.formula = formula.into();
                    d.values = values.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
                };
                set(CustomField::new(CfKind::Text, 2), false, "Zone", "", &[("North", "Building A"), ("South", "Building B"), ("East", "")]);
                set(CustomField::new(CfKind::Number, 1), false, "Days", "[Duration]/480", &[]);
                set(CustomField::new(CfKind::Text, 3), false, "Progress State", r#"IIf([% Complete]=100,"Done","Open")"#, &[]);
                set(CustomField::new(CfKind::Number, 2), false, "Grade", "", &[("1", "Low"), ("2.5", "Mid"), ("10", "High")]);
                set(CustomField::new(CfKind::Cost, 1), false, "", "", &[("100", "Small"), ("2500.5", "Large")]);
                set(CustomField::new(CfKind::Duration, 1), false, "", "", &[("2d", "Short"), ("3w", "Long")]);
                set(CustomField::new(CfKind::Flag, 1), false, "Approved", "", &[]);
                set(CustomField::new(CfKind::Text, 1), true, "Department", "", &[("Civil", ""), ("MEP", "")]);
                set(CustomField::new(CfKind::Number, 1), true, "", "[Max Units]*2", &[]);
                // a value from a list on the first task
                if let Some(t) = leaves.first().map(|&i| &mut p.tasks[i]) {
                    t.custom.retain(|c| c.0 != CustomField::new(CfKind::Text, 2));
                    t.custom.push((CustomField::new(CfKind::Text, 2), crate::custom::CfValue::Text("South".into())));
                }
            }
            if on("sprints") {
                // Project > Manage Sprints: two sprints (one made here, UID 0), tasks in them, a
                // task in the Task Board's "Next up"
                let s0 = p.tasks.iter().filter_map(|t| t.start_dt).min().map(|d| d.date()).unwrap_or(p.start);
                p.sprints = vec![
                    crate::model::Sprint { uid: 2, name: "Sprint 1".into(), start: s0, finish: s0 + chrono::Duration::days(13) },
                    crate::model::Sprint { uid: 0, name: "สปรินต์ 2".into(), start: s0 + chrono::Duration::days(14), finish: s0 + chrono::Duration::days(27) },
                ];
                if let (Some(&a), Some(&b), Some(&c)) = (leaves.first(), leaves.get(1), leaves.get(2)) {
                    p.tasks[a].sprint = Some(2);
                    p.tasks[b].sprint = Some(0);
                    p.tasks[c].board_status = 1;
                }
            }
            if on("drawings") {
                // Format > Drawing: a text box and an arrow on the chart
                let s0 = p.tasks.iter().filter_map(|t| t.start_dt).min().unwrap_or_else(|| p.calendar.day_start(p.start));
                p.drawings = vec![
                    crate::drawing::Drawing { kind: 0, x0: s0, x1: s0 + chrono::Duration::days(5), y0: 20.0, y1: 60.0, text: "หมายเหตุ: Note".into() },
                    crate::drawing::Drawing { kind: 2, x0: s0 + chrono::Duration::days(1), x1: s0 + chrono::Duration::days(9), y0: 80.0, y1: 140.0, text: String::new() },
                ];
            }
            if on("pool") {
                // Resource > Resource Pool > Share Resources (NERAMIT_POOL_FILE: the pool)
                if let Ok(pool) = std::env::var("NERAMIT_POOL_FILE") {
                    p.resource_pool = Some(crate::model::ResourcePool { path: pool, sharer_first: false });
                }
            }
            if on("xlinks") {
                // Links Between Projects: a project next to this one (written here as XML), its
                // second task before this project's first leaf, this project's last leaf before
                // its first task
                let mut other = Project::new();
                other.start = p.start;
                for (k, (name, days)) in [("Permit", 3i64), ("Inspection", 2)].iter().enumerate() {
                    let mut t = Task::new(k as u32 + 1, name, p.start);
                    t.set_duration_minutes(days * 480, DurUnit::Days, false, 480);
                    t.manual = false;
                    other.tasks.push(t);
                }
                other.tasks[1].predecessors.push(Link::new(1, LinkType::FS));
                other.next_uid = 3;
                crate::schedule::schedule(&mut other).unwrap();
                // (a .mpp: MPP links to .mpp files)
                let other_path = dir.join("other.mpp");
                std::fs::write(&other_path, save_new_mpp(&other).unwrap()).unwrap();
                let op = other_path.display().to_string();
                let mpd = p.calendar.minutes_per_day();
                if let (Some(&a), Some(&b)) = (leaves.first(), leaves.last()) {
                    let (ua, ub) = (p.tasks[a].uid, p.tasks[b].uid);
                    let g1 = p.alloc_uid();
                    let ga = crate::fileio::external_task(&op, 2, g1, p.tasks[a].outline_level, p.start, mpd);
                    p.tasks.insert(a, ga);
                    let k = p.index_of_uid(ua).unwrap();
                    p.tasks[k].predecessors.push(Link::new(g1, LinkType::FS));
                    let g2 = p.alloc_uid();
                    let mut gb = crate::fileio::external_task(&op, 1, g2, p.tasks[p.index_of_uid(ub).unwrap()].outline_level, p.start, mpd);
                    gb.predecessors.push(Link::new(ub, LinkType::FS));
                    let at = p.index_of_uid(ub).unwrap() + 1;
                    p.tasks.insert(at, gb);
                }
            }
            if on("xlinks_top") {
                // the same at the top of the outline (NERAMIT_XLINK_OTHER: the other project)
                if let (Ok(op), Some(&a)) = (std::env::var("NERAMIT_XLINK_OTHER"), leaves.first()) {
                    let ua = p.tasks[a].uid;
                    let g1 = p.alloc_uid();
                    let ga = crate::fileio::external_task(&op, 2, g1, 1, p.start, p.calendar.minutes_per_day());
                    p.tasks.insert(0, ga);
                    let k = p.index_of_uid(ua).unwrap();
                    p.tasks[k].predecessors.push(Link::new(g1, LinkType::FS));
                }
            }
            if on("xlinks_more") {
                // one more external task from a project the file already links to
                if let Some(x) = p.tasks.iter().find_map(|t| t.external.clone()) {
                    if let Some(k) = (0..p.tasks.len()).find(|&k| p.tasks[k].external.is_none()) {
                        let uk = p.tasks[k].uid;
                        let g = p.alloc_uid();
                        let gt = crate::fileio::external_task(&x.path, 1, g, 1, p.start, p.calendar.minutes_per_day());
                        p.tasks.insert(0, gt);
                        let k = p.index_of_uid(uk).unwrap();
                        p.tasks[k].predecessors.push(Link::new(g, LinkType::SS));
                    }
                }
            }
            if on("wbs_manual") {
                let i = leaves[0];
                p.tasks[i].wbs = "X.9".into();
            }
            if on("cal") {
                // the project calendar: Saturday mornings worked and a three-day holiday
                let s = p.tasks.iter().filter_map(|t| t.start_dt).min().map(|d| d.date()).unwrap_or(p.start);
                p.calendar.working_days[5] = true;
                if p.calendar.work_times.len() != 7 {
                    p.calendar.work_times = vec![Vec::new(); 7];
                }
                p.calendar.work_times[5] = vec![(8 * 60, 12 * 60)];
                p.calendar.holidays.push(Holiday { name: "Songkran Neramit Planner".into(), start: s + chrono::Duration::days(10), finish: s + chrono::Duration::days(12) });
            }
            if on("outdent") {
                let k = (0..p.tasks.len()).rev().find(|&i| !p.tasks[i].is_summary && p.tasks[i].outline_level > 1 && (i + 1 == p.tasks.len() || p.tasks[i + 1].outline_level < p.tasks[i].outline_level)).unwrap();
                p.tasks[k].outline_level -= 1;
            }
            if let Err(e) = crate::schedule::schedule(&mut p) {
                println!("{sc}: skipped ({e})");
                continue;
            }
            crate::schedule::sync_assignments(&mut p);
            crate::schedule::schedule(&mut p).unwrap();
            let bytes = save_mpp_at(&orig, &p, Some(&dir.join(format!("{sc}.mpp")))).unwrap_or_else(|e| panic!("{sc}: {e}"));
            std::fs::write(dir.join(format!("{sc}.mpp")), &bytes).unwrap();
            crate::fileio::save(&dir.join(format!("{sc}_ours.xml")), &p).unwrap();
            // our own reader sees the same tasks, links and resources
            let q = crate::mpp::load_mpp(&bytes).unwrap();
            if sc == "xlinks" {
                let ext = |x: &Project| x.tasks.iter().filter_map(|t| t.external.as_ref().map(|e| (t.name.clone(), e.path.clone(), e.id, t.start_dt, t.finish_dt))).collect::<Vec<_>>();
                assert_eq!(ext(&q), ext(&p));
                assert_eq!(ext(&q).len(), 2);
                // Project XML keeps the external predecessor (CrossProjectName), read back from
                // the other project's file
                let x = crate::fileio::load(&dir.join(format!("{sc}_ours.xml"))).unwrap();
                let names: Vec<String> = x.tasks.iter().filter(|t| t.external.is_some()).map(|t| t.name.clone()).collect();
                assert_eq!(names, vec!["Inspection".to_string()]);
            }
            if sc == "pool" {
                assert_eq!(q.resource_pool, p.resource_pool);
            }
            if sc == "drawings" {
                assert_eq!(q.drawings, p.drawings);
                let x = crate::fileio::load(&dir.join(format!("{sc}_ours.xml"))).unwrap();
                assert_eq!(x.drawings, p.drawings);
            }
            if sc == "sprints" {
                assert_eq!(q.sprints.iter().map(|s| (&s.name, s.start, s.finish)).collect::<Vec<_>>(), p.sprints.iter().map(|s| (&s.name, s.start, s.finish)).collect::<Vec<_>>());
                let names = |x: &Project| x.tasks.iter().map(|t| (t.sprint.and_then(|u| x.sprints.iter().find(|s| s.uid == u)).map(|s| s.name.clone()), t.board_status)).collect::<Vec<_>>();
                assert_eq!(names(&q), names(&p));
            }
            if sc == "cfdefs" {
                // the definitions read back; the same project as a new .mpp too
                for d in p.custom_defs.iter().filter(|d| !d.alias.is_empty() || !d.formula.is_empty() || !d.values.is_empty()) {
                    let e = q.custom_defs.iter().find(|e| e.field == d.field && e.resource == d.resource).unwrap_or_else(|| panic!("{:?} lost", d.field));
                    assert_eq!((&e.alias, &e.formula, &e.values), (&d.alias, &d.formula, &d.values), "{:?}", d.field);
                }
                let fresh = save_new_mpp(&p).unwrap();
                std::fs::write(dir.join("cfdefs_new.mpp"), &fresh).unwrap();
                crate::fileio::save(&dir.join("cfdefs_new_ours.xml"), &p).unwrap();
                let n = crate::mpp::load_mpp(&fresh).unwrap();
                assert_eq!(n.custom_defs.iter().filter(|d| !d.values.is_empty()).count(), p.custom_defs.iter().filter(|d| !d.values.is_empty()).count());
            }
            assert_eq!(q.tasks.len(), p.tasks.len(), "{sc}");
            assert_eq!(q.resources.len(), p.resources.len(), "{sc}");
            let links = |x: &Project| x.tasks.iter().map(|t| t.predecessors.len()).sum::<usize>();
            assert_eq!(links(&q), links(&p), "{sc}");
            let asg = |x: &Project| x.tasks.iter().filter(|t| !t.is_summary).map(|t| t.assignments.len()).sum::<usize>();
            assert_eq!(asg(&q), asg(&p), "{sc}");
            for (a, b) in p.tasks.iter().zip(&q.tasks) {
                assert_eq!((a.name.as_str(), a.outline_level), (b.name.as_str(), b.outline_level), "{sc}");
            }
            println!("{sc}: {} tasks, {} resources", p.tasks.len(), p.resources.len());
        }
    }
}
