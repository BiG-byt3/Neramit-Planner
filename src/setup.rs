//! Per-user install support, run by NeramitPlanner-Setup: `--register` ties .neramit files (and, when the
//! user asks for it, .mpp / .mpt) to this NeramitPlanner.exe so a double-click opens them, adds
//! "Open with" entries, shortcuts and an entry in Settings > Apps; `--uninstall` takes all of it
//! away again and gives .mpp back to the program that had it.
//! Everything lives under HKEY_CURRENT_USER and the user's own folders: no administrator rights.

use std::ffi::c_void;
use std::path::{Path, PathBuf};

use windows_sys::core::{GUID, HRESULT};
use windows_sys::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree};
use windows_sys::Win32::System::Registry::{
    RegDeleteKeyValueW, RegDeleteKeyW, RegDeleteTreeW, RegGetValueW, RegSetKeyValueW, HKEY, HKEY_CLASSES_ROOT, HKEY_CURRENT_USER, REG_DWORD, REG_SZ,
    RRF_RT_REG_SZ,
};
use windows_sys::Win32::UI::Shell::{
    SHChangeNotify, SHGetKnownFolderPath, SHOpenWithDialog, FOLDERID_Desktop, FOLDERID_Programs, OAIF_ALLOW_REGISTRATION, OAIF_FORCE_REGISTRATION,
    OAIF_REGISTER_EXT, OPENASINFO, SHCNE_ASSOCCHANGED, SHCNF_IDLIST,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, IDYES, MB_ICONINFORMATION, MB_ICONQUESTION, MB_YESNO};

const PRODUCT: &str = "Neramit Planner";
const PROGID_NERAMIT: &str = "NeramitPlanner.neramit";
const PROGID_SCHEDULE: &str = "NeramitPlanner.schedule";
const CLASSES: &str = r"Software\Classes";
const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\NeramitPlanner";
const CAPABILITIES: &str = r"Software\BiGByte Studio\Neramit Planner\Capabilities";
const MENU_VERB: &str = "NeramitPlanner.open";
const SHORTCUT: &str = "Neramit Planner.lnk";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn set_str(key: &str, name: Option<&str>, value: &str) -> bool {
    let (k, v) = (wide(key), wide(value));
    let n = name.map(wide);
    // SAFETY: NUL-terminated UTF-16 buffers that outlive the call; the size includes the NUL
    unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            k.as_ptr(),
            n.as_ref().map(|n| n.as_ptr()).unwrap_or(std::ptr::null()),
            REG_SZ,
            v.as_ptr() as *const c_void,
            (v.len() * 2) as u32,
        ) == 0
    }
}

fn set_dword(key: &str, name: &str, value: u32) -> bool {
    let (k, n) = (wide(key), wide(name));
    // SAFETY: as in set_str; the data is a 4-byte value
    unsafe { RegSetKeyValueW(HKEY_CURRENT_USER, k.as_ptr(), n.as_ptr(), REG_DWORD, &value as *const u32 as *const c_void, 4) == 0 }
}

/// A string value (None = the key's default value).
fn get_value(root: HKEY, key: &str, name: Option<&str>) -> Option<String> {
    let k = wide(key);
    let n = name.map(wide);
    let mut buf = [0u16; 1024];
    let mut size = (buf.len() * 2) as u32;
    // SAFETY: the buffer and its byte size are passed together
    let r = unsafe { RegGetValueW(root, k.as_ptr(), n.as_ref().map(|n| n.as_ptr()).unwrap_or(std::ptr::null()), RRF_RT_REG_SZ, std::ptr::null_mut(), buf.as_mut_ptr() as *mut c_void, &mut size) };
    if r != 0 {
        return None;
    }
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..n])).filter(|s| !s.is_empty())
}

fn delete_tree(key: &str) {
    let k = wide(key);
    // SAFETY: NUL-terminated key name
    unsafe {
        RegDeleteTreeW(HKEY_CURRENT_USER, k.as_ptr());
        RegDeleteKeyW(HKEY_CURRENT_USER, k.as_ptr());
    }
}

fn delete_value(key: &str, name: Option<&str>) {
    let k = wide(key);
    let n = name.map(wide);
    // SAFETY: NUL-terminated names; a null value name is the default value
    unsafe {
        RegDeleteKeyValueW(HKEY_CURRENT_USER, k.as_ptr(), n.as_ref().map(|n| n.as_ptr()).unwrap_or(std::ptr::null()));
    }
}

fn known_folder(id: &GUID) -> Option<PathBuf> {
    let mut p: *mut u16 = std::ptr::null_mut();
    // SAFETY: the shell allocates the string; it is freed with CoTaskMemFree once copied
    unsafe {
        let hr = SHGetKnownFolderPath(id, 0, std::ptr::null_mut(), &mut p);
        if hr < 0 || p.is_null() {
            return None;
        }
        let mut n = 0;
        while *p.add(n) != 0 {
            n += 1;
        }
        let s = String::from_utf16_lossy(std::slice::from_raw_parts(p, n));
        CoTaskMemFree(p as *const c_void);
        Some(PathBuf::from(s))
    }
}

fn shortcut_paths() -> Vec<PathBuf> {
    [&FOLDERID_Programs, &FOLDERID_Desktop].into_iter().filter_map(known_folder).map(|d| d.join(SHORTCUT)).collect()
}

const CLSID_SHELL_LINK: GUID = GUID::from_u128(0x00021401_0000_0000_c000_000000000046);
const IID_SHELL_LINK_W: GUID = GUID::from_u128(0x000214f9_0000_0000_c000_000000000046);
const IID_PERSIST_FILE: GUID = GUID::from_u128(0x0000010b_0000_0000_c000_000000000046);

#[repr(C)]
struct Unknown {
    query: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: usize,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
}

#[repr(C)]
struct ShellLinkVtbl {
    base: Unknown,
    _get_path: usize,
    _get_id_list: usize,
    _set_id_list: usize,
    _get_description: usize,
    set_description: unsafe extern "system" fn(*mut c_void, *const u16) -> HRESULT,
    _get_working_directory: usize,
    set_working_directory: unsafe extern "system" fn(*mut c_void, *const u16) -> HRESULT,
    _get_arguments: usize,
    _set_arguments: usize,
    _get_hotkey: usize,
    _set_hotkey: usize,
    _get_show_cmd: usize,
    _set_show_cmd: usize,
    _get_icon_location: usize,
    set_icon_location: unsafe extern "system" fn(*mut c_void, *const u16, i32) -> HRESULT,
    _set_relative_path: usize,
    _resolve: usize,
    set_path: unsafe extern "system" fn(*mut c_void, *const u16) -> HRESULT,
}

#[repr(C)]
struct PersistFileVtbl {
    base: Unknown,
    _get_class_id: usize,
    _is_dirty: usize,
    _load: usize,
    save: unsafe extern "system" fn(*mut c_void, *const u16, i32) -> HRESULT,
}

unsafe fn vtbl<T>(p: *mut c_void) -> &'static T {
    &**(p as *mut *const T)
}

/// Writes a .lnk at `link` that starts `exe`.
fn make_shortcut(link: &Path, exe: &Path) -> bool {
    let target = wide(&exe.to_string_lossy());
    let dir = wide(&exe.parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default());
    let desc = wide(&format!("{PRODUCT} - Project Scheduling"));
    let file = wide(&link.to_string_lossy());
    // SAFETY: plain COM calls through the documented IShellLinkW / IPersistFile vtables; every
    // interface obtained is released before returning
    unsafe {
        let _ = CoInitializeEx(std::ptr::null(), 0x2);
        let mut sl: *mut c_void = std::ptr::null_mut();
        if CoCreateInstance(&CLSID_SHELL_LINK, std::ptr::null_mut(), 0x1, &IID_SHELL_LINK_W, &mut sl) < 0 || sl.is_null() {
            return false;
        }
        let v = vtbl::<ShellLinkVtbl>(sl);
        (v.set_path)(sl, target.as_ptr());
        (v.set_working_directory)(sl, dir.as_ptr());
        (v.set_description)(sl, desc.as_ptr());
        (v.set_icon_location)(sl, target.as_ptr(), 0);
        let mut pf: *mut c_void = std::ptr::null_mut();
        let mut ok = false;
        if (v.base.query)(sl, &IID_PERSIST_FILE, &mut pf) >= 0 && !pf.is_null() {
            ok = (vtbl::<PersistFileVtbl>(pf).save)(pf, file.as_ptr(), 1) >= 0;
            (vtbl::<Unknown>(pf).release)(pf);
        }
        (v.base.release)(sl);
        ok
    }
}

fn notify_shell() {
    // SAFETY: no item pointers are needed for SHCNE_ASSOCCHANGED
    unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED as i32, SHCNF_IDLIST, std::ptr::null(), std::ptr::null()) };
}

fn message(text: &str, style: u32) -> i32 {
    let (t, c) = (wide(text), wide(PRODUCT));
    // SAFETY: NUL-terminated strings; no owner window
    unsafe { MessageBoxW(std::ptr::null_mut(), t.as_ptr(), c.as_ptr(), style) }
}

/// What the installer was asked to set up (stored in the Settings > Apps entry, so an update
/// with `Setup /S` keeps the same choices).
#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub desktop: bool,
    pub start_menu: bool,
    /// .mpp / .mpt open with Neramit Planner on a double-click
    pub mpp_default: bool,
    /// "Open with Neramit Planner" in the right-click menu of .mpp / .mpt / .xml
    pub context_menu: bool,
    /// Neramit Planner's MCP server added to the AI assistants on this PC (aiconnect.rs)
    pub ai: bool,
}

impl Options {
    pub fn parse(s: &str) -> Options {
        let has = |k: &str| s.split(',').any(|p| p.trim() == k);
        Options { desktop: has("desktop"), start_menu: has("startmenu"), mpp_default: has("mpp"), context_menu: has("menu"), ai: has("ai") }
    }
    pub fn to_string(self) -> String {
        [(self.desktop, "desktop"), (self.start_menu, "startmenu"), (self.mpp_default, "mpp"), (self.context_menu, "menu"), (self.ai, "ai")]
            .iter()
            .filter(|(on, _)| *on)
            .map(|(_, k)| *k)
            .collect::<Vec<_>>()
            .join(",")
    }
}

/// The tasks the installed copy was registered with.
fn installed_options() -> Options {
    Options::parse(&get_value(HKEY_CURRENT_USER, UNINSTALL_KEY, Some("InstallOptions")).unwrap_or_default())
}

/// Keeps the installed copy's "Connect to AI assistants" choice in step with the Options page
/// (Setup shows it next time; nothing happens for a copy that is not installed).
pub fn remember_ai(on: bool) {
    let Some(s) = get_value(HKEY_CURRENT_USER, UNINSTALL_KEY, Some("InstallOptions")) else { return };
    let mut o = Options::parse(&s);
    if o.ai != on {
        o.ai = on;
        set_str(UNINSTALL_KEY, Some("InstallOptions"), &o.to_string());
    }
}

/// The folder Setup installed into (None when Neramit Planner is not installed).
pub fn install_location() -> Option<PathBuf> {
    get_value(HKEY_CURRENT_USER, UNINSTALL_KEY, Some("InstallLocation")).map(PathBuf::from)
}

/// The program the user picked with "Always use this app" for `ext` (Windows protects that choice:
/// a program may not change it, only ask the user through the Open with dialog).
fn user_choice(ext: &str) -> Option<String> {
    get_value(HKEY_CURRENT_USER, &format!(r"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\{ext}\UserChoice"), Some("ProgId"))
}

const MPP_EXTS: [&str; 2] = [".mpp", ".mpt"];

/// Makes Neramit Planner the program for .mpp / .mpt. False when the user has chosen another program
/// in Windows: then only `choose_mpp_default` (the Windows dialog) can change it.
pub fn set_mpp_default() -> bool {
    for ext in MPP_EXTS {
        let key = format!(r"{CLASSES}\{ext}");
        let prev = get_value(HKEY_CURRENT_USER, &key, None);
        if prev.as_deref() != Some(PROGID_SCHEDULE) {
            // what was there before comes back when Neramit Planner is removed
            set_str(UNINSTALL_KEY, Some(&format!("Previous{ext}")), prev.as_deref().unwrap_or(""));
            set_str(&key, None, PROGID_SCHEDULE);
        }
    }
    notify_shell();
    is_mpp_default()
}

pub fn is_mpp_default() -> bool {
    MPP_EXTS.iter().all(|ext| match user_choice(ext) {
        Some(p) => p == PROGID_SCHEDULE,
        None => get_value(HKEY_CLASSES_ROOT, ext, None).as_deref() == Some(PROGID_SCHEDULE),
    })
}

/// The Windows "How do you want to open .mpp files?" dialog, with "Always" for the extension.
pub fn choose_mpp_default() {
    let file = wide(".mpp");
    let info = OPENASINFO { pcszFile: file.as_ptr(), pcszClass: std::ptr::null(), oaifInFlags: OAIF_ALLOW_REGISTRATION | OAIF_REGISTER_EXT | OAIF_FORCE_REGISTRATION };
    // SAFETY: the info struct and its string outlive the (modal) call; the dialog needs COM
    unsafe {
        let _ = CoInitializeEx(std::ptr::null(), 0x2);
        SHOpenWithDialog(std::ptr::null_mut(), &info);
    }
    notify_shell();
}

/// `--register [desktop,startmenu,mpp,menu]`: file types, "Open with", shortcuts and the
/// Settings > Apps entry for this exe. Exit code 0, 1 when something could not be written, 2 when
/// .mpp stays with the program the user chose in Windows.
pub fn register(opts: Options) -> i32 {
    let Ok(exe) = std::env::current_exe() else { return 1 };
    let dir = exe.parent().map(Path::to_path_buf).unwrap_or_default();
    // the installer moves a running older copy aside as *.old; it is free once that copy has closed
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            if e.path().extension().map(|x| x == "old").unwrap_or(false) {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    let exe_s = exe.to_string_lossy().to_string();
    let open_cmd = format!("\"{exe_s}\" \"%1\"");
    let icon = format!("\"{exe_s}\",0");
    let mut ok = true;
    for (progid, name) in [(PROGID_NERAMIT, "Neramit Planner File"), (PROGID_SCHEDULE, "Project Schedule File")] {
        let k = format!(r"{CLASSES}\{progid}");
        ok &= set_str(&k, None, name);
        ok &= set_str(&format!(r"{k}\DefaultIcon"), None, &icon);
        ok &= set_str(&format!(r"{k}\shell\open\command"), None, &open_cmd);
    }
    ok &= set_str(&format!(r"{CLASSES}\.neramit"), None, PROGID_NERAMIT);
    ok &= set_str(&format!(r"{CLASSES}\.neramit\OpenWithProgids"), Some(PROGID_NERAMIT), "");
    for ext in MPP_EXTS {
        ok &= set_str(&format!(r"{CLASSES}\{ext}\OpenWithProgids"), Some(PROGID_SCHEDULE), "");
    }
    let app = format!(r"{CLASSES}\Applications\NeramitPlanner.exe");
    ok &= set_str(&app, Some("FriendlyAppName"), PRODUCT);
    ok &= set_str(&format!(r"{app}\shell\open\command"), None, &open_cmd);
    for ext in [".neramit", ".mpp", ".mpt", ".xml"] {
        ok &= set_str(&format!(r"{app}\SupportedTypes"), Some(ext), "");
    }
    // Settings > Default apps > Neramit Planner
    ok &= set_str(CAPABILITIES, Some("ApplicationName"), PRODUCT);
    ok &= set_str(CAPABILITIES, Some("ApplicationDescription"), "Project scheduling with Gantt charts");
    for (ext, progid) in [(".neramit", PROGID_NERAMIT), (".mpp", PROGID_SCHEDULE), (".mpt", PROGID_SCHEDULE)] {
        ok &= set_str(&format!(r"{CAPABILITIES}\FileAssociations"), Some(ext), progid);
    }
    ok &= set_str(r"Software\RegisteredApplications", Some(PRODUCT), CAPABILITIES);
    for ext in [".mpp", ".mpt", ".xml"] {
        let k = format!(r"{CLASSES}\SystemFileAssociations\{ext}\shell\{MENU_VERB}");
        if opts.context_menu {
            ok &= set_str(&k, None, "Open with Neramit Planner");
            ok &= set_str(&k, Some("Icon"), &icon);
            ok &= set_str(&format!(r"{k}\command"), None, &open_cmd);
        } else {
            delete_tree(&k);
        }
    }
    // AI assistants: connected when asked; an earlier install's connection goes when not
    if opts.ai {
        crate::aiconnect::connect(&exe);
    } else if installed_options().ai {
        crate::aiconnect::disconnect();
    }
    let size_kb = std::fs::metadata(&exe).map(|m| (m.len() / 1024) as u32).unwrap_or(0);
    ok &= set_str(UNINSTALL_KEY, Some("DisplayName"), PRODUCT);
    ok &= set_str(UNINSTALL_KEY, Some("DisplayVersion"), env!("CARGO_PKG_VERSION"));
    ok &= set_str(UNINSTALL_KEY, Some("Publisher"), env!("CARGO_PKG_AUTHORS"));
    ok &= set_str(UNINSTALL_KEY, Some("DisplayIcon"), &icon);
    ok &= set_str(UNINSTALL_KEY, Some("InstallLocation"), &dir.to_string_lossy());
    ok &= set_str(UNINSTALL_KEY, Some("InstallOptions"), &opts.to_string());
    ok &= set_str(UNINSTALL_KEY, Some("UninstallString"), &format!("\"{exe_s}\" --uninstall"));
    ok &= set_str(UNINSTALL_KEY, Some("QuietUninstallString"), &format!("\"{exe_s}\" --uninstall --quiet"));
    ok &= set_dword(UNINSTALL_KEY, "NoModify", 1);
    ok &= set_dword(UNINSTALL_KEY, "NoRepair", 1);
    ok &= set_dword(UNINSTALL_KEY, "EstimatedSize", size_kb);
    // an update keeps the user's choice: a shortcut that was not wanted is not made again
    for (folder, want) in [(&FOLDERID_Programs, opts.start_menu), (&FOLDERID_Desktop, opts.desktop)] {
        if let Some(link) = known_folder(folder).map(|d| d.join(SHORTCUT)) {
            if want {
                make_shortcut(&link, &exe);
            } else {
                let _ = std::fs::remove_file(link);
            }
        }
    }
    // 2: done, but Windows keeps the .mpp program the user picked before (Setup offers its dialog)
    let blocked = opts.mpp_default && !set_mpp_default();
    notify_shell();
    match (ok, blocked) {
        (false, _) => 1,
        (true, true) => 2,
        (true, false) => 0,
    }
}

/// Removes what `register` wrote; settings in %APPDATA%\Neramit Planner stay.
fn unregister() {
    // (also an assistant connected from File > Options; only our own entries go)
    crate::aiconnect::disconnect();
    for ext in MPP_EXTS {
        let key = format!(r"{CLASSES}\{ext}");
        delete_value(&format!(r"{key}\OpenWithProgids"), Some(PROGID_SCHEDULE));
        if get_value(HKEY_CURRENT_USER, &key, None).as_deref() == Some(PROGID_SCHEDULE) {
            match get_value(HKEY_CURRENT_USER, UNINSTALL_KEY, Some(&format!("Previous{ext}"))) {
                Some(prev) => {
                    set_str(&key, None, &prev);
                }
                None => delete_value(&key, None),
            }
        }
    }
    for progid in [PROGID_NERAMIT, PROGID_SCHEDULE] {
        delete_tree(&format!(r"{CLASSES}\{progid}"));
    }
    if get_value(HKEY_CURRENT_USER, &format!(r"{CLASSES}\.neramit"), None).as_deref() == Some(PROGID_NERAMIT) {
        delete_tree(&format!(r"{CLASSES}\.neramit"));
    }
    for ext in [".mpp", ".mpt", ".xml"] {
        delete_tree(&format!(r"{CLASSES}\SystemFileAssociations\{ext}\shell\{MENU_VERB}"));
    }
    delete_tree(&format!(r"{CLASSES}\Applications\NeramitPlanner.exe"));
    delete_value(r"Software\RegisteredApplications", Some(PRODUCT));
    delete_tree(CAPABILITIES);
    delete_tree(r"Software\BiGByte Studio\Neramit Planner");
    delete_tree(UNINSTALL_KEY);
    for link in shortcut_paths() {
        let _ = std::fs::remove_file(link);
    }
    notify_shell();
}

/// `--uninstall [--quiet]`: unregisters, then removes the install folder once this process has exited.
pub fn uninstall(quiet: bool) -> i32 {
    // as Setup: Thai when Windows is in Thai (LANG_THAI)
    let thai = unsafe { windows_sys::Win32::Globalization::GetUserDefaultUILanguage() } & 0x3FF == 0x1E;
    let ask = if thai { "ต้องการถอนการติดตั้ง Neramit Planner ออกจากเครื่องนี้หรือไม่\n\nไฟล์งานและค่าตั้งของคุณจะยังอยู่" } else { "Remove Neramit Planner from this computer?\n\nYour project files and settings are kept." };
    if !quiet && message(ask, MB_YESNO | MB_ICONQUESTION) != IDYES {
        return 1;
    }
    unregister();
    // a running program cannot delete its own file: as installers do, a copy of it in a folder of
    // its own under %TEMP% removes the program once this process has exited
    if let Ok(exe) = std::env::current_exe() {
        let helper_dir = uninstall_helper_dir();
        let helper = helper_dir.join("NeramitPlanner-Uninstall.exe");
        let started = std::fs::create_dir_all(&helper_dir).is_ok()
            && std::fs::copy(&exe, &helper).is_ok()
            && std::process::Command::new(&helper)
                .arg("--uninstall-finish")
                .arg(exe.parent().unwrap_or(Path::new("")))
                .arg(std::process::id().to_string())
                .current_dir(std::env::temp_dir())
                .spawn()
                .is_ok();
        if !started && !quiet {
            let dir = exe.parent().map(|d| d.display().to_string()).unwrap_or_default();
            message(&if thai { format!("ลบไฟล์โปรแกรมเองไม่ได้ กรุณาลบโฟลเดอร์นี้ด้วยตนเอง:\n{dir}") } else { format!("The program files could not be removed. Delete this folder yourself:\n{dir}") }, MB_ICONINFORMATION);
        }
    }
    if !quiet {
        message(if thai { "ถอนการติดตั้ง Neramit Planner เรียบร้อยแล้ว" } else { "Neramit Planner has been removed." }, MB_ICONINFORMATION);
    }
    0
}

/// Where `uninstall` puts the copy of the program that removes it (Setup clears it).
pub fn uninstall_helper_dir() -> PathBuf {
    std::env::temp_dir().join("Neramit Planner Uninstall")
}

/// `--uninstall-finish <dir> <pid>` (the copy `uninstall` starts): waits for that process to exit,
/// then removes the program from `dir`.
pub fn uninstall_finish(dir: &Path, pid: u32) -> i32 {
    use windows_sys::Win32::System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE};
    // SAFETY: a handle opened here and closed here
    unsafe {
        let h = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if !h.is_null() {
            WaitForSingleObject(h, 30_000);
            windows_sys::Win32::Foundation::CloseHandle(h);
        }
    }
    // only a folder that holds nothing but the program is removed (never a user's Downloads)
    let only_ours = std::fs::read_dir(dir)
        .map(|rd| rd.flatten().all(|e| e.file_name().to_string_lossy().starts_with("NeramitPlanner")))
        .unwrap_or(false);
    // (the file can stay locked for a moment after its process has gone)
    for _ in 0..20 {
        let done = if only_ours {
            std::fs::remove_dir_all(dir).is_ok() || !dir.exists()
        } else {
            std::fs::read_dir(dir)
                .map(|rd| {
                    rd.flatten()
                        .filter(|e| e.file_name().to_string_lossy().starts_with("NeramitPlanner.exe"))
                        .all(|e| std::fs::remove_file(e.path()).is_ok())
                })
                .unwrap_or(true)
        };
        if done {
            return 0;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    1
}
