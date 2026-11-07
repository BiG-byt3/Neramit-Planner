//! Headless QA: run the whole program without a window from a script of steps (open a file,
//! click, drag, type, run commands, take screenshots). Frames are drawn by egui as usual and the
//! triangles are rasterised here on the CPU, so screenshots need no window or GPU and input never
//! touches the real mouse or keyboard.
//!
//! `NeramitPlanner --qa-script steps.txt` (one step per line, `#` comments):
//!   size W H | open PATH | frames N | move X Y | click X Y | dclick X Y | rclick X Y
//!   drag X1 Y1 X2 Y2 | key NAME [ctrl] [shift] [alt] | type TEXT | cmd NAME | view NAME
//!   table N | select TASK | shot FILE.png | crop FILE.png X Y W H [SCALE] | expect FIELD VALUE
//!   print FIELD | bench N | profile | saveas PATH | log TEXT | newwindow VIEW | wait MS | picture FILE.png | boq FILE.xlsx

use crate::app::{App, View};
use crate::ribbon::Cmd;
use eframe::egui::{self, epaint, Event, Key, Modifiers, PointerButton, Pos2, Rect, Vec2};
use std::collections::HashMap;

struct Runner {
    ctx: egui::Context,
    app: App,
    size: Vec2,
    time: f64,
    pointer: Pos2,
    textures: HashMap<egui::TextureId, epaint::ColorImage>,
    last: Option<egui::FullOutput>,
    log: Vec<String>,
}

impl Runner {
    fn frame(&mut self, events: Vec<Event>, _modifiers: Modifiers) {
        self.time += 1.0 / 30.0;
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.size)),
            time: Some(self.time),
            events,
            focused: true,
            ..Default::default()
        };
        let app = &mut self.app;
        let out = self.ctx.run_ui(input, |ui| crate::app_frame(app, ui));
        for (id, deltas) in &out.textures_delta.set {
            for delta in deltas.iter() {
            let egui::ImageData::Color(img) = &delta.image;
            match delta.pos {
                None => {
                    self.textures.insert(*id, epaint::ColorImage::clone(img));
                }
                Some([x0, y0]) => {
                    if let Some(t) = self.textures.get_mut(id) {
                        for y in 0..img.size[1] {
                            for x in 0..img.size[0] {
                                if x0 + x < t.size[0] && y0 + y < t.size[1] {
                                    t.pixels[(y0 + y) * t.size[0] + x0 + x] = img.pixels[y * img.size[0] + x];
                                }
                            }
                        }
                    }
                }
            }
            }
        }
        for id in &out.textures_delta.free {
            self.textures.remove(id);
        }
        // answer screenshot requests (Copy Picture) as the window would: with this frame
        let shots: Vec<egui::ScreenshotCallback> = out
            .viewport_output
            .values()
            .flat_map(|v| v.commands.iter())
            .filter_map(|c| match c {
                egui::ViewportCommand::Screenshot(cb) => Some(cb.clone()),
                _ => None,
            })
            .collect();
        if !shots.is_empty() {
            if let Some((w, h, rgba)) = self.render_output(&out) {
                let pixels = rgba.chunks_exact(4).map(|c| egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])).collect();
                let img = std::sync::Arc::new(egui::ColorImage::new([w, h], pixels));
                for cb in shots {
                    cb.complete(img.clone());
                }
            }
        }
        self.last = Some(out);
    }

    fn frames(&mut self, n: usize) {
        for _ in 0..n {
            self.frame(Vec::new(), Modifiers::NONE);
        }
    }

    fn pointer_to(&mut self, p: Pos2) {
        self.pointer = p;
        self.frame(vec![Event::PointerMoved(p)], Modifiers::NONE);
    }

    fn button(&mut self, b: PointerButton, pressed: bool, m: Modifiers) {
        let p = self.pointer;
        self.frame(vec![Event::PointerButton { pos: p, button: b, pressed, modifiers: m }], m);
    }

    fn click(&mut self, p: Pos2, b: PointerButton, n: usize) {
        self.pointer_to(p);
        for _ in 0..n {
            self.button(b, true, Modifiers::NONE);
            self.button(b, false, Modifiers::NONE);
        }
        self.frames(3);
    }

    fn drag(&mut self, a: Pos2, b: Pos2) {
        self.pointer_to(a);
        self.button(PointerButton::Primary, true, Modifiers::NONE);
        for k in 1..=10 {
            self.pointer_to(a + (b - a) * (k as f32 / 10.0));
        }
        self.button(PointerButton::Primary, false, Modifiers::NONE);
        self.frames(3);
    }

    fn key(&mut self, key: Key, m: Modifiers) {
        self.frame(vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: m }], m);
        self.frame(vec![Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers: m }], m);
        self.frames(2);
    }

    fn shot(&mut self, path: &str) {
        let Some((w, h, rgba)) = self.render() else { return };
        let _ = std::fs::write(path, crate::png(w as u32, h as u32, &rgba));
        self.log.push(format!("shot {path}"));
    }

    /// A part of the screen enlarged `scale` times (nearest pixel), to check small details.
    fn crop(&mut self, path: &str, x: usize, y: usize, cw: usize, ch: usize, scale: usize) {
        let Some((w, h, rgba)) = self.render() else { return };
        let scale = scale.clamp(1, 8);
        let (cw, ch) = (cw.min(w.saturating_sub(x)), ch.min(h.saturating_sub(y)));
        let mut out = Vec::with_capacity(cw * ch * scale * scale * 4);
        for yy in 0..ch * scale {
            for xx in 0..cw * scale {
                let k = ((y + yy / scale) * w + x + xx / scale) * 4;
                out.extend_from_slice(&rgba[k..k + 4]);
            }
        }
        let _ = std::fs::write(path, crate::png((cw * scale) as u32, (ch * scale) as u32, &out));
        self.log.push(format!("crop {path}"));
    }

    /// Rasterise the last frame (premultiplied sRGB, as egui's painters do). Pixels on an edge
    /// shared by two triangles are drawn once (top-left rule), as a GPU does.
    fn render(&mut self) -> Option<(usize, usize, Vec<u8>)> {
        self.frames(2);
        let out = self.last.take()?;
        self.render_output(&out)
    }

    fn render_output(&self, out: &egui::FullOutput) -> Option<(usize, usize, Vec<u8>)> {
        let ppp = out.pixels_per_point;
        let prims = self.ctx.tessellate(out.shapes.clone(), ppp);
        let (w, h) = ((self.size.x * ppp) as usize, (self.size.y * ppp) as usize);
        let mut fb = vec![[0f32; 4]; w * h];
        for cp in &prims {
            let epaint::Primitive::Mesh(mesh) = &cp.primitive else { continue };
            let Some(tex) = self.textures.get(&mesh.texture_id) else { continue };
            let clip = cp.clip_rect;
            let (cx0, cy0) = ((clip.min.x * ppp).max(0.0) as i32, (clip.min.y * ppp).max(0.0) as i32);
            let (cx1, cy1) = (((clip.max.x * ppp).ceil() as i32).min(w as i32), ((clip.max.y * ppp).ceil() as i32).min(h as i32));
            for tri in mesh.indices.chunks_exact(3) {
                let v = [&mesh.vertices[tri[0] as usize], &mesh.vertices[tri[1] as usize], &mesh.vertices[tri[2] as usize]];
                let p: Vec<(f32, f32)> = v.iter().map(|v| (v.pos.x * ppp, v.pos.y * ppp)).collect();
                let area = (p[1].0 - p[0].0) * (p[2].1 - p[0].1) - (p[2].0 - p[0].0) * (p[1].1 - p[0].1);
                if area.abs() < 1e-6 {
                    continue;
                }
                let minx = (p.iter().map(|q| q.0).fold(f32::MAX, f32::min).floor() as i32).max(cx0);
                let maxx = (p.iter().map(|q| q.0).fold(f32::MIN, f32::max).ceil() as i32).min(cx1);
                let miny = (p.iter().map(|q| q.1).fold(f32::MAX, f32::min).floor() as i32).max(cy0);
                let maxy = (p.iter().map(|q| q.1).fold(f32::MIN, f32::max).ceil() as i32).min(cy1);
                let col: Vec<[f32; 4]> = v.iter().map(|v| v.color.to_array().map(|c| c as f32 / 255.0)).collect();
                // edge k is opposite vertex k; a top edge (horizontal, triangle below) or a left
                // edge (triangle to its right) owns the pixels that lie exactly on it
                let top_left = |k: usize| {
                    let (a, b, c) = (p[(k + 1) % 3], p[(k + 2) % 3], p[k]);
                    if (a.1 - b.1).abs() < 1e-6 {
                        c.1 > a.1
                    } else {
                        c.0 > a.0 + (c.1 - a.1) / (b.1 - a.1) * (b.0 - a.0)
                    }
                };
                let tl = [top_left(0), top_left(1), top_left(2)];
                let inside = |w: f32, k: usize| w > 1e-5 || (w > -1e-5 && tl[k]);
                for y in miny..maxy {
                    for x in minx..maxx {
                        let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                        let w0 = ((p[1].0 - px) * (p[2].1 - py) - (p[2].0 - px) * (p[1].1 - py)) / area;
                        let w1 = ((p[2].0 - px) * (p[0].1 - py) - (p[0].0 - px) * (p[2].1 - py)) / area;
                        let w2 = 1.0 - w0 - w1;
                        if !(inside(w0, 0) && inside(w1, 1) && inside(w2, 2)) {
                            continue;
                        }
                        let u = w0 * v[0].uv.x + w1 * v[1].uv.x + w2 * v[2].uv.x;
                        let vv = w0 * v[0].uv.y + w1 * v[1].uv.y + w2 * v[2].uv.y;
                        let tx = ((u * tex.size[0] as f32) as usize).min(tex.size[0] - 1);
                        let ty = ((vv * tex.size[1] as f32) as usize).min(tex.size[1] - 1);
                        let t = tex.pixels[ty * tex.size[0] + tx].to_array().map(|c| c as f32 / 255.0);
                        let mut src = [0f32; 4];
                        for k in 0..4 {
                            src[k] = (w0 * col[0][k] + w1 * col[1][k] + w2 * col[2][k]) * t[k];
                        }
                        let d = &mut fb[y as usize * w + x as usize];
                        for k in 0..4 {
                            d[k] = src[k] + d[k] * (1.0 - src[3]);
                        }
                    }
                }
            }
        }
        let rgba: Vec<u8> = fb
            .iter()
            .flat_map(|c| {
                let a = c[3].clamp(0.0, 1.0);
                // composite on white, unpremultiplied output
                let f = |x: f32| ((x + (1.0 - a)) * 255.0).round().clamp(0.0, 255.0) as u8;
                [f(c[0]), f(c[1]), f(c[2]), 255]
            })
            .collect();
        Some((w, h, rgba))
    }

    fn field(&self, name: &str) -> String {
        let a = &self.app;
        match name {
            "tasks" => a.doc.tasks.len().to_string(),
            "view" => format!("{:?}", a.view),
            "dialog" => a.dialog.as_ref().map(|d| format!("{d:?}").split('(').next().unwrap_or("").to_string()).unwrap_or_else(|| "None".into()),
            "message" => a.message.as_ref().map(|m| m.1.clone()).unwrap_or_else(|| "None".into()),
            "status" => a.status.clone(),
            "sel" => a.current_task().map(|i| a.doc.tasks[i].name.clone()).unwrap_or_default(),
            "dirty" => a.dirty.to_string(),
            "styles" => crate::format::TextItem::ALL.iter().map(|i| { let t = a.doc.formatting.text_style(*i); format!("{:?}={}/{}/{}", i, t.font, t.size, t.px()) }).collect::<Vec<_>>().join(" | "),
            "closing" => a.confirm_close_ok.to_string(),
            "update" => format!("{:?}", crate::update::status()).chars().take(200).collect(),
            "backstage" => format!("{:?}", a.backstage),
            "ppd" => format!("{}", a.px_per_day),
            "table" => a.table.to_string(),
            "namew" => a.task_cols.iter().find(|c| c.0 == crate::app::TCol::Name).map(|c| c.1.to_string()).unwrap_or_default(),
            "undo" => a.undo.len().to_string(),
            // the project's windows: "1,2*,3h" (* active, h hidden) and whether they are tiled
            "windows" => {
                let w: Vec<String> = a.window_list().iter().map(|(n, h)| format!("{n}{}{}", if *n == a.win_num { "*" } else { "" }, if *h { "h" } else { "" })).collect();
                format!("{}{}", w.join(","), if a.arranged { " arranged" } else { "" })
            }
            "title" => a.title(),
            _ => {
                // TASKNAME.field : the task's cell text in that column title
                if let Some((task, col)) = name.split_once('.') {
                    if let Some(i) = a.doc.tasks.iter().position(|t| t.name.trim() == task) {
                        if let Some(c) = crate::app::TCol::ALL.iter().find(|c| c.title().eq_ignore_ascii_case(col)) {
                            return a.cell_text(i, *c);
                        }
                    }
                }
                format!("<unknown {name}>")
            }
        }
    }
}

fn key_of(s: &str) -> Option<Key> {
    Key::from_name(s)
}

fn view_of(s: &str) -> Option<View> {
    Some(match s.to_lowercase().as_str() {
        "gantt" => View::Gantt,
        "tracking" => View::TrackingGantt,
        "sheet" => View::TaskSheet,
        "resources" => View::ResourceSheet,
        "taskusage" => View::TaskUsage,
        "resourceusage" => View::ResourceUsage,
        "network" => View::NetworkDiagram,
        "calendar" => View::Calendar,
        "taskboard" => View::TaskBoard,
        "report" => View::Report,
        _ => return None,
    })
}

fn cmd_of(s: &str) -> Option<Cmd> {
    Some(match s {
        "TaskInfo" => Cmd::TaskInfo,
        "About" => Cmd::About,
        "Options" => Cmd::Options(0),
        "OptionsAI" => Cmd::Options(crate::fmt_dialogs::AI_PAGE),
        "CheckUpdates" => Cmd::CheckUpdates,
        "Exit" => Cmd::Exit,
        "SetBaseline" => Cmd::SetBaseline,
        "ClearBaseline" => Cmd::ClearBaseline,
        "UpdateTasks" => Cmd::UpdateTasks,
        "UpdateProject" => Cmd::UpdateProject,
        "MarkOnTrack" => Cmd::MarkOnTrack,
        "AssignResources" => Cmd::AssignResources,
        "LevelAll" => Cmd::LevelAll,
        "ClearLeveling" => Cmd::ClearLeveling,
        "PrintPage" => Cmd::PrintPage,
        "SplitTask" => Cmd::SplitTask,
        "Undo" => Cmd::Undo,
        "Redo" => Cmd::Redo,
        "InsertTask" => Cmd::InsertTask,
        "Indent" => Cmd::Indent,
        "Outdent" => Cmd::Outdent,
        "Link" => Cmd::Link,
        "Unlink" => Cmd::Unlink,
        "ZoomIn" => Cmd::ZoomIn,
        "ZoomOut" => Cmd::ZoomOut,
        "EntireProject" => Cmd::EntireProject,
        "ScrollToTask" => Cmd::ScrollToTask,
        "FormatBar" => Cmd::FormatBar,
        "BarStyles" => Cmd::BarStyles,
        "ProjectInfo" => Cmd::ProjectInfo,
        "WorkingTime" => Cmd::WorkingTime,
        "Payments" => Cmd::Payments,
        "ResourceInfo" => Cmd::ResourceInfo,
        "MoveTaskCustom" => Cmd::MoveTaskCustom,
        "MoveProject" => Cmd::MoveProject,
        "Percent50" => Cmd::Percent(50),
        "Percent100" => Cmd::Percent(100),
        "Percent0" => Cmd::Percent(0),
        "ToggleProjectSummary" => Cmd::ToggleProjectSummary,
        "LevelingOptions" => Cmd::LevelingOptions,
        "FormatPainter" => Cmd::FormatPainter,
        "ToggleDetails" => Cmd::ToggleDetails,
        "DrawTextBox" => Cmd::DrawingTool(0),
        "DrawArrow" => Cmd::DrawingTool(2),
        "DrawRect" => Cmd::DrawingTool(3),
        "DrawOval" => Cmd::DrawingTool(4),
        "ManageSprints" => Cmd::ManageSprints,
        "MacroRecord" => Cmd::MacroRecord,
        "MacroStop" => Cmd::MacroStop,
        "CustomFields" => Cmd::CustomFields,
        "ProgressLines" => Cmd::ProgressLines,
        "WbsDefine" => Cmd::WbsDefine,
        "WbsRenumber" => Cmd::WbsRenumber(false),
        "Table7" => Cmd::Table(7),
        "Table8" => Cmd::Table(8),
        "TaskPathPreds" => Cmd::TaskPath(1),
        "TaskPathDriving" => Cmd::TaskPath(2),
        "TaskPathSuccs" => Cmd::TaskPath(4),
        "TaskPathDriven" => Cmd::TaskPath(8),
        "BaselineBars" => Cmd::BaselineBars(Some(0)),
        "Slippage" => Cmd::Slippage(Some(0)),
        "NextOverallocation" => Cmd::NextOverallocation,
        "ResourceNotes" => Cmd::ResourceNotes,
        "NewWindow" => Cmd::NewWindow,
        "ProjectLinks" => Cmd::ProjectLinks,
        "ShareResources" => Cmd::ShareResources,
        "RefreshPool" => Cmd::RefreshPool,
        "UpdatePool" => Cmd::UpdatePool,
        "ArrangeAll" => Cmd::ArrangeAll,
        "HideWindow" => Cmd::HideWindow,
        "CloseWindow" => Cmd::CloseWindow,
        "SwitchWindow1" => Cmd::SwitchWindow(1),
        "SwitchWindow2" => Cmd::SwitchWindow(2),
        "SwitchWindow3" => Cmd::SwitchWindow(3),
        "Unhide1" => Cmd::UnhideWindow(1),
        "Unhide2" => Cmd::UnhideWindow(2),
        _ => return None,
    })
}

/// (private bytes, working set) of this process.
pub fn process_memory() -> (usize, usize) {
    use windows_sys::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX};
    let mut c: PROCESS_MEMORY_COUNTERS_EX = unsafe { std::mem::zeroed() };
    c.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
    let ok = unsafe { K32GetProcessMemoryInfo(windows_sys::Win32::System::Threading::GetCurrentProcess(), &mut c as *mut _ as *mut PROCESS_MEMORY_COUNTERS, c.cb) };
    if ok == 0 {
        return (0, 0);
    }
    (c.PrivateUsage, c.WorkingSetSize)
}

pub fn run(path: &str) -> i32 {
    let script = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read {path}: {e}");
            return 2;
        }
    };
    // a QA run leaves the user's settings, recent files and favourites alone
    crate::app::NO_PERSIST.store(true, std::sync::atomic::Ordering::Relaxed);
    let ctx = egui::Context::default();
    let app = App::with_ctx(&ctx, None);
    crate::i18n::set_lang(if app.settings.language == 1 { crate::i18n::Lang::Thai } else { crate::i18n::Lang::English });
    let mut r = Runner { ctx, app, size: Vec2::new(1600.0, 900.0), time: 0.0, pointer: Pos2::ZERO, textures: HashMap::new(), last: None, log: Vec::new() };
    r.frames(5);
    let mut failures = 0;
    for (n, line) in script.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        let num = |k: usize| parts.get(k).and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            match parts[0] {
                "size" => r.size = Vec2::new(num(1), num(2)),
                "lang" => {
                    // the display language and, as when switching it in the program, its dates
                    let th = parts.get(1) == Some(&"th");
                    crate::i18n::set_lang(if th { crate::i18n::Lang::Thai } else { crate::i18n::Lang::English });
                    r.app.settings.language = th as u8;
                    r.app.loc = r.app.settings.locale();
                }
                "open" => {
                    let p = line[5..].trim();
                    r.app.open_path(std::path::PathBuf::from(p));
                    r.frames(6);
                }
                "frames" => r.frames(num(1) as usize),
                // a ribbon tab: tab help | view | format | project | report | resource | task
                "tab" => {
                    use crate::app::RibbonTab as T;
                    r.app.backstage = None;
                    r.app.tab = match parts.get(1).copied().unwrap_or("") {
                        "resource" => T::Resource,
                        "report" => T::Report,
                        "project" => T::Project,
                        "view" => T::View,
                        "help" => T::Help,
                        "format" => T::Format,
                        _ => T::Task,
                    };
                    r.frames(3);
                }
                // background work (update check / download): real time passes, frames keep drawing
                "wait" => {
                    for _ in 0..(num(1) as usize / 100).max(1) {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        r.frames(1);
                    }
                }
                "compare" => {
                    // Report > Compare Projects without the file dialog
                    let p = line[8..].trim().to_string();
                    crate::compare::compare_with(&mut r.app, std::path::Path::new(&p));
                    r.frames(4);
                }
                "report" => {
                    // a report by its position in the Report tab's lists
                    use crate::reports::ReportKind as K;
                    let all = [K::ProjectOverview, K::CostOverview, K::WorkOverview, K::UpcomingTasks, K::Burndown, K::ResourceOverview, K::OverallocatedResources, K::CashFlow, K::CostOverruns, K::EarnedValue, K::TaskCostOverview, K::ResourceCostOverview, K::CriticalTasks, K::LateTasks, K::MilestoneReport, K::SlippingTasks, K::BoardStatus, K::GettingStarted, K::CustomTable, K::SCurve, K::Payments];
                    let k = all[(num(1) as usize).min(all.len() - 1)];
                    let ctx = r.ctx.clone();
                    crate::ribbon::run(&mut r.app, &ctx, Cmd::Report(k));
                    r.frames(4);
                }
                "boq" => {
                    // Project > Import BOQ without the file dialog
                    crate::boq::open(&mut r.app, std::path::Path::new(line[3..].trim()));
                    r.frames(4);
                }
                "picture" => {
                    // Copy Picture, written to a file instead of the clipboard
                    r.app.picture.qa_path = Some(std::path::PathBuf::from(line[7..].trim()));
                    crate::picture::request(&mut r.app, false);
                    r.frames(8);
                    r.log.push(format!("picture {}", line[7..].trim()));
                }
                "saveas" => {
                    // File > Save As without the file dialog
                    let p = line[7..].trim();
                    let ok = r.app.save_to(std::path::PathBuf::from(p));
                    r.log.push(format!("saveas {p}: {}", if ok { "saved" } else { "refused" }));
                    r.frames(3);
                }
                "profile" => {
                    let t = std::time::Instant::now();
                    for _ in 0..20 {
                        std::hint::black_box(r.app.visible_rows());
                    }
                    let vr = t.elapsed().as_secs_f64() * 1000.0 / 20.0;
                    let t = std::time::Instant::now();
                    for _ in 0..20 {
                        for i in 0..r.app.doc.tasks.len().min(2000) {
                            for c in r.app.task_cols.clone() {
                                std::hint::black_box(r.app.cell_text(i, c.0));
                            }
                        }
                    }
                    let ct = t.elapsed().as_secs_f64() * 1000.0 / 20.0;
                    r.log.push(format!("profile: visible_rows {vr:.2} ms, cell_text (all cells) {ct:.2} ms, wrap {:?}", r.app.wrap_cols));
                    for c in r.app.task_cols.clone() {
                        let t = std::time::Instant::now();
                        for i in 0..r.app.doc.tasks.len() {
                            std::hint::black_box(r.app.cell_text(i, c.0));
                        }
                        r.log.push(format!("  {:?}: {:.2} ms", c.0, t.elapsed().as_secs_f64() * 1000.0));
                    }
                }
                // memory in use: private bytes and working set (MB)
                "mem" => {
                    let (private, ws) = process_memory();
                    r.log.push(format!("mem {}: private {:.1} MB, working set {:.1} MB", parts.get(1).unwrap_or(&""), private as f64 / 1048576.0, ws as f64 / 1048576.0));
                }
                "bench" => {
                    // average time to build one frame (layout and painting, without the GPU)
                    let n = (num(1) as usize).max(1);
                    let t = std::time::Instant::now();
                    for _ in 0..n {
                        r.frame(Vec::new(), Modifiers::NONE);
                    }
                    r.log.push(format!("bench {:?}: {:.1} ms/frame", r.app.view, t.elapsed().as_secs_f64() * 1000.0 / n as f64));
                }
                "move" => r.pointer_to(Pos2::new(num(1), num(2))),
                "click" => r.click(Pos2::new(num(1), num(2)), PointerButton::Primary, 1),
                "dclick" => r.click(Pos2::new(num(1), num(2)), PointerButton::Primary, 2),
                "rclick" => r.click(Pos2::new(num(1), num(2)), PointerButton::Secondary, 1),
                "drag" => r.drag(Pos2::new(num(1), num(2)), Pos2::new(num(3), num(4))),
                "key" => {
                    let mut m = Modifiers::NONE;
                    for x in &parts[2..] {
                        match *x {
                            "ctrl" => {
                                m.ctrl = true;
                                m.command = true;
                            }
                            "shift" => m.shift = true,
                            "alt" => m.alt = true,
                            _ => {}
                        }
                    }
                    match key_of(parts[1]) {
                        Some(k) => r.key(k, m),
                        None => r.log.push(format!("line {}: unknown key {}", n + 1, parts[1])),
                    }
                }
                "type" => {
                    let text = line[5..].to_string();
                    r.frame(vec![Event::Text(text)], Modifiers::NONE);
                    r.frames(2);
                }
                "cmd" => match cmd_of(parts[1]) {
                    Some(c) => {
                        let ctx = r.ctx.clone();
                        crate::ribbon::run(&mut r.app, &ctx, c);
                        r.frames(4);
                    }
                    None => r.log.push(format!("line {}: unknown cmd {}", n + 1, parts[1])),
                },
                "newwindow" => match view_of(parts[1]) {
                    Some(v) => {
                        r.app.new_window(v);
                        r.frames(4);
                    }
                    None => r.log.push(format!("line {}: unknown view {}", n + 1, parts[1])),
                },
                "view" => match view_of(parts[1]) {
                    Some(v) => {
                        r.app.view = v;
                        r.frames(4);
                    }
                    None => r.log.push(format!("line {}: unknown view {}", n + 1, parts[1])),
                },
                "table" => {
                    r.app.apply_table(num(1) as u8);
                    r.frames(3);
                }
                "select" => {
                    // select the task by name
                    let name = line[7..].trim();
                    if let Some(i) = r.app.doc.tasks.iter().position(|t| t.name.trim() == name) {
                        r.app.select_task(i);
                    }
                    r.frames(3);
                }
                "shot" => r.shot(parts[1]),
                "crop" => r.crop(parts[1], num(2) as usize, num(3) as usize, num(4) as usize, num(5) as usize, parts.get(6).map(|_| num(6) as usize).unwrap_or(3)),
                "log" => {
                    let s = line[4..].to_string();
                    r.log.push(s);
                }
                "expect" => {
                    // `expect FIELD VALUE`, or `expect FIELD WITH SPACES | VALUE`
                    let rest = line[6..].trim();
                    let (name, want) = match rest.split_once(" | ") {
                        Some((f, v)) => (f.trim().to_string(), v.trim().to_string()),
                        None => (parts[1].to_string(), parts[2..].join(" ")),
                    };
                    let got = r.field(&name);
                    if got != want {
                        r.log.push(format!("FAIL line {}: {} = {:?}, expected {:?}", n + 1, name, got, want));
                        failures += 1;
                    } else {
                        r.log.push(format!("ok {} = {:?}", name, got));
                    }
                }
                "print" => {
                    let name = line[5..].trim();
                    let got = r.field(name);
                    r.log.push(format!("{} = {:?}", name, got));
                }
                other => r.log.push(format!("line {}: unknown step {other}", n + 1)),
            }
        }));
        if result.is_err() {
            r.log.push(format!("PANIC at line {}: {line}", n + 1));
            failures += 1;
            // a panic inside a frame leaves egui mid-pass: start a fresh context
            let ctx = egui::Context::default();
            r.app = App::with_ctx(&ctx, None);
            r.ctx = ctx;
            r.textures.clear();
            r.frames(3);
        }
    }
    for l in &r.log {
        println!("{l}");
    }
    println!("{} failure(s)", failures);
    if failures > 0 {
        1
    } else {
        0
    }
}

