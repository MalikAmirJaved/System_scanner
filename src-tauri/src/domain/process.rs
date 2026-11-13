//! Read-only process inventory and the paging policy applied to it.
//!
//! Platform-neutral: this module never imports `windows-sys` or Tauri. Win32
//! enumeration arrives through [`crate::ports::processes::ProcessInfoPort`].

use crate::domain::system::Availability;
use crate::ports::ProbeError;
use crate::ports::processes::ProcessInfoPort;

/// Default page size when the caller does not ask for one.
pub const DEFAULT_PAGE_SIZE: u32 = 100;
/// Hard ceiling. Larger requests are rejected, not clamped silently.
pub const MAX_PAGE_SIZE: u32 = 500;
/// Highest accepted `offset`. Stops integer games from turning into deep scans.
pub const MAX_OFFSET: u32 = 1_000_000;

/// One process, as observed during a single enumeration.
///
/// Every field except the identifiers is fallible: Windows denies access to
/// system-owned processes, and that is reported, never guessed at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessInfo {
    pub pid: u32,
    pub parent_pid: u32,
    pub name: Availability<String>,
    pub thread_count: Availability<u32>,
    pub started_at_ms: Availability<u64>,
    pub working_set_bytes: Availability<u64>,
}

impl ProcessInfo {
    /// Identity that survives a refresh.
    ///
    /// Windows reuses process IDs, so PID alone is not an identity. When the
    /// start time is readable the key is `"<pid>-<startedAtMs>"`; when it is not,
    /// the key is `"pid:<pid>"`, which is unique only within this snapshot and
    /// says so.
    pub fn key(&self) -> String {
        match self.started_at_ms {
            Availability::Available(started) => format!("{}-{}", self.pid, started),
            Availability::Unavailable(_) => format!("pid:{}", self.pid),
        }
    }
}

/// Everything the adapter could enumerate, plus whether its cap stopped it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessInventory {
    pub processes: Vec<ProcessInfo>,
    /// The adapter hit its hard cap, so `processes` is a prefix of the machine's
    /// actual process list.
    pub truncated: bool,
}

/// A validated request for one page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRequest {
    pub offset: u32,
    pub limit: u32,
}

/// Why a page request was rejected. Messages are static, so nothing the caller
/// sent is echoed back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageRequestError {
    LimitTooSmall,
    LimitTooLarge,
    OffsetTooLarge,
}

impl PageRequestError {
    pub const fn message(self) -> &'static str {
        match self {
            Self::LimitTooSmall => "limit must be at least 1",
            Self::LimitTooLarge => "limit must be at most 500",
            Self::OffsetTooLarge => "offset must be at most 1000000",
        }
    }
}

impl PageRequest {
    /// Validates untrusted IPC input. Defaults are applied here, not in the UI.
    pub fn new(offset: Option<u32>, limit: Option<u32>) -> Result<Self, PageRequestError> {
        let limit = limit.unwrap_or(DEFAULT_PAGE_SIZE);
        if limit == 0 {
            return Err(PageRequestError::LimitTooSmall);
        }
        if limit > MAX_PAGE_SIZE {
            return Err(PageRequestError::LimitTooLarge);
        }

        let offset = offset.unwrap_or(0);
        if offset > MAX_OFFSET {
            return Err(PageRequestError::OffsetTooLarge);
        }

        Ok(Self { offset, limit })
    }
}

/// One page of the inventory, ordered deterministically by process ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessPage {
    /// Total processes enumerated, before paging. A lower bound when truncated.
    pub total: usize,
    pub offset: u32,
    pub limit: u32,
    pub truncated: bool,
    pub items: Vec<ProcessInfo>,
}

/// Enumerates through the port and returns the requested page.
///
/// Paging happens after sorting, so the same inventory always yields the same
/// page boundaries. A failure to enumerate is an error; it is never reported as
/// an empty list.
pub fn collect_page(
    port: &dyn ProcessInfoPort,
    request: PageRequest,
) -> Result<ProcessPage, ProbeError> {
    let inventory = port.inventory()?;
    Ok(build_page(inventory, request))
}

fn build_page(mut inventory: ProcessInventory, request: PageRequest) -> ProcessPage {
    // Sort by PID, then by start time to stay deterministic if an adapter ever
    // reports the same PID twice.
    inventory.processes.sort_by(|left, right| {
        left.pid
            .cmp(&right.pid)
            .then_with(|| started_at(left).cmp(&started_at(right)))
    });

    let total = inventory.processes.len();
    let skip = usize::try_from(request.offset).unwrap_or(usize::MAX);
    let take = usize::try_from(request.limit).unwrap_or(0);
    let items = inventory
        .processes
        .into_iter()
        .skip(skip)
        .take(take)
        .collect();

    ProcessPage {
        total,
        offset: request.offset,
        limit: request.limit,
        truncated: inventory.truncated,
        items,
    }
}

fn started_at(process: &ProcessInfo) -> Option<u64> {
    match process.started_at_ms {
        Availability::Available(value) => Some(value),
        Availability::Unavailable(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::system::UnavailableReason;

    fn process(pid: u32, started_at_ms: Option<u64>) -> ProcessInfo {
        ProcessInfo {
            pid,
            parent_pid: 0,
            name: Availability::Available(format!("process-{pid}")),
            thread_count: Availability::Available(2),
            started_at_ms: match started_at_ms {
                Some(value) => Availability::Available(value),
                None => Availability::Unavailable(UnavailableReason::AccessDenied),
            },
            working_set_bytes: Availability::Available(1_048_576),
        }
    }

    struct FakePort {
        result: Result<ProcessInventory, ProbeError>,
    }

    impl ProcessInfoPort for FakePort {
        fn inventory(&self) -> Result<ProcessInventory, ProbeError> {
            self.result.clone()
        }
    }

    fn port_with(processes: Vec<ProcessInfo>) -> FakePort {
        FakePort {
            result: Ok(ProcessInventory {
                processes,
                truncated: false,
            }),
        }
    }

    #[test]
    fn pages_are_sliced_after_a_deterministic_sort() {
        let port = port_with(vec![
            process(30, None),
            process(10, None),
            process(20, None),
        ]);

        let page = collect_page(&port, PageRequest::new(Some(0), Some(2)).unwrap()).unwrap();

        assert_eq!(page.total, 3);
        assert_eq!(page.items.len(), 2);
        assert_eq!(
            page.items.iter().map(|item| item.pid).collect::<Vec<_>>(),
            vec![10, 20]
        );

        let second = collect_page(&port, PageRequest::new(Some(2), Some(2)).unwrap()).unwrap();
        assert_eq!(
            second.items.iter().map(|item| item.pid).collect::<Vec<_>>(),
            vec![30]
        );
    }

    #[test]
    fn an_offset_past_the_end_yields_an_empty_page_not_an_error() {
        let port = port_with(vec![process(10, None)]);

        let page = collect_page(&port, PageRequest::new(Some(50), Some(10)).unwrap()).unwrap();

        assert_eq!(page.total, 1);
        assert!(page.items.is_empty());
    }

    #[test]
    fn page_requests_are_validated_in_the_domain() {
        assert_eq!(
            PageRequest::new(None, None).unwrap().limit,
            DEFAULT_PAGE_SIZE
        );
        assert_eq!(
            PageRequest::new(None, Some(MAX_PAGE_SIZE)).unwrap().limit,
            MAX_PAGE_SIZE
        );
        assert_eq!(
            PageRequest::new(None, Some(0)),
            Err(PageRequestError::LimitTooSmall)
        );
        assert_eq!(
            PageRequest::new(None, Some(MAX_PAGE_SIZE + 1)),
            Err(PageRequestError::LimitTooLarge)
        );
        assert_eq!(
            PageRequest::new(Some(MAX_OFFSET + 1), None),
            Err(PageRequestError::OffsetTooLarge)
        );
        assert!(!PageRequestError::LimitTooLarge.message().is_empty());
    }

    #[test]
    fn a_denied_field_stays_with_its_row() {
        let mut denied = process(10, None);
        denied.working_set_bytes = Availability::Unavailable(UnavailableReason::AccessDenied);
        let port = port_with(vec![denied]);

        let page = collect_page(&port, PageRequest::new(None, None).unwrap()).unwrap();

        assert_eq!(page.items.len(), 1, "a denied field must not drop the row");
        assert_eq!(
            page.items[0].working_set_bytes,
            Availability::Unavailable(UnavailableReason::AccessDenied)
        );
        assert!(matches!(
            page.items[0].name,
            Availability::Available(ref name) if name == "process-10"
        ));
    }

    #[test]
    fn a_truncated_inventory_is_reported_and_keeps_its_flag() {
        let port = FakePort {
            result: Ok(ProcessInventory {
                processes: vec![process(10, None)],
                truncated: true,
            }),
        };

        let page = collect_page(&port, PageRequest::new(None, None).unwrap()).unwrap();

        assert!(page.truncated);
        assert_eq!(page.total, 1);
    }

    #[test]
    fn an_enumeration_failure_is_an_error_not_an_empty_page() {
        let port = FakePort {
            result: Err(ProbeError::AccessDenied),
        };

        let result = collect_page(&port, PageRequest::new(None, None).unwrap());

        assert_eq!(result, Err(ProbeError::AccessDenied));
    }

    #[test]
    fn keys_use_the_start_time_only_when_it_is_known() {
        assert_eq!(
            process(42, Some(1_700_000_000_000)).key(),
            "42-1700000000000"
        );
        assert_eq!(process(42, None).key(), "pid:42");
    }
}
