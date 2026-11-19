use serde::Serialize;

use crate::domain::process as domain;
use crate::dto::unavailable::{UnavailableFieldDto, take};

/// Bump when a breaking field change is made to this DTO.
///
/// v2 added `depth` (required) and `imagePath` (optional, paired with `unavailable`).
pub const PROCESS_LIST_SCHEMA_VERSION: u32 = 2;

/// Wire shape of `list_processes` data (see `API.md` §3.3).
///
/// A missing leaf field in a row is always accompanied by an entry in that row's
/// own `unavailable` list; `null` alone never means "could not read".
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessListDto {
    pub captured_at: u64,
    pub schema_version: u32,
    /// Processes enumerated, before paging. A lower bound when `truncated`.
    pub total: usize,
    pub offset: u32,
    pub limit: u32,
    /// The adapter hit its hard cap; the list is a prefix of the machine.
    pub truncated: bool,
    pub items: Vec<ProcessDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessDto {
    /// Stable across refreshes unless the start time was unreadable.
    pub key: String,
    pub pid: u32,
    pub parent_pid: u32,
    /// Ancestors above this process in this snapshot; `0` for a root, a process
    /// whose parent is gone, or a chain that cycles.
    pub depth: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Full image path. Present only when readable; otherwise named in
    /// `unavailable`. Stays on this machine: never logged, never in a bundle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_set_bytes: Option<u64>,
    pub unavailable: Vec<UnavailableFieldDto>,
}

impl ProcessListDto {
    pub fn from_page(page: &domain::ProcessPage, captured_at: u64) -> Self {
        let mut items = Vec::with_capacity(page.items.len());

        for (index, row) in page.items.iter().enumerate() {
            items.push(process_from(index, row));
        }

        Self {
            captured_at,
            schema_version: PROCESS_LIST_SCHEMA_VERSION,
            total: page.total,
            offset: page.offset,
            limit: page.limit,
            truncated: page.truncated,
            items,
        }
    }
}

fn process_from(index: usize, row: &domain::ProcessRow) -> ProcessDto {
    let process = &row.info;
    let prefix = format!("items[{index}]");
    let mut unavailable = Vec::new();

    let name = take(&format!("{prefix}.name"), &process.name, &mut unavailable);
    let image_path = take(
        &format!("{prefix}.imagePath"),
        &process.image_path,
        &mut unavailable,
    );
    let thread_count = take(
        &format!("{prefix}.threadCount"),
        &process.thread_count,
        &mut unavailable,
    );
    let start_time = take(
        &format!("{prefix}.startTime"),
        &process.started_at_ms,
        &mut unavailable,
    );
    let working_set_bytes = take(
        &format!("{prefix}.workingSetBytes"),
        &process.working_set_bytes,
        &mut unavailable,
    );

    ProcessDto {
        key: process.key(),
        pid: process.pid,
        parent_pid: process.parent_pid,
        depth: row.depth,
        name,
        image_path,
        thread_count,
        start_time,
        working_set_bytes,
        unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::process::{ProcessInfo, ProcessPage, ProcessRow};
    use crate::domain::system::{Availability, UnavailableReason};

    fn process(pid: u32) -> ProcessInfo {
        ProcessInfo {
            pid,
            parent_pid: 1,
            name: Availability::Available("example.exe".to_owned()),
            image_path: Availability::Available("C:\\bin\\example.exe".to_owned()),
            thread_count: Availability::Available(4),
            started_at_ms: Availability::Available(1_700_000_000_000),
            working_set_bytes: Availability::Available(1_048_576),
        }
    }

    fn page(items: Vec<ProcessInfo>) -> ProcessPage {
        ProcessPage {
            total: 312,
            offset: 0,
            limit: 100,
            truncated: false,
            items: items
                .into_iter()
                .map(|info| ProcessRow { info, depth: 0 })
                .collect(),
        }
    }

    #[test]
    fn a_full_page_serializes_camel_case_with_schema_version() {
        let dto = ProcessListDto::from_page(&page(vec![process(4)]), 1_762_387_200_000);
        let value = serde_json::to_value(&dto).unwrap();

        assert_eq!(value["capturedAt"], 1_762_387_200_000u64);
        assert_eq!(value["schemaVersion"], PROCESS_LIST_SCHEMA_VERSION);
        assert_eq!(value["total"], 312);
        assert_eq!(value["offset"], 0);
        assert_eq!(value["limit"], 100);
        assert_eq!(value["truncated"], false);

        let item = &value["items"][0];
        assert_eq!(item["pid"], 4);
        assert_eq!(item["parentPid"], 1);
        assert_eq!(item["depth"], 0);
        assert_eq!(item["name"], "example.exe");
        assert_eq!(item["imagePath"], "C:\\bin\\example.exe");
        assert_eq!(item["threadCount"], 4);
        assert_eq!(item["startTime"], 1_700_000_000_000u64);
        assert_eq!(item["workingSetBytes"], 1_048_576u64);
        assert_eq!(item["key"], "4-1700000000000");
        assert_eq!(item["unavailable"], serde_json::json!([]));
        assert!(item.get("parent_pid").is_none());
    }

    #[test]
    fn a_denied_field_is_omitted_and_named_by_row_path() {
        let mut denied = process(4);
        denied.started_at_ms = Availability::Unavailable(UnavailableReason::AccessDenied);
        denied.working_set_bytes = Availability::Unavailable(UnavailableReason::AccessDenied);
        denied.image_path = Availability::Unavailable(UnavailableReason::AccessDenied);

        let dto = ProcessListDto::from_page(&page(vec![denied]), 0);
        let value = serde_json::to_value(&dto).unwrap();

        let item = &value["items"][0];
        assert_eq!(item["name"], "example.exe");
        assert!(item.get("imagePath").is_none());
        assert!(item.get("startTime").is_none());
        assert!(item.get("workingSetBytes").is_none());
        // Without a start time the key is explicit about being snapshot-local.
        assert_eq!(item["key"], "pid:4");
        assert_eq!(
            item["unavailable"],
            serde_json::json!([
                { "field": "items[0].imagePath", "reason": "access_denied" },
                { "field": "items[0].startTime", "reason": "access_denied" },
                { "field": "items[0].workingSetBytes", "reason": "access_denied" }
            ])
        );
    }

    #[test]
    fn a_truncated_inventory_is_visible_on_the_wire() {
        let mut truncated = page(vec![process(4)]);
        truncated.truncated = true;

        let value = serde_json::to_value(ProcessListDto::from_page(&truncated, 0)).unwrap();

        assert_eq!(value["truncated"], true);
    }
}
