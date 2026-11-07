//! Project > WBS: the code mask (Define Code: a prefix, then per outline level a sequence -
//! numbers, upper- or lowercase letters, characters - its length and a separator) and Renumber,
//! which gives every task (or the selected ones) the code of its place in the outline. Without a
//! mask the WBS is the outline number, as in MPP.

use crate::model::{Project, Task};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WbsLevel {
    /// 0 Numbers (ordered), 1 Uppercase Letters (ordered), 2 Lowercase Letters (ordered),
    /// 3 Characters (unordered) - XML WBSMask Type
    pub seq: u8,
    /// 0 = any length
    pub len: u8,
    pub sep: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WbsMask {
    pub prefix: String,
    pub levels: Vec<WbsLevel>,
    /// Generate WBS code for new task
    pub generate: bool,
    /// Verify uniqueness of new WBS codes
    pub verify_unique: bool,
}

impl Default for WbsMask {
    fn default() -> Self {
        WbsMask { prefix: String::new(), levels: vec![WbsLevel { seq: 0, len: 0, sep: ".".into() }], generate: true, verify_unique: true }
    }
}

pub const SEQUENCES: [&str; 4] = ["Numbers (ordered)", "Uppercase Letters (ordered)", "Lowercase Letters (ordered)", "Characters (unordered)"];

/// 1 -> "A", 26 -> "Z", 27 -> "AA" (bijective base 26)
fn letters(mut n: usize, upper: bool) -> String {
    let mut s = Vec::new();
    while n > 0 {
        n -= 1;
        s.push(((if upper { b'A' } else { b'a' }) + (n % 26) as u8) as char);
        n /= 26;
    }
    s.iter().rev().collect()
}

/// One level's code for the n-th task (1-based) under its parent.
fn level_code(l: &WbsLevel, n: usize) -> String {
    let s = match l.seq {
        1 => letters(n, true),
        2 => letters(n, false),
        3 => "*".to_string(),
        _ => n.to_string(),
    };
    if l.len > 0 && l.seq == 0 && s.len() < l.len as usize {
        format!("{}{}", "0".repeat(l.len as usize - s.len()), s)
    } else {
        s
    }
}

/// The code of a task from its place in the outline (position of each level, 1-based).
pub fn code(mask: &WbsMask, path: &[usize]) -> String {
    let mut out = mask.prefix.clone();
    let default = WbsLevel { seq: 0, len: 0, sep: ".".into() };
    for (k, &n) in path.iter().enumerate() {
        let l = mask.levels.get(k).or(mask.levels.last()).unwrap_or(&default);
        if k > 0 {
            out.push_str(&sep_after(mask, k));
        }
        out.push_str(&level_code(l, n));
    }
    out
}

/// The outline place (position under each ancestor, 1-based) of every task.
pub fn paths(tasks: &[Task]) -> Vec<Vec<usize>> {
    let mut out = Vec::with_capacity(tasks.len());
    let mut counter: Vec<usize> = Vec::new();
    for t in tasks {
        let lv = t.outline_level.max(1) as usize;
        counter.truncate(lv);
        while counter.len() < lv {
            counter.push(0);
        }
        counter[lv - 1] += 1;
        out.push(counter.clone());
    }
    out
}

/// Project > WBS > Renumber: the mask's codes for every task (or only `only`).
pub fn renumber(p: &mut Project, only: Option<&[usize]>) {
    let Some(mask) = p.wbs_mask.clone() else {
        // no mask: the codes are the outline numbers again
        for (i, t) in p.tasks.iter_mut().enumerate() {
            if only.map(|o| o.contains(&i)).unwrap_or(true) {
                t.wbs.clear();
            }
        }
        return;
    };
    let ps = paths(&p.tasks);
    for (i, t) in p.tasks.iter_mut().enumerate() {
        if only.map(|o| o.contains(&i)).unwrap_or(true) {
            t.wbs = code(&mask, &ps[i]);
        }
    }
}

/// The separator after outline level `level` (1-based) under the mask.
/// (below the mask's levels MPP carries on with the last level's sequence and ".")
pub fn sep_after(mask: &WbsMask, level: usize) -> String {
    mask.levels.get(level.saturating_sub(1)).map(|l| l.sep.clone()).unwrap_or_else(|| ".".into())
}

/// How a .mpp keeps each task's code (MPP keeps a generated code as its own level's part
/// only, after the parent's code and the separator): Some(part) for a code that follows its
/// parent's, None for one typed in (kept whole).
pub fn own_parts(p: &Project) -> Vec<Option<String>> {
    let Some(mask) = &p.wbs_mask else { return vec![None; p.tasks.len()] };
    let mut stack: Vec<usize> = Vec::new();
    let mut out = Vec::with_capacity(p.tasks.len());
    for (i, t) in p.tasks.iter().enumerate() {
        while stack.last().map(|&s| p.tasks[s].outline_level >= t.outline_level).unwrap_or(false) {
            stack.pop();
        }
        let head = match stack.last() {
            Some(&s) => format!("{}{}", p.tasks[s].wbs, sep_after(mask, p.tasks[s].outline_level as usize)),
            None => mask.prefix.clone(),
        };
        let own = if !t.wbs.is_empty() && t.wbs.len() > head.len() && t.wbs.starts_with(&head) && stack.last().map(|&s| !p.tasks[s].wbs.is_empty()).unwrap_or(true) {
            Some(t.wbs[head.len()..].to_string())
        } else {
            None
        };
        out.push(own);
        stack.push(i);
    }
    out
}

/// After reading a .mpp: generated codes (kept as their own part, marked with a leading \u{1})
/// put together with their parents' codes.
pub fn compose(p: &mut Project) {
    let Some(mask) = p.wbs_mask.clone() else {
        for t in p.tasks.iter_mut() {
            if let Some(rest) = t.wbs.strip_prefix('\u{1}') {
                t.wbs = rest.to_string();
            }
        }
        return;
    };
    let mut stack: Vec<usize> = Vec::new();
    for i in 0..p.tasks.len() {
        while stack.last().map(|&s| p.tasks[s].outline_level >= p.tasks[i].outline_level).unwrap_or(false) {
            stack.pop();
        }
        if let Some(own) = p.tasks[i].wbs.strip_prefix('\u{1}').map(|s| s.to_string()) {
            let head = match stack.last() {
                Some(&s) => format!("{}{}", p.tasks[s].wbs, sep_after(&mask, p.tasks[s].outline_level as usize)),
                None => mask.prefix.clone(),
            };
            p.tasks[i].wbs = format!("{head}{own}");
        }
        stack.push(i);
    }
}

/// Code for a task just inserted (Generate WBS code for new task): its place under the mask.
pub fn code_for_new(p: &Project, i: usize) -> Option<String> {
    let mask = p.wbs_mask.as_ref().filter(|m| m.generate)?;
    let ps = paths(&p.tasks);
    Some(code(mask, &ps[i]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes() {
        let m = WbsMask {
            prefix: "PRJ-".into(),
            levels: vec![WbsLevel { seq: 0, len: 2, sep: ".".into() }, WbsLevel { seq: 1, len: 0, sep: "-".into() }, WbsLevel { seq: 2, len: 0, sep: "/".into() }],
            generate: true,
            verify_unique: true,
        };
        assert_eq!(code(&m, &[1]), "PRJ-01");
        assert_eq!(code(&m, &[3, 2]), "PRJ-03.B");
        assert_eq!(code(&m, &[3, 28, 1]), "PRJ-03.AB-a");
        assert_eq!(letters(26, true), "Z");
        assert_eq!(letters(27, true), "AA");
    }
}
