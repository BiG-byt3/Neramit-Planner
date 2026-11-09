//! The fonts installed on this PC, for the Font box: every TrueType / OpenType family in the
//! Windows Fonts folder and in the user's own fonts folder (fonts installed "for me only").
//!
//! Only each file's table directory, `name`, `OS/2` and `cmap` headers are read (a few KB), so
//! scanning several hundred fonts stays quick; the whole file is read only when a family is used.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// One face of a family: a file and its index inside a .ttc collection.
#[derive(Clone, Debug, PartialEq)]
pub struct Face {
    pub path: PathBuf,
    pub index: u32,
}

/// An installed family under the name Windows programs show for it (name ID 1).
#[derive(Clone, Debug)]
pub struct Family {
    pub name: String,
    pub regular: Face,
    pub bold: Option<Face>,
}

static FAMILIES: OnceLock<Vec<Family>> = OnceLock::new();

/// The installed families, sorted by name (scanned once, on first use).
pub fn families() -> &'static [Family] {
    FAMILIES.get_or_init(|| scan(&font_dirs()))
}

/// Scan the font folders in the background so the Font box opens at once.
pub fn warm_up() {
    std::thread::spawn(|| {
        families();
    });
}

/// The installed family called `name` (ignoring case).
pub fn find(name: &str) -> Option<&'static Family> {
    families().iter().find(|f| f.name.eq_ignore_ascii_case(name))
}

fn font_dirs() -> Vec<PathBuf> {
    let windows = PathBuf::from(std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into())).join("Fonts");
    let user = std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("Microsoft").join("Windows").join("Fonts"));
    [Some(windows), user].into_iter().flatten().collect()
}

/// What the scan learns about one face.
struct FaceInfo {
    family: String,
    bold: bool,
    italic: bool,
}

pub(crate) fn scan(dirs: &[PathBuf]) -> Vec<Family> {
    let mut out: Vec<Family> = Vec::new();
    let mut add = |info: FaceInfo, face: Face| {
        if info.italic {
            return;
        }
        match out.iter_mut().find(|f| f.name.eq_ignore_ascii_case(&info.family)) {
            // a family met first by its bold face uses that face as regular until one turns up
            None => out.push(Family { name: info.family, regular: face.clone(), bold: info.bold.then_some(face) }),
            Some(fam) if info.bold => {
                fam.bold.get_or_insert(face);
            }
            Some(fam) => {
                if fam.bold.as_ref() == Some(&fam.regular) {
                    fam.regular = face;
                }
            }
        }
    };
    for dir in dirs {
        let Ok(rd) = std::fs::read_dir(dir) else { continue };
        let mut files: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
        files.sort();
        for path in files {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
            if !matches!(ext.as_str(), "ttf" | "otf" | "ttc" | "otc") {
                continue;
            }
            for (index, info) in read_faces(&path).into_iter().enumerate() {
                if let Some(info) = info {
                    add(info, Face { path: path.clone(), index: index as u32 });
                }
            }
        }
    }
    out.sort_by_key(|f| f.name.to_lowercase());
    out
}

fn read_at(f: &mut File, off: u64, len: usize) -> Option<Vec<u8>> {
    let mut buf = vec![0u8; len];
    f.seek(SeekFrom::Start(off)).ok()?;
    f.read_exact(&mut buf).ok()?;
    Some(buf)
}

fn be16(b: &[u8], o: usize) -> Option<u16> {
    b.get(o..o + 2).map(|x| u16::from_be_bytes([x[0], x[1]]))
}

fn be32(b: &[u8], o: usize) -> Option<u32> {
    b.get(o..o + 4).map(|x| u32::from_be_bytes([x[0], x[1], x[2], x[3]]))
}

/// Every face in a font file (several in a .ttc), `None` for faces the Font box leaves out.
fn read_faces(path: &Path) -> Vec<Option<FaceInfo>> {
    let Ok(mut f) = File::open(path) else { return Vec::new() };
    let Some(head) = read_at(&mut f, 0, 12) else { return Vec::new() };
    let offsets: Vec<u32> = if &head[0..4] == b"ttcf" {
        let n = be32(&head, 8).unwrap_or(0).min(64) as usize;
        match read_at(&mut f, 12, 4 * n) {
            Some(b) => (0..n).filter_map(|k| be32(&b, 4 * k)).collect(),
            None => return Vec::new(),
        }
    } else {
        vec![0]
    };
    offsets.into_iter().map(|off| read_face(&mut f, off as u64)).collect()
}

fn read_face(f: &mut File, off: u64) -> Option<FaceInfo> {
    let dir = read_at(f, off, 12)?;
    let n = be16(&dir, 4)? as usize;
    let recs = read_at(f, off + 12, 16 * n)?;
    let table = |tag: &[u8; 4]| {
        (0..n).map(|k| 16 * k).find(|&r| recs.get(r..r + 4) == Some(&tag[..])).and_then(|r| Some((be32(&recs, r + 8)? as u64, be32(&recs, r + 12)? as usize)))
    };

    // symbol fonts (Wingdings, Marlett ...) draw pictures for letters: not a text font
    let (cmap_off, _) = table(b"cmap")?;
    let cmap_head = read_at(f, cmap_off, 4)?;
    let cn = be16(&cmap_head, 2)? as usize;
    let cmap_recs = read_at(f, cmap_off + 4, 8 * cn)?;
    let unicode = (0..cn).any(|k| matches!((be16(&cmap_recs, 8 * k), be16(&cmap_recs, 8 * k + 2)), (Some(0), _) | (Some(3), Some(1 | 10))));
    if !unicode {
        return None;
    }

    let (name_off, name_len) = table(b"name")?;
    let name = read_at(f, name_off, name_len.min(1 << 20))?;
    let family = name_string(&name, 1)?;
    if family.is_empty() || family.starts_with('@') || family.starts_with('.') {
        return None;
    }
    let sub = name_string(&name, 2).unwrap_or_default().to_ascii_lowercase();
    let sel = table(b"OS/2").and_then(|(o, len)| read_at(f, o, len.min(64))).and_then(|b| be16(&b, 62)).unwrap_or(0);
    Some(FaceInfo {
        family,
        bold: sel & 0x20 != 0 || sub == "bold",
        italic: sel & 0x01 != 0 || sub.contains("italic") || sub.contains("oblique"),
    })
}

/// A string from the `name` table: the Windows US-English record, else any Windows one, else Mac Roman.
fn name_string(t: &[u8], id: u16) -> Option<String> {
    let count = be16(t, 2)? as usize;
    let base = be16(t, 4)? as usize;
    let mut best: Option<(u8, String)> = None;
    for k in 0..count {
        let r = 6 + 12 * k;
        let (Some(platform), Some(encoding), Some(lang), Some(nid), Some(len), Some(o)) = (be16(t, r), be16(t, r + 2), be16(t, r + 4), be16(t, r + 6), be16(t, r + 8), be16(t, r + 10)) else { break };
        if nid != id {
            continue;
        }
        let Some(raw) = t.get(base + o as usize..base + o as usize + len as usize) else { continue };
        let (rank, s) = match (platform, encoding) {
            (3, 0 | 1 | 10) => {
                let units: Vec<u16> = raw.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
                (if lang == 0x409 { 0 } else { 1 }, String::from_utf16_lossy(&units))
            }
            (1, 0) => (2, raw.iter().map(|&b| b as char).collect()),
            _ => continue,
        };
        if best.as_ref().map_or(true, |(r, _)| rank < *r) {
            best = Some((rank, s.trim().to_string()));
        }
    }
    best.map(|(_, s)| s)
}

#[cfg(test)]
mod tests {
    /// The Windows fonts the program used to list by hand are found by the scan, with a bold face.
    #[test]
    fn finds_the_windows_fonts() {
        let fams = super::families();
        if fams.is_empty() {
            return; // not on Windows
        }
        for name in ["Arial", "Tahoma", "Segoe UI", "Times New Roman"] {
            let f = super::find(name).unwrap_or_else(|| panic!("{name} not found"));
            let bold = f.bold.as_ref().unwrap_or_else(|| panic!("{name} has no bold"));
            assert_ne!(bold, &f.regular, "{name}");
        }
        assert!(super::find("Wingdings").is_none(), "symbol fonts are left out");
        assert!(fams.windows(2).all(|w| w[0].name.to_lowercase() <= w[1].name.to_lowercase()));
    }
}
