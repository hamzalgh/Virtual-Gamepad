//! Fixed size counters, emitted once per reporting interval and on shutdown.
use crate::protocol::DecodeError;

pub fn terminal_event_enabled(target: &str, level: tracing::Level, debug: bool) -> bool {
    if target == "virtual_gamepad_agent::diagnostics" {
        debug
    } else if target.starts_with("virtual_gamepad_agent") {
        level
            <= if debug {
                tracing::Level::DEBUG
            } else {
                tracing::Level::INFO
            }
    } else {
        level <= tracing::Level::WARN
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counters {
    pub decoded: u64,
    pub accepted: u64,
    pub estimated_loss: u64,
    pub invalid: u64,
    pub stale: u64,
    pub wrong_source: u64,
    pub wrong_session: u64,
    pub unsupported: u64,
    pub invalid_length: u64,
    pub invalid_magic: u64,
    pub invalid_version: u64,
    pub invalid_type: u64,
    pub invalid_crc: u64,
    pub invalid_reserved: u64,
    pub invalid_range: u64,
    pub invalid_dpad: u64,
    /// Subset of invalid, datagrams larger than the v1 envelope.
    pub oversized: u64,
    pub refused: u64,
    pub replies: u64,
    pub reply_errors: u64,
    pub io_errors: u64,
    pub last_rejection: Option<DecodeError>,
}
impl Counters {
    pub fn reject(&mut self, error: DecodeError) {
        self.invalid = self.invalid.saturating_add(1);
        self.last_rejection = Some(error);
        let count = match error {
            DecodeError::Length => &mut self.invalid_length,
            DecodeError::Magic => &mut self.invalid_magic,
            DecodeError::Version => &mut self.invalid_version,
            DecodeError::MessageType => &mut self.invalid_type,
            DecodeError::Crc => &mut self.invalid_crc,
            DecodeError::Reserved => &mut self.invalid_reserved,
            DecodeError::Range => &mut self.invalid_range,
            DecodeError::DpadConflict => &mut self.invalid_dpad,
        };
        *count = count.saturating_add(1);
    }
    pub fn refuse(&mut self, error: crate::session::AdmissionError) {
        use crate::session::AdmissionError;
        if error == AdmissionError::Stale {
            self.stale = self.stale.saturating_add(1);
            return;
        }
        self.refused = self.refused.saturating_add(1);
        let count = match error {
            AdmissionError::WrongSource => Some(&mut self.wrong_source),
            AdmissionError::WrongSession => Some(&mut self.wrong_session),
            AdmissionError::UnsupportedState => Some(&mut self.unsupported),
            _ => None,
        };
        if let Some(count) = count {
            *count = count.saturating_add(1);
        }
    }
    pub fn log(&self, event: &'static str) {
        tracing::info!(target: "virtual_gamepad_agent::diagnostics", event, decoded = self.decoded, accepted = self.accepted,
            estimated_loss = self.estimated_loss,
            invalid = self.invalid, stale = self.stale, oversized = self.oversized,
            refused = self.refused, replies = self.replies, reply_errors = self.reply_errors, io_errors = self.io_errors,
            wrong_source = self.wrong_source, wrong_session = self.wrong_session, unsupported = self.unsupported,
            length = self.invalid_length, magic = self.invalid_magic, version = self.invalid_version,
            message_type = self.invalid_type, crc = self.invalid_crc, reserved = self.invalid_reserved,
            range = self.invalid_range, dpad = self.invalid_dpad, log_drops = log_drops(),
            rejection = ?self.last_rejection, "UDP diagnostics");
    }
}

use std::{
    io::{self, Write},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
};
const LOG_BYTES: usize = 2048;
static LOG_DROPS: AtomicU64 = AtomicU64::new(0);
pub fn log_drops() -> u64 {
    LOG_DROPS.load(Ordering::Relaxed)
}
fn dropped() {
    let _ = LOG_DROPS.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
        Some(n.saturating_add(1))
    });
}
struct LogEntry {
    bytes: [u8; LOG_BYTES],
    len: usize,
    flush: Option<mpsc::SyncSender<()>>,
}
#[derive(Clone)]
pub struct LogWriter(mpsc::SyncSender<LogEntry>);
pub struct LogEvent {
    writer: LogWriter,
    bytes: [u8; LOG_BYTES],
    len: usize,
    overflow: bool,
}
impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogWriter {
    type Writer = LogEvent;
    fn make_writer(&'a self) -> LogEvent {
        LogEvent {
            writer: self.clone(),
            bytes: [0; LOG_BYTES],
            len: 0,
            overflow: false,
        }
    }
}
impl Write for LogEvent {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let available = (LOG_BYTES - self.len).min(bytes.len());
        self.bytes[self.len..self.len + available].copy_from_slice(&bytes[..available]);
        self.len += available;
        self.overflow |= available < bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Drop for LogEvent {
    fn drop(&mut self) {
        if self.overflow
            || self
                .writer
                .0
                .try_send(LogEntry {
                    bytes: self.bytes,
                    len: self.len,
                    flush: None,
                })
                .is_err()
        {
            dropped();
        }
    }
}
/// Dropped only after controller cleanup. Never wait indefinitely for a console.
pub struct LogGuard(LogWriter);
impl Drop for LogGuard {
    fn drop(&mut self) {
        let (tx, rx) = mpsc::sync_channel(1);
        if self
            .0
            .0
            .try_send(LogEntry {
                bytes: [0; LOG_BYTES],
                len: 0,
                flush: Some(tx),
            })
            .is_ok()
        {
            let _ = rx.recv_timeout(std::time::Duration::from_millis(100));
        }
    }
}
/// Fixed 64 event queue. No terminal I/O runs in the input or UDP tasks.
pub fn terminal_writer() -> io::Result<(LogWriter, LogGuard)> {
    queued_writer(io::stdout())
}
fn queued_writer(mut output: impl Write + Send + 'static) -> io::Result<(LogWriter, LogGuard)> {
    let (tx, rx) = mpsc::sync_channel::<LogEntry>(64);
    std::thread::Builder::new()
        .name("terminal-log".into())
        .spawn(move || {
            while let Ok(entry) = rx.recv() {
                if let Some(done) = entry.flush {
                    let _ = output.flush();
                    let _ = done.try_send(());
                } else if output.write_all(&entry.bytes[..entry.len]).is_err() {
                    dropped();
                }
            }
        })?;
    let writer = LogWriter(tx);
    Ok((writer.clone(), LogGuard(writer)))
}

static DEBUG_ENABLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub fn set_debug(enabled: bool) {
    DEBUG_ENABLED.store(enabled, Ordering::Relaxed);
}
pub fn debug_enabled() -> bool {
    DEBUG_ENABLED.load(Ordering::Relaxed)
}
/// Disk I/O stays on the bounded logging worker, never on the receiver/output owner.
pub fn file_writer(root: &std::path::Path) -> io::Result<(LogWriter, LogGuard)> {
    queued_writer(RotatingLog {
        root: root.to_owned(),
    })
}
struct RotatingLog {
    root: std::path::PathBuf,
}
impl Write for RotatingLog {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        use std::fs;
        let path = self.root.join("agent.log");
        crate::pairing_store::reject_link(&path)?;
        if fs::metadata(&path).map(|m| m.len()).unwrap_or(0) + bytes.len() as u64 > 1024 * 1024 {
            for index in (1..=3).rev() {
                let target = self.root.join(format!("agent.log.{index}"));
                let source = if index == 1 {
                    path.clone()
                } else {
                    self.root.join(format!("agent.log.{}", index - 1))
                };
                crate::pairing_store::reject_link(&target)?;
                crate::pairing_store::reject_link(&source)?;
                if source.exists() {
                    if target.exists() {
                        fs::remove_file(&target)?;
                    }
                    fs::rename(source, target)?;
                }
            }
        }
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?
            .write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::fmt::MakeWriter;
    #[test]
    fn rotated_logs_keep_three_backups_and_bounded_current_file() {
        let root = std::env::temp_dir().join(format!(
            "vg-log-test-{}",
            crate::session::random_id().unwrap()
        ));
        std::fs::create_dir(&root).unwrap();
        let mut writer = RotatingLog { root: root.clone() };
        for _ in 0..6 {
            writer.write_all(&vec![b'x'; 1024 * 1024]).unwrap();
        }
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 4);
        for entry in std::fs::read_dir(&root).unwrap() {
            assert_eq!(entry.unwrap().metadata().unwrap().len(), 1024 * 1024);
        }
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn terminal_filter_keeps_errors_but_hides_dependency_chatter_and_optional_diagnostics() {
        use tracing::Level;
        assert!(!terminal_event_enabled(
            "mdns_sd::service_daemon",
            Level::DEBUG,
            true
        ));
        assert!(!terminal_event_enabled(
            "mdns_sd::service_daemon",
            Level::INFO,
            true
        ));
        assert!(terminal_event_enabled("mdns_sd", Level::ERROR, false));
        assert!(!terminal_event_enabled(
            "virtual_gamepad_agent::diagnostics",
            Level::INFO,
            false
        ));
        assert!(terminal_event_enabled(
            "virtual_gamepad_agent::diagnostics",
            Level::INFO,
            true
        ));
        assert!(terminal_event_enabled(
            "virtual_gamepad_agent::paired_session",
            Level::WARN,
            false
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn waiting_output_is_silent_even_with_diagnostic_logging_enabled() {
        let (tx, rx) = mpsc::sync_channel(64);
        let subscriber = tracing_subscriber::fmt()
            .with_writer(LogWriter(tx))
            .finish();
        let _guard = tracing::subscriber::set_default(subscriber);
        let (requests, request_rx) = tokio::sync::watch::channel(None);
        let (latest, state) = crate::input::LatestState::channel();
        let (health, _health_rx) = tokio::sync::watch::channel(Default::default());
        let task = tokio::spawn(crate::input::run_output(
            crate::virtual_gamepad::fake::RecordingBackend::default(),
            request_rx,
            state,
            health,
        ));
        tokio::task::yield_now().await;
        tokio::time::advance(std::time::Duration::from_secs(3)).await;
        tokio::task::yield_now().await;
        assert_eq!(rx.try_iter().count(), 0);
        drop((requests, latest));
        task.await.unwrap().unwrap();
    }
    #[test]
    fn blocked_or_failed_logger_never_waits_and_storage_is_bounded() {
        let (tx, rx) = mpsc::sync_channel(1);
        let writer = LogWriter(tx);
        let before = log_drops();
        for _ in 0..1000 {
            writer.make_writer().write_all(b"event\n").unwrap();
        }
        assert!(log_drops() >= before + 999);
        assert_eq!(rx.try_iter().count(), 1);
        drop(rx);
        writer.make_writer().write_all(b"closed\n").unwrap();
        let mut event = writer.make_writer();
        event.write_all(&[0; LOG_BYTES + 1]).unwrap();
        assert!(event.overflow);
    }
    #[test]
    fn categories_saturate_instead_of_wrapping() {
        let mut counters = Counters {
            invalid: u64::MAX,
            invalid_crc: u64::MAX,
            ..Default::default()
        };
        counters.reject(DecodeError::Crc);
        assert_eq!(counters.invalid, u64::MAX);
        assert_eq!(counters.invalid_crc, u64::MAX);
        counters.refuse(crate::session::AdmissionError::WrongSource);
        assert_eq!(counters.wrong_source, 1);
        assert_eq!(counters.refused, 1);
    }

    #[tokio::test]
    async fn malformed_flood_counts_every_packet_but_logs_only_at_report_rate() {
        use crate::{input::LatestState, network::Receiver, session::DenyAll};
        let (tx, rx) = mpsc::sync_channel(64);
        let subscriber = tracing_subscriber::fmt()
            .with_writer(LogWriter(tx))
            .finish();
        let _guard = tracing::subscriber::set_default(subscriber);
        let server = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let address = server.local_addr().unwrap();
        let phone = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let (latest, _states) = LatestState::channel();
        let (receiver, mut metrics) = Receiver::new(server, DenyAll, latest);
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let started = std::time::Instant::now();
        let task = tokio::spawn(receiver.run(async {
            let _ = stopped.await;
        }));
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            for expected in 1..=1000 {
                phone.send_to(&[0; 40], address).await.unwrap();
                while metrics.borrow_and_update().invalid < expected {
                    metrics.changed().await.unwrap();
                }
            }
        })
        .await
        .unwrap();
        stop.send(()).unwrap();
        let counters = task.await.unwrap().unwrap();
        assert_eq!(counters.invalid_magic, 1000);
        assert_eq!(counters.invalid, 1000);
        let logs = rx.try_iter().count() as u64;
        assert_eq!(logs, 1); // Waiting traffic is quiet; only the stopped report remains.
        assert!(
            logs <= started.elapsed().as_secs() + 2,
            "per packet logging is forbidden"
        );
    }
}
