// Where: crates/wiki_runtime/src/lib.rs
// What: Service-level orchestration for the wiki store and lexical search.
// Why: Higher layers need one object that coordinates source-of-truth writes and search projection refresh.
use std::path::PathBuf;

use wiki_store::WikiStore;
use wiki_types::{
    CommitPageRevisionInput, CommitPageRevisionOutput, CreatePageInput, LexicalSearchRequest,
    PageBundle, SearchHit,
};

pub struct WikiService {
    store: WikiStore,
}

impl WikiService {
    pub fn new(database_path: PathBuf) -> Self {
        Self {
            store: WikiStore::new(database_path),
        }
    }

    pub fn run_migrations(&self) -> Result<(), String> {
        self.store.run_migrations()
    }

    pub fn create_page(&self, input: CreatePageInput) -> Result<String, String> {
        self.store.create_page(input)
    }

    pub fn commit_page_revision(
        &self,
        input: CommitPageRevisionInput,
    ) -> Result<CommitPageRevisionOutput, String> {
        self.store.commit_page_revision(input)
    }

    pub fn search_lexical(&self, request: LexicalSearchRequest) -> Result<Vec<SearchHit>, String> {
        self.store.search_lexical(request)
    }

    pub fn get_page_by_slug(&self, slug: &str) -> Result<Option<PageBundle>, String> {
        self.store.get_page_by_slug(slug)
    }
}
