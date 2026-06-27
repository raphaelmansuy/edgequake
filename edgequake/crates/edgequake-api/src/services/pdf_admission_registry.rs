//! Process-local PDF ingest admission slots (P-G15 TOCTOU guard).
//!
//! Closes the race where two concurrent uploads pass `find_active_pdf_processing_task`
//! before either task row exists. Registry entries self-heal when storage shows no
//! active task (see `admit_pdf_processing_enqueue`).

use std::collections::HashMap;
use std::sync::Mutex;

use uuid::Uuid;

#[derive(Debug, Default)]
pub struct PdfAdmissionRegistry {
    slots: Mutex<HashMap<(Uuid, Uuid), String>>,
}

impl PdfAdmissionRegistry {
    /// Register `track_id` for `(workspace_id, pdf_id)` or return an existing holder.
    pub fn try_register(&self, workspace_id: Uuid, pdf_id: Uuid, track_id: &str) -> Option<String> {
        let mut map = self.slots.lock().expect("pdf admission registry lock");
        let key = (workspace_id, pdf_id);
        if let Some(existing) = map.get(&key) {
            if existing != track_id {
                return Some(existing.clone());
            }
            return None;
        }
        map.insert(key, track_id.to_string());
        None
    }

    pub fn get(&self, workspace_id: Uuid, pdf_id: Uuid) -> Option<String> {
        self.slots
            .lock()
            .expect("pdf admission registry lock")
            .get(&(workspace_id, pdf_id))
            .cloned()
    }

    pub fn release(&self, workspace_id: Uuid, pdf_id: Uuid) {
        self.slots
            .lock()
            .expect("pdf admission registry lock")
            .remove(&(workspace_id, pdf_id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_register_returns_existing_on_conflict() {
        let reg = PdfAdmissionRegistry::default();
        let ws = Uuid::new_v4();
        let pdf = Uuid::new_v4();
        assert!(reg.try_register(ws, pdf, "track-a").is_none());
        assert_eq!(
            reg.try_register(ws, pdf, "track-b").as_deref(),
            Some("track-a")
        );
    }

    #[test]
    fn release_allows_new_registration() {
        let reg = PdfAdmissionRegistry::default();
        let ws = Uuid::new_v4();
        let pdf = Uuid::new_v4();
        reg.try_register(ws, pdf, "track-a");
        reg.release(ws, pdf);
        assert!(reg.try_register(ws, pdf, "track-b").is_none());
    }
}
