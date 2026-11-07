//! Project > Spelling: the words of task names, resource names and notes checked with the
//! spell checker that comes with Windows (ISpellCheckerFactory, Windows 8 and later): English for
//! Latin text, Thai for Thai text when Windows has a Thai dictionary.

use std::collections::HashMap;
use std::ffi::c_void;

use windows_sys::core::{GUID, HRESULT};
use windows_sys::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree};

const CLSID_FACTORY: GUID = GUID { data1: 0x7AB36653, data2: 0x1796, data3: 0x484B, data4: [0xBD, 0xFA, 0xE7, 0x4F, 0x1D, 0xB7, 0xC1, 0xDC] };
const IID_FACTORY: GUID = GUID { data1: 0x8E018A9D, data2: 0x2415, data3: 0x4677, data4: [0xBF, 0x08, 0x79, 0x4E, 0xA6, 0x1F, 0x94, 0xBB] };

#[repr(C)]
struct Unknown {
    query: usize,
    add_ref: usize,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
}

#[repr(C)]
struct FactoryVtbl {
    base: Unknown,
    supported_languages: usize,
    is_supported: unsafe extern "system" fn(*mut c_void, *const u16, *mut i32) -> HRESULT,
    create: unsafe extern "system" fn(*mut c_void, *const u16, *mut *mut c_void) -> HRESULT,
}

#[repr(C)]
struct CheckerVtbl {
    base: Unknown,
    language_tag: usize,
    check: unsafe extern "system" fn(*mut c_void, *const u16, *mut *mut c_void) -> HRESULT,
    suggest: unsafe extern "system" fn(*mut c_void, *const u16, *mut *mut c_void) -> HRESULT,
}

#[repr(C)]
struct EnumErrorsVtbl {
    base: Unknown,
    next: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
}

#[repr(C)]
struct ErrorVtbl {
    base: Unknown,
    start: unsafe extern "system" fn(*mut c_void, *mut u32) -> HRESULT,
    length: unsafe extern "system" fn(*mut c_void, *mut u32) -> HRESULT,
    action: unsafe extern "system" fn(*mut c_void, *mut i32) -> HRESULT,
    replacement: unsafe extern "system" fn(*mut c_void, *mut *mut u16) -> HRESULT,
}

#[repr(C)]
struct EnumStringVtbl {
    base: Unknown,
    next: unsafe extern "system" fn(*mut c_void, u32, *mut *mut u16, *mut u32) -> HRESULT,
}

unsafe fn vtbl<T>(p: *mut c_void) -> &'static T {
    &**(p as *mut *const T)
}

unsafe fn release(p: *mut c_void) {
    if !p.is_null() {
        (vtbl::<Unknown>(p).release)(p);
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe fn take_wide(p: *mut u16) -> String {
    if p.is_null() {
        return String::new();
    }
    let mut n = 0;
    while *p.add(n) != 0 {
        n += 1;
    }
    let s = String::from_utf16_lossy(std::slice::from_raw_parts(p, n));
    CoTaskMemFree(p as *const c_void);
    s
}

/// A misspelt word: where it is in the text (char offsets), and what Windows suggests.
#[derive(Clone, Debug)]
pub struct Miss {
    pub start: usize,
    pub len: usize,
    pub word: String,
    pub suggestions: Vec<String>,
}

pub struct Speller {
    factory: *mut c_void,
    checkers: HashMap<&'static str, *mut c_void>,
}

impl Drop for Speller {
    fn drop(&mut self) {
        unsafe {
            for (_, c) in self.checkers.drain() {
                release(c);
            }
            release(self.factory);
        }
    }
}

impl Speller {
    /// The Windows spell checker (None when Windows has none).
    pub fn new() -> Option<Speller> {
        unsafe {
            // COINIT_APARTMENTTHREADED; S_FALSE / RPC_E_CHANGED_MODE when already set up are fine
            let _ = CoInitializeEx(std::ptr::null(), 0x2);
            let mut f: *mut c_void = std::ptr::null_mut();
            // CLSCTX_INPROC_SERVER | CLSCTX_LOCAL_SERVER
            let hr = CoCreateInstance(&CLSID_FACTORY, std::ptr::null_mut(), 0x1 | 0x4, &IID_FACTORY, &mut f);
            if hr < 0 || f.is_null() {
                return None;
            }
            let mut sp = Speller { factory: f, checkers: HashMap::new() };
            for lang in ["en-US", "th-TH"] {
                let tag = wide(lang);
                let mut ok = 0i32;
                if (vtbl::<FactoryVtbl>(f).is_supported)(f, tag.as_ptr(), &mut ok) >= 0 && ok != 0 {
                    let mut c: *mut c_void = std::ptr::null_mut();
                    if (vtbl::<FactoryVtbl>(f).create)(f, tag.as_ptr(), &mut c) >= 0 && !c.is_null() {
                        sp.checkers.insert(lang, c);
                    }
                }
            }
            if sp.checkers.is_empty() {
                return None;
            }
            Some(sp)
        }
    }

    fn check_with(&self, lang: &str, text: &str) -> Vec<Miss> {
        let Some(&c) = self.checkers.get(lang) else { return Vec::new() };
        let w = wide(text);
        let units: Vec<u16> = text.encode_utf16().collect();
        // UTF-16 offset -> char offset
        let mut char_at = Vec::with_capacity(units.len() + 1);
        for (k, ch) in text.chars().enumerate() {
            for _ in 0..ch.len_utf16() {
                char_at.push(k);
            }
        }
        char_at.push(text.chars().count());
        let mut out = Vec::new();
        unsafe {
            let mut errs: *mut c_void = std::ptr::null_mut();
            if (vtbl::<CheckerVtbl>(c).check)(c, w.as_ptr(), &mut errs) < 0 || errs.is_null() {
                return out;
            }
            loop {
                let mut e: *mut c_void = std::ptr::null_mut();
                let hr = (vtbl::<EnumErrorsVtbl>(errs).next)(errs, &mut e);
                if hr != 0 || e.is_null() {
                    break;
                }
                let (mut st, mut len, mut action) = (0u32, 0u32, 0i32);
                (vtbl::<ErrorVtbl>(e).start)(e, &mut st);
                (vtbl::<ErrorVtbl>(e).length)(e, &mut len);
                (vtbl::<ErrorVtbl>(e).action)(e, &mut action);
                let mut repl: *mut u16 = std::ptr::null_mut();
                let _ = (vtbl::<ErrorVtbl>(e).replacement)(e, &mut repl);
                let replacement = take_wide(repl);
                release(e);
                let (s16, e16) = (st as usize, (st + len) as usize);
                if e16 > units.len() || len == 0 {
                    continue;
                }
                let word = String::from_utf16_lossy(&units[s16..e16]);
                let mut suggestions = Vec::new();
                if !replacement.is_empty() {
                    suggestions.push(replacement);
                }
                // corrective action 1 = get suggestions
                if action == 1 {
                    let ww = wide(&word);
                    let mut en: *mut c_void = std::ptr::null_mut();
                    if (vtbl::<CheckerVtbl>(c).suggest)(c, ww.as_ptr(), &mut en) >= 0 && !en.is_null() {
                        for _ in 0..8 {
                            let mut s: *mut u16 = std::ptr::null_mut();
                            let mut got = 0u32;
                            if (vtbl::<EnumStringVtbl>(en).next)(en, 1, &mut s, &mut got) != 0 || got == 0 {
                                break;
                            }
                            suggestions.push(take_wide(s));
                        }
                        release(en);
                    }
                }
                out.push(Miss { start: char_at[s16], len: char_at[e16] - char_at[s16], word, suggestions });
            }
            release(errs);
        }
        out
    }

    /// The misspelt words of a text: Thai runs with the Thai dictionary (when there is one), the
    /// rest with English.
    pub fn check(&self, text: &str) -> Vec<Miss> {
        let chars: Vec<char> = text.chars().collect();
        let thai = |c: char| ('\u{0E00}'..='\u{0E7F}').contains(&c);
        let mut out = Vec::new();
        let mut k = 0;
        while k < chars.len() {
            let t = thai(chars[k]);
            let mut j = k;
            while j < chars.len() && (thai(chars[j]) == t || (!chars[j].is_alphabetic() && !thai(chars[j]) && t)) {
                j += 1;
            }
            let run: String = chars[k..j].iter().collect();
            let lang = if t { "th-TH" } else { "en-US" };
            for mut m in self.check_with(lang, &run) {
                m.start += k;
                out.push(m);
            }
            k = j.max(k + 1);
        }
        out
    }
}

/// One thing to fix: which text (task / resource, index, field) and the word.
#[derive(Clone, Debug)]
pub struct Issue {
    /// 0 task name, 1 task notes, 2 resource name
    pub field: u8,
    pub index: usize,
    pub miss: Miss,
}

/// Every misspelt word of the project (task names and notes, resource names).
pub fn find_all(p: &crate::model::Project) -> Result<Vec<Issue>, String> {
    let sp = Speller::new().ok_or("Windows has no spell checker for these languages.")?;
    let mut out = Vec::new();
    for (i, t) in p.tasks.iter().enumerate() {
        for m in sp.check(&t.name) {
            out.push(Issue { field: 0, index: i, miss: m });
        }
        for m in sp.check(&t.notes) {
            out.push(Issue { field: 1, index: i, miss: m });
        }
    }
    for (i, r) in p.resources.iter().enumerate() {
        for m in sp.check(&r.name) {
            out.push(Issue { field: 2, index: i, miss: m });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Windows' own spell checker (skipped where Windows has none)
    #[test]
    fn windows_spell_checker() {
        let Some(sp) = Speller::new() else { return };
        let m = sp.check("Pour the foundaton concrete");
        assert_eq!(m.len(), 1, "{m:?}");
        assert_eq!(m[0].word, "foundaton");
        assert_eq!(m[0].start, 9);
        assert!(m[0].suggestions.iter().any(|s| s == "foundation"), "{:?}", m[0].suggestions);
    }
}

/// The text an issue is in.
pub fn text_of<'a>(p: &'a crate::model::Project, is: &Issue) -> Option<&'a str> {
    match is.field {
        0 => p.tasks.get(is.index).map(|t| t.name.as_str()),
        1 => p.tasks.get(is.index).map(|t| t.notes.as_str()),
        _ => p.resources.get(is.index).map(|r| r.name.as_str()),
    }
}

/// Replace an issue's word (only when the text still has it there).
pub fn replace(p: &mut crate::model::Project, is: &Issue, with: &str) -> bool {
    let Some(text) = text_of(p, is).map(|s| s.to_string()) else { return false };
    let chars: Vec<char> = text.chars().collect();
    if is.miss.start + is.miss.len > chars.len() {
        return false;
    }
    let cur: String = chars[is.miss.start..is.miss.start + is.miss.len].iter().collect();
    if cur != is.miss.word {
        return false;
    }
    let new: String = chars[..is.miss.start].iter().collect::<String>() + with + &chars[is.miss.start + is.miss.len..].iter().collect::<String>();
    match is.field {
        0 => p.tasks[is.index].name = new,
        1 => p.tasks[is.index].notes = new,
        _ => p.resources[is.index].name = new,
    }
    true
}
