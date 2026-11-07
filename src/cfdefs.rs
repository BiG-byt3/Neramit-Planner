//! Project > Custom Fields in .mpp files (MPP14 files): renamed fields, formulas
//! and lookup tables.
//!
//! - The task / resource table's own Props, item 0x04400001, four sections. 1, aliases: u32 size
//!   (counted from byte 4), the size again, u32 count, count x (u32 field, u32 offset of the name
//!   from byte 4), the names (UTF-16, NUL-terminated; newer MPP14 give each 104 bytes).
//!   2 and 3: empty in every file seen (u32 8, u32 8, u32 0). 4, field attributes: u32 count, u32
//!   size, count x 88 bytes - u32 field, u32 kind (8 lookup table, 5 rollup), u32 lookup id
//!   (0x0F404064 + n, else -1), u32, u32 0, the field's GUID (000039B7-8BBE-4CEB-82C4-FA8C0B4000xx),
//!   its lookup table's GUID, the default value's GUID, 20 bytes.
//! - The project's Props 0x02401390, formulas: u32 size, u32 count, count x (u32 field, u32 offset),
//!   records of u32 flags (0x0D; 0x50 | rollup << 12 with a summary rollup), u32 -1, the record's
//!   length seven times, the formula (UTF-16 NUL-terminated, padded to 8 bytes) naming fields as
//!   [MSPJ<field number>]. Props 0x02401392 = 1 once the project has custom field definitions;
//!   0x024013C5 lists the fields with a lookup table (u32 0, u32 count, (field, lookup id) pairs).
//! - TBkndOutlCode: one row per lookup value after four rows of its own - FixedData u32 uid, u32
//!   uid, u32 1, u16 0; Fixed2Data the value's GUID, u16 0x15, the table's GUID; var 0x16400016 the
//!   value, 0x16400008 its description, 0x16400017 its place in the list (f64).

use crate::custom::{CustomField, CustomFieldDef};
use crate::mpp::{f64_at, u16_at, u32_at, utf16, utf16z};

pub const DEFS_ITEM: u32 = 0x0440_0001;
pub const P_FORMULAS: u32 = 0x0240_1390;
pub const P_HAS_DEFS: u32 = 0x0240_1392;
pub const P_LOOKUP_FIELDS: u32 = 0x0240_13C5;
pub const P_LOOKUP_MASKS: u32 = 0x0240_138C;
pub const V_VALUE: u32 = 0x1640_0016;
pub const V_DESCRIPTION: u32 = 0x1640_0008;
pub const V_ORDER: u32 = 0x1640_0017;
const NAME_SLOT: usize = 104;

/// The definitions item of a table's Props.
#[derive(Clone, Debug, Default)]
pub struct DefsItem {
    pub aliases: Vec<(u32, String)>,
    /// sections 2 and 3, as the file has them
    pub middle: Vec<u8>,
    /// section 4 entries (88 bytes each)
    pub attrs: Vec<Vec<u8>>,
    /// an older layout (sections 1 - 4 alike): kept as it is
    pub raw_tail: Option<Vec<u8>>,
}

fn section(b: &[u8], o: usize) -> Option<(usize, &[u8])> {
    let size = u32_at(b, o)? as usize;
    let end = o + 4 + size;
    Some((end, b.get(o..end)?))
}

/// The definitions item (0x04400001) of a TBkndTask / TBkndRsc Props stream.
pub fn parse_item(b: &[u8], v14: bool) -> Option<DefsItem> {
    let (end1, s1) = section(b, 0)?;
    let count = u32_at(s1, 8)? as usize;
    let mut aliases = Vec::new();
    for k in 0..count.min(500) {
        let field = u32_at(s1, 12 + 8 * k)?;
        let off = u32_at(s1, 16 + 8 * k)? as usize + 4;
        let name: Vec<u16> = s1.get(off..)?.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|c| *c != 0).collect();
        let name = String::from_utf16_lossy(&name);
        if !name.trim().is_empty() {
            aliases.push((field, name));
        }
    }
    let (end2, _) = section(b, end1)?;
    let (end3, _) = section(b, end2)?;
    let middle = b[end1..end3].to_vec();
    let mut item = DefsItem { aliases, middle, attrs: Vec::new(), raw_tail: None };
    if !v14 {
        item.raw_tail = Some(b[end3..].to_vec());
        return Some(item);
    }
    let n = u32_at(b, end3).unwrap_or(0) as usize;
    for k in 0..n.min(500) {
        let o = end3 + 8 + 88 * k;
        if let Some(e) = b.get(o..o + 88) {
            item.attrs.push(e.to_vec());
        }
    }
    Some(item)
}

/// The item again with `aliases` (field, name) in field order.
pub fn build_item(item: &DefsItem) -> Vec<u8> {
    let mut aliases = item.aliases.clone();
    aliases.sort_by_key(|a| a.0);
    aliases.dedup_by_key(|a| a.0);
    let head = 12 + 8 * aliases.len();
    let mut s1 = vec![0u8; head + NAME_SLOT * aliases.len()];
    let size = (s1.len() - 4) as u32;
    s1[0..4].copy_from_slice(&size.to_le_bytes());
    s1[4..8].copy_from_slice(&size.to_le_bytes());
    s1[8..12].copy_from_slice(&(aliases.len() as u32).to_le_bytes());
    for (k, (field, name)) in aliases.iter().enumerate() {
        let at = head + NAME_SLOT * k;
        s1[12 + 8 * k..16 + 8 * k].copy_from_slice(&field.to_le_bytes());
        s1[16 + 8 * k..20 + 8 * k].copy_from_slice(&((at - 4) as u32).to_le_bytes());
        // (MPP keeps at most 51 characters and the NUL)
        let mut z: Vec<u8> = name.encode_utf16().take(NAME_SLOT / 2 - 1).flat_map(|c| c.to_le_bytes()).collect();
        z.resize(NAME_SLOT, 0);
        s1[at..at + NAME_SLOT].copy_from_slice(&z);
    }
    let mut out = s1;
    out.extend_from_slice(&item.middle);
    match &item.raw_tail {
        Some(t) => out.extend_from_slice(t),
        None => {
            out.extend_from_slice(&(item.attrs.len() as u32).to_le_bytes());
            out.extend_from_slice(&((88 * item.attrs.len()) as u32).to_le_bytes());
            for e in &item.attrs {
                out.extend_from_slice(e);
            }
        }
    }
    out
}

/// An empty item (MPP14).
#[cfg(test)]
pub fn empty_item() -> DefsItem {
    let mut middle = Vec::new();
    for _ in 0..2 {
        middle.extend_from_slice(&[8, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0]);
    }
    DefsItem { aliases: Vec::new(), middle, attrs: Vec::new(), raw_tail: None }
}

/// The formulas of Props 0x02401390: (field, flags, formula as stored).
pub fn parse_formulas(b: &[u8]) -> Vec<(u32, u32, String)> {
    let n = u32_at(b, 4).unwrap_or(0) as usize;
    let mut out = Vec::new();
    for k in 0..n.min(2000) {
        let (Some(field), Some(off)) = (u32_at(b, 8 + 8 * k), u32_at(b, 12 + 8 * k)) else { break };
        let off = off as usize;
        let Some(flags) = u32_at(b, off) else { continue };
        let text: Vec<u16> = b.get(off + 36..).unwrap_or(&[]).chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|c| *c != 0).collect();
        out.push((field, flags, String::from_utf16_lossy(&text)));
    }
    out
}

/// Props 0x02401390 from (field, flags, formula as stored), in field order.
pub fn build_formulas(list: &[(u32, u32, String)]) -> Vec<u8> {
    let mut list = list.to_vec();
    list.sort_by_key(|f| f.0);
    let mut recs: Vec<Vec<u8>> = Vec::new();
    for (_, flags, text) in &list {
        let mut s = utf16z(text);
        let padded = s.len().div_ceil(8) * 8;
        s.resize(padded, 0);
        let len = (36 + s.len()) as u32;
        let mut r = Vec::with_capacity(len as usize);
        r.extend_from_slice(&flags.to_le_bytes());
        r.extend_from_slice(&u32::MAX.to_le_bytes());
        for _ in 0..7 {
            r.extend_from_slice(&len.to_le_bytes());
        }
        r.extend_from_slice(&s);
        recs.push(r);
    }
    let head = 8 + 8 * list.len();
    let total = head + recs.iter().map(|r| r.len()).sum::<usize>();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(list.len() as u32).to_le_bytes());
    let mut at = head;
    for ((field, _, _), r) in list.iter().zip(&recs) {
        out.extend_from_slice(&field.to_le_bytes());
        out.extend_from_slice(&(at as u32).to_le_bytes());
        at += r.len();
    }
    for r in recs {
        out.extend_from_slice(&r);
    }
    out
}

/// The name of a field by field number.
pub fn field_name(id: u32) -> Option<&'static str> {
    crate::field_ids::FIELDS.iter().find(|f| f.0 == id).map(|f| f.1)
}

/// The field number of a task / resource field by name (the standard: of two numbers with one name,
/// the later, e.g. Start = 188744965).
pub fn field_by_name(name: &str, resource: bool) -> Option<u32> {
    let (lo, hi) = if resource { (205_520_896u32, 222_298_112u32) } else { (188_743_680u32, 205_520_896u32) };
    crate::field_ids::FIELDS.iter().filter(|f| f.0 >= lo && f.0 < hi && f.1.eq_ignore_ascii_case(name.trim())).map(|f| f.0).max()
}

/// Apply `f` to every [..] reference outside "quoted" text.
fn map_refs(s: &str, mut f: impl FnMut(&str) -> Option<String>) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    let mut quoted = false;
    while let Some(c) = chars.next() {
        if c == '"' {
            quoted = !quoted;
            out.push(c);
        } else if c == '[' && !quoted {
            let mut name = String::new();
            let mut closed = false;
            for d in chars.by_ref() {
                if d == ']' {
                    closed = true;
                    break;
                }
                name.push(d);
            }
            match f(&name) {
                Some(r) if closed => {
                    out.push('[');
                    out.push_str(&r);
                    out.push(']');
                }
                _ => {
                    out.push('[');
                    out.push_str(&name);
                    if closed {
                        out.push(']');
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// A formula as stored ([MSPJ188743713]) with field names ([% Complete]).
pub fn formula_from_file(s: &str) -> String {
    map_refs(s, |name| {
        let id: u32 = name.strip_prefix("MSPJ")?.parse().ok()?;
        field_name(id).map(|n| n.to_string())
    })
}

/// A formula with field names (or a renamed field's alias) as stored in the file.
pub fn formula_to_file(s: &str, resource: bool, defs: &[CustomFieldDef]) -> String {
    map_refs(s, |name| {
        let id = field_by_name(name, resource).or_else(|| {
            defs.iter().find(|d| d.resource == resource && d.alias.trim().eq_ignore_ascii_case(name.trim())).map(|d| d.field.field_id(resource))
        })?;
        Some(format!("MSPJ{id}"))
    })
}

/// The custom field of a field number, with whether it is a resource field.
pub fn custom_of(id: u32) -> Option<(CustomField, bool)> {
    crate::custom::by_field_id(id)
}

/// A lookup value in the type its field has (TBkndOutlCode Fixed2Data's code: 0x15 text and dates,
/// 0x0F number, 0x09 cost in cents, 0x06 duration in tenths of a minute and its format).
pub fn decode_value(code: u16, v: &[u8]) -> String {
    match code {
        0x0F => f64_at(v, 0).map(fmt_num).unwrap_or_default(),
        0x09 => f64_at(v, 0).map(|c| fmt_num(c / 100.0)).unwrap_or_default(),
        0x06 => {
            let tenths = u32_at(v, 0).unwrap_or(0) as f64;
            let fmt = u16_at(v, 4).unwrap_or(7) as i64;
            let (unit, elapsed) = crate::model::DurUnit::from_code(fmt);
            let per = match unit {
                crate::model::DurUnit::Minutes => 1.0,
                crate::model::DurUnit::Hours => 60.0,
                crate::model::DurUnit::Days => if elapsed { 1440.0 } else { 480.0 },
                crate::model::DurUnit::Weeks => if elapsed { 10080.0 } else { 2400.0 },
                crate::model::DurUnit::Months => if elapsed { 43200.0 } else { 9600.0 },
            };
            let short = match unit {
                crate::model::DurUnit::Minutes => "m",
                crate::model::DurUnit::Hours => "h",
                crate::model::DurUnit::Days => "d",
                crate::model::DurUnit::Weeks => "w",
                crate::model::DurUnit::Months => "mo",
            };
            format!("{}{}{}", fmt_num(tenths / 10.0 / per), if elapsed { "e" } else { "" }, short)
        }
        _ => utf16(v),
    }
}

fn fmt_num(x: f64) -> String {
    if (x - x.round()).abs() < 1e-9 {
        format!("{}", x.round() as i64)
    } else {
        format!("{}", (x * 1e6).round() / 1e6)
    }
}

/// The type codes of a custom field kind: (section 4 data type, section 4 field type, lookup value
/// code, FixedMeta bytes 8-9).
pub fn type_codes(kind: crate::custom::CfKind) -> Option<(u32, u32, u16, u16)> {
    use crate::custom::CfKind as K;
    Some(match kind {
        K::Text => (8, 7, 0x15, 0x03d3),
        K::Number => (5, 5, 0x0F, 0x03d3),
        K::Cost => (5, 0, 0x09, 0x03d3),
        K::Duration => (3, 2, 0x06, 0x03d3),
        K::Date => (19, 1, 0x15, 0x0393),
        K::Start => (19, 6, 0x15, 0x0393),
        K::Finish => (19, 3, 0x15, 0x0393),
        K::Flag => return None,
    })
}

/// A lookup value as the file keeps it (`mpd` minutes per day for durations).
pub fn encode_value(kind: crate::custom::CfKind, value: &str, du: crate::util::DurUnits) -> Vec<u8> {
    use crate::custom::CfKind as K;
    let num = || value.trim().replace(',', "").trim_start_matches(['$', '฿']).parse::<f64>().unwrap_or(0.0);
    match kind {
        K::Number => num().to_le_bytes().to_vec(),
        K::Cost => (num() * 100.0).round().to_le_bytes().to_vec(),
        K::Duration => {
            let (m, unit, el) = crate::util::parse_dur(value, du, crate::model::DurUnit::Days).unwrap_or((0, crate::model::DurUnit::Days, false));
            let mut v = ((m * 10) as u32).to_le_bytes().to_vec();
            v.extend_from_slice(&(unit.code(el) as u16).to_le_bytes());
            v
        }
        _ => utf16z(value),
    }
}

/// The lookup tables of the file: table GUID -> (value, description) in list order.
pub fn lookup_values(cfb: &crate::mpp::Cfb<'_>, rt: &str) -> std::collections::HashMap<[u8; 16], Vec<(String, String)>> {
    let mut out: std::collections::HashMap<[u8; 16], Vec<(f64, String, String)>> = std::collections::HashMap::new();
    let t = "TBkndOutlCode";
    let (m, d, m2, d2) = (cfb.stream(&[rt, t, "FixedMeta"]), cfb.stream(&[rt, t, "FixedData"]), cfb.stream(&[rt, t, "Fixed2Meta"]), cfb.stream(&[rt, t, "Fixed2Data"]));
    let var = crate::mpp::parse_var(&cfb.stream(&[rt, t, "VarMeta"]), &cfb.stream(&[rt, t, "Var2Data"]));
    let n = u32_at(&m, 8).unwrap_or(0) as usize;
    for i in 0..n {
        let b = 16 + 10 * i;
        if m.get(b) != Some(&0) {
            continue;
        }
        let (Some(off), Some(off2)) = (u32_at(&m, b + 4), u32_at(&m2, b + 4)) else { continue };
        let (off, off2) = (off as usize, off2 as usize);
        let Some(uid) = u32_at(&d, off) else { continue };
        let (Some(code), Some(g)) = (u16_at(&d2, off2 + 16), d2.get(off2 + 18..off2 + 34)) else { continue };
        let table: [u8; 16] = g.try_into().unwrap();
        let Some(value) = var.get(&(uid, V_VALUE)).map(|v| decode_value(code, v)) else { continue };
        let desc = var.get(&(uid, V_DESCRIPTION)).map(|v| utf16(v)).unwrap_or_default();
        let order = var.get(&(uid, V_ORDER)).and_then(|v| f64_at(v, 0)).unwrap_or(uid as f64);
        out.entry(table).or_default().push((order, value, desc));
    }
    out.into_iter()
        .map(|(k, mut v)| {
            v.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            (k, v.into_iter().map(|x| (x.1, x.2)).collect())
        })
        .collect()
}

/// Props 0x0240138C with the code masks of new lookup tables and without those of tables gone:
/// u32 count, (table GUID, u32 levels, levels x 18 bytes: u32 sequence - 3 characters, 7 numbers,
/// 5 cost, 4 dates and starts, 9 finishes, 6 durations - u32 separator, u32 length, u16 0x0105,
/// u32 0).
pub fn rewrite_masks(old: &[u8], add: &[([u8; 16], crate::custom::CfKind)], gone: &[[u8; 16]]) -> Vec<u8> {
    use crate::custom::CfKind as K;
    let n = u32_at(old, 0).unwrap_or(0) as usize;
    let mut entries: Vec<Vec<u8>> = Vec::new();
    let mut o = 4;
    for _ in 0..n {
        let Some(levels) = u32_at(old, o + 16) else { break };
        let end = o + 20 + 18 * levels as usize;
        let Some(e) = old.get(o..end) else { break };
        if !gone.iter().any(|g| g[..] == e[..16]) {
            entries.push(e.to_vec());
        }
        o = end;
    }
    for (g, kind) in add {
        let seq: u32 = match kind {
            K::Text => 3,
            K::Number => 7,
            K::Cost => 5,
            K::Duration => 6,
            K::Finish => 9,
            _ => 4,
        };
        let mut e = g.to_vec();
        e.extend_from_slice(&1u32.to_le_bytes());
        e.extend_from_slice(&seq.to_le_bytes());
        e.extend_from_slice(&(b'.' as u32).to_le_bytes());
        e.extend_from_slice(&0u32.to_le_bytes());
        e.extend_from_slice(&0x0105u16.to_le_bytes());
        e.extend_from_slice(&0u32.to_le_bytes());
        entries.push(e);
    }
    let mut out = (entries.len() as u32).to_le_bytes().to_vec();
    for e in entries {
        out.extend_from_slice(&e);
    }
    out.extend_from_slice(old.get(o..).unwrap_or(&[]));
    out
}

/// Props 0x024013C5 without the lookup ids of `fields` (u32 0, u32 count, (field, id) pairs).
pub fn without_lookup_fields(old: &[u8], fields: &[u32]) -> Vec<u8> {
    let n = u32_at(old, 4).unwrap_or(0) as usize;
    let pairs: Vec<(u32, u32)> = (0..n).filter_map(|k| Some((u32_at(old, 8 + 8 * k)?, u32_at(old, 12 + 8 * k)?))).filter(|p| !fields.contains(&p.0)).collect();
    let mut out = old.get(..4).unwrap_or(&[0, 0, 0, 0]).to_vec();
    out.extend_from_slice(&(pairs.len() as u32).to_le_bytes());
    for (f, id) in pairs {
        out.extend_from_slice(&f.to_le_bytes());
        out.extend_from_slice(&id.to_le_bytes());
    }
    out
}

/// The outline code sequence of a field kind's lookup table (the standard mask Type).
pub fn mask_sequence(kind: crate::custom::CfKind) -> u32 {
    use crate::custom::CfKind as K;
    match kind {
        K::Text => 3,
        K::Number => 7,
        K::Cost => 5,
        K::Duration => 6,
        K::Finish => 9,
        _ => 4,
    }
}

/// A GUID for a field's lookup table (n = 0) and its values (n = 1, 2 ..) in Project XML.
pub fn xml_table_guid(field: u32, n: u32) -> String {
    format!("{:08X}-B1C5-4B1D-8F00-{:04X}{:08X}", 0x5A11_0000u32 + n, n & 0xFFFF, field)
}

/// A lookup value as Project XML writes it (Type 15 numbers "1.00", 9 cost in cents, 6 durations
/// in tenths of a minute).
pub fn xml_value(kind: crate::custom::CfKind, value: &str, du: crate::util::DurUnits) -> String {
    use crate::custom::CfKind as K;
    let b = encode_value(kind, value, du);
    match kind {
        K::Number => format!("{:.2}", f64_at(&b, 0).unwrap_or(0.0)),
        K::Cost => format!("{:.2}", f64_at(&b, 0).unwrap_or(0.0)),
        K::Duration => format!("{}", u32_at(&b, 0).unwrap_or(0)),
        _ => value.to_string(),
    }
}

/// A lookup value of Project XML as Neramit Planner shows it.
pub fn from_xml_value(ty: i64, v: &str) -> String {
    match ty {
        15 => v.trim().parse::<f64>().map(fmt_num).unwrap_or_else(|_| v.to_string()),
        9 => v.trim().parse::<f64>().map(|c| fmt_num(c / 100.0)).unwrap_or_else(|_| v.to_string()),
        6 => match v.trim().parse::<f64>() {
            Ok(t) => {
                let mut b = (t as u32).to_le_bytes().to_vec();
                b.extend_from_slice(&7u16.to_le_bytes());
                decode_value(0x06, &b)
            }
            Err(_) => v.to_string(),
        },
        _ => v.to_string(),
    }
}

/// A section 4 entry for a field with a lookup table (no default value, no lookup id yet).
pub fn new_attr(id: u32, kind: crate::custom::CfKind, table: [u8; 16]) -> Option<Vec<u8>> {
    let (dtype, ftype, _, _) = type_codes(kind)?;
    let mut e = Vec::with_capacity(88);
    e.extend_from_slice(&id.to_le_bytes());
    e.extend_from_slice(&dtype.to_le_bytes());
    e.extend_from_slice(&u32::MAX.to_le_bytes());
    e.extend_from_slice(&ftype.to_le_bytes());
    e.extend_from_slice(&0u32.to_le_bytes());
    e.extend_from_slice(&field_guid(id));
    e.extend_from_slice(&table);
    e.resize(88, 0);
    Some(e)
}

/// The lookup table GUID of a section 4 entry (zero: none).
pub fn attr_table(e: &[u8]) -> Option<[u8; 16]> {
    let g: [u8; 16] = e.get(36..52)?.try_into().ok()?;
    (g != [0; 16]).then_some(g)
}

/// The field GUID MPP gives a local custom field ({000039B7-8BBE-4CEB-82C4-FA8C0B4000xx}).
pub fn field_guid(id: u32) -> [u8; 16] {
    let mut g = [0xb7, 0x39, 0, 0, 0xbe, 0x8b, 0xeb, 0x4c, 0x82, 0xc4, 0xfa, 0x8c, 0, 0, 0, 0];
    g[12..16].copy_from_slice(&id.to_be_bytes());
    g
}

/// Every definition of the file: aliases (both tables), formulas, lookup tables.
pub fn read_defs(cfb: &crate::mpp::Cfb<'_>, rt: &str, props: &std::collections::HashMap<u32, Vec<u8>>, v14: bool) -> Vec<CustomFieldDef> {
    let mut defs: Vec<CustomFieldDef> = Vec::new();
    fn get(defs: &mut Vec<CustomFieldDef>, f: CustomField, resource: bool) -> usize {
        match defs.iter().position(|d| d.field == f && d.resource == resource) {
            Some(i) => i,
            None => {
                defs.push(CustomFieldDef::new(f, resource));
                defs.len() - 1
            }
        }
    }
    let lists = if v14 { lookup_values(cfb, rt) } else { Default::default() };
    for (table, resource) in [("TBkndTask", false), ("TBkndRsc", true)] {
        let tp = crate::mpp::parse_props(&cfb.stream(&[rt, table, "Props"]));
        let Some(item) = tp.get(&DEFS_ITEM).and_then(|b| parse_item(b, v14)) else { continue };
        for (id, name) in item.aliases {
            if let Some((f, r)) = custom_of(id).filter(|x| x.1 == resource) {
                let k = get(&mut defs, f, r);
                defs[k].alias = name;
            }
        }
        for e in &item.attrs {
            let Some(id) = u32_at(e, 0) else { continue };
            let Some((f, r)) = custom_of(id).filter(|x| x.1 == resource) else { continue };
            let Some(table) = attr_table(e) else { continue };
            if let Some(v) = lists.get(&table) {
                let k = get(&mut defs, f, r);
                defs[k].values = v.clone();
            }
        }
    }
    if let Some(b) = props.get(&P_FORMULAS) {
        for (id, _flags, text) in parse_formulas(b) {
            if let Some((f, r)) = custom_of(id) {
                let k = get(&mut defs, f, r);
                defs[k].formula = formula_from_file(&text);
            }
        }
    }
    defs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formulas_round_trip() {
        // the standard record (newer MPP14): Number2 = [Duration]/480
        let f = vec![(0x0B40_0058, 0x0D, "[MSPJ188744967]/480".to_string())];
        let b = build_formulas(&f);
        assert_eq!(b.len(), 92);
        assert_eq!(u32_at(&b, 0), Some(92));
        assert_eq!(u32_at(&b, 12), Some(16));
        assert_eq!(u32_at(&b, 16 + 8), Some(76));
        assert_eq!(parse_formulas(&b), f);
        assert_eq!(formula_from_file("[MSPJ188744967]/480"), "[Duration]/480");
        assert_eq!(formula_to_file("[Duration]/480", false, &[]), "[MSPJ188744967]/480");
        assert_eq!(formula_to_file(r#"IIf([% Complete]=100,"[Done]","No")"#, false, &[]), r#"IIf([MSPJ188743712]=100,"[Done]","No")"#);
        assert_eq!(formula_to_file("[Max Units]*2", true, &[]), "[MSPJ205520900]*2");
    }

    #[test]
    fn aliases_round_trip() {
        let mut item = empty_item();
        item.aliases = vec![(0x0B40_0057, "Days".into()), (0x0B40_0033, "Phase Name".into())];
        let b = build_item(&item);
        // the standard layout: header, entries, 104-byte names, then the three other sections
        assert_eq!(u32_at(&b, 0), Some((12 + 16 + 208 - 4) as u32));
        assert_eq!(u32_at(&b, 12), Some(0x0B40_0033));
        assert_eq!(u32_at(&b, 16), Some(0x18));
        let back = parse_item(&b, true).unwrap();
        assert_eq!(back.aliases, vec![(0x0B40_0033, "Phase Name".to_string()), (0x0B40_0057, "Days".to_string())]);
        assert_eq!(b.len(), 236 + 24 + 8);
        assert_eq!(field_guid(0x0B40_0033)[12..], [0x0b, 0x40, 0x00, 0x33]);
    }
}
