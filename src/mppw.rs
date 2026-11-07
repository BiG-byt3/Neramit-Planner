//! Writing .mpp files: a copy of the .mpp the project was opened from, with the values edited in
//! Neramit Planner written into its records. The compound file keeps every directory entry MPP
//! wrote (names, tree, CLSIDs, timestamps); only the streams that changed get new contents, and
//! the sectors, FAT, mini FAT and mini stream are laid out again (MS-CFB).

use std::collections::HashMap;

const FREESECT: u32 = 0xFFFF_FFFF;
const ENDOFCHAIN: u32 = 0xFFFF_FFFE;
const FATSECT: u32 = 0xFFFF_FFFD;
const DIFSECT: u32 = 0xFFFF_FFFC;
const NOSTREAM: u32 = 0xFFFF_FFFF;

fn put_u32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

/// Rewrite a compound file with some streams replaced (directory entry index -> contents).
pub fn rewrite_cfb(orig: &[u8], replace: &HashMap<usize, Vec<u8>>) -> Result<Vec<u8>, String> {
    let cfb = crate::mpp::Cfb::new(orig)?;
    let ssz = cfb.sector_size();
    let mssz = 64usize;
    let cutoff = 4096usize;
    let n = cfb.entry_count();
    let epf = ssz / 4; // FAT entries per sector
    // stream contents
    let data: Vec<Vec<u8>> = (0..n).map(|i| if cfb.entry_kind(i) == 2 { replace.get(&i).cloned().unwrap_or_else(|| cfb.entry_data(i)) } else { Vec::new() }).collect();

    // ---- mini stream (streams below the cutoff)
    let mut ministream: Vec<u8> = Vec::new();
    let mut minifat: Vec<u32> = Vec::new();
    let mut start = vec![ENDOFCHAIN; n];
    for i in 0..n {
        let len = data[i].len();
        if cfb.entry_kind(i) == 2 && len > 0 && len < cutoff {
            let first = minifat.len() as u32;
            let k = len.div_ceil(mssz);
            for j in 0..k {
                minifat.push(if j + 1 < k { first + j as u32 + 1 } else { ENDOFCHAIN });
            }
            ministream.extend_from_slice(&data[i]);
            ministream.resize(ministream.len() + (k * mssz - len), 0);
            start[i] = first;
        }
    }

    // ---- directory bytes (original entries, new start / size)
    let per = ssz / 128;
    let dir_entries = n.div_ceil(per) * per;
    let mut dir = vec![0u8; dir_entries * 128];
    for i in 0..dir_entries {
        let e = &mut dir[i * 128..i * 128 + 128];
        if i < n {
            e.copy_from_slice(cfb.entry_raw(i));
        } else {
            // unused entry
            put_u32(e, 68, NOSTREAM);
            put_u32(e, 72, NOSTREAM);
            put_u32(e, 76, NOSTREAM);
        }
    }

    // ---- sector runs: directory, mini FAT, mini stream, big streams
    let secs = |len: usize| len.div_ceil(ssz);
    let mut minifat_bytes: Vec<u8> = minifat.iter().flat_map(|v| v.to_le_bytes()).collect();
    minifat_bytes.resize(secs(minifat_bytes.len()) * ssz, 0xFF);
    let big: Vec<usize> = (0..n).filter(|&i| cfb.entry_kind(i) == 2 && data[i].len() >= cutoff).collect();
    let runs_len: Vec<usize> = [secs(dir.len()), secs(minifat_bytes.len()), secs(ministream.len())].into_iter().chain(big.iter().map(|&i| secs(data[i].len()))).collect();
    let data_secs: usize = runs_len.iter().sum();
    // FAT and DIFAT sectors needed to describe everything (including themselves)
    let (mut nf, mut nd) = (1usize, 0usize);
    loop {
        let total = nf + nd + data_secs;
        let need_f = total.div_ceil(epf);
        let need_d = if need_f > 109 { (need_f - 109).div_ceil(epf - 1) } else { 0 };
        if need_f == nf && need_d == nd {
            break;
        }
        nf = need_f;
        nd = need_d;
    }
    let total = nf + nd + data_secs;
    let mut fat = vec![FREESECT; nf * epf];
    for k in 0..nf {
        fat[k] = FATSECT;
    }
    for k in nf..nf + nd {
        fat[k] = DIFSECT;
    }
    // chain each run
    let mut run_start = Vec::with_capacity(runs_len.len());
    let mut next = nf + nd;
    for &len in &runs_len {
        if len == 0 {
            run_start.push(ENDOFCHAIN);
            continue;
        }
        run_start.push(next as u32);
        for j in 0..len {
            fat[next + j] = if j + 1 < len { (next + j + 1) as u32 } else { ENDOFCHAIN };
        }
        next += len;
    }
    debug_assert_eq!(next, total);
    let (dir_start, minifat_start, ministream_start) = (run_start[0], run_start[1], run_start[2]);
    for (k, &i) in big.iter().enumerate() {
        start[i] = run_start[3 + k];
    }
    // directory: start sector and size of every stream; the root entry holds the mini stream
    for i in 0..n {
        let e = &mut dir[i * 128..i * 128 + 128];
        match cfb.entry_kind(i) {
            2 => {
                let len = data[i].len();
                put_u32(e, 116, if len == 0 { ENDOFCHAIN } else { start[i] });
                put_u32(e, 120, len as u32);
                put_u32(e, 124, 0);
            }
            5 => {
                put_u32(e, 116, if ministream.is_empty() { ENDOFCHAIN } else { ministream_start });
                put_u32(e, 120, ministream.len() as u32);
                put_u32(e, 124, 0);
            }
            _ => {}
        }
    }

    // ---- header
    let mut out = vec![0u8; ssz * (1 + total)];
    out[..0x22].copy_from_slice(&orig[..0x22]);
    let h = &mut out[..512];
    put_u32(h, 0x28, if ssz == 4096 { secs(dir.len()) as u32 } else { 0 });
    put_u32(h, 0x2C, nf as u32);
    put_u32(h, 0x30, dir_start);
    put_u32(h, 0x34, 0);
    put_u32(h, 0x38, cutoff as u32);
    put_u32(h, 0x3C, if minifat.is_empty() { ENDOFCHAIN } else { minifat_start });
    put_u32(h, 0x40, if minifat.is_empty() { 0 } else { secs(minifat_bytes.len()) as u32 });
    put_u32(h, 0x44, if nd > 0 { nf as u32 } else { ENDOFCHAIN });
    put_u32(h, 0x48, nd as u32);
    for k in 0..109 {
        put_u32(h, 0x4C + k * 4, if k < nf { k as u32 } else { FREESECT });
    }
    // ---- sectors
    let sector = |k: usize| -> std::ops::Range<usize> { (k + 1) * ssz..(k + 2) * ssz };
    for k in 0..nf {
        let r = sector(k);
        let chunk: Vec<u8> = fat[k * epf..(k + 1) * epf].iter().flat_map(|v| v.to_le_bytes()).collect();
        out[r].copy_from_slice(&chunk);
    }
    // DIFAT sectors: FAT sector ids beyond the 109 in the header, last entry links the next
    for d in 0..nd {
        let r = sector(nf + d);
        let mut chunk = vec![0xFFu8; ssz];
        for j in 0..epf - 1 {
            let k = 109 + d * (epf - 1) + j;
            put_u32(&mut chunk, j * 4, if k < nf { k as u32 } else { FREESECT });
        }
        put_u32(&mut chunk, (epf - 1) * 4, if d + 1 < nd { (nf + d + 1) as u32 } else { ENDOFCHAIN });
        out[r].copy_from_slice(&chunk);
    }
    let write_run = |out: &mut Vec<u8>, first: u32, bytes: &[u8]| {
        if first == ENDOFCHAIN {
            return;
        }
        let o = (first as usize + 1) * ssz;
        out[o..o + bytes.len()].copy_from_slice(bytes);
    };
    write_run(&mut out, dir_start, &dir);
    write_run(&mut out, minifat_start, &minifat_bytes);
    write_run(&mut out, ministream_start, &ministream);
    for &i in &big {
        write_run(&mut out, start[i], &data[i]);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A rewritten copy holds the same streams (checked on the real files when
    /// NERAMIT_MPP_TESTDATA is set).
    #[test]
    fn rewrite_keeps_every_stream() {
        let Ok(dir) = std::env::var("NERAMIT_MPP_TESTDATA") else { return };
        for e in std::fs::read_dir(&dir).unwrap() {
            let path = e.unwrap().path();
            if path.extension().map(|x| x != "mpp").unwrap_or(true) {
                continue;
            }
            let orig = std::fs::read(&path).unwrap();
            let copy = rewrite_cfb(&orig, &HashMap::new()).unwrap();
            let a = crate::mpp::Cfb::new(&orig).unwrap();
            let b = crate::mpp::Cfb::new(&copy).unwrap();
            assert_eq!(a.entry_count(), b.entry_count());
            for i in 0..a.entry_count() {
                if a.entry_kind(i) == 2 {
                    assert_eq!(a.entry_data(i), b.entry_data(i), "{}: stream {i}", path.display());
                }
            }
            let pa = crate::mpp::load_mpp(&orig).unwrap();
            let pb = crate::mpp::load_mpp(&copy).unwrap();
            assert_eq!(pa.tasks.len(), pb.tasks.len());
        }
    }

    /// Writes an edited copy of a real file for MPP to check (NERAMIT_MPP_EDIT =
    /// "input.mpp;output.mpp;expected.xml"): a longer Thai name, a longer duration that moves its
    /// successors, 50 % complete, a changed lag, a constraint and a new title.
    #[test]
    fn write_edited_copy() {
        let Ok(spec) = std::env::var("NERAMIT_MPP_EDIT") else { return };
        let parts: Vec<&str> = spec.split(';').collect();
        let orig = std::fs::read(parts[0]).unwrap();
        let mut p = crate::mpp::load_mpp(&orig).unwrap();
        crate::schedule::schedule(&mut p).unwrap();
        let cal = p.calendar.clone();
        let mpd = cal.minutes_per_day();
        // the first leaf tasks with and without links
        let leaves: Vec<usize> = (0..p.tasks.len()).filter(|&i| !p.tasks[i].is_summary && !p.tasks[i].is_milestone()).collect();
        let linked = leaves.iter().copied().find(|&i| !p.tasks[i].predecessors.is_empty()).unwrap_or(leaves[0]);
        let (a, b, c) = (leaves[0], leaves[1], leaves[2]);
        let modes = parts.get(3).copied().unwrap_or("name,dur,lag,pct,snet,title");
        let on = |m: &str| modes.split(',').any(|x| x == m);
        if on("name") {
            p.tasks[a].name = format!("{} - แก้ไขโดย Neramit Planner", p.tasks[a].name);
        }
        if on("samename") {
            // same length: written in place
            let n: String = p.tasks[a].name.chars().rev().collect();
            p.tasks[a].name = n;
        }
        if on("dur") {
            let d = p.tasks[linked].dur_minutes(mpd);
            let (u, e) = (p.tasks[linked].dur_unit, p.tasks[linked].dur_elapsed);
            p.tasks[linked].set_duration_minutes(d + 2 * mpd, u, e, mpd);
        }
        if on("lag") {
            p.tasks[linked].predecessors[0].set_lag_minutes(mpd, crate::model::DurUnit::Days, false, mpd);
        }
        if on("pct") {
            p.tasks[b].set_percent(50, &cal);
        }
        let snet = p.tasks[c].start + chrono::Duration::days(7);
        if on("snet") {
            p.tasks[c].set_snet(snet);
        }
        if on("title") {
            p.title = "Neramit Planner edit test".into();
        }
        if on("res") {
            // a new rate for Crew, the engineer full time on task B (Fixed Units: shorter)
            if let Some(r) = p.resources.iter_mut().find(|r| r.name == "Crew") {
                r.std_rate = 55.0;
            }
            let eng = p.resources.iter().find(|r| r.name == "Engineer").map(|r| r.uid).unwrap();
            if let Some(t) = p.tasks.iter_mut().find(|t| t.name.starts_with("R1 B")) {
                if let Some(x) = t.assignments.iter_mut().find(|x| x.res_uid == eng) {
                    x.units = 1.0;
                    x.work_min = 1920;
                }
                t.work_ref_dur = t.dur_minutes(mpd);
            }
        }
        if on("indent") {
            // indent the task under the one above it (that one becomes a summary)
            let k = leaves.iter().copied().find(|&i| i > 0 && p.tasks[i - 1].outline_level == p.tasks[i].outline_level && !p.tasks[i - 1].is_summary).unwrap();
            p.tasks[k].outline_level += 1;
        }
        crate::schedule::schedule(&mut p).unwrap();
        let bytes = crate::mppt::save_mpp(&orig, &p).unwrap();
        std::fs::write(parts[1], &bytes).unwrap();
        crate::fileio::save(std::path::Path::new(parts[2]), &p).unwrap();
        // our own reader sees the edits
        let q = crate::mpp::load_mpp(&bytes).unwrap();
        assert_eq!(q.tasks[a].name, p.tasks[a].name);
        assert_eq!(q.tasks[linked].dur_min, p.tasks[linked].dur_min);
        assert_eq!(q.tasks[b].percent_complete, p.tasks[b].percent_complete);
        assert_eq!(q.tasks[c].constraint_date, p.tasks[c].constraint_date);
        assert_eq!(q.title, p.title);
        let _ = snet;
        println!("edited: name {a}, duration+lag {linked}, 50% {b}, SNET {c}");
    }
}
