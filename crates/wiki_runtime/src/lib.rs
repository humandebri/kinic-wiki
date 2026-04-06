// Where: crates/wiki_runtime/src/lib.rs
// What: Service-level orchestration for the wiki store and lexical search.
// Why: Higher layers need one object that coordinates source-of-truth writes and search projection refresh.
use std::path::PathBuf;

use wiki_search::WikiSearch;
use wiki_store::WikiStore;
use wiki_types::{
    CommitPageRevisionInput, CommitPageRevisionOutput, CreatePageInput, LexicalSearchRequest,
    PageBundle, SearchHit,
};

pub struct WikiService {
    store: WikiStore,
    search: WikiSearch,
}

impl WikiService {
    pub fn new(database_path: PathBuf, search_path: PathBuf) -> Self {
        Self {
            store: WikiStore::new(database_path),
            search: WikiSearch::new(search_path),
        }
    }

    pub fn run_migrations(&self) -> Result<(), String> {
        self.store.run_migrations()?;
        self.search.run_migrations()?;
        Ok(())
    }

    pub fn create_page(&self, input: CreatePageInput) -> Result<String, String> {
        self.store.create_page(input)
    }

    pub fn commit_page_revision(
        &self,
        input: CommitPageRevisionInput,
    ) -> Result<CommitPageRevisionOutput, String> {
        self.store.commit_page_revision(&self.search, input)
    }

    pub fn search_lexical(&self, request: LexicalSearchRequest) -> Result<Vec<SearchHit>, String> {
        self.search.lexical_search(request)
    }

    pub fn get_page_by_slug(&self, slug: &str) -> Result<Option<PageBundle>, String> {
        self.store.get_page_by_slug(slug)
    }
}
