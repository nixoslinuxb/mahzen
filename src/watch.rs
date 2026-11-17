//! Lightweight external-change detection.
//!
//! Instead of pulling in `notify`/inotify (and its platform surface), we poll
//! the modification time and size of every note plus the set of files in the
//! vault. A 500 ms poll is cheap (a few stat calls), works on any filesystem
//! (including network mounts where inotify is unreliable), and needs no
//! background thread: the app calls [`Watcher::poll`] once per repaint.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// A file's cheap identity: size + mtime (nanosecond precision when available).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stamp {
    pub len: u64,
    pub modified: Option<SystemTime>,
}

pub fn stamp_of(path: &Path) -> Option<Stamp> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() {
        return None;
    }
    Some(Stamp {
        len: meta.len(),
        modified: meta.modified().ok(),
    })
}

/// What changed between two polls.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Changes {
    /// Notes whose content changed on disk.
    pub modified: Vec<PathBuf>,
    /// Notes that appeared.
    pub added: Vec<PathBuf>,
    /// Notes that disappeared.
    pub removed: Vec<PathBuf>,
}

impl Changes {
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.modified.is_empty() && self.added.is_empty() && self.removed.is_empty()
    }

    /// A short human summary, e.g. `2 changed, 1 new`.
    #[allow(dead_code)]
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if !self.modified.is_empty() {
            parts.push(format!("{} changed", self.modified.len()));
        }
        if !self.added.is_empty() {
            parts.push(format!("{} new", self.added.len()));
        }
        if !self.removed.is_empty() {
            parts.push(format!("{} removed", self.removed.len()));
        }
        parts.join(", ")
    }
}

/// Tracks the last known state of a set of files.
#[derive(Default)]
pub struct Watcher {
    known: BTreeMap<PathBuf, Stamp>,
    /// When the current state was captured (for a "recently changed" window).
    last_poll: Option<SystemTime>,
}

impl Watcher {
    pub fn new() -> Self {
        Self::default()
    }

    /// Forget everything (used when switching vaults).
    pub fn clear(&mut self) {
        self.known.clear();
        self.last_poll = None;
    }

    pub fn is_primed(&self) -> bool {
        self.last_poll.is_some()
    }

    /// Capture the current state without reporting it (first scan).
    pub fn prime(&mut self, paths: &[PathBuf]) {
        self.known.clear();
        for p in paths {
            if let Some(s) = stamp_of(p) {
                self.known.insert(p.clone(), s);
            }
        }
        self.last_poll = Some(SystemTime::now());
    }

    /// Compare the current state of `paths` with the last poll, update the
    /// baseline, and return what changed. The first call primes and reports
    /// nothing.
    pub fn poll(&mut self, paths: &[PathBuf]) -> Changes {
        if !self.is_primed() {
            self.prime(paths);
            return Changes::default();
        }
        let mut current: BTreeMap<PathBuf, Stamp> = BTreeMap::new();
        for p in paths {
            if let Some(s) = stamp_of(p) {
                current.insert(p.clone(), s);
            }
        }
        let mut changes = Changes::default();
        for (path, stamp) in &current {
            match self.known.get(path) {
                None => changes.added.push(path.clone()),
                Some(old) if old != stamp => changes.modified.push(path.clone()),
                Some(_) => {}
            }
        }
        for path in self.known.keys() {
            if !current.contains_key(path) {
                changes.removed.push(path.clone());
            }
        }
        self.known = current;
        self.last_poll = Some(SystemTime::now());
        changes
    }

    /// Record that we just wrote `path` ourselves, so our own save is not
    /// reported back as an external change.
    pub fn note_own_write(&mut self, path: &Path) {
        if let Some(stamp) = stamp_of(path) {
            self.known.insert(path.to_path_buf(), stamp);
        } else {
            self.known.remove(path);
        }
    }
}

/// Whether a stamp is newer than `since` (used to spot files another program
/// touched while the app was closed).
#[allow(dead_code)]
pub fn modified_since(stamp: &Stamp, since: SystemTime) -> bool {
    stamp.modified.map(|m| m > since).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    fn tmp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "encd-watch-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn first_poll_just_primes() {
        let dir = tmp("prime");
        let a = dir.join("a.md");
        fs::write(&a, b"one").unwrap();
        let mut w = Watcher::new();
        let changes = w.poll(std::slice::from_ref(&a));
        assert!(changes.is_empty(), "first poll must not report anything");
        assert!(w.is_primed());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn detects_modified_added_and_removed() {
        let dir = tmp("changes");
        let a = dir.join("a.md");
        let b = dir.join("b.md");
        fs::write(&a, b"one").unwrap();
        let mut w = Watcher::new();
        w.prime(std::slice::from_ref(&a));
        // Modify a, add b.
        std::thread::sleep(std::time::Duration::from_millis(10));
        let mut f = fs::OpenOptions::new().write(true).open(&a).unwrap();
        f.write_all(b"more").unwrap();
        f.flush().unwrap();
        fs::write(&b, b"new").unwrap();
        let ch = w.poll(&[a.clone(), b.clone()]);
        assert_eq!(ch.modified, vec![a.clone()]);
        assert_eq!(ch.added, vec![b.clone()]);
        // Remove b.
        fs::remove_file(&b).unwrap();
        let ch = w.poll(std::slice::from_ref(&a));
        assert_eq!(ch.removed, vec![b]);
    }

    #[test]
    fn own_writes_are_not_reported() {
        let dir = tmp("own");
        let a = dir.join("a.md");
        fs::write(&a, b"one").unwrap();
        let mut w = Watcher::new();
        w.prime(std::slice::from_ref(&a));
        fs::write(&a, b"written by us").unwrap();
        w.note_own_write(&a);
        let ch = w.poll(std::slice::from_ref(&a));
        assert!(ch.is_empty(), "our own save must not look external");
        let _ = fs::remove_dir_all(&dir);
    }
}
