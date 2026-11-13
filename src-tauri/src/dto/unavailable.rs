//! The shared "this field could not be collected" wire shape.
//!
//! Every DTO that reports gaps uses these helpers so field paths and reason codes
//! stay identical across commands.

use serde::Serialize;

use crate::domain::system::Availability;

/// One field path that could not be collected, with a stable reason code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnavailableFieldDto {
    pub field: String,
    pub reason: &'static str,
}

/// Takes a whole group out of an [`Availability`], recording the gap when absent.
pub(crate) fn take_group<'a, T>(
    field: &str,
    group: &'a Availability<T>,
    unavailable: &mut Vec<UnavailableFieldDto>,
) -> Option<&'a T> {
    match group {
        Availability::Available(value) => Some(value),
        Availability::Unavailable(reason) => {
            unavailable.push(UnavailableFieldDto {
                field: field.to_owned(),
                reason: reason.as_str(),
            });
            None
        }
    }
}

/// Takes one leaf field out of an [`Availability`], recording the gap when absent.
pub(crate) fn take<T: Clone>(
    field: &str,
    value: &Availability<T>,
    unavailable: &mut Vec<UnavailableFieldDto>,
) -> Option<T> {
    match value {
        Availability::Available(value) => Some(value.clone()),
        Availability::Unavailable(reason) => {
            unavailable.push(UnavailableFieldDto {
                field: field.to_owned(),
                reason: reason.as_str(),
            });
            None
        }
    }
}
