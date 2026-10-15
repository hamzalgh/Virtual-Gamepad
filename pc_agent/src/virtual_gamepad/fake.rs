//! Recording backend for deterministic resource and safety tests, without a driver.
use super::{BackendError, BackendHealth, VirtualGamepadBackend, XboxReport};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Call {
    Attach,
    Submit(XboxReport),
    Neutral,
    Detach,
}

pub struct RecordingBackend {
    pub calls: Vec<Call>,
    pub failures: Vec<Call>,
    health: BackendHealth,
}
impl Default for RecordingBackend {
    fn default() -> Self {
        Self {
            calls: Vec::new(),
            failures: Vec::new(),
            health: BackendHealth::Detached,
        }
    }
}
impl RecordingBackend {
    fn record(&mut self, call: Call, operation: &'static str) -> Result<(), BackendError> {
        self.calls.push(call);
        if self.failures.contains(&call) {
            self.health = BackendHealth::Fault;
            return Err(BackendError {
                operation,
                detail: "Injected test failure".into(),
                native_code: None,
                attempt: 1,
                cleanup_failed: false,
            });
        }
        Ok(())
    }
}
impl VirtualGamepadBackend for RecordingBackend {
    fn attach(&mut self) -> Result<(), BackendError> {
        self.record(Call::Attach, "attach")?;
        self.health = BackendHealth::Ready;
        Ok(())
    }
    fn submit(&mut self, report: XboxReport) -> Result<(), BackendError> {
        self.record(Call::Submit(report), "submit")
    }
    fn neutral(&mut self) -> Result<(), BackendError> {
        self.record(Call::Neutral, "neutral")
    }
    fn detach(&mut self) -> Result<(), BackendError> {
        self.record(Call::Detach, "detach")?;
        self.health = BackendHealth::Detached;
        Ok(())
    }
    fn health(&self) -> BackendHealth {
        self.health
    }
}

// Shared recording is useful when an asynchronous owner consumes the backend.
impl VirtualGamepadBackend for std::sync::Arc<std::sync::Mutex<RecordingBackend>> {
    fn attach(&mut self) -> Result<(), BackendError> {
        self.lock().unwrap().attach()
    }
    fn submit(&mut self, report: XboxReport) -> Result<(), BackendError> {
        self.lock().unwrap().submit(report)
    }
    fn neutral(&mut self) -> Result<(), BackendError> {
        self.lock().unwrap().neutral()
    }
    fn detach(&mut self) -> Result<(), BackendError> {
        self.lock().unwrap().detach()
    }
    fn health(&self) -> BackendHealth {
        self.lock().unwrap().health()
    }
}
