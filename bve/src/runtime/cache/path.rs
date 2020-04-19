use async_std::{path::PathBuf, sync::RwLock};
use indexmap::IndexSet;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct PathHandle(usize);

pub struct PathSet {
    inner: RwLock<IndexSet<PathBuf>>,
}

impl PathSet {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(IndexSet::new()),
        }
    }

    pub async fn insert(&self, path: PathBuf) -> PathHandle {
        debug_assert_eq!(
            Some(path.clone()),
            path.canonicalize().await.ok(),
            "Path must be canonical"
        );
        PathHandle(self.inner.write().await.insert_full(path).0)
    }

    pub async fn get(&self, path: PathHandle) -> PathBuf {
        self.inner
            .read()
            .await
            .get_index(path.0)
            .expect("Invalid path handle")
            .clone()
    }
}