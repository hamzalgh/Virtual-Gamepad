//! The only ViGEm boundary. No application unsafe code or notification threads.
//! The pinned client performs synchronous driver calls, including wait_ready.
//! Its driver calls cannot be cancelled by a Tokio timeout. Forced termination
//! may skip cleanup; the viability gate must observe that behavior separately.
use super::{BackendError, BackendHealth, VirtualGamepadBackend, XboxReport};
use vigem_client::{Client, Error, TargetId, XButtons, XGamepad, Xbox360Wired};

pub struct VigemBackend {
    target: Option<Xbox360Wired<Client>>,
    health: BackendHealth,
    attempt: u8,
}
impl Default for VigemBackend {
    fn default() -> Self {
        Self {
            target: None,
            health: BackendHealth::Detached,
            attempt: 0,
        }
    }
}

fn report(value: XboxReport) -> XGamepad {
    XGamepad {
        buttons: XButtons { raw: value.buttons },
        // XInput uses positive-up Y, matching the protocol. No second inversion.
        thumb_lx: value.left_x,
        thumb_ly: value.left_y,
        thumb_rx: value.right_x,
        thumb_ry: value.right_y,
        left_trigger: value.left_trigger,
        right_trigger: value.right_trigger,
    }
}

// This seam exercises the real startup algorithm without invoking the driver.
trait StartupTarget {
    fn plugin(&mut self) -> Result<(), Error>;
    fn wait_ready(&mut self) -> Result<(), Error>;
    fn neutral(&mut self) -> Result<(), Error>;
    fn unplug(&mut self) -> Result<(), Error>;
}
impl StartupTarget for Xbox360Wired<Client> {
    fn plugin(&mut self) -> Result<(), Error> {
        self.plugin()
    }
    fn wait_ready(&mut self) -> Result<(), Error> {
        self.wait_ready()
    }
    fn neutral(&mut self) -> Result<(), Error> {
        self.update(&XGamepad::default())
    }
    fn unplug(&mut self) -> Result<(), Error> {
        self.unplug()
    }
}
fn failure(operation: &'static str, error: Error, attempt: u8) -> BackendError {
    BackendError {
        operation,
        detail: error.to_string(),
        native_code: match error {
            Error::WinError(code) => Some(code),
            _ => None,
        },
        attempt,
        cleanup_failed: false,
    }
}

fn with_first(mut error: BackendError, first: &Option<BackendError>) -> BackendError {
    if let Some(first) = first {
        error.detail.push_str(&format!("; first attempt: {first}"));
    }
    error
}

fn startup<T: StartupTarget>(
    target: &mut Option<T>,
    attempt: &mut u8,
    mut connect: impl FnMut() -> Result<T, Error>,
    current: &dyn Fn() -> bool,
) -> Result<bool, BackendError> {
    let mut first_failure = None;
    for number in 1..=2 {
        if !current() {
            return Ok(false);
        }
        *attempt = number;
        // Own before plugin, including partial driver mutations.
        *target =
            Some(connect().map_err(|e| with_first(failure("connect", e, number), &first_failure))?);
        if !current() {
            return Ok(false);
        }
        let owned = target.as_mut().expect("owned before plugin");
        owned
            .plugin()
            .map_err(|e| with_first(failure("attach", e, number), &first_failure))?;
        if !current() {
            return Ok(false);
        }
        match owned.wait_ready() {
            Ok(()) => {
                if number == 2 {
                    tracing::warn!(
                        "Fresh controller target passed readiness (attempt 2); neutral activation still required"
                    );
                }
                return Ok(current());
            }
            Err(error) => {
                let retry = number == 1 && matches!(error, Error::WinError(483));
                let mut error = with_first(failure("wait ready", error, number), &first_failure);
                if !retry {
                    return Err(error);
                }
                // Never wait again on the cleared boot event, or infer readiness
                // from an unchanged neutral update (which can be a driver no-op).
                tracing::warn!(%error, "Controller startup failed; cleaning target before one fresh attempt");
                let neutral = owned.neutral();
                let unplug = owned.unplug(); // always attempt even if neutral failed
                if let Err(ref cleanup) = neutral {
                    error
                        .detail
                        .push_str(&format!("; recovery neutral failed: {cleanup}"));
                }
                if let Err(ref cleanup) = unplug {
                    error
                        .detail
                        .push_str(&format!("; recovery unplug failed: {cleanup}"));
                }
                if neutral.is_err() || unplug.is_err() {
                    error.cleanup_failed = true;
                    return Err(error);
                }
                // Explicit unplug succeeded. Drop client/event before connecting again.
                *target = None;
                first_failure = Some(error);
            }
        }
    }
    unreachable!("second attempt returns")
}

impl VigemBackend {
    fn error(&mut self, operation: &'static str, error: Error) -> BackendError {
        self.health = BackendHealth::Fault;
        failure(operation, error, self.attempt)
    }
}
impl VirtualGamepadBackend for VigemBackend {
    fn attach(&mut self) -> Result<(), BackendError> {
        self.attach_checked(&|| true).map(|_| ())
    }
    fn attach_checked(&mut self, current: &dyn Fn() -> bool) -> Result<bool, BackendError> {
        if self.target.is_some() {
            return Err(self.error("attach", Error::AlreadyConnected));
        }
        let result = startup(
            &mut self.target,
            &mut self.attempt,
            || Client::connect().map(|client| Xbox360Wired::new(client, TargetId::XBOX360_WIRED)),
            current,
        );
        self.health = match result {
            Ok(true) => BackendHealth::Ready,
            Ok(false) => BackendHealth::Detached,
            Err(_) => BackendHealth::Fault,
        };
        result
    }
    fn submit(&mut self, value: XboxReport) -> Result<(), BackendError> {
        let Some(target) = self.target.as_mut() else {
            return Err(self.error("submit", Error::NotPluggedIn));
        };
        target
            .update(&report(value))
            .map_err(|error| self.error("update", error))
    }
    fn neutral(&mut self) -> Result<(), BackendError> {
        if self
            .target
            .as_ref()
            .is_none_or(|target| !target.is_attached())
        {
            return Ok(());
        }
        self.target
            .as_mut()
            .expect("attached target checked")
            .update(&XGamepad::default())
            .map_err(|error| self.error("neutral", error))
    }
    fn detach(&mut self) -> Result<(), BackendError> {
        let Some(target) = self.target.as_mut() else {
            return Ok(());
        };
        if target.is_attached() {
            // Keep the target on failure so Drop can make a final cleanup attempt.
            target
                .unplug()
                .map_err(|error| self.error("detach", error))?;
        }
        self.target = None;
        self.health = BackendHealth::Detached;
        Ok(())
    }
    fn health(&self) -> BackendHealth {
        self.health
    }
}
impl Drop for VigemBackend {
    fn drop(&mut self) {
        let _ = self.neutral();
        let _ = self.detach();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, collections::VecDeque, rc::Rc};

    #[derive(Default)]
    struct Plan {
        connect: Option<u32>,
        plugin: Option<u32>,
        wait: Option<u32>,
        neutral: Option<u32>,
        unplug: Option<u32>,
    }
    struct ScriptTarget {
        id: usize,
        plan: Plan,
        events: Rc<RefCell<Vec<(usize, &'static str)>>>,
    }
    impl ScriptTarget {
        fn call(&self, name: &'static str, code: Option<u32>) -> Result<(), Error> {
            self.events.borrow_mut().push((self.id, name));
            code.map_or(Ok(()), |code| Err(Error::WinError(code)))
        }
    }
    impl StartupTarget for ScriptTarget {
        fn plugin(&mut self) -> Result<(), Error> {
            self.call("plugin", self.plan.plugin)
        }
        fn wait_ready(&mut self) -> Result<(), Error> {
            self.call("wait", self.plan.wait)
        }
        fn neutral(&mut self) -> Result<(), Error> {
            self.call("neutral", self.plan.neutral)
        }
        fn unplug(&mut self) -> Result<(), Error> {
            self.call("unplug", self.plan.unplug)
        }
    }
    impl Drop for ScriptTarget {
        fn drop(&mut self) {
            self.events.borrow_mut().push((self.id, "drop"));
        }
    }
    struct Script {
        plans: VecDeque<Plan>,
        events: Rc<RefCell<Vec<(usize, &'static str)>>>,
        target: Option<ScriptTarget>,
        attempt: u8,
    }
    impl Script {
        fn new(plans: Vec<Plan>) -> Self {
            Self {
                plans: plans.into(),
                events: Rc::default(),
                target: None,
                attempt: 0,
            }
        }
        fn run(&mut self, current: &dyn Fn() -> bool) -> Result<bool, BackendError> {
            let mut next = 0;
            startup(
                &mut self.target,
                &mut self.attempt,
                || {
                    next += 1;
                    self.events.borrow_mut().push((next, "connect"));
                    let plan = self.plans.pop_front().expect("unexpected extra target");
                    if let Some(code) = plan.connect {
                        return Err(Error::WinError(code));
                    }
                    Ok(ScriptTarget {
                        id: next,
                        plan,
                        events: self.events.clone(),
                    })
                },
                current,
            )
        }
    }
    fn timeout() -> Plan {
        Plan {
            wait: Some(483),
            ..Default::default()
        }
    }

    #[test]
    fn startup_483_recreates_only_after_neutral_unplug_and_drop() {
        let mut script = Script::new(vec![timeout(), Plan::default()]);
        assert!(script.run(&|| true).unwrap());
        assert_eq!(
            *script.events.borrow(),
            vec![
                (1, "connect"),
                (1, "plugin"),
                (1, "wait"),
                (1, "neutral"),
                (1, "unplug"),
                (1, "drop"),
                (2, "connect"),
                (2, "plugin"),
                (2, "wait")
            ]
        );
        assert_eq!(script.attempt, 2);
        assert_eq!(script.target.as_ref().unwrap().id, 2);
    }
    #[test]
    fn startup_first_success_and_other_errors_never_retry() {
        let mut script = Script::new(vec![Plan::default()]);
        assert!(script.run(&|| true).unwrap());
        assert_eq!(script.events.borrow().len(), 3);
        for plan in [
            Plan {
                connect: Some(5),
                ..Default::default()
            },
            Plan {
                plugin: Some(483),
                ..Default::default()
            },
            Plan {
                wait: Some(5),
                ..Default::default()
            },
        ] {
            let mut script = Script::new(vec![plan]);
            let error = script.run(&|| true).unwrap_err();
            assert_eq!(error.attempt, 1);
            assert!(script.events.borrow().iter().all(|(id, _)| *id == 1));
        }
    }
    #[test]
    fn startup_second_failure_preserves_first_and_never_tries_a_third_target() {
        for plan in [
            timeout(),
            Plan {
                connect: Some(5),
                ..Default::default()
            },
            Plan {
                plugin: Some(5),
                ..Default::default()
            },
        ] {
            let mut script = Script::new(vec![timeout(), plan]);
            let error = script.run(&|| true).unwrap_err();
            assert_eq!(error.attempt, 2);
            assert!(error.detail.contains("first attempt:"));
            assert!(error.detail.contains("483"));
            assert!(script.events.borrow().iter().all(|(id, _)| *id <= 2));
        }
    }
    #[test]
    fn startup_cleanup_failure_prevents_replacement_and_retains_owner() {
        for (neutral, unplug) in [(Some(5), None), (None, Some(6)), (Some(5), Some(6))] {
            let mut script = Script::new(vec![Plan {
                neutral,
                unplug,
                ..timeout()
            }]);
            let error = script.run(&|| true).unwrap_err();
            assert!(error.cleanup_failed);
            assert_eq!(error.native_code, Some(483));
            assert_eq!(script.target.as_ref().unwrap().id, 1);
            assert_eq!(script.events.borrow().last(), Some(&(1, "unplug")));
            if neutral.is_some() {
                assert!(error.detail.contains("neutral failed"));
            }
            if unplug.is_some() {
                assert!(error.detail.contains("unplug failed"));
            }
        }
    }
    #[test]
    fn startup_cancellation_stops_before_next_driver_call_or_replacement() {
        for limit in [0, 1, 2, 6, 7, 8, 9] {
            let mut script = Script::new(vec![timeout(), Plan::default()]);
            let events = script.events.clone();
            assert!(!script.run(&|| events.borrow().len() < limit).unwrap());
            assert_eq!(events.borrow().len(), limit);
        }
    }
    #[test]
    fn full_report_and_neutral_match_every_vigem_field() {
        let mapped = report(XboxReport {
            buttons: 0xf3d9,
            left_x: i16::MIN,
            left_y: i16::MAX,
            right_x: -23170,
            right_y: 23170,
            left_trigger: 128,
            right_trigger: 255,
        });
        assert_eq!(
            mapped,
            XGamepad {
                buttons: XButtons { raw: 0xf3d9 },
                thumb_lx: i16::MIN,
                thumb_ly: i16::MAX,
                thumb_rx: -23170,
                thumb_ry: 23170,
                left_trigger: 128,
                right_trigger: 255,
            }
        );
        assert_eq!(report(XboxReport::NEUTRAL), XGamepad::default());
    }
    #[test]
    fn left_axes_copy_exactly_and_unimplemented_fields_remain_neutral() {
        for (x, y) in [
            (i16::MIN, 0),
            (i16::MAX, 0),
            (0, i16::MIN),
            (0, i16::MAX),
            (23170, -23170),
            (-23170, 23170),
            (0, 0),
        ] {
            for a in [false, true] {
                let mapped = report(XboxReport {
                    buttons: if a { 0x1000 } else { 0 },
                    left_x: x,
                    left_y: y,
                    ..XboxReport::NEUTRAL
                });
                assert_eq!((mapped.thumb_lx, mapped.thumb_ly), (x, y));
                assert_eq!(mapped.buttons.raw, if a { XButtons::A } else { 0 });
                assert_eq!(
                    XGamepad {
                        buttons: XButtons { raw: 0 },
                        thumb_lx: 0,
                        thumb_ly: 0,
                        ..mapped
                    },
                    XGamepad::default()
                );
            }
        }
    }
    #[test]
    fn a_maps_to_xbox_a_and_every_other_field_stays_neutral() {
        let pressed = report(XboxReport {
            buttons: 0x1000,
            ..XboxReport::NEUTRAL
        });
        assert_eq!(pressed.buttons.raw, 0x1000);
        assert_eq!(
            XGamepad {
                buttons: XButtons { raw: 0 },
                ..pressed
            },
            XGamepad::default()
        );
        assert_eq!(report(XboxReport::NEUTRAL), XGamepad::default());
    }
}
