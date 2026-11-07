//! Undo / Redo steps. A step is the whole project as it was; on a large project the steps behind
//! the latest one are packed (JSON, deflated) on a background thread, so 20 levels of Undo on a
//! 5000-task project take a few MB instead of ~90 MB.

use crate::model::Project;
use std::thread::JoinHandle;

/// Projects smaller than this stay as they are (packing them saves little).
const PACK_FROM_TASKS: usize = 300;

/// A packed project, with what its JSON leaves out.
struct Packed {
    bytes: Vec<u8>,
    task_src: Vec<Option<u32>>,
    res_src: Vec<Option<u32>>,
    file_format: Option<u8>,
}

impl Packed {
    /// None when the project does not come back the same from its JSON (it then stays unpacked).
    fn new(p: &Project) -> Option<Packed> {
        let json = serde_json::to_vec(p).ok()?;
        // (a value JSON cannot hold, e.g. NaN, would not read back)
        serde_json::from_slice::<Project>(&json).ok()?;
        Some(Packed {
            bytes: miniz_oxide::deflate::compress_to_vec(&json, 1),
            task_src: p.tasks.iter().map(|t| t.src_uid).collect(),
            res_src: p.resources.iter().map(|r| r.src_uid).collect(),
            file_format: p.file_format,
        })
    }

    fn unpack(&self) -> Option<Project> {
        let json = miniz_oxide::inflate::decompress_to_vec(&self.bytes).ok()?;
        let mut p: Project = serde_json::from_slice(&json).ok()?;
        for (t, s) in p.tasks.iter_mut().zip(&self.task_src) {
            t.src_uid = *s;
        }
        for (r, s) in p.resources.iter_mut().zip(&self.res_src) {
            r.src_uid = *s;
        }
        p.file_format = self.file_format;
        Some(p)
    }
}

/// One step: the project itself until its packed copy is ready.
struct Step {
    live: Option<Box<Project>>,
    packed: Option<Packed>,
    job: Option<JoinHandle<Option<Packed>>>,
}

impl Step {
    fn pack_later(&mut self) {
        if self.job.is_some() || self.packed.is_some() {
            return;
        }
        if let Some(p) = self.live.as_deref().filter(|p| p.tasks.len() >= PACK_FROM_TASKS) {
            let copy = p.clone();
            self.job = std::thread::Builder::new().name("undo-pack".into()).spawn(move || Packed::new(&copy)).ok();
        }
    }

    /// Takes a finished packed copy (waits for it when `wait`) and lets the project go.
    fn settle(&mut self, wait: bool) {
        if !self.job.as_ref().is_some_and(|h| wait || h.is_finished()) {
            return;
        }
        if let Some(Ok(Some(pk))) = self.job.take().map(|h| h.join()) {
            self.packed = Some(pk);
            self.live = None;
        }
    }

    fn into_project(mut self) -> Option<Project> {
        // (the project is still here while its packing runs: no need to wait)
        if let Some(p) = self.live.take() {
            return Some(*p);
        }
        self.settle(true);
        self.packed.as_ref().and_then(Packed::unpack)
    }
}

/// A stack of steps; the top one is never packed, so the next Undo is immediate.
#[derive(Default)]
pub struct UndoStack {
    steps: Vec<Step>,
}

impl UndoStack {
    pub fn push(&mut self, p: Project) {
        if let Some(top) = self.steps.last_mut() {
            top.pack_later();
        }
        self.steps.push(Step { live: Some(Box::new(p)), packed: None, job: None });
        self.tick();
    }

    pub fn pop(&mut self) -> Option<Project> {
        // (a step that cannot be read back is skipped)
        while let Some(s) = self.steps.pop() {
            if let Some(p) = s.into_project() {
                return Some(p);
            }
        }
        None
    }

    /// Drops the newest step without reading it back (a change that was given up).
    pub fn discard_last(&mut self) {
        self.steps.pop();
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    pub fn clear(&mut self) {
        self.steps.clear();
    }

    /// Keeps the newest `keep` steps.
    pub fn keep_last(&mut self, keep: usize) {
        if self.steps.len() > keep {
            self.steps.drain(..self.steps.len() - keep);
        }
    }

    /// Frees the projects whose packed copies are ready (called every frame; cheap).
    pub fn tick(&mut self) {
        for s in self.steps.iter_mut() {
            s.settle(false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_steps_come_back_the_same() {
        let bytes = include_bytes!("../examples/sample.neramit");
        let f: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        let mut p: Project = serde_json::from_value(f["project"].clone()).unwrap();
        crate::schedule::schedule(&mut p).unwrap();
        p.tasks[2].src_uid = Some(77);
        p.file_format = Some(14);
        let pk = Packed::new(&p).expect("packs");
        let q = pk.unpack().expect("unpacks");
        assert_eq!(serde_json::to_string(&p).unwrap(), serde_json::to_string(&q).unwrap());
        assert_eq!(q.tasks[2].src_uid, Some(77));
        assert_eq!(q.file_format, Some(14));
    }

    #[test]
    fn large_projects_are_packed_behind_the_top_step() {
        let mut p = Project::new();
        for k in 0..PACK_FROM_TASKS + 5 {
            let t = p.new_task(&format!("งาน {k}"));
            p.tasks.push(t);
        }
        let mut u = UndoStack::default();
        for k in 0..3 {
            p.title = format!("v{k}");
            u.push(p.clone());
        }
        for s in u.steps.iter_mut() {
            s.settle(true);
        }
        assert!(u.steps[0].live.is_none() && u.steps[0].packed.is_some());
        assert!(u.steps[2].live.is_some());
        let titles: Vec<String> = std::iter::from_fn(|| u.pop()).map(|p| p.title).collect();
        assert_eq!(titles, ["v2", "v1", "v0"]);
    }
}
