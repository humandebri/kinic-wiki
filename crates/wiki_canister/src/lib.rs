// Where: crates/wiki_canister/src/lib.rs
// What: ICP canister entrypoints and durable state wiring for the Kinic wiki.
// Why: The Obsidian plugin should call query/update methods directly without a local HTTP adapter.
mod hash;
mod markdown;
mod ops;
mod render;
mod snapshot;
mod state;

use std::cell::RefCell;

use candid::export_service;
use ic_cdk::{post_upgrade, pre_upgrade, query, update};
use state::WikiCanisterState;
use wiki_types::{
    CommitWikiChangesRequest, CommitWikiChangesResponse, ExportWikiSnapshotRequest,
    ExportWikiSnapshotResponse, FetchWikiUpdatesRequest, FetchWikiUpdatesResponse, Status,
};

thread_local! {
    static STATE: RefCell<WikiCanisterState> = RefCell::new(WikiCanisterState::new());
}

#[query]
fn status() -> Status {
    STATE.with(|state| state.borrow().status())
}

#[query]
fn export_wiki_snapshot(request: ExportWikiSnapshotRequest) -> Result<ExportWikiSnapshotResponse, String> {
    STATE.with(|state| state.borrow().export_snapshot(request))
}

#[query]
fn fetch_wiki_updates(request: FetchWikiUpdatesRequest) -> Result<FetchWikiUpdatesResponse, String> {
    STATE.with(|state| state.borrow().fetch_updates(request))
}

#[update]
fn commit_wiki_changes(request: CommitWikiChangesRequest) -> Result<CommitWikiChangesResponse, String> {
    let updated_at = ic_cdk::api::time()
        .checked_div(1_000_000_000)
        .and_then(|value| i64::try_from(value).ok())
        .unwrap_or(i64::MAX);
    STATE.with(|state| state.borrow_mut().commit_wiki_changes(request, updated_at))
}

#[pre_upgrade]
fn pre_upgrade_hook() {
    let bytes = STATE
        .with(|state| state.borrow().encode())
        .unwrap_or_else(|error| ic_cdk::trap(&error));
    ic_cdk::storage::stable_save((bytes,))
        .unwrap_or_else(|error| ic_cdk::trap(&error.to_string()));
}

#[post_upgrade]
fn post_upgrade_hook() {
    let (bytes,): (Vec<u8>,) =
        ic_cdk::storage::stable_restore().unwrap_or_else(|error| ic_cdk::trap(&error.to_string()));
    let state = WikiCanisterState::decode(&bytes).unwrap_or_else(|error| ic_cdk::trap(&error));
    STATE.with(|slot| *slot.borrow_mut() = state);
}

export_service!();

pub fn candid_interface() -> String {
    __export_service()
}

#[cfg(test)]
mod tests {
    use super::state::WikiCanisterState;
    use wiki_types::{
        CommitWikiChangesRequest, ExportWikiSnapshotRequest, FetchWikiUpdatesRequest,
        KnownPageRevision, PageChangeInput, PageChangeType, WikiPageType,
    };

    #[test]
    fn export_fetch_and_commit_survive_roundtrip() {
        let mut state = WikiCanisterState::new();
        let page_id = state.create_page_for_test("alpha", WikiPageType::Overview, "Alpha", 10);
        let first_revision = state
            .commit_revision_for_test(&page_id, "# Alpha\n\nbody", "Alpha", 11)
            .expect("revision should commit");
        state.refresh_system_pages(11);
        state.bump_snapshot_revision();

        let snapshot = state
            .export_snapshot(ExportWikiSnapshotRequest {
                include_system_pages: true,
                page_slugs: None,
            })
            .expect("snapshot should export");
        let bytes = state.encode().expect("state should encode");
        assert!(!bytes.is_empty());
        let fetched = state
            .fetch_updates(FetchWikiUpdatesRequest {
                known_snapshot_revision: snapshot.snapshot_revision.clone(),
                known_page_revisions: vec![KnownPageRevision {
                    page_id: page_id.clone(),
                    revision_id: first_revision.clone(),
                }],
                include_system_pages: true,
            })
            .expect("fetch should succeed");
        assert!(fetched.changed_pages.is_empty());

        let committed = state
            .commit_wiki_changes(
                CommitWikiChangesRequest {
                    base_snapshot_revision: snapshot.snapshot_revision,
                    page_changes: vec![PageChangeInput {
                        change_type: PageChangeType::Update,
                        page_id: page_id.clone(),
                        base_revision_id: first_revision,
                        new_markdown: Some("# Alpha\n\nupdated".to_string()),
                    }],
                },
                12,
            )
            .expect("commit should succeed");
        assert_eq!(committed.committed_pages.len(), 1);
        assert!(committed.rejected_pages.is_empty());
    }

    #[test]
    fn commit_returns_delete_and_conflict_payloads() {
        let mut state = WikiCanisterState::new();
        let alpha_id = state.create_page_for_test("alpha", WikiPageType::Entity, "Alpha", 10);
        let beta_id = state.create_page_for_test("beta", WikiPageType::Entity, "Beta", 10);
        let alpha_revision = state
            .commit_revision_for_test(&alpha_id, "# Alpha\n\nbody", "Alpha", 11)
            .expect("alpha should commit");
        let beta_revision = state
            .commit_revision_for_test(&beta_id, "# Beta\n\nbody", "Beta", 11)
            .expect("beta should commit");
        state.refresh_system_pages(11);
        state.bump_snapshot_revision();
        let snapshot = state.snapshot_revision_string();

        state
            .commit_revision_for_test(&beta_id, "# Beta\n\nremote change", "Beta", 12)
            .expect("remote change should commit");

        let response = state
            .commit_wiki_changes(
                CommitWikiChangesRequest {
                    base_snapshot_revision: snapshot,
                    page_changes: vec![
                        PageChangeInput {
                            change_type: PageChangeType::Delete,
                            page_id: alpha_id.clone(),
                            base_revision_id: alpha_revision,
                            new_markdown: None,
                        },
                        PageChangeInput {
                            change_type: PageChangeType::Update,
                            page_id: beta_id.clone(),
                            base_revision_id: beta_revision,
                            new_markdown: Some("# Beta\n\nlocal change".to_string()),
                        },
                    ],
                },
                13,
            )
            .expect("commit should succeed");

        assert_eq!(response.manifest_delta.removed_page_ids, vec![alpha_id]);
        assert_eq!(response.rejected_pages.len(), 1);
        assert!(response.rejected_pages[0]
            .conflict_markdown
            .as_deref()
            .unwrap_or_default()
            .contains("<<<<<<< LOCAL"));
    }

    #[test]
    fn new_state_exports_system_pages_before_any_commit() {
        let state = WikiCanisterState::new();

        let snapshot = state
            .export_snapshot(ExportWikiSnapshotRequest {
                include_system_pages: true,
                page_slugs: None,
            })
            .expect("snapshot should export");

        let slugs = snapshot
            .system_pages
            .iter()
            .map(|page| page.slug.as_str())
            .collect::<Vec<_>>();
        assert!(slugs.contains(&"index.md"));
        assert!(slugs.contains(&"log.md"));
    }
}
