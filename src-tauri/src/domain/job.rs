//! Job lifecycle rules. Pure: no I/O, no Tauri, no platform crates.
//!
//! The state machine mirrors `docs/SYSTEM_DESIGN.md` §3. Transitions are
//! one-way and a terminal state is never overwritten.

/// Where a job is in its life.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobState {
    Queued,
    Running,
    /// Cancellation was requested; the work is stopping.
    Cancelling,
    Completed,
    Failed,
    Cancelled,
}

/// A transition the state machine does not allow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidTransition {
    pub from: JobState,
    pub to: JobState,
}

impl JobState {
    /// Stable snake_case code for IPC and logs. Never rename a value.
    pub fn code(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Cancelling => "cancelling",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    /// Whether a new job must be refused because this one still holds the slot.
    pub fn is_active(self) -> bool {
        !self.is_terminal()
    }

    /// Whether `next` is a legal successor.
    ///
    /// `Cancelling -> Completed` is legal on purpose: if the work finished before
    /// it observed the cancel, reporting `cancelled` would understate the result.
    pub fn can_become(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Queued, Self::Running | Self::Failed | Self::Cancelled)
                | (
                    Self::Running,
                    Self::Cancelling | Self::Completed | Self::Failed | Self::Cancelled
                )
                | (
                    Self::Cancelling,
                    Self::Completed | Self::Failed | Self::Cancelled
                )
        )
    }

    pub fn advance(self, next: Self) -> Result<Self, InvalidTransition> {
        if self.can_become(next) {
            Ok(next)
        } else {
            Err(InvalidTransition {
                from: self,
                to: next,
            })
        }
    }
}

/// Rate limiter for progress events: at most one per `min_interval_ms`.
///
/// Time is passed in, so the rule is testable without sleeping.
#[derive(Clone, Copy, Debug)]
pub struct ProgressGate {
    min_interval_ms: u64,
    last_ms: Option<u64>,
}

impl ProgressGate {
    pub fn new(min_interval_ms: u64) -> Self {
        Self {
            min_interval_ms,
            last_ms: None,
        }
    }

    /// Returns `true` (and records the time) when an event may be sent now.
    /// The first call is always due. A clock that moves backwards is treated as
    /// "not due" rather than panicking.
    pub fn due(&mut self, now_ms: u64) -> bool {
        let due = match self.last_ms {
            None => true,
            Some(last) => now_ms.saturating_sub(last) >= self.min_interval_ms && now_ms >= last,
        };
        if due {
            self.last_ms = Some(now_ms);
        }
        due
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [JobState; 6] = [
        JobState::Queued,
        JobState::Running,
        JobState::Cancelling,
        JobState::Completed,
        JobState::Failed,
        JobState::Cancelled,
    ];

    #[test]
    fn terminal_states_never_change() {
        for from in ALL.into_iter().filter(|state| state.is_terminal()) {
            for to in ALL {
                assert!(!from.can_become(to), "{from:?} -> {to:?} must be refused");
            }
        }
    }

    #[test]
    fn the_happy_and_cancel_paths_are_allowed() {
        assert!(JobState::Queued.can_become(JobState::Running));
        assert!(JobState::Running.can_become(JobState::Completed));
        assert!(JobState::Running.can_become(JobState::Cancelling));
        assert!(JobState::Cancelling.can_become(JobState::Cancelled));
        assert!(JobState::Cancelling.can_become(JobState::Completed));
        assert!(JobState::Cancelling.can_become(JobState::Failed));
    }

    #[test]
    fn cancelling_is_only_reachable_from_running() {
        for from in ALL {
            let allowed = from.can_become(JobState::Cancelling);
            assert_eq!(allowed, from == JobState::Running, "{from:?}");
        }
    }

    #[test]
    fn states_cannot_move_backwards() {
        assert!(!JobState::Running.can_become(JobState::Queued));
        assert!(!JobState::Cancelling.can_become(JobState::Running));
        assert!(!JobState::Running.can_become(JobState::Running));
    }

    #[test]
    fn advance_reports_the_refused_pair() {
        assert_eq!(
            JobState::Completed.advance(JobState::Running),
            Err(InvalidTransition {
                from: JobState::Completed,
                to: JobState::Running
            })
        );
        assert_eq!(
            JobState::Queued.advance(JobState::Running),
            Ok(JobState::Running)
        );
    }

    #[test]
    fn only_non_terminal_jobs_hold_the_slot() {
        assert!(JobState::Queued.is_active());
        assert!(JobState::Running.is_active());
        assert!(JobState::Cancelling.is_active());
        assert!(!JobState::Completed.is_active());
        assert!(!JobState::Failed.is_active());
        assert!(!JobState::Cancelled.is_active());
    }

    #[test]
    fn state_codes_are_stable() {
        let codes: Vec<&str> = ALL.iter().map(|state| state.code()).collect();
        assert_eq!(
            codes,
            [
                "queued",
                "running",
                "cancelling",
                "completed",
                "failed",
                "cancelled"
            ]
        );
    }

    #[test]
    fn progress_is_limited_to_one_event_per_interval() {
        let mut gate = ProgressGate::new(100);
        assert!(gate.due(0), "the first event is always due");
        assert!(!gate.due(50));
        assert!(!gate.due(99));
        assert!(gate.due(100));
        assert!(!gate.due(150));
        assert!(gate.due(250));

        let sent = (0..1_000).filter(|ms| gate.due(1_000 + ms)).count();
        assert_eq!(sent, 10, "1 s of entries yields at most 10 events");
    }

    #[test]
    fn a_zero_interval_never_suppresses_and_a_backwards_clock_is_safe() {
        let mut open = ProgressGate::new(0);
        assert!(open.due(5));
        assert!(open.due(5));

        let mut gate = ProgressGate::new(10);
        assert!(gate.due(100));
        assert!(!gate.due(50), "time going backwards is not due");
    }
}
