//! Platform independent output and resource ownership for the supported controls.
pub mod fake;
#[cfg(windows)]
pub mod vigem;

use std::fmt;

/// Backend independent Xbox report. Buttons use the documented XInput bit positions.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct XboxReport {
    pub buttons: u16,
    pub left_x: i16,
    pub left_y: i16,
    pub right_x: i16,
    pub right_y: i16,
    pub left_trigger: u8,
    pub right_trigger: u8,
}
impl XboxReport {
    pub const NEUTRAL: Self = Self {
        buttons: 0,
        left_x: 0,
        left_y: 0,
        right_x: 0,
        right_y: 0,
        left_trigger: 0,
        right_trigger: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendHealth {
    Detached,
    Ready,
    Fault,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendError {
    pub operation: &'static str,
    pub detail: String,
    pub native_code: Option<u32>,
    pub attempt: u8,
    pub cleanup_failed: bool,
}

/// Fixed size, secret free facts suitable for the latest status channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendDiagnostic {
    pub operation: &'static str,
    pub native_code: Option<u32>,
    pub attempt: u8,
    pub cleanup_failed: bool,
}
impl BackendDiagnostic {
    pub fn message(self) -> String {
        let code = self
            .native_code
            .map(|n| format!("Windows error {n}"))
            .unwrap_or_else(|| "no native error code".into());
        let cause = if self.operation == "wait ready" && self.native_code == Some(483) {
            "The virtual controller did not finish starting. A driver boot timeout is possible; the cause is not confirmed."
        } else {
            "The virtual controller could not complete this operation. See the local log for details."
        };
        format!(
            "Controller {} failed ({code}, target attempt {}). {cause} {}In the tray, Stop listening, wait for cleanup, then Start. In the terminal, exit normally and relaunch. Pair with a fresh code. If it repeats, share the local log and Device Manager driver details.",
            self.operation,
            self.attempt,
            if self.cleanup_failed {
                "Cleanup also failed; controller removal is unconfirmed. "
            } else {
                ""
            }
        )
    }
}
impl BackendError {
    pub fn diagnostic(&self) -> BackendDiagnostic {
        BackendDiagnostic {
            operation: self.operation,
            native_code: self.native_code,
            attempt: self.attempt,
            cleanup_failed: self.cleanup_failed,
        }
    }
}
impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Controller backend {} failed (target attempt {}): {}",
            self.operation, self.attempt, self.detail
        )
    }
}
impl std::error::Error for BackendError {}

/// Synchronous calls belong to the output owner, never the UDP receive loop.
/// Failed attach may retain a partial target; detach must remain callable.
pub trait VirtualGamepadBackend {
    fn attach(&mut self) -> Result<(), BackendError>;
    /// The predicate can stop subsequent calls, never cancel an in-flight call.
    /// False means startup was cancelled and the owner must clean up.
    fn attach_checked(&mut self, current: &dyn Fn() -> bool) -> Result<bool, BackendError> {
        if !current() {
            return Ok(false);
        }
        self.attach()?;
        Ok(current())
    }
    fn submit(&mut self, report: XboxReport) -> Result<(), BackendError>;
    fn neutral(&mut self) -> Result<(), BackendError>;
    fn detach(&mut self) -> Result<(), BackendError>;
    fn health(&self) -> BackendHealth;
}

/// One owner attempts both neutral and detach on every ordinary exit.
/// Cleanup errors are retained rather than hidden by the original failure.
pub struct Gamepad<B: VirtualGamepadBackend> {
    backend: B,
    active: bool,
    cleanup_needed: bool,
    faulted: bool,
    cleanup_errors: Vec<BackendError>,
}
impl<B: VirtualGamepadBackend> Gamepad<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            active: false,
            cleanup_needed: false,
            faulted: false,
            cleanup_errors: Vec::new(),
        }
    }

    pub fn attach(&mut self) -> Result<(), BackendError> {
        self.attach_checked(&|| true).map(|_| ())
    }

    pub fn attach_checked(&mut self, current: &dyn Fn() -> bool) -> Result<bool, BackendError> {
        if self.active {
            return Ok(true);
        }
        if self.faulted {
            return Err(BackendError {
                operation: "attach",
                detail: "Restart the backend after a fault.".into(),
                native_code: None,
                attempt: 0,
                cleanup_failed: false,
            });
        }
        self.cleanup_needed = true;
        let result = self.backend.attach_checked(current).and_then(|started| {
            if !started || !current() {
                return Ok(false);
            }
            self.backend.neutral()?;
            Ok(current())
        });
        match result {
            Ok(true) => {
                self.active = true;
                Ok(true)
            }
            Ok(false) => {
                self.shutdown();
                Ok(false)
            }
            Err(error) => {
                self.faulted = true;
                self.shutdown();
                Err(error)
            }
        }
    }

    pub fn submit(&mut self, report: XboxReport) -> Result<(), BackendError> {
        if !self.active {
            return Err(BackendError {
                operation: "submit",
                detail: "No ready controller. Reconnect first.".into(),
                native_code: None,
                attempt: 0,
                cleanup_failed: false,
            });
        }
        if let Err(error) = self.backend.submit(report) {
            self.faulted = true;
            self.shutdown();
            return Err(error);
        }
        Ok(())
    }

    pub fn health(&self) -> BackendHealth {
        if self.faulted {
            BackendHealth::Fault
        } else {
            self.backend.health()
        }
    }

    pub fn shutdown(&mut self) {
        if !self.cleanup_needed {
            return;
        }
        self.active = false;
        self.cleanup_needed = false;
        if let Err(error) = self.backend.neutral() {
            tracing::error!(%error, "Controller cleanup neutral failed");
            self.cleanup_errors.push(error);
        }
        // Never short circuit detach when neutral fails.
        if let Err(error) = self.backend.detach() {
            tracing::error!(%error, "Controller cleanup detach failed");
            self.cleanup_errors.push(error);
        }
        if !self.cleanup_errors.is_empty() {
            self.faulted = true;
        }
    }

    pub fn cleanup_errors(&self) -> &[BackendError] {
        &self.cleanup_errors
    }
}
impl<B: VirtualGamepadBackend> Drop for Gamepad<B> {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fake::{Call::*, RecordingBackend};

    #[test]
    fn normal_order_shutdown_is_idempotent_and_relaunch_starts_neutral() {
        let mut pad = Gamepad::new(RecordingBackend::default());
        pad.attach().unwrap();
        pad.attach().unwrap();
        pad.submit(XboxReport {
            buttons: 0x1000,
            ..XboxReport::NEUTRAL
        })
        .unwrap();
        pad.submit(XboxReport::NEUTRAL).unwrap();
        pad.shutdown();
        pad.shutdown();
        assert_eq!(pad.health(), BackendHealth::Detached);
        assert_eq!(
            pad.backend.calls,
            [
                Attach,
                Neutral,
                Submit(XboxReport {
                    buttons: 0x1000,
                    ..XboxReport::NEUTRAL
                }),
                Submit(XboxReport::NEUTRAL),
                Neutral,
                Detach
            ]
        );
        assert!(
            pad.submit(XboxReport {
                buttons: 0x1000,
                ..XboxReport::NEUTRAL
            })
            .is_err()
        );
        pad.attach().unwrap();
        assert_eq!(&pad.backend.calls[6..], &[Attach, Neutral]);
    }

    #[test]
    fn update_failure_neutralizes_detaches_and_refuses_future_input() {
        let pressed = XboxReport {
            buttons: 0x1000,
            ..XboxReport::NEUTRAL
        };
        let mut backend = RecordingBackend::default();
        backend.failures.push(Submit(pressed));
        let mut pad = Gamepad::new(backend);
        pad.attach().unwrap();
        assert!(pad.submit(pressed).is_err());
        assert_eq!(pad.health(), BackendHealth::Fault);
        assert_eq!(
            pad.backend.calls,
            [Attach, Neutral, Submit(pressed), Neutral, Detach]
        );
        assert!(pad.submit(pressed).is_err());
        assert!(pad.attach().is_err());
        pad.shutdown();
        assert_eq!(pad.backend.calls.len(), 5);
    }

    #[test]
    fn partial_attach_and_neutral_failures_still_attempt_detach() {
        for failures in [vec![Attach], vec![Neutral], vec![Neutral, Detach]] {
            let mut backend = RecordingBackend::default();
            backend.failures = failures;
            let mut pad = Gamepad::new(backend);
            assert!(pad.attach().is_err());
            assert_eq!(pad.health(), BackendHealth::Fault);
            assert_eq!(pad.backend.calls.last(), Some(&Detach));
            let count = pad.backend.calls.len();
            pad.shutdown();
            assert_eq!(pad.backend.calls.len(), count);
            assert_eq!(
                pad.cleanup_errors().len(),
                pad.backend
                    .failures
                    .iter()
                    .filter(|call| **call != Attach)
                    .count()
            );
        }
    }

    #[test]
    fn shutdown_after_held_a_keeps_both_cleanup_errors_and_never_resubmits() {
        let mut pad = Gamepad::new(RecordingBackend::default());
        pad.attach().unwrap();
        pad.submit(XboxReport {
            buttons: 0x1000,
            ..XboxReport::NEUTRAL
        })
        .unwrap();
        pad.backend.failures = vec![Neutral, Detach];
        pad.shutdown();
        assert_eq!(pad.health(), BackendHealth::Fault);
        assert_eq!(pad.cleanup_errors().len(), 2);
        assert_eq!(&pad.backend.calls[3..], &[Neutral, Detach]);
        assert!(
            pad.submit(XboxReport {
                buttons: 0x1000,
                ..XboxReport::NEUTRAL
            })
            .is_err()
        );
        assert!(pad.attach().is_err());
        pad.shutdown();
        assert_eq!(pad.backend.calls.len(), 5);
    }

    #[test]
    fn drop_runs_cleanup_after_early_return() {
        use std::{cell::RefCell, rc::Rc};
        struct Shared(Rc<RefCell<RecordingBackend>>);
        impl VirtualGamepadBackend for Shared {
            fn attach(&mut self) -> Result<(), BackendError> {
                self.0.borrow_mut().attach()
            }
            fn submit(&mut self, r: XboxReport) -> Result<(), BackendError> {
                self.0.borrow_mut().submit(r)
            }
            fn neutral(&mut self) -> Result<(), BackendError> {
                self.0.borrow_mut().neutral()
            }
            fn detach(&mut self) -> Result<(), BackendError> {
                self.0.borrow_mut().detach()
            }
            fn health(&self) -> BackendHealth {
                self.0.borrow().health()
            }
        }
        let recording = Rc::new(RefCell::new(RecordingBackend::default()));
        {
            let mut pad = Gamepad::new(Shared(recording.clone()));
            pad.attach().unwrap();
            pad.submit(XboxReport {
                buttons: 0x1000,
                ..XboxReport::NEUTRAL
            })
            .unwrap();
        }
        assert_eq!(
            recording.borrow().calls,
            [
                Attach,
                Neutral,
                Submit(XboxReport {
                    buttons: 0x1000,
                    ..XboxReport::NEUTRAL
                }),
                Neutral,
                Detach
            ]
        );
    }
}
