//! Handle-based directory access.
//!
//! A path is not an identity. The previous walker checked a directory's
//! attributes by path and then opened it by path again, so a directory swapped
//! for a junction in between was followed (threat-model finding F-01, KI-055).
//! This adapter binds the two steps to one object:
//!
//! - open once with `FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS`,
//!   so a link is opened, never traversed;
//! - read the volume serial and 128-bit file id from that same handle;
//! - enumerate children through that handle, which also yields each child's
//!   file id, so the parent can record an identity for every directory it queues.
//!
//! The caller compares the recorded identity with the one it reads after
//! opening; a mismatch is reported, never scanned. Where a volume cannot supply
//! an id the check is unverifiable, and that is not treated as a match.
//!
//! Memory stays bounded: enumeration is read in fixed-size batches and only the
//! current batch is held. Names are untrusted data and are never logged here.

use std::ffi::OsString;
use std::io;
use std::path::Path;

use crate::domain::traversal::FileIdentity;

/// Metadata of the directory that was opened, read from the same handle
/// enumeration uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirectoryInfo {
    pub is_reparse_point: bool,
    pub size_bytes: u64,
    pub modified_at_ms: Option<i64>,
}

/// One child as reported by the directory handle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectoryEntry {
    pub name: OsString,
    pub is_dir: bool,
    pub is_file: bool,
    pub is_reparse_point: bool,
    pub size_bytes: u64,
    pub modified_at_ms: Option<i64>,
    /// `None` when the volume does not supply an id; callers must not treat
    /// that as a verified identity.
    pub identity: Option<FileIdentity>,
}

/// Upper bound on entries one [`DirectoryHandle::read_batch`] call may return.
///
/// On Windows a batch is already bounded by the fixed enumeration buffer; the
/// std fallback applies this cap so a huge directory is never materialised as
/// one list.
#[cfg_attr(windows, allow(dead_code))]
pub const MAX_BATCH: usize = 512;

#[cfg(windows)]
mod imp {
    use super::{DirectoryEntry, DirectoryInfo, FileIdentity, OsString, Path, io};
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_NO_MORE_FILES, FILETIME, HANDLE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, CreateFileW, FILE_ATTRIBUTE_DIRECTORY,
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        FILE_ID_EXTD_DIR_INFO, FILE_ID_INFO, FILE_LIST_DIRECTORY, FILE_READ_ATTRIBUTES,
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FileIdExtdDirectoryInfo,
        FileIdExtdDirectoryRestartInfo, FileIdInfo, GetFileInformationByHandle,
        GetFileInformationByHandleEx, OPEN_EXISTING,
    };

    /// 16 KiB of enumeration buffer, kept 8-byte aligned for the entry struct.
    const BUFFER_WORDS: usize = 2048;
    /// `FILETIME` ticks between 1601-01-01 and 1970-01-01, in 100 ns units.
    const EPOCH_DIFFERENCE_100NS: i64 = 116_444_736_000_000_000;

    /// One directory bound to a single open handle.
    pub struct DirectoryHandle {
        handle: HANDLE,
        /// Volume serial of the opened directory, used to label its children.
        volume_serial: Option<u64>,
        /// Set once the enumeration has been restarted, so later calls continue.
        started: bool,
    }

    impl DirectoryHandle {
        /// Open a directory without following a reparse point in its final
        /// component.
        pub fn open(path: &Path) -> io::Result<Self> {
            let wide = wide_path(path);
            // SAFETY: `wide` is NUL-terminated and lives for the call; the
            // remaining arguments are plain flags and null, as CreateFileW
            // requires for a directory open.
            let handle = unsafe {
                CreateFileW(
                    wide.as_ptr(),
                    FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES,
                    FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                    std::ptr::null(),
                    OPEN_EXISTING,
                    FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
                    std::ptr::null_mut(),
                )
            };
            if handle == INVALID_HANDLE_VALUE {
                return Err(io::Error::last_os_error());
            }

            let mut opened = Self {
                handle,
                volume_serial: None,
                started: false,
            };
            opened.volume_serial = opened.read_identity().map(|id| id.volume_serial);
            Ok(opened)
        }

        /// Attributes and timestamps of the opened directory itself.
        pub fn info(&self) -> io::Result<DirectoryInfo> {
            let mut raw = BY_HANDLE_FILE_INFORMATION::default();
            // SAFETY: `raw` is a valid, writable BY_HANDLE_FILE_INFORMATION and
            // `self.handle` was returned by a successful CreateFileW call.
            let ok = unsafe { GetFileInformationByHandle(self.handle, &mut raw) };
            if ok == 0 {
                return Err(io::Error::last_os_error());
            }

            Ok(DirectoryInfo {
                is_reparse_point: raw.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0,
                size_bytes: u64::from(raw.nFileSizeHigh) << 32 | u64::from(raw.nFileSizeLow),
                modified_at_ms: filetime_to_unix_ms(raw.ftLastWriteTime),
            })
        }

        /// Identity of the opened directory, if the volume supplies one.
        pub fn identity(&self) -> Option<FileIdentity> {
            self.read_identity()
        }

        fn read_identity(&self) -> Option<FileIdentity> {
            let mut raw = FILE_ID_INFO::default();
            // SAFETY: `raw` is a valid, writable FILE_ID_INFO and the buffer
            // size matches its size; `self.handle` is an open directory handle.
            let ok = unsafe {
                GetFileInformationByHandleEx(
                    self.handle,
                    FileIdInfo,
                    std::ptr::from_mut(&mut raw).cast(),
                    std::mem::size_of::<FILE_ID_INFO>() as u32,
                )
            };
            if ok == 0 {
                return None;
            }
            Some(FileIdentity {
                volume_serial: raw.VolumeSerialNumber,
                file_id: raw.FileId.Identifier,
            })
        }

        /// Read the next batch of children through the handle.
        ///
        /// The first call restarts the enumeration; later calls continue where
        /// the previous one stopped. `.` and `..` are omitted.
        pub fn read_batch(&mut self) -> io::Result<Vec<DirectoryEntry>> {
            let mut buffer = vec![0u64; BUFFER_WORDS];
            let class = if self.started {
                FileIdExtdDirectoryInfo
            } else {
                FileIdExtdDirectoryRestartInfo
            };
            // SAFETY: the buffer is 8-byte aligned and `BUFFER_WORDS * 8` bytes
            // long; the system writes at most that many bytes and reports how
            // much it used through the entry offsets.
            let ok = unsafe {
                GetFileInformationByHandleEx(
                    self.handle,
                    class,
                    buffer.as_mut_ptr().cast(),
                    (BUFFER_WORDS * std::mem::size_of::<u64>()) as u32,
                )
            };
            if ok == 0 {
                let error = io::Error::last_os_error();
                if error.raw_os_error() == Some(ERROR_NO_MORE_FILES as i32) {
                    self.started = true;
                    return Ok(Vec::new());
                }
                return Err(error);
            }
            self.started = true;

            let bytes = buffer.as_ptr().cast::<u8>();
            let total = BUFFER_WORDS * std::mem::size_of::<u64>();
            let mut offset = 0usize;
            let mut entries = Vec::new();
            while offset < total {
                // SAFETY: the entry starts inside the buffer. The system writes
                // complete records and links them by `NextEntryOffset`; every
                // read below stays within the buffer bounds checked here.
                let info = unsafe { &*(bytes.add(offset).cast::<FILE_ID_EXTD_DIR_INFO>()) };

                let name_length = info.FileNameLength as usize / 2;
                let name_ptr = std::ptr::addr_of!(info.FileName).cast::<u16>();
                if name_length > 0 && offset + name_length * 2 <= total {
                    // SAFETY: the name is `name_length` UTF-16 units stored in
                    // the same record and bounds-checked above.
                    let name = unsafe { std::slice::from_raw_parts(name_ptr, name_length) };
                    let is_dot = (name.len() == 1 && name[0] == b'.' as u16)
                        || (name.len() == 2 && name[0] == b'.' as u16 && name[1] == b'.' as u16);
                    if !is_dot {
                        entries.push(self.entry(info, name));
                    }
                }

                let step = info.NextEntryOffset as usize;
                if step == 0 || offset + step >= total {
                    break;
                }
                offset += step;
            }

            Ok(entries)
        }

        fn entry(&self, info: &FILE_ID_EXTD_DIR_INFO, name: &[u16]) -> DirectoryEntry {
            let is_dir = info.FileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0;
            let is_reparse_point = info.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0;
            DirectoryEntry {
                name: OsString::from_wide(name),
                is_dir,
                // A reparse point is reported as such, never as a directory to
                // descend into or a file to read.
                is_file: !is_dir && !is_reparse_point,
                is_reparse_point,
                size_bytes: u64::try_from(info.EndOfFile).unwrap_or(0),
                modified_at_ms: ticks_to_unix_ms(info.LastWriteTime),
                identity: self.volume_serial.map(|volume_serial| FileIdentity {
                    volume_serial,
                    file_id: info.FileId.Identifier,
                }),
            }
        }
    }

    impl Drop for DirectoryHandle {
        fn drop(&mut self) {
            // SAFETY: the handle came from a successful CreateFileW call and is
            // closed exactly once here.
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }

    /// Encode a path for CreateFileW, adding the `\\?\` prefix so paths beyond
    /// `MAX_PATH` work and the system does not normalise away the rest.
    fn wide_path(path: &Path) -> Vec<u16> {
        let wide = path.as_os_str().encode_wide().collect::<Vec<u16>>();
        let long_prefix = [b'\\' as u16, b'\\' as u16, b'?' as u16, b'\\' as u16];
        let unc_prefix = [b'\\' as u16, b'\\' as u16];

        let mut prefixed = Vec::with_capacity(wide.len() + 8);
        if wide.starts_with(&long_prefix) {
            prefixed.extend_from_slice(&wide);
        } else if wide.starts_with(&unc_prefix) {
            prefixed.extend_from_slice(&long_prefix);
            prefixed.extend_from_slice(&[b'U' as u16, b'N' as u16, b'C' as u16]);
            prefixed.push(b'\\' as u16);
            prefixed.extend_from_slice(&wide[2..]);
        } else if wide.len() >= 2 && wide[1] == b':' as u16 {
            prefixed.extend_from_slice(&long_prefix);
            prefixed.extend_from_slice(&wide);
        } else {
            // Relative paths are rejected before this point; keep them intact so
            // the call fails honestly instead of reading the wrong directory.
            prefixed.extend_from_slice(&wide);
        }

        prefixed.push(0);
        prefixed
    }

    fn filetime_to_unix_ms(time: FILETIME) -> Option<i64> {
        let ticks = (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime);
        let difference = u64::try_from(EPOCH_DIFFERENCE_100NS).ok()?;
        i64::try_from(ticks.checked_sub(difference)? / 10_000).ok()
    }

    fn ticks_to_unix_ms(ticks: i64) -> Option<i64> {
        let unix = ticks.checked_sub(EPOCH_DIFFERENCE_100NS)?;
        if unix <= 0 { None } else { Some(unix / 10_000) }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::{DirectoryEntry, DirectoryInfo, FileIdentity, MAX_BATCH, OsString, Path, io};
    use std::fs;
    use std::path::PathBuf;

    /// One directory bound to an open directory stream.
    ///
    /// Identity verification is not implemented off Windows: the recorded id is
    /// `None`, so callers see an unverifiable rather than a match.
    pub struct DirectoryHandle {
        path: PathBuf,
        reader: Option<fs::ReadDir>,
        info: DirectoryInfo,
    }

    impl DirectoryHandle {
        pub fn open(path: &Path) -> io::Result<Self> {
            let metadata = fs::symlink_metadata(path)?;
            let info = DirectoryInfo {
                is_reparse_point: metadata.file_type().is_symlink(),
                size_bytes: metadata.len(),
                modified_at_ms: metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                    .and_then(|elapsed| i64::try_from(elapsed.as_millis()).ok()),
            };
            Ok(Self {
                path: path.to_path_buf(),
                reader: Some(fs::read_dir(path)?),
                info,
            })
        }

        pub fn info(&self) -> io::Result<DirectoryInfo> {
            Ok(self.info)
        }

        pub fn identity(&self) -> Option<FileIdentity> {
            None
        }

        pub fn read_batch(&mut self) -> io::Result<Vec<DirectoryEntry>> {
            let reader = match self.reader.as_mut() {
                Some(reader) => reader,
                None => return Ok(Vec::new()),
            };

            let mut entries = Vec::new();
            while entries.len() < MAX_BATCH {
                let Some(item) = reader.next() else {
                    return Ok(entries);
                };
                let item = item?;
                let metadata = match fs::symlink_metadata(item.path()) {
                    Ok(metadata) => metadata,
                    Err(_) => continue,
                };
                let file_type = metadata.file_type();
                let is_reparse_point = file_type.is_symlink();
                entries.push(DirectoryEntry {
                    name: item.file_name(),
                    is_dir: file_type.is_dir(),
                    is_file: file_type.is_file(),
                    is_reparse_point,
                    size_bytes: if file_type.is_file() {
                        metadata.len()
                    } else {
                        0
                    },
                    modified_at_ms: metadata
                        .modified()
                        .ok()
                        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                        .and_then(|elapsed| i64::try_from(elapsed.as_millis()).ok()),
                    identity: None,
                });
            }
            Ok(entries)
        }
    }
}

pub use imp::DirectoryHandle;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir(std::path::PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or(0);
            let path = std::env::temp_dir().join(format!(
                "system-scanner-dir-{label}-{}-{nanos}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("temp dir must be creatable");
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn read_all(handle: &mut DirectoryHandle) -> Vec<DirectoryEntry> {
        let mut all = Vec::new();
        loop {
            let batch = handle.read_batch().expect("enumeration must succeed");
            if batch.is_empty() {
                return all;
            }
            all.extend(batch);
        }
    }

    #[test]
    fn a_directory_reports_its_children_and_skips_dot_entries() {
        let temp = TempDir::new("children");
        fs::write(temp.0.join("alpha.txt"), b"a").expect("fixture file");
        fs::create_dir(temp.0.join("sub")).expect("fixture dir");

        let mut handle = DirectoryHandle::open(&temp.0).expect("directory must open");
        let mut names: Vec<String> = read_all(&mut handle)
            .into_iter()
            .map(|entry| entry.name.to_string_lossy().into_owned())
            .collect();
        names.sort();

        assert_eq!(names, ["alpha.txt", "sub"]);
    }

    #[test]
    fn enumeration_batches_cover_a_directory_larger_than_one_buffer() {
        let temp = TempDir::new("batches");
        for index in 0..1200 {
            fs::write(temp.0.join(format!("file-{index:04}.bin")), b"x").expect("fixture file");
        }

        let mut handle = DirectoryHandle::open(&temp.0).expect("directory must open");
        let first = handle.read_batch().expect("first batch");
        assert!(!first.is_empty(), "the first batch must not be empty");

        let mut total = first.len();
        let mut batches = 0;
        loop {
            let batch = handle.read_batch().expect("continuation batch");
            batches += 1;
            if batch.is_empty() {
                break;
            }
            assert!(batch.len() <= MAX_BATCH, "a batch must stay bounded");
            total += batch.len();
        }

        assert_eq!(total, 1200, "every entry must be reported exactly once");
        assert!(batches > 1, "1200 entries must span more than one buffer");
    }

    #[test]
    fn a_child_carries_size_attributes_and_identity() {
        let temp = TempDir::new("identity-child");
        fs::write(temp.0.join("sized.bin"), vec![0u8; 11]).expect("fixture file");

        let mut handle = DirectoryHandle::open(&temp.0).expect("directory must open");
        let entry = read_all(&mut handle)
            .into_iter()
            .find(|entry| entry.name == "sized.bin")
            .expect("child must be reported");

        assert!(!entry.is_dir);
        assert!(entry.is_file);
        assert!(!entry.is_reparse_point);
        assert_eq!(entry.size_bytes, 11);
        assert!(entry.modified_at_ms.unwrap_or_default() > 0);
        #[cfg(windows)]
        assert!(
            entry.identity.is_some(),
            "a child id lets the parent verify it when it is opened later"
        );
    }

    #[cfg(windows)]
    #[test]
    fn the_same_directory_reports_the_same_identity_and_another_does_not() {
        let temp = TempDir::new("identity-stable");
        let other = temp.0.join("other");
        fs::create_dir(&other).expect("fixture dir");

        let first = DirectoryHandle::open(&temp.0).expect("directory must open");
        let again = DirectoryHandle::open(&temp.0).expect("directory must reopen");
        let different = DirectoryHandle::open(&other).expect("directory must open");

        let identity = first.identity().expect("NTFS must supply an identity");
        assert_eq!(first.identity(), again.identity());
        assert_ne!(identity, different.identity().unwrap_or(identity));

        let volume = identity.volume_serial;
        assert!(volume > 0, "a volume serial must be reported");
    }

    #[cfg(windows)]
    #[test]
    fn a_junction_is_opened_but_reported_as_a_reparse_point() {
        let temp = TempDir::new("junction-info");
        let target = temp.0.join("target");
        let link = temp.0.join("link");
        fs::create_dir(&target).expect("fixture dir");
        let output = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(&target)
            .output()
            .expect("cmd must be available");
        assert!(
            output.status.success(),
            "junction fixture must be created: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let opened = DirectoryHandle::open(&link).expect("a junction opens as its own object");
        let info = opened.info().expect("attributes must be readable");
        assert!(
            info.is_reparse_point,
            "opening must not hide the reparse attribute"
        );

        fs::remove_dir(&link).expect("junction must be removable");
    }

    #[test]
    fn a_missing_directory_is_an_error_rather_than_an_empty_listing() {
        let temp = TempDir::new("missing");
        let missing = temp.0.join("gone");
        assert!(DirectoryHandle::open(&missing).is_err());
    }
}
