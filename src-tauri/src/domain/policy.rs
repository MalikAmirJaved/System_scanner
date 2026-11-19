//! Platform-neutral protected-path policy.
//!
//! Threat-model finding T-02 (a junction or crafted path redirects a write to a
//! system location) needs two independent answers: the object opened must be the
//! object recorded ([`super::traversal::FileIdentity`]), and the location must be
//! one the product is allowed to change. This module is the second answer.
//!
//! It is deliberately pure: it takes paths as text, resolves them lexically, and
//! reports a reason. It reads no environment variable, touches no filesystem, and
//! imports no platform crate, so the same rules are testable anywhere. Whatever
//! supplies the real system roots, and whatever turns a name into the object it
//! denotes (canonicalization, 8.3 short names, reparse resolution), belongs in an
//! adapter that calls [`ProtectedPathPolicy::evaluate`].
//!
//! Modeling note: the rules follow Windows path semantics (drive-prefixed,
//! case-insensitive, backslash-separated), because that is the only platform the
//! product targets. A path this module cannot resolve to a drive-rooted sequence
//! is **denied** rather than assumed safe, so an unknown syntax can never be used
//! to reach a protected location by accident.
//!
//! No command consults this policy yet: nothing in the product changes state, and
//! the read-only walker must stay able to scan system directories, which is a
//! legitimate use. The first write path must call it and report a denial.

use std::path::{Component, Path, PathBuf, Prefix};

/// Why a location is protected.
///
/// Stable snake_case codes, for reporting and future audit rows. Never rename a
/// value; add a new one instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtectedReason {
    /// The Windows directory itself (`%SystemRoot%`).
    SystemRoot,
    /// The 32-bit system directory (`%SystemRoot%\System32`).
    System32,
    /// The 32-bit `Program Files` tree.
    ProgramFiles,
    /// The `Program Files (x86)` tree.
    ProgramFilesX86,
    /// Machine-wide Microsoft configuration (`%ProgramData%\Microsoft`).
    ProgramDataMicrosoft,
    /// Per-user Microsoft state (`%LOCALAPPDATA%\Microsoft`).
    AppDataMicrosoft,
    /// This application's own install directory, outside its store.
    AppInstall,
    /// This application's own data directory, outside its store.
    AppData,
    /// The path could not be lexically resolved, so it cannot be proven safe.
    Unresolvable,
}

impl ProtectedReason {
    /// Stable snake_case code. Never rename a value.
    pub fn code(self) -> &'static str {
        match self {
            Self::SystemRoot => "system_root",
            Self::System32 => "system32",
            Self::ProgramFiles => "program_files",
            Self::ProgramFilesX86 => "program_files_x86",
            Self::ProgramDataMicrosoft => "program_data_microsoft",
            Self::AppDataMicrosoft => "app_data_microsoft",
            Self::AppInstall => "app_install",
            Self::AppData => "app_data",
            Self::Unresolvable => "unresolvable",
        }
    }
}

/// The verdict for one path.
///
/// A denial always carries a reason: nothing is refused silently, and nothing is
/// allowed silently either.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathDecision {
    /// No protected root contains this path.
    Allowed,
    /// A protected root contains this path, or it could not be resolved.
    Denied(ProtectedReason),
}

impl PathDecision {
    pub fn is_allowed(self) -> bool {
        matches!(self, Self::Allowed)
    }

    /// The denial reason, when this decision is a denial.
    pub fn reason(self) -> Option<ProtectedReason> {
        match self {
            Self::Denied(reason) => Some(reason),
            Self::Allowed => None,
        }
    }
}

/// One location that must not be modified, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtectedRoot {
    pub path: PathBuf,
    pub reason: ProtectedReason,
}

impl ProtectedRoot {
    pub fn new(path: impl Into<PathBuf>, reason: ProtectedReason) -> Self {
        Self {
            path: path.into(),
            reason,
        }
    }
}

/// A protected root reduced to comparable components.
#[derive(Clone, Debug)]
struct NormalizedRoot {
    components: Vec<String>,
    reason: ProtectedReason,
}

/// Deny-by-default write policy over a set of protected roots.
///
/// "Deny by default" here means a protected location is refused unless the caller
/// is explicitly allowed to touch it; a path outside every root is allowed. The
/// policy is a ceiling on authority, not a grant of it: an allowed decision still
/// needs the operation's own checks.
#[derive(Clone, Debug, Default)]
pub struct ProtectedPathPolicy {
    /// Most specific root first, so a path inside `System32` reports `system32`
    /// rather than the enclosing `system_root`.
    roots: Vec<NormalizedRoot>,
}

impl ProtectedPathPolicy {
    /// Build a policy. Roots that cannot be resolved, or that are not
    /// drive-rooted, are dropped: they cannot match anything, and a root that
    /// silently matched everything would be worse than a missing one.
    pub fn new(roots: impl IntoIterator<Item = ProtectedRoot>) -> Self {
        let mut roots: Vec<NormalizedRoot> = roots
            .into_iter()
            .filter_map(|root| {
                let components = normalize(&root.path)?;
                Some(NormalizedRoot {
                    components,
                    reason: root.reason,
                })
            })
            .collect();

        roots.sort_by_key(|root| std::cmp::Reverse(root.components.len()));
        Self { roots }
    }

    /// The default roots, given the machine-specific directories by the caller.
    ///
    /// Taking these as arguments rather than reading the environment keeps the
    /// module pure and lets tests describe a machine instead of inheriting one.
    /// The caller is responsible for passing the *canonical* location of each
    /// directory; a junction that resolves elsewhere is caught by the identity
    /// check on the opened handle, not here.
    pub fn default_roots(
        windows_dir: impl Into<PathBuf>,
        program_files: impl Into<PathBuf>,
        program_files_x86: impl Into<PathBuf>,
        program_data: impl Into<PathBuf>,
        local_app_data: impl Into<PathBuf>,
        app_install: impl Into<PathBuf>,
        app_data: impl Into<PathBuf>,
    ) -> Self {
        let windows_dir = windows_dir.into();
        let program_data = program_data.into();
        let local_app_data = local_app_data.into();

        Self::new([
            ProtectedRoot::new(windows_dir.join("System32"), ProtectedReason::System32),
            ProtectedRoot::new(
                program_data.join("Microsoft"),
                ProtectedReason::ProgramDataMicrosoft,
            ),
            ProtectedRoot::new(
                local_app_data.join("Microsoft"),
                ProtectedReason::AppDataMicrosoft,
            ),
            ProtectedRoot::new(windows_dir, ProtectedReason::SystemRoot),
            ProtectedRoot::new(program_files, ProtectedReason::ProgramFiles),
            ProtectedRoot::new(program_files_x86, ProtectedReason::ProgramFilesX86),
            ProtectedRoot::new(app_install, ProtectedReason::AppInstall),
            ProtectedRoot::new(app_data, ProtectedReason::AppData),
        ])
    }

    /// Decide whether one path may be modified.
    pub fn evaluate(&self, path: &Path) -> PathDecision {
        let components = match normalize(path) {
            Some(components) => components,
            None => return PathDecision::Denied(ProtectedReason::Unresolvable),
        };

        for root in &self.roots {
            if components.len() >= root.components.len()
                && components[..root.components.len()] == root.components[..]
            {
                return PathDecision::Denied(root.reason);
            }
        }

        PathDecision::Allowed
    }

    /// Convenience for callers that only need a yes/no answer.
    pub fn is_protected(&self, path: &Path) -> bool {
        !self.evaluate(path).is_allowed()
    }
}

/// Reduce a Windows path to upper-case components, resolving `.` and `..`
/// lexically and trimming the trailing dots and spaces that Windows ignores.
///
/// Returns `None` for anything that is not a drive-rooted absolute path: a
/// relative path, a rooted path without a drive, a device or UNC namespace, or a
/// path that climbs above its root.
fn normalize(path: &Path) -> Option<Vec<String>> {
    let mut components: Vec<String> = Vec::new();
    let mut has_root = false;

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => {
                if !components.is_empty() || has_root {
                    return None;
                }
                let letter = match prefix.kind() {
                    Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => letter,
                    // Device (`\\.\`) and network (`\\server\share`) namespaces have
                    // no fixed location; a share can point into a protected tree.
                    _ => return None,
                };
                components.push(format!("{}:", char::from(letter).to_ascii_uppercase()));
            }
            Component::RootDir => {
                if components.len() != 1 || has_root {
                    return None;
                }
                has_root = true;
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if !has_root || components.len() <= 1 {
                    return None;
                }
                components.pop();
            }
            Component::Normal(segment) => {
                if !has_root {
                    return None;
                }
                components.push(normalize_segment(segment.to_str()?)?);
            }
        }
    }

    if !has_root {
        return None;
    }

    Some(components)
}

/// Fold one path segment the way Windows compares names.
fn normalize_segment(segment: &str) -> Option<String> {
    if segment.contains('\0') {
        return None;
    }

    // Windows ignores trailing dots and spaces in a name; `C:\Windows. ` names
    // the Windows directory.
    let trimmed = segment.trim_end_matches(['.', ' ']);
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.to_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A plausible machine, described instead of inherited.
    fn policy() -> ProtectedPathPolicy {
        ProtectedPathPolicy::default_roots(
            r"C:\Windows",
            r"C:\Program Files",
            r"C:\Program Files (x86)",
            r"C:\ProgramData",
            r"C:\Users\me\AppData\Local",
            r"C:\Users\me\AppData\Local\Programs\System Scanner",
            r"C:\Users\me\AppData\Roaming\System Scanner",
        )
    }

    fn reason(path: &str) -> Option<ProtectedReason> {
        policy().evaluate(Path::new(path)).reason()
    }

    #[test]
    fn system_locations_are_denied_with_the_most_specific_reason() {
        assert_eq!(reason(r"C:\Windows"), Some(ProtectedReason::SystemRoot));
        assert_eq!(
            reason(r"C:\Windows\System32\drivers\etc\hosts"),
            Some(ProtectedReason::System32),
            "a System32 path reports the narrower root, not the enclosing one"
        );
        assert_eq!(
            reason(r"C:\Program Files\App\app.exe"),
            Some(ProtectedReason::ProgramFiles)
        );
        assert_eq!(
            reason(r"C:\Program Files (x86)\App\app.exe"),
            Some(ProtectedReason::ProgramFilesX86)
        );
        assert_eq!(
            reason(r"C:\ProgramData\Microsoft\Windows\Start Menu\x.lnk"),
            Some(ProtectedReason::ProgramDataMicrosoft)
        );
        assert_eq!(
            reason(r"C:\Users\me\AppData\Local\Microsoft\Edge\User Data"),
            Some(ProtectedReason::AppDataMicrosoft)
        );
        assert_eq!(
            reason(r"C:\Users\me\AppData\Roaming\System Scanner\scan.db"),
            Some(ProtectedReason::AppData)
        );
        assert_eq!(
            reason(r"C:\Users\me\AppData\Local\Programs\System Scanner\app.exe"),
            Some(ProtectedReason::AppInstall)
        );
    }

    #[test]
    fn ordinary_user_locations_are_allowed() {
        for path in [
            r"C:\",
            r"C:\Users\me\Downloads\file.exe",
            r"D:\Media\movie.mkv",
            r"C:\Users\me\AppData\Local\Temp\file.tmp",
            r"C:\WindowsApps\not-the-windows-dir",
        ] {
            assert_eq!(
                policy().evaluate(Path::new(path)),
                PathDecision::Allowed,
                "{path}"
            );
        }
    }

    #[test]
    fn matching_respects_component_boundaries() {
        // A name that merely starts with a protected name is a different directory.
        assert_eq!(reason(r"C:\Windows10\setup.exe"), None);
        assert_eq!(reason(r"C:\Program Files Extra\app.exe"), None);
        assert_eq!(reason(r"C:\ProgramData\MicrosoftX\app.exe"), None);
        // But a real child is covered, however deep.
        assert_eq!(
            reason(r"C:\Windows\Temp\a\b\c.dll"),
            Some(ProtectedReason::SystemRoot)
        );
    }

    #[test]
    fn comparison_is_case_insensitive_and_ignores_separators_and_trailing_dots() {
        for path in [
            r"c:\windows\system32\cmd.exe",
            r"C:/Windows/System32/cmd.exe",
            r"C:\WINDOWS\\System32\cmd.exe",
            r"C:\Windows\System32.",
            "C:\\Windows. \\System32 \\cmd.exe",
        ] {
            assert!(
                policy().is_protected(Path::new(path)),
                "{path} must still name a protected location"
            );
        }

        assert_eq!(
            reason(r"\\?\C:\Windows\System32\cmd.exe"),
            Some(ProtectedReason::System32),
            "a verbatim drive path names the same location"
        );
    }

    #[test]
    fn dot_segments_are_resolved_before_the_decision() {
        assert_eq!(
            reason(r"C:\Users\me\AppData\..\..\..\Windows\System32\cmd.exe"),
            Some(ProtectedReason::System32)
        );
        assert_eq!(
            reason(r"C:\Windows\Temp\..\notepad.exe"),
            Some(ProtectedReason::SystemRoot)
        );
        assert_eq!(reason(r"C:\Users\me\Downloads\..\Downloads\file.exe"), None);
    }

    #[test]
    fn paths_that_cannot_be_resolved_are_denied_not_guessed() {
        for path in [
            r"..\Windows\System32",
            r"Windows\System32",
            r"\Windows\System32",
            r"C:Windows",
            r"C:\..\Windows",
            r"\\?\GLOBALROOT\Device\HarddiskVolume2\Windows",
            r"\\.\PhysicalDrive0",
            r"\\server\share\Windows",
            r"C:\Windows\System32\..\..\..\..\..\..\Windows",
        ] {
            assert_eq!(
                policy().evaluate(Path::new(path)),
                PathDecision::Denied(ProtectedReason::Unresolvable),
                "{path}"
            );
        }

        // Escaping above the drive root is unresolvable, not "outside the root".
        assert_eq!(reason(r"C:\.."), Some(ProtectedReason::Unresolvable));
    }

    #[test]
    fn the_root_itself_is_protected_but_nothing_beyond_it_is_implied() {
        assert_eq!(reason(r"C:\Windows\"), Some(ProtectedReason::SystemRoot));
        assert_eq!(
            reason(r"C:\Program Files"),
            Some(ProtectedReason::ProgramFiles)
        );
        assert_eq!(
            reason(r"C:\Windows\System32\drivers"),
            Some(ProtectedReason::System32)
        );
        // A sibling of a protected root is not covered by it.
        assert_eq!(reason(r"C:\Windows.old\file.dll"), None);
    }

    #[test]
    fn unusable_roots_are_dropped_instead_of_matching_everything() {
        let policy = ProtectedPathPolicy::new([
            ProtectedRoot::new(r"Windows", ProtectedReason::SystemRoot),
            ProtectedRoot::new(r"\\server\share", ProtectedReason::ProgramFiles),
            ProtectedRoot::new(r"D:\Keep\Safe", ProtectedReason::AppData),
        ]);

        assert_eq!(
            policy
                .evaluate(Path::new(r"D:\Keep\Safe\file.txt"))
                .reason(),
            Some(ProtectedReason::AppData)
        );
        assert_eq!(
            policy.evaluate(Path::new(r"C:\Users\me\file.txt")),
            PathDecision::Allowed
        );
    }

    #[test]
    fn an_empty_policy_protects_nothing() {
        let policy = ProtectedPathPolicy::default();
        assert_eq!(
            policy.evaluate(Path::new(r"C:\Windows\System32\cmd.exe")),
            PathDecision::Allowed
        );
        assert!(!policy.is_protected(Path::new(r"C:\Windows")));
    }

    #[test]
    fn every_denial_carries_a_stable_distinct_reason() {
        let reasons = [
            ProtectedReason::SystemRoot,
            ProtectedReason::System32,
            ProtectedReason::ProgramFiles,
            ProtectedReason::ProgramFilesX86,
            ProtectedReason::ProgramDataMicrosoft,
            ProtectedReason::AppDataMicrosoft,
            ProtectedReason::AppInstall,
            ProtectedReason::AppData,
            ProtectedReason::Unresolvable,
        ];
        let codes: Vec<&str> = reasons.iter().map(|reason| reason.code()).collect();
        assert_eq!(
            codes,
            [
                "system_root",
                "system32",
                "program_files",
                "program_files_x86",
                "program_data_microsoft",
                "app_data_microsoft",
                "app_install",
                "app_data",
                "unresolvable"
            ]
        );

        let mut sorted = codes.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), codes.len(), "codes must be distinct");
        assert_eq!(PathDecision::Allowed.reason(), None);
        assert!(PathDecision::Allowed.is_allowed());
        assert!(!PathDecision::Denied(ProtectedReason::AppData).is_allowed());
    }
}
