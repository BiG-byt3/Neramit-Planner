//! Links from a .mpp to other files (MPP14 files): the resource pool a project
//! shares and the projects its external tasks come from.
//!
//! - Props 0x024000A2, the link table: u32 size, u32 n, n x (u16 offset, u16 field) and the fields
//!   one after the other: field 0 the number of links; for link i (base 64 i) field 8 + base its
//!   GUID, u16 kind (8 tasks of another project, 4 the resource pool) twice; 24 + base the UIDs of
//!   this project's external tasks from it (u32 each); 32 + base the file as an absolute OLE file
//!   moniker; 56 + base the same relative to this file.
//! - A file moniker (MS-OLEDS 2.3.7): CLSID {00000303-0000-0000-C000-000000000046}, u16 cAnti
//!   (the ".." of a relative one: 1 for the same folder), u32 + the ANSI path and its NUL,
//!   u16 0xFFFF, u16 0xDEAD, 20 zero bytes, u32 size of what follows (0, or 6 + the Unicode
//!   path's bytes: u32 its bytes, u16 3, the path).
//! - Resource pool: Props 0x0240000B the pool's moniker, 0x0240000F 3 own resources, 2 the pool's
//!   (pool takes precedence) or 1 (sharer takes precedence); 0x02400024 = 1 and 0x024000AF = 1
//!   while sharing.
//! - Links between projects: an external task is a task row of its own (var 0x0B4000F2 its UID in
//!   the other project and flags, 0x0B4000FF its ID there); Props 0x024000A4 counts the projects.

pub const P_LINKS: u32 = 0x0240_00A2;
pub const P_POOL_MONIKER: u32 = 0x0240_000B;
pub const P_POOL_MODE: u32 = 0x0240_000F;
pub const P_POOL_ON: u32 = 0x0240_0024;
pub const P_POOL_COUNT: u32 = 0x0240_00AF;
pub const P_EXT_PROJECTS: u32 = 0x0240_00A4;
pub const T_EXT_UID: u32 = 0x0B40_00F2;
pub const T_EXT_ID: u32 = 0x0B40_00FF;
pub const KIND_TASKS: u16 = 8;
pub const KIND_POOL: u16 = 4;
const CLSID: [u8; 16] = [0x03, 0x03, 0, 0, 0, 0, 0, 0, 0xc0, 0, 0, 0, 0, 0, 0, 0x46];

#[derive(Clone, Debug, PartialEq)]
pub struct FileLink {
    pub guid: [u8; 16],
    pub kind: u16,
    pub uids: Vec<u32>,
    /// the absolute and relative monikers as the file has them (written back as they are)
    pub abs: Vec<u8>,
    pub rel: Vec<u8>,
}

impl FileLink {
    pub fn path(&self) -> String {
        moniker_path(&self.abs).unwrap_or_default()
    }

    /// A new link to `path` from the file at `this` (its folder decides the relative moniker).
    pub fn new(guid: [u8; 16], kind: u16, path: &str, this: Option<&std::path::Path>) -> FileLink {
        let file_name = std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| path.to_string());
        let same_dir = this.and_then(|t| t.parent()).zip(std::path::Path::new(path).parent()).map(|(a, b)| a == b).unwrap_or(false);
        let rel = if same_dir { moniker(&file_name, 1) } else { moniker(path, 0) };
        FileLink { guid, kind, uids: Vec::new(), abs: moniker(path, 0), rel }
    }
}

/// Props 0x02400029: the GUID MPP gives a project file when saving it; a link to the file
/// names it (the link records' GUIDs are those of the files linked; with another one
/// MPP does not trust the link, e.g. it shows the external task's ID as 0).
pub const P_PROJECT_GUID: u32 = 0x0240_0029;

/// The GUID of the project file at `path` (a .mpp), for a link to it.
pub fn project_guid(path: &str) -> Option<[u8; 16]> {
    let bytes = std::fs::read(path).ok()?;
    let cfb = crate::mpp::Cfb::new(&bytes).ok()?;
    let v = crate::mpp::version_of(&cfb).ok()?;
    let props = crate::mpp::parse_props(&cfb.stream(&[v.root, "Props"]));
    props.get(&P_PROJECT_GUID)?.get(..16)?.try_into().ok()
}

/// An OLE file moniker for `path` (`anti`: the ".." count of a relative one).
pub fn moniker(path: &str, anti: u16) -> Vec<u8> {
    let mut m = CLSID.to_vec();
    m.extend_from_slice(&anti.to_le_bytes());
    let ascii = path.is_ascii();
    let ansi: Vec<u8> = path.chars().map(|c| if c.is_ascii() { c as u8 } else { b'?' }).chain(std::iter::once(0)).collect();
    m.extend_from_slice(&(ansi.len() as u32).to_le_bytes());
    m.extend_from_slice(&ansi);
    m.extend_from_slice(&0xFFFFu16.to_le_bytes());
    m.extend_from_slice(&0xDEADu16.to_le_bytes());
    m.extend_from_slice(&[0u8; 20]);
    if ascii && anti > 0 {
        m.extend_from_slice(&0u32.to_le_bytes());
    } else {
        let u: Vec<u8> = path.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
        m.extend_from_slice(&((u.len() + 6) as u32).to_le_bytes());
        m.extend_from_slice(&(u.len() as u32).to_le_bytes());
        m.extend_from_slice(&3u16.to_le_bytes());
        m.extend_from_slice(&u);
    }
    m
}

/// The path of a file moniker (its Unicode path when it has one).
pub fn moniker_path(m: &[u8]) -> Option<String> {
    use crate::mpp::{u16_at, u32_at};
    if m.get(..16)? != CLSID {
        return None;
    }
    let alen = u32_at(m, 18)? as usize;
    let ansi = m.get(22..22 + alen)?;
    let ansi: String = ansi.iter().take_while(|b| **b != 0).map(|b| *b as char).collect();
    let k = 22 + alen + 4 + 20;
    let size = u32_at(m, k)? as usize;
    if size >= 6 {
        let ub = u32_at(m, k + 4)? as usize;
        let _key = u16_at(m, k + 8)?;
        let u: Vec<u16> = m.get(k + 10..k + 10 + ub)?.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        return Some(String::from_utf16_lossy(&u));
    }
    Some(ansi)
}

/// The links of Props 0x024000A2.
pub fn parse_links(b: &[u8]) -> Vec<FileLink> {
    use crate::mpp::{u16_at, u32_at};
    let n = u32_at(b, 4).unwrap_or(0) as usize;
    let pairs: Vec<(usize, u16)> = (0..n.min(4096)).filter_map(|i| Some((u16_at(b, 8 + 4 * i)? as usize, u16_at(b, 10 + 4 * i)?))).collect();
    let end_of = |k: usize| pairs.get(k + 1).map(|p| p.0).unwrap_or(b.len());
    let mut out: Vec<FileLink> = Vec::new();
    for (k, &(off, field)) in pairs.iter().enumerate() {
        let seg = b.get(off..end_of(k).max(off)).unwrap_or(&[]);
        if field == 0 {
            continue;
        }
        let i = ((field as usize).saturating_sub(8)) / 64;
        while out.len() <= i {
            out.push(FileLink { guid: [0; 16], kind: 0, uids: Vec::new(), abs: Vec::new(), rel: Vec::new() });
        }
        match field as usize % 64 {
            8 => {
                if seg.len() >= 18 {
                    out[i].guid = seg[..16].try_into().unwrap();
                    out[i].kind = u16_at(seg, 16).unwrap_or(0);
                }
            }
            24 => out[i].uids = seg.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect(),
            32 => out[i].abs = seg.to_vec(),
            56 => out[i].rel = seg.to_vec(),
            _ => {}
        }
    }
    out.retain(|l| !l.abs.is_empty());
    out
}

/// Props 0x024000A2 for `links` (none: the standard empty table).
pub fn build_links(links: &[FileLink]) -> Vec<u8> {
    let mut fields: Vec<(u16, Vec<u8>)> = vec![(0, (links.len() as u32).to_le_bytes().to_vec())];
    for (i, l) in links.iter().enumerate() {
        let base = 64 * i as u16;
        let mut g = l.guid.to_vec();
        g.extend_from_slice(&l.kind.to_le_bytes());
        g.extend_from_slice(&l.kind.to_le_bytes());
        fields.push((8 + base, g));
        if !l.uids.is_empty() {
            fields.push((24 + base, l.uids.iter().flat_map(|u| u.to_le_bytes()).collect()));
        }
        fields.push((32 + base, l.abs.clone()));
        fields.push((56 + base, l.rel.clone()));
    }
    let head = 8 + 4 * fields.len();
    let total = head + fields.iter().map(|f| f.1.len()).sum::<usize>();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(fields.len() as u32).to_le_bytes());
    let mut at = head;
    for (field, data) in &fields {
        out.extend_from_slice(&(at as u16).to_le_bytes());
        out.extend_from_slice(&field.to_le_bytes());
        at += data.len();
    }
    for (_, data) in fields {
        out.extend_from_slice(&data);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Link tables written (NERAMIT_LINK_SAMPLES: sharer.mpp, main2.mpp) read and built again
    /// to the same bytes.
    #[test]
    fn link_tables_round_trip() {
        assert_eq!(build_links(&[]), vec![16, 0, 0, 0, 1, 0, 0, 0, 12, 0, 0, 0, 0, 0, 0, 0]);
        let m = moniker("pool.mpp", 1);
        assert_eq!(m.len(), 59);
        assert_eq!(moniker_path(&m).as_deref(), Some("pool.mpp"));
        let m = moniker("D:\\แผนงาน\\pool.mpp", 0);
        assert_eq!(moniker_path(&m).as_deref(), Some("D:\\แผนงาน\\pool.mpp"));
        let Ok(dir) = std::env::var("NERAMIT_LINK_SAMPLES") else { return };
        for (f, kinds) in [("sharer.mpp", vec![KIND_POOL]), ("main2.mpp", vec![KIND_TASKS, KIND_TASKS])] {
            let bytes = std::fs::read(std::path::Path::new(&dir).join(f)).unwrap();
            let cfb = crate::mpp::Cfb::new(&bytes).unwrap();
            let props = crate::mpp::parse_props(&cfb.stream(&["   114", "Props"]));
            let raw = props.get(&P_LINKS).unwrap();
            let links = parse_links(raw);
            assert_eq!(links.iter().map(|l| l.kind).collect::<Vec<_>>(), kinds, "{f}");
            assert_eq!(&build_links(&links), raw, "{f}");
            println!("{f}: {:?}", links.iter().map(|l| (l.path(), moniker_path(&l.rel), l.uids.clone())).collect::<Vec<_>>());
        }
    }
}
