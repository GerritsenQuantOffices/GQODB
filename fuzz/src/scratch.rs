//! Per-process scratch files for the path-based readers (`Reader::open`, `inspect`).
use std::{
    path::PathBuf,
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};

static DIR: OnceLock<PathBuf> = OnceLock::new();
static NEXT: AtomicU64 = AtomicU64::new(0);

/// Directory private to this process. `/dev/shm` when present (keeps the run off the disk).
pub fn dir() -> &'static PathBuf {
    DIR.get_or_init(|| {
        let base = if std::path::Path::new("/dev/shm").is_dir() {
            PathBuf::from("/dev/shm")
        } else {
            std::env::temp_dir()
        };
        let dir = base.join(format!("gqodb-fuzz-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    })
}

/// A fresh, unused path inside the scratch directory.
pub fn fresh(name: &str) -> PathBuf {
    dir().join(format!("{}-{name}", NEXT.fetch_add(1, Ordering::Relaxed)))
}

/// Write `bytes` to a fixed per-process file and return its path.
pub fn write(name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir().join(name);
    std::fs::write(&path, bytes).expect("scratch write");
    path
}
