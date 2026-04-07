// Where: crates/wiki_canister/src/state.rs
// What: Durable in-memory state for the wiki canister.
// Why: The canister cannot depend on SQLite, so it needs its own source-of-truth model in stable memory.
use std::collections::BTreeMap;

use candid::CandidType;
use serde::{Deserialize, Serialize};
use wiki_types::{SectionHashEntry, SystemPageSnapshot, WikiPageType, WikiSyncManifest, WikiSyncManifestEntry};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct PageState {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub page_type: WikiPageType,
    pub current_revision_id: Option<String>,
    pub summary_1line: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct RevisionState {
    pub id: String,
    pub page_id: String,
    pub revision_no: u64,
    pub markdown: String,
    pub change_reason: String,
    pub author_type: String,
    pub created_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct SectionState {
    pub section_path: String,
    pub ordinal: u64,
    pub heading: Option<String>,
    pub text: String,
    pub content_hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType)]
pub struct LogEventState {
    pub id: String,
    pub event_type: String,
    pub title: String,
    pub body_markdown: String,
    pub related_page_id: Option<String>,
    pub created_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType, Default)]
pub struct WikiCanisterState {
    pub pages: BTreeMap<String, PageState>,
    pub page_slug_index: BTreeMap<String, String>,
    pub revisions: BTreeMap<String, RevisionState>,
    pub sections_by_page: BTreeMap<String, Vec<SectionState>>,
    pub system_pages: BTreeMap<String, SystemPageSnapshot>,
    pub log_events: Vec<LogEventState>,
    pub snapshot_revision: u64,
    pub next_page_id: u64,
    pub next_revision_id: u64,
    pub next_log_id: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CandidType, Default)]
pub struct StableState {
    pub pages: Vec<(String, PageState)>,
    pub page_slug_index: Vec<(String, String)>,
    pub revisions: Vec<(String, RevisionState)>,
    pub sections_by_page: Vec<(String, Vec<SectionState>)>,
    pub system_pages: Vec<(String, SystemPageSnapshot)>,
    pub log_events: Vec<LogEventState>,
    pub snapshot_revision: u64,
    pub next_page_id: u64,
    pub next_revision_id: u64,
    pub next_log_id: u64,
}

impl WikiCanisterState {
    pub fn new() -> Self {
        let mut state = Self::default();
        state.refresh_system_pages(0);
        state
    }

    #[cfg(test)]
    pub fn allocate_page_id(&mut self) -> String {
        self.next_page_id += 1;
        format!("page_{}", self.next_page_id)
    }

    pub fn allocate_revision_id(&mut self) -> String {
        self.next_revision_id += 1;
        format!("rev_{}", self.next_revision_id)
    }

    pub fn allocate_log_id(&mut self) -> String {
        self.next_log_id += 1;
        format!("log_{}", self.next_log_id)
    }

    pub fn snapshot_revision_string(&self) -> String {
        format!("snapshot_{}", self.snapshot_revision)
    }

    pub fn bump_snapshot_revision(&mut self) {
        self.snapshot_revision += 1;
    }

    pub fn manifest(&self) -> WikiSyncManifest {
        WikiSyncManifest {
            snapshot_revision: self.snapshot_revision_string(),
            pages: self
                .pages
                .values()
                .filter_map(|page| page.current_revision_id.as_ref().map(|revision_id| (page, revision_id)))
                .map(|(page, revision_id)| WikiSyncManifestEntry {
                    page_id: page.id.clone(),
                    slug: page.slug.clone(),
                    revision_id: revision_id.clone(),
                    updated_at: page.updated_at,
                })
                .collect(),
        }
    }

    pub fn page_by_id(&self, page_id: &str) -> Option<&PageState> {
        self.pages.get(page_id)
    }

    pub fn page_by_slug(&self, slug: &str) -> Option<&PageState> {
        self.page_slug_index
            .get(slug)
            .and_then(|page_id| self.pages.get(page_id))
    }

    pub fn revision(&self, revision_id: &str) -> Option<&RevisionState> {
        self.revisions.get(revision_id)
    }

    pub fn section_hashes(&self, page_id: &str) -> Vec<SectionHashEntry> {
        self.sections_by_page
            .get(page_id)
            .into_iter()
            .flatten()
            .map(|section| SectionHashEntry {
                section_path: section.section_path.clone(),
                content_hash: section.content_hash.clone(),
            })
            .collect()
    }

    pub fn to_stable(&self) -> StableState {
        StableState {
            pages: self.pages.clone().into_iter().collect(),
            page_slug_index: self.page_slug_index.clone().into_iter().collect(),
            revisions: self.revisions.clone().into_iter().collect(),
            sections_by_page: self.sections_by_page.clone().into_iter().collect(),
            system_pages: self.system_pages.clone().into_iter().collect(),
            log_events: self.log_events.clone(),
            snapshot_revision: self.snapshot_revision,
            next_page_id: self.next_page_id,
            next_revision_id: self.next_revision_id,
            next_log_id: self.next_log_id,
        }
    }

    pub fn from_stable(stable: StableState) -> Self {
        Self {
            pages: stable.pages.into_iter().collect(),
            page_slug_index: stable.page_slug_index.into_iter().collect(),
            revisions: stable.revisions.into_iter().collect(),
            sections_by_page: stable.sections_by_page.into_iter().collect(),
            system_pages: stable.system_pages.into_iter().collect(),
            log_events: stable.log_events,
            snapshot_revision: stable.snapshot_revision,
            next_page_id: stable.next_page_id,
            next_revision_id: stable.next_revision_id,
            next_log_id: stable.next_log_id,
        }
    }
}
