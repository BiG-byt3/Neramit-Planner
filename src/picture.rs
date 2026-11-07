//! Copy Picture / Save Picture: the view as it is on screen (grid, Gantt chart, report ...), as an
//! image to paste into LINE, an e-mail or a document, or as a PNG file.
//!
//! The command waits two frames so the ribbon menu that started it has closed, asks the window for
//! a screenshot, and crops it to the view area. Headless QA answers the screenshot request with
//! its own rasteriser (`picture PATH` writes the PNG there instead of the clipboard or a dialog).

use crate::app::App;
use crate::i18n::tr;
use eframe::egui;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, Default)]
pub struct PictureState {
    /// a request: (save to a file, frames still to wait)
    pub job: Option<(bool, u8)>,
    /// what to do with the screenshot when it arrives
    pub save: bool,
    pub slot: Arc<Mutex<Option<Arc<egui::ColorImage>>>>,
    /// QA: write the picture here
    pub qa_path: Option<std::path::PathBuf>,
}

pub fn request(app: &mut App, save: bool) {
    app.picture.job = Some((save, 2));
}

/// Called once per frame, after the frame's commands ran.
pub fn tick(app: &mut App, ctx: &egui::Context) {
    let ready = app.picture.slot.lock().ok().and_then(|mut s| s.take());
    if let Some(img) = ready {
        finish(app, &img, ctx.pixels_per_point());
    }
    let Some((save, wait)) = app.picture.job.as_mut() else { return };
    if *wait > 0 {
        *wait -= 1;
        ctx.request_repaint();
        return;
    }
    app.picture.save = *save;
    app.picture.job = None;
    let slot = app.picture.slot.clone();
    let c = ctx.clone();
    ctx.request_screenshot(move |img| {
        if let Ok(mut s) = slot.lock() {
            *s = Some(img);
        }
        c.request_repaint();
    });
}

/// The view area of a screenshot as straight (not premultiplied) RGBA.
pub fn crop(img: &egui::ColorImage, area: egui::Rect, ppp: f32) -> (usize, usize, Vec<u8>) {
    let [iw, ih] = img.size;
    let x0 = ((area.left() * ppp).round().max(0.0) as usize).min(iw);
    let y0 = ((area.top() * ppp).round().max(0.0) as usize).min(ih);
    let x1 = ((area.right() * ppp).round().max(0.0) as usize).clamp(x0, iw);
    let y1 = ((area.bottom() * ppp).round().max(0.0) as usize).clamp(y0, ih);
    let (w, h) = (x1 - x0, y1 - y0);
    let mut rgba = Vec::with_capacity(w * h * 4);
    for y in y0..y1 {
        for x in x0..x1 {
            let mut c = img.pixels[y * iw + x].to_srgba_unmultiplied();
            c[3] = 255;
            rgba.extend_from_slice(&c);
        }
    }
    (w, h, rgba)
}

fn finish(app: &mut App, img: &egui::ColorImage, ppp: f32) {
    let (w, h, rgba) = crop(img, app.view_rect, ppp);
    if w == 0 || h == 0 {
        return;
    }
    let png = crate::png(w as u32, h as u32, &rgba);
    if let Some(p) = app.picture.qa_path.take() {
        let _ = std::fs::write(&p, png);
        app.status = format!("picture {w}x{h}");
        return;
    }
    if app.picture.save {
        let stem = {
            let t = app.doc_name();
            let t = t.trim_end_matches(".neramit").trim_end_matches(".mpp").trim_end_matches(".xml").to_string();
            if t.trim().is_empty() { "Neramit Planner".to_string() } else { t }
        };
        let name = format!("{stem} {}.png", chrono::Local::now().format("%Y-%m-%d"));
        if let Some(p) = rfd::FileDialog::new().set_title(tr("Save Picture")).set_file_name(name).add_filter("PNG", &["png"]).save_file() {
            app.status = match std::fs::write(&p, png) {
                Ok(()) => crate::i18n::trf("Picture saved: {}", &[&p.display().to_string()]),
                Err(e) => format!("{}: {e}", tr("Cannot save the picture")),
            };
        }
    } else {
        app.status = match set_clipboard_image(w, h, &rgba) {
            Ok(()) => tr("Picture copied: paste it into LINE, an e-mail or a document").to_string(),
            Err(e) => format!("{}: {e}", tr("Cannot copy the picture")),
        };
    }
}

/// Put an image on the Windows clipboard as a device-independent bitmap (CF_DIB), which LINE,
/// Office, mail programs and Paint all accept.
#[cfg(windows)]
pub fn set_clipboard_image(w: usize, h: usize, rgba: &[u8]) -> Result<(), String> {
    use std::ffi::c_void;
    #[link(name = "user32")]
    extern "system" {
        fn OpenClipboard(owner: *mut c_void) -> i32;
        fn EmptyClipboard() -> i32;
        fn SetClipboardData(format: u32, mem: *mut c_void) -> *mut c_void;
        fn CloseClipboard() -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalAlloc(flags: u32, bytes: usize) -> *mut c_void;
        fn GlobalLock(mem: *mut c_void) -> *mut c_void;
        fn GlobalUnlock(mem: *mut c_void) -> i32;
        fn GlobalFree(mem: *mut c_void) -> *mut c_void;
    }
    const CF_DIB: u32 = 8;
    const GMEM_MOVEABLE: u32 = 2;
    let data = dib(w, h, rgba);
    unsafe {
        let mem = GlobalAlloc(GMEM_MOVEABLE, data.len());
        if mem.is_null() {
            return Err("out of memory".into());
        }
        let ptr = GlobalLock(mem) as *mut u8;
        if ptr.is_null() {
            GlobalFree(mem);
            return Err("out of memory".into());
        }
        std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
        GlobalUnlock(mem);
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            GlobalFree(mem);
            return Err("the clipboard is in use".into());
        }
        EmptyClipboard();
        let ok = !SetClipboardData(CF_DIB, mem).is_null();
        CloseClipboard();
        if !ok {
            GlobalFree(mem);
            return Err("the clipboard refused the picture".into());
        }
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn set_clipboard_image(_w: usize, _h: usize, _rgba: &[u8]) -> Result<(), String> {
    Err("not supported".into())
}

/// BITMAPINFOHEADER + 32-bit BGRA rows, bottom row first.
fn dib(w: usize, h: usize, rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(40 + w * h * 4);
    let le32 = |v: u32| v.to_le_bytes();
    out.extend_from_slice(&le32(40)); // biSize
    out.extend_from_slice(&(w as i32).to_le_bytes());
    out.extend_from_slice(&(h as i32).to_le_bytes()); // positive: bottom-up
    out.extend_from_slice(&1u16.to_le_bytes()); // planes
    out.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
    out.extend_from_slice(&le32(0)); // BI_RGB
    out.extend_from_slice(&le32((w * h * 4) as u32));
    out.extend_from_slice(&le32(3780)); // 96 dpi
    out.extend_from_slice(&le32(3780));
    out.extend_from_slice(&le32(0));
    out.extend_from_slice(&le32(0));
    for y in (0..h).rev() {
        for x in 0..w {
            let k = (y * w + x) * 4;
            out.extend_from_slice(&[rgba[k + 2], rgba[k + 1], rgba[k], 255]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_and_dib() {
        let mut img = egui::ColorImage::filled([4, 3], egui::Color32::WHITE);
        img.pixels[1 * 4 + 2] = egui::Color32::from_rgb(10, 20, 30);
        let (w, h, rgba) = crop(&img, egui::Rect::from_min_max(egui::pos2(1.0, 1.0), egui::pos2(3.0, 3.0)), 1.0);
        assert_eq!((w, h), (2, 2));
        assert_eq!(&rgba[4..8], &[10, 20, 30, 255]);
        let d = dib(w, h, &rgba);
        assert_eq!(d.len(), 40 + 16);
        // bottom-up: the first row stored is the last row of the picture, BGR order
        assert_eq!(&d[40 + 8 + 4..40 + 8 + 8], &[30, 20, 10, 255]);
    }
}
