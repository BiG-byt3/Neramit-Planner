//! Online updates from the GitHub releases of the project (tools/release.py publishes them).
//!
//! `latest.json` of the newest release names the version, the size and SHA-256 of
//! NeramitPlanner-Setup-<version>.exe and an ECDSA P-256 signature over "NeramitPlanner|<version>|<sha256>"
//! made with the studio's private key; only a file that matches the signed digest is run. The
//! installer is then started as `/S --relaunch` when the program closes. Nothing happens on its
//! own: the user starts a check with Help > Check for Updates. HTTP goes through
//! WinHTTP (system proxy) and the checks through Windows CNG, so no extra libraries are needed.
//! Portable copies download the signed standalone executable and replace only their own file
//! through a helper that waits for the old process to exit. Project files and settings stay put.

use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use eframe::egui;
use windows_sys::Win32::Networking::WinHttp::*;
use windows_sys::Win32::Security::Cryptography::{
    BCryptCloseAlgorithmProvider, BCryptDestroyKey, BCryptHash, BCryptImportKeyPair, BCryptOpenAlgorithmProvider, BCryptVerifySignature,
    BCRYPT_ECCPUBLIC_BLOB, BCRYPT_ECDSA_P256_ALGORITHM, BCRYPT_ECDSA_PUBLIC_P256_MAGIC, BCRYPT_SHA256_ALG_HANDLE,
};

/// Where releases are published; `NERAMIT_UPDATE_BASE` points elsewhere for tests (http allowed).
const REPO_URL: &str = "https://github.com/BiG-byt3/Neramit-Planner";
/// The public half of the release signing key (X || Y), see tools/release.py.
const PUBLIC_KEY: &str = "079365895bc684e6f65070f57d60cfd988eb0a52935c89e6f0a3051c2bb05defeb880ad40d7ee125f0699e862fa0a16759b8d1fd8e28496b9741d57d7800658f";
const MAX_DOWNLOAD: u64 = 512 * 1024 * 1024;

#[derive(Clone, Debug)]
struct Artifact {
    size: u64,
    sha256: String,
}

#[derive(Clone, Debug)]
pub struct Release {
    pub version: String,
    installer: Artifact,
    portable: Option<Artifact>,
    manifest: Vec<u8>,
    pub notes_en: String,
    pub notes_th: String,
}

#[derive(Clone, Debug)]
pub enum Status {
    Idle,
    Checking,
    UpToDate,
    Available(Release),
    /// the release and the share downloaded so far (0..1)
    Downloading(Release, f32),
    Ready(Release, PathBuf),
    Failed(String),
}

static STATUS: Mutex<Status> = Mutex::new(Status::Idle);
/// A download finished while the dialog was hidden: it opens again once.
static NOTIFY: AtomicBool = AtomicBool::new(false);
/// The downloaded installer to run when the window closes, and whether it reopens the program.
static INSTALL_ON_EXIT: Mutex<Option<(Release, PathBuf, bool)>> = Mutex::new(None);

pub fn status() -> Status {
    STATUS.lock().map(|s| s.clone()).unwrap_or(Status::Idle)
}

fn set(s: Status) {
    if let Ok(mut g) = STATUS.lock() {
        *g = s;
    }
}

pub fn take_notify() -> bool {
    NOTIFY.swap(false, Ordering::Relaxed)
}

fn base() -> String {
    std::env::var("NERAMIT_UPDATE_BASE").unwrap_or_else(|_| REPO_URL.into()).trim_end_matches('/').to_string()
}

/// The release page in the browser (single-exe copies download from there).
pub fn page_url() -> String {
    format!("{}/releases/latest", base())
}

pub fn current() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn parse_version(v: &str) -> Option<[u32; 3]> {
    let parts: Vec<_> = v.split('.').collect();
    if parts.len() != 3 || parts.iter().any(|s| s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) || (s.len() > 1 && s.starts_with('0'))) {
        return None;
    }
    Some([parts[0].parse().ok()?, parts[1].parse().ok()?, parts[2].parse().ok()?])
}

pub fn is_newer(v: &str) -> bool {
    matches!((parse_version(v), parse_version(current())), (Some(new), Some(old)) if new > old)
}

/// This exe is the one NeramitPlanner-Setup installed, so Setup can replace it.
pub fn installed() -> bool {
    let (Ok(exe), Some(dir)) = (std::env::current_exe(), crate::setup::install_location()) else { return false };
    let norm = |p: &std::path::Path| p.to_string_lossy().trim_end_matches(['\\', '/']).to_lowercase();
    exe.parent().map(|d| norm(d) == norm(&dir)).unwrap_or(false)
}

/// Looks for a newer release in the background (Help > Check for Updates).
pub fn check(ctx: &egui::Context) {
    // busy, or already downloaded: the dialog shows that instead
    if matches!(status(), Status::Checking | Status::Downloading(..) | Status::Ready(..)) {
        return;
    }
    set(Status::Checking);
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        let r = fetch_latest();
        let s = match r {
            Ok(rel) if is_newer(&rel.version) => Status::Available(rel),
            Ok(_) => Status::UpToDate,
            Err(e) => Status::Failed(e),
        };
        set(s);
        ctx.request_repaint();
    });
}

fn fetch_latest() -> Result<Release, String> {
    let body = http_get(&format!("{}/releases/latest/download/latest.json", base()), 256 * 1024, |_, _| {})?;
    parse_manifest(&body)
}

/// Read-only release diagnostics, used to verify a published release end to end.
pub fn cli_check(download_assets: bool) -> i32 {
    let result = fetch_latest().and_then(|rel| {
        let mut verified = Vec::new();
        if download_assets {
            for (name, artifact) in std::iter::once((format!("NeramitPlanner-Setup-{}.exe", rel.version), &rel.installer))
                .chain(rel.portable.as_ref().map(|a| (format!("NeramitPlanner-{}.exe", rel.version), a))) {
                let url = format!("{}/releases/download/v{}/{name}", base(), rel.version);
                validate_data(&http_get(&url, artifact.size, |_, _| {})?, artifact)?;
                verified.push(name);
            }
        }
        Ok(serde_json::json!({"current": current(), "latest": rel.version, "newer_available": is_newer(&rel.version), "signature_verified": true, "portable_available": rel.portable.is_some(), "verified_assets": verified}))
    });
    match result {
        Ok(value) => { println!("{value}"); 0 }
        Err(error) => { eprintln!("{}", serde_json::json!({"error": error})); 1 }
    }
}

fn parse_manifest(body: &[u8]) -> Result<Release, String> {
    let j: serde_json::Value = serde_json::from_slice(&body).map_err(|e| format!("latest.json: {e}"))?;
    let s = |k: &str| j.get(k).and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let version = s("version");
    if parse_version(&version).is_none() {
        return Err("The update version is invalid.".into());
    }
    let artifact = |prefix: &str, product: &str| -> Result<Artifact, String> {
        let size = j.get(format!("{prefix}size")).and_then(|v| v.as_u64()).unwrap_or(0);
        let sha256 = s(&format!("{prefix}sha256")).to_lowercase();
        let sig = hex(&s(&format!("{prefix}signature"))).filter(|b| b.len() == 64);
        if size == 0 || size > MAX_DOWNLOAD || hex(&sha256).filter(|b| b.len() == 32).is_none()
            || !sig.map(|sig| verify(format!("{product}|{version}|{sha256}").as_bytes(), &sig)).unwrap_or(false) {
            return Err("The update information is not signed by BiGByte Studio.".into());
        }
        Ok(Artifact { size, sha256 })
    };
    let installer = artifact("", "NeramitPlanner")?;
    let portable = if j.get("portable_sha256").is_some() { Some(artifact("portable_", "NeramitPlannerPortable")?) } else { None };
    Ok(Release { version, installer, portable, manifest: body.to_vec(), notes_en: s("notes_en"), notes_th: s("notes_th") })
}

pub fn can_download(rel: &Release) -> bool {
    installed() || rel.portable.is_some()
}

fn selected_artifact(rel: &Release, is_installed: bool) -> Result<&Artifact, String> {
    if is_installed { Ok(&rel.installer) } else { rel.portable.as_ref().ok_or_else(|| "This release has no portable update. Open the download page instead.".into()) }
}

/// Downloads the installer or portable executable and checks it against the signed digest.
pub fn download(ctx: &egui::Context, rel: Release) {
    if matches!(status(), Status::Downloading(..)) {
        return;
    }
    set(Status::Downloading(rel.clone(), 0.0));
    let ctx = ctx.clone();
    let is_installed = installed();
    std::thread::spawn(move || {
        let name = format!("NeramitPlanner-{}{}.exe", if is_installed { "Setup-" } else { "" }, rel.version);
        let url = format!("{}/releases/download/v{}/{name}", base(), rel.version);
        let mut last = 0.0f32;
        let r = selected_artifact(&rel, is_installed).and_then(|artifact| http_get(&url, artifact.size, |got, total| {
            let total = if total > 0 { total } else { artifact.size };
            let f = (got as f32 / total as f32).min(1.0);
            if f - last >= 0.01 {
                last = f;
                set(Status::Downloading(rel.clone(), f));
                ctx.request_repaint();
            }
        }))
        .and_then(|data| {
            validate_data(&data, selected_artifact(&rel, is_installed)?)?;
            // kept with the program's own data (not %TEMP%); an earlier download is replaced
            let dir = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("Neramit Planner").join("Updates").join(&rel.version);
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let path = dir.join(&name);
            std::fs::write(&path, &data).map_err(|e| e.to_string())?;
            std::fs::write(dir.join("latest.json"), &rel.manifest).map_err(|e| e.to_string())?;
            Ok(path)
        });
        set(match r {
            Ok(p) => {
                NOTIFY.store(true, Ordering::Relaxed);
                Status::Ready(rel, p)
            }
            Err(e) => Status::Failed(e),
        });
        ctx.request_repaint();
    });
}

fn validate_data(data: &[u8], artifact: &Artifact) -> Result<(), String> {
    if data.len() as u64 != artifact.size || to_hex(&sha256(data)) != artifact.sha256 {
        return Err("The downloaded file does not match the signed release.".into());
    }
    Ok(())
}

pub fn install_on_exit(rel: Release, path: PathBuf, relaunch: bool) {
    if let Ok(mut g) = INSTALL_ON_EXIT.lock() {
        *g = Some((rel, path, relaunch));
    }
}

/// Canceling the save/exit prompt also cancels the requested restart.
pub fn cancel_restart() {
    if let Ok(mut g) = INSTALL_ON_EXIT.lock() {
        if g.as_ref().map(|(_, _, relaunch)| *relaunch).unwrap_or(false) {
            *g = None;
        }
    }
}

/// Called as the window closes: starts a downloaded installer without questions.
pub fn on_exit() -> Result<(), String> {
    if let Some((rel, p, relaunch)) = INSTALL_ON_EXIT.lock().ok().and_then(|mut g| g.take()) {
        let result: Result<(), String> = (|| {
        // Recheck at execution time too: the user may have left the download ready for days.
        let is_installed = installed();
        let trusted = parse_manifest(&rel.manifest)?;
        let data = std::fs::read(&p).map_err(|e| e.to_string())?;
        validate_data(&data, selected_artifact(&trusted, is_installed)?)?;
        let mut c = std::process::Command::new(p);
        if is_installed {
            c.args(["/S", "--wait-pid", &std::process::id().to_string()]);
        } else {
            c.arg("--finish-portable-update").arg(std::env::current_exe().map_err(|e| e.to_string())?).arg(std::process::id().to_string());
        }
        if relaunch {
            c.arg("--relaunch");
        }
        c.spawn().map_err(|e| format!("Could not start the update: {e}"))?;
        Ok(())
        })();
        if let Err(ref error) = result {
            // Ready must not trap the user with a missing/corrupted file: a new check can retry.
            set(Status::Failed(error.clone()));
        }
        result?;
    }
    Ok(())
}

/// The downloaded, signed standalone executable runs this helper before it opens any UI.
pub fn finish_portable_update(args: &[std::ffi::OsString]) -> i32 {
    let finish = || -> Result<(), String> {
        let target = args.first().map(PathBuf::from).filter(|p| p.is_absolute() && p.extension().map(|e| e.eq_ignore_ascii_case("exe")).unwrap_or(false)).ok_or("Invalid update target.")?;
        let pid = args.get(1).and_then(|p| p.to_str()).and_then(|p| p.parse::<u32>().ok()).filter(|pid| *pid > 0 && *pid != std::process::id()).ok_or("Invalid update process.")?;
        let source = std::env::current_exe().map_err(|e| e.to_string())?;
        if source == target { return Err("Invalid update target.".into()); }
        let manifest = std::fs::read(source.parent().ok_or("Invalid update folder.")?.join("latest.json")).map_err(|e| e.to_string())?;
        let rel = parse_manifest(&manifest)?;
        if rel.version != current() { return Err("The update version is invalid.".into()); }
        let artifact = selected_artifact(&rel, false)?;
        validate_data(&std::fs::read(&source).map_err(|e| e.to_string())?, artifact)?;
        wait_for_exit(pid)?;
        let backup = replace_portable(&source, &target, artifact)?;
        if args.iter().any(|a| a == "--relaunch") {
            if let Err(error) = std::process::Command::new(&target).current_dir(target.parent().unwrap_or(Path::new("."))).spawn() {
                let recovery = restore_portable(&target, &backup)
                    .map(|_| "The previous program was restored.".to_string())
                    .unwrap_or_else(|e| format!("Could not restore the previous program ({e}). It is preserved at {}.", backup.display()));
                return Err(format!("Could not open Neramit Planner: {error}\n\n{recovery}"));
            }
        }
        let _ = std::fs::remove_file(backup);
        Ok(())
    };
    match finish() {
        Ok(()) => 0,
        Err(e) => {
            rfd::MessageDialog::new().set_title("Neramit Planner Update").set_description(&format!("Could not complete the update. Your project files have not been changed.\n\n{e}")).set_level(rfd::MessageLevel::Error).show();
            1
        }
    }
}

fn wait_for_exit(pid: u32) -> Result<(), String> {
    use windows_sys::Win32::System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE};
    // SAFETY: the process handle is opened here, waited on and closed here.
    unsafe {
        let h = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if h.is_null() {
            return if windows_sys::Win32::Foundation::GetLastError() == windows_sys::Win32::Foundation::ERROR_INVALID_PARAMETER { Ok(()) } else { Err("Could not wait for Neramit Planner to close.".into()) };
        }
        let result = WaitForSingleObject(h, 60_000);
        windows_sys::Win32::Foundation::CloseHandle(h);
        if result != windows_sys::Win32::Foundation::WAIT_OBJECT_0 { return Err("Close Neramit Planner before updating.".into()); }
    }
    Ok(())
}

fn replace_portable(source: &Path, target: &Path, artifact: &Artifact) -> Result<PathBuf, String> {
    use std::io::Write;
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
    let suffix = format!("{}-{stamp}", std::process::id());
    let staged = target.with_extension(format!("exe.{suffix}.new"));
    let backup = target.with_extension(format!("exe.{suffix}.rollback"));
    let result = (|| {
        let data = std::fs::read(source).map_err(|e| e.to_string())?;
        validate_data(&data, artifact)?;
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&staged).map_err(|e| e.to_string())?;
        file.write_all(&data).and_then(|_| file.sync_all()).map_err(|e| e.to_string())?;
        drop(file);
        validate_data(&std::fs::read(&staged).map_err(|e| e.to_string())?, artifact)?;
        let (dest, src, old) = (wide(&target.to_string_lossy()), wide(&staged.to_string_lossy()), wide(&backup.to_string_lossy()));
        // SAFETY: NUL-terminated paths outlive the call and are all on the same filesystem.
        if unsafe { windows_sys::Win32::Storage::FileSystem::ReplaceFileW(dest.as_ptr(), src.as_ptr(), old.as_ptr(), 0, std::ptr::null(), std::ptr::null()) } == 0 {
            let error = std::io::Error::last_os_error();
            if !target.exists() && backup.is_file() {
                if let Err(recovery) = std::fs::rename(&backup, target) {
                    return Err(format!("Could not replace {}: {error}. Could not restore it ({recovery}). The previous program is preserved at {}.", target.display(), backup.display()));
                }
            }
            let recovery = if backup.is_file() { format!(" The previous program is preserved at {}.", backup.display()) } else { String::new() };
            return Err(format!("Could not replace {}: {error}.{recovery}", target.display()));
        }
        Ok(backup.clone())
    })();
    let _ = std::fs::remove_file(staged);
    result
}

fn restore_portable(target: &Path, backup: &Path) -> Result<(), String> {
    if !target.exists() {
        return std::fs::rename(backup, target).map_err(|e| e.to_string());
    }
    let (dest, src) = (wide(&target.to_string_lossy()), wide(&backup.to_string_lossy()));
    // SAFETY: NUL-terminated paths outlive the call; no additional backup is requested.
    if unsafe { windows_sys::Win32::Storage::FileSystem::ReplaceFileW(dest.as_ptr(), src.as_ptr(), std::ptr::null(), 0, std::ptr::null(), std::ptr::null()) } == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}

pub fn open_page() {
    let url: Vec<u16> = page_url().encode_utf16().chain(std::iter::once(0)).collect();
    let verb: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: NUL-terminated strings that outlive the call
    unsafe {
        windows_sys::Win32::UI::Shell::ShellExecuteW(std::ptr::null_mut(), verb.as_ptr(), url.as_ptr(), std::ptr::null(), std::ptr::null(), windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL);
    }
}

fn hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok()).collect()
}

fn to_hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn sha256(data: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    // SAFETY: the SHA-256 pseudo-handle (Windows 10+); input and output sizes are passed with the buffers
    unsafe { BCryptHash(BCRYPT_SHA256_ALG_HANDLE, std::ptr::null(), 0, data.as_ptr(), data.len() as u32, out.as_mut_ptr(), 32) };
    out
}

/// ECDSA P-256 / SHA-256 signature (r || s) of `msg` against PUBLIC_KEY.
fn verify(msg: &[u8], sig: &[u8]) -> bool {
    let Some(key) = hex(PUBLIC_KEY) else { return false };
    let mut blob = Vec::with_capacity(8 + 64);
    blob.extend_from_slice(&BCRYPT_ECDSA_PUBLIC_P256_MAGIC.to_le_bytes());
    blob.extend_from_slice(&32u32.to_le_bytes());
    blob.extend_from_slice(&key);
    let hash = sha256(msg);
    // SAFETY: CNG handles are closed on every path; buffers are passed with their sizes
    unsafe {
        let mut alg = std::ptr::null_mut();
        if BCryptOpenAlgorithmProvider(&mut alg, BCRYPT_ECDSA_P256_ALGORITHM, std::ptr::null(), 0) != 0 {
            return false;
        }
        let mut k = std::ptr::null_mut();
        let mut ok = false;
        if BCryptImportKeyPair(alg, std::ptr::null_mut(), BCRYPT_ECCPUBLIC_BLOB, &mut k, blob.as_ptr(), blob.len() as u32, 0) == 0 {
            ok = BCryptVerifySignature(k, std::ptr::null(), hash.as_ptr(), 32, sig.as_ptr(), sig.len() as u32, 0) == 0;
            BCryptDestroyKey(k);
        }
        BCryptCloseAlgorithmProvider(alg, 0);
        ok
    }
}

struct Handle(*mut c_void);

impl Drop for Handle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: a WinHTTP handle this module opened
            unsafe { WinHttpCloseHandle(self.0) };
        }
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// GET `url` (redirects followed, e.g. GitHub to its download host); `progress(received, total)`.
fn http_get(url: &str, max_bytes: u64, mut progress: impl FnMut(u64, u64)) -> Result<Vec<u8>, String> {
    let (secure, rest) = match (url.strip_prefix("https://"), url.strip_prefix("http://")) {
        (Some(r), _) => (true, r),
        (_, Some(r)) => (false, r),
        _ => return Err(format!("Bad address: {url}")),
    };
    let (host_port, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
    let (host, port) = match host_port.rsplit_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().map_err(|_| format!("Bad address: {url}"))?),
        None => (host_port, if secure { 443 } else { 80 }),
    };
    let net_err = || "Could not connect to the update server. Check the internet connection.".to_string();
    let agent = wide(&format!("NeramitPlanner/{}", current()));
    let (h, p, get) = (wide(host), wide(if path.is_empty() { "/" } else { path }), wide("GET"));
    // SAFETY: WinHTTP calls with NUL-terminated strings; every handle is closed by its guard
    unsafe {
        let mut session = Handle(WinHttpOpen(agent.as_ptr(), WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, std::ptr::null(), std::ptr::null(), 0));
        if session.0.is_null() {
            // before Windows 8.1: the proxy set with netsh
            session = Handle(WinHttpOpen(agent.as_ptr(), WINHTTP_ACCESS_TYPE_DEFAULT_PROXY, std::ptr::null(), std::ptr::null(), 0));
        }
        if session.0.is_null() {
            return Err(net_err());
        }
        WinHttpSetTimeouts(session.0, 15_000, 15_000, 20_000, 30_000);
        let conn = Handle(WinHttpConnect(session.0, h.as_ptr(), port, 0));
        if conn.0.is_null() {
            return Err(net_err());
        }
        // WINHTTP_FLAG_REFRESH: never an older latest.json from a cache
        let flags = 0x100 | if secure { WINHTTP_FLAG_SECURE } else { 0 };
        let req = Handle(WinHttpOpenRequest(conn.0, get.as_ptr(), p.as_ptr(), std::ptr::null(), std::ptr::null(), std::ptr::null(), flags));
        if req.0.is_null() || WinHttpSendRequest(req.0, std::ptr::null(), 0, std::ptr::null(), 0, 0, 0) == 0 || WinHttpReceiveResponse(req.0, std::ptr::null_mut()) == 0 {
            return Err(net_err());
        }
        let query = |level: u32| {
            let mut v = 0u32;
            let mut len = 4u32;
            let ok = WinHttpQueryHeaders(req.0, level | WINHTTP_QUERY_FLAG_NUMBER, std::ptr::null(), &mut v as *mut u32 as *mut c_void, &mut len, std::ptr::null_mut()) != 0;
            ok.then_some(v)
        };
        let code = query(WINHTTP_QUERY_STATUS_CODE).unwrap_or(0);
        if code == 404 {
            return Err("No release has been published yet.".into());
        }
        if code != 200 {
            return Err(format!("The update server answered HTTP {code}."));
        }
        let total = query(WINHTTP_QUERY_CONTENT_LENGTH).unwrap_or(0) as u64;
        if total > max_bytes { return Err("The update download is too large.".into()); }
        let mut out = Vec::with_capacity(total as usize);
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let mut n = 0u32;
            if WinHttpReadData(req.0, buf.as_mut_ptr() as *mut c_void, buf.len() as u32, &mut n) == 0 {
                return Err(net_err());
            }
            if n == 0 {
                break;
            }
            if out.len() as u64 + n as u64 > max_bytes { return Err("The update download is too large.".into()); }
            out.extend_from_slice(&buf[..n as usize]);
            progress(out.len() as u64, total);
        }
        if total > 0 && out.len() as u64 != total { return Err("The update download was interrupted. Try again.".into()); }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // An existing signed release; only public metadata and its signature are embedded here.
    fn signed_fixture() -> serde_json::Value {
        serde_json::json!({
            "version": "1.0.0", "size": 7578624,
            "sha256": "f31c9d67baabf3562a2368c70f27633685b66fe30e9da2dd9a50c95b211ec2f8",
            "signature": "94f5a55a8a6adf33c9435cd0fe018781c9415317712c9de288838040955c32291a9f9c71f63a906c306e1e4184be6e06b6c86d563da4667d8b89fbc15eb1a2e6"
        })
    }

    #[test]
    fn versions_are_numeric_stable_and_cannot_be_paths() {
        assert!(parse_version("1.10.0") > parse_version("1.9.9"));
        for invalid in ["", "1", "1.0", "1.0.0.0", "1.0.0-beta", "v1.0.0", "1.0../evil", "01.0.0", "1.+0.0"] {
            assert!(parse_version(invalid).is_none(), "{invalid}");
            assert!(!is_newer(invalid));
        }
        assert!(!is_newer(current()));
    }

    #[test]
    fn windows_sha256_matches_known_vectors() {
        assert_eq!(to_hex(&sha256(b"")), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(to_hex(&sha256(b"abc")), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn signed_manifest_is_accepted_but_tampering_is_rejected() {
        let fixture = signed_fixture();
        let rel = parse_manifest(&serde_json::to_vec(&fixture).unwrap()).unwrap();
        assert_eq!(rel.version, "1.0.0");
        assert!(rel.portable.is_none()); // legacy installer releases remain compatible
        for (field, replacement) in [("version", serde_json::json!("1.0.1")), ("sha256", serde_json::json!("00".repeat(32))), ("size", serde_json::json!(MAX_DOWNLOAD + 1)), ("signature", serde_json::json!("00".repeat(64)))] {
            let mut changed = fixture.clone();
            changed[field] = replacement;
            assert!(parse_manifest(&serde_json::to_vec(&changed).unwrap()).is_err(), "{field}");
        }
        let mut changed = fixture;
        changed["portable_size"] = serde_json::json!(100);
        changed["portable_sha256"] = changed["sha256"].clone();
        changed["portable_signature"] = changed["signature"].clone();
        assert!(parse_manifest(&serde_json::to_vec(&changed).unwrap()).is_err()); // installer signatures cannot authorize portable binaries
    }

    #[test]
    fn download_validation_rejects_changed_and_truncated_bytes() {
        let artifact = Artifact { size: 3, sha256: to_hex(&sha256(b"abc")) };
        assert!(validate_data(b"abc", &artifact).is_ok());
        assert!(validate_data(b"ab", &artifact).is_err());
        assert!(validate_data(b"abd", &artifact).is_err());
    }

    #[test]
    fn portable_replacement_preserves_other_files_and_rejects_bad_payload() {
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("neramit-update-test-{}-{stamp}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("download.exe");
        let target = dir.join("planner.exe");
        let project = dir.join("my-project.neramit");
        std::fs::write(&source, b"new").unwrap();
        std::fs::write(&target, b"old").unwrap();
        std::fs::write(&project, b"project data").unwrap();
        let bad = Artifact { size: 3, sha256: "00".repeat(32) };
        assert!(replace_portable(&source, &target, &bad).is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"old");
        let good = Artifact { size: 3, sha256: to_hex(&sha256(b"new")) };
        let backup = replace_portable(&source, &target, &good).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"new");
        assert_eq!(std::fs::read(&backup).unwrap(), b"old");
        restore_portable(&target, &backup).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"old");
        assert_eq!(std::fs::read(&project).unwrap(), b"project data");
        for entry in std::fs::read_dir(&dir).unwrap() { std::fs::remove_file(entry.unwrap().path()).unwrap(); }
        std::fs::remove_dir(&dir).unwrap();
    }

    #[test]
    fn locked_portable_target_keeps_the_original() {
        use std::os::windows::fs::OpenOptionsExt;
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("neramit-locked-update-test-{}-{stamp}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("download.exe");
        let target = dir.join("planner.exe");
        std::fs::write(&source, b"new").unwrap();
        std::fs::write(&target, b"old").unwrap();
        let locked = std::fs::OpenOptions::new().read(true).share_mode(0).open(&target).unwrap();
        let artifact = Artifact { size: 3, sha256: to_hex(&sha256(b"new")) };
        assert!(replace_portable(&source, &target, &artifact).is_err());
        drop(locked);
        assert_eq!(std::fs::read(&target).unwrap(), b"old");
        for entry in std::fs::read_dir(&dir).unwrap() { std::fs::remove_file(entry.unwrap().path()).unwrap(); }
        std::fs::remove_dir(&dir).unwrap();
    }

    #[test]
    fn missing_ready_download_returns_to_retryable_failed_state() {
        let rel = parse_manifest(&serde_json::to_vec(&signed_fixture()).unwrap()).unwrap();
        let missing = std::env::temp_dir().join(format!("neramit-missing-update-test-{}.exe", std::process::id()));
        assert!(!missing.exists());
        set(Status::Ready(rel.clone(), missing.clone()));
        install_on_exit(rel, missing, true);
        assert!(on_exit().is_err());
        assert!(matches!(status(), Status::Failed(_)));
        assert!(INSTALL_ON_EXIT.lock().unwrap().is_none());
        set(Status::Idle);
    }
}
