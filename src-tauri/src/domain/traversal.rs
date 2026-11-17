//! Platform-neutral traversal vocabulary and rules.
//!
//! The filesystem adapter enumerates entries; this module decides what an entry
//! means and carries the counters. It performs no I/O and imports neither
//! `std::fs` nor any platform crate.

use std::io::ErrorKind;
use std::path::PathBuf;

/// What an entry is, as far as traversal policy cares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
    /// A symlink, junction, mount point, or other reparse point.
    ReparsePoint,
    /// Something that is neither a regular file, a directory, nor a reparse
    /// point (a device, for example).
    Other,
}

/// Why a directory was not descended into.
///
/// A skipped entry is still reported; it is never silently dropped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkipReason {
    /// The directory would exceed the configured depth.
    DepthLimit,
    /// Following it would risk a cycle or escaping the selected tree.
    ReparsePoint,
}

/// The outcome for exactly one observed entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryOutcome {
    Ok,
    Skipped(SkipReason),
    Denied,
    Disappeared,
    Unsupported,
    Failed,
}

/// One observed filesystem entry.
///
/// Paths stay in memory for the lifetime of the stream; the prototype does not
/// persist or log them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileEntry {
    pub path: PathBuf,
    pub depth: u32,
    pub kind: EntryKind,
    pub outcome: EntryOutcome,
    pub size_bytes: Option<u64>,
    pub modified_at_ms: Option<i64>,
}

impl FileEntry {
    pub fn is_directory(&self) -> bool {
        self.kind == EntryKind::Directory
    }
}

/// Traversal budgets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraversalLimits {
    /// How deep directory descent may go. The selected roots are depth 0.
    pub max_depth: u32,
    /// Hard cap on entries produced. `None` means no cap.
    pub max_entries: Option<u64>,
}

impl Default for TraversalLimits {
    fn default() -> Self {
        Self {
            max_depth: 64,
            max_entries: None,
        }
    }
}

impl TraversalLimits {
    /// Clamp values that come from outside the module. A zero entry cap is
    /// treated as "no cap" rather than "scan nothing".
    pub fn normalized(self) -> Self {
        Self {
            max_depth: self.max_depth.min(4096),
            max_entries: self.max_entries.filter(|max| *max > 0),
        }
    }
}

/// Counters for one traversal.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TraversalSummary {
    pub entries: u64,
    pub files: u64,
    pub directories: u64,
    pub reparse_points: u64,
    pub bytes: u64,
    pub skipped: u64,
    pub denied: u64,
    pub disappeared: u64,
    pub unsupported: u64,
    pub failed: u64,
    /// The entry budget stopped the walk early.
    pub truncated: bool,
    /// Cancellation stopped the walk early.
    pub cancelled: bool,
}

impl TraversalSummary {
    /// Count one entry. Byte totals include only entries with `Ok` and a size.
    pub fn record(&mut self, entry: &FileEntry) {
        self.entries = self.entries.saturating_add(1);

        match entry.kind {
            EntryKind::File => self.files = self.files.saturating_add(1),
            EntryKind::Directory => self.directories = self.directories.saturating_add(1),
            EntryKind::ReparsePoint => {
                self.reparse_points = self.reparse_points.saturating_add(1);
            }
            EntryKind::Other => {}
        }

        if entry.outcome == EntryOutcome::Ok {
            self.bytes = self.bytes.saturating_add(entry.size_bytes.unwrap_or(0));
        }

        match entry.outcome {
            EntryOutcome::Ok => {}
            EntryOutcome::Skipped(_) => self.skipped = self.skipped.saturating_add(1),
            EntryOutcome::Denied => self.denied = self.denied.saturating_add(1),
            EntryOutcome::Disappeared => {
                self.disappeared = self.disappeared.saturating_add(1);
            }
            EntryOutcome::Unsupported => self.unsupported = self.unsupported.saturating_add(1),
            EntryOutcome::Failed => self.failed = self.failed.saturating_add(1),
        }
    }

    /// Fold a worker-local summary into a combined one.
    pub fn merge(&mut self, other: Self) {
        self.entries = self.entries.saturating_add(other.entries);
        self.files = self.files.saturating_add(other.files);
        self.directories = self.directories.saturating_add(other.directories);
        self.reparse_points = self.reparse_points.saturating_add(other.reparse_points);
        self.bytes = self.bytes.saturating_add(other.bytes);
        self.skipped = self.skipped.saturating_add(other.skipped);
        self.denied = self.denied.saturating_add(other.denied);
        self.disappeared = self.disappeared.saturating_add(other.disappeared);
        self.unsupported = self.unsupported.saturating_add(other.unsupported);
        self.failed = self.failed.saturating_add(other.failed);
        self.truncated |= other.truncated;
        self.cancelled |= other.cancelled;
    }
}

/// Map an I/O failure to an honest outcome.
pub fn classify_io_error(kind: ErrorKind) -> EntryOutcome {
    match kind {
        ErrorKind::PermissionDenied => EntryOutcome::Denied,
        ErrorKind::NotFound => EntryOutcome::Disappeared,
        ErrorKind::Unsupported | ErrorKind::InvalidInput => EntryOutcome::Unsupported,
        _ => EntryOutcome::Failed,
    }
}

/// Decide what an entry is. Reparse points win over everything else so a
/// junction is never mistaken for a directory to descend into.
pub fn classify_kind(is_dir: bool, is_file: bool, is_reparse: bool) -> EntryKind {
    if is_reparse {
        EntryKind::ReparsePoint
    } else if is_dir {
        EntryKind::Directory
    } else if is_file {
        EntryKind::File
    } else {
        EntryKind::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(kind: EntryKind, outcome: EntryOutcome, size: Option<u64>) -> FileEntry {
        FileEntry {
            path: PathBuf::from(r"C:\example"),
            depth: 0,
            kind,
            outcome,
            size_bytes: size,
            modified_at_ms: None,
        }
    }

    #[test]
    fn summary_counts_each_outcome_and_only_ok_bytes() {
        let mut summary = TraversalSummary::default();

        summary.record(&entry(EntryKind::Directory, EntryOutcome::Ok, None));
        summary.record(&entry(EntryKind::File, EntryOutcome::Ok, Some(10)));
        summary.record(&entry(EntryKind::File, EntryOutcome::Denied, Some(999)));
        summary.record(&entry(
            EntryKind::Directory,
            EntryOutcome::Skipped(SkipReason::DepthLimit),
            None,
        ));
        summary.record(&entry(
            EntryKind::ReparsePoint,
            EntryOutcome::Skipped(SkipReason::ReparsePoint),
            None,
        ));

        assert_eq!(summary.entries, 5);
        assert_eq!(summary.files, 2);
        assert_eq!(summary.directories, 2);
        assert_eq!(summary.reparse_points, 1);
        assert_eq!(summary.bytes, 10, "denied entries must not count bytes");
        assert_eq!(summary.skipped, 2);
        assert_eq!(summary.denied, 1);
    }

    #[test]
    fn guard_paths_map_to_stable_outcomes() {
        assert_eq!(
            classify_io_error(ErrorKind::PermissionDenied),
            EntryOutcome::Denied
        );
        assert_eq!(
            classify_io_error(ErrorKind::NotFound),
            EntryOutcome::Disappeared
        );
        assert_eq!(
            classify_io_error(ErrorKind::Unsupported),
            EntryOutcome::Unsupported
        );
        assert_eq!(classify_io_error(ErrorKind::Other), EntryOutcome::Failed);
    }

    #[test]
    fn reparse_points_are_never_classified_as_directories_or_files() {
        assert_eq!(classify_kind(true, false, true), EntryKind::ReparsePoint);
        assert_eq!(classify_kind(false, true, true), EntryKind::ReparsePoint);
        assert_eq!(classify_kind(true, false, false), EntryKind::Directory);
        assert_eq!(classify_kind(false, true, false), EntryKind::File);
        assert_eq!(classify_kind(false, false, false), EntryKind::Other);
    }

    #[test]
    fn limits_are_clamped_and_a_zero_entry_cap_means_unbounded() {
        let normalized = TraversalLimits {
            max_depth: u32::MAX,
            max_entries: Some(0),
        }
        .normalized();

        assert_eq!(normalized.max_depth, 4096);
        assert_eq!(normalized.max_entries, None);
        assert_eq!(TraversalLimits::default().max_entries, None);
    }

    #[test]
    fn merging_sums_counters_and_keeps_flags() {
        let mut left = TraversalSummary {
            entries: 2,
            bytes: 10,
            ..TraversalSummary::default()
        };
        let right = TraversalSummary {
            entries: 3,
            bytes: 5,
            truncated: true,
            ..TraversalSummary::default()
        };

        left.merge(right);

        assert_eq!(left.entries, 5);
        assert_eq!(left.bytes, 15);
        assert!(left.truncated);
        assert!(!left.cancelled);
    }
}
