//! Presentation independent ownership. Await `run` through cleanup; never abort it.
use crate::{
    config::Config,
    discovery::Discovery,
    input::{LatestState, OutputHealth},
    network::Receiver,
    paired_session::{PairedSession, PairingCommand},
    pairing_store::{PairingRecord, PairingStore},
    session::{DevelopmentTrust, Handler},
    virtual_gamepad::VirtualGamepadBackend,
};
use std::{
    future::Future,
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::mpsc::{self, SyncSender},
};
use tokio::{net::UdpSocket, sync::watch, time::Instant};

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type CodeReceiver = mpsc::Receiver<(u32, Instant)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lifecycle {
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
}

/// Deliberately excludes session identifiers and pairing secrets.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BackendStatus {
    pub ready: bool,
    pub attached: bool,
    pub timed_out: bool,
    pub fault: bool,
    pub diagnostic: Option<crate::virtual_gamepad::BackendDiagnostic>,
}
impl From<OutputHealth> for BackendStatus {
    fn from(value: OutputHealth) -> Self {
        Self {
            ready: value.ready,
            attached: value.attached,
            timed_out: value.timed_out,
            fault: value.fault,
            diagnostic: value.diagnostic,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceStatus {
    pub lifecycle: Lifecycle,
    pub bound: SocketAddr,
    /// Startup inventory only; neither reachability nor firewall evidence.
    pub addresses: Vec<Ipv4Addr>,
    pub development: bool,
    pub backend: BackendStatus,
    pub counters: crate::telemetry::Counters,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandError {
    Unavailable,
    Busy,
}

#[derive(Clone)]
pub struct ApplicationHandle {
    commands: SyncSender<PairingCommand>,
    shutdown: watch::Sender<bool>,
    status: watch::Receiver<ServiceStatus>,
    pairing: watch::Receiver<crate::paired_session::PairingView>,
}
impl ApplicationHandle {
    pub fn pairing(&self) -> crate::paired_session::PairingView {
        *self.pairing.borrow()
    }
    pub fn command(&self, command: PairingCommand) -> Result<(), CommandError> {
        let status = self.status.borrow();
        if status.lifecycle != Lifecycle::Running || status.development || *self.shutdown.borrow() {
            return Err(CommandError::Unavailable);
        }
        self.commands
            .try_send(command)
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => CommandError::Busy,
                mpsc::TrySendError::Disconnected(_) => CommandError::Unavailable,
            })
    }
    /// A separate latest value signal, unaffected by a saturated command queue.
    pub fn exit(&self) {
        self.shutdown.send_replace(true);
    }
    pub fn status(&self) -> ServiceStatus {
        self.status.borrow().clone()
    }
    pub fn subscribe(&self) -> watch::Receiver<ServiceStatus> {
        self.status.clone()
    }
}

pub struct Application {
    config: Config,
    socket: UdpSocket,
    store: Option<(PairingStore, PairingRecord)>,
    commands: mpsc::Receiver<PairingCommand>,
    codes: SyncSender<(u32, Instant)>,
    shutdown: watch::Receiver<bool>,
    status: watch::Sender<ServiceStatus>,
    discovery: bool,
    pairing: watch::Sender<crate::paired_session::PairingView>,
}
impl Application {
    pub async fn prepare(config: Config) -> Result<(Self, ApplicationHandle, CodeReceiver), Error> {
        let directory = if config.manual_trust_first_client {
            PathBuf::new()
        } else {
            PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("LocalAppData is unavailable")?)
                .join("VirtualGamepad")
        };
        Self::prepare_at(config, directory, true).await
    }
    async fn prepare_at(
        config: Config,
        directory: PathBuf,
        discovery: bool,
    ) -> Result<(Self, ApplicationHandle, CodeReceiver), Error> {
        // Defend composition too, even if Config was constructed directly.
        if config.manual_trust_first_client && !cfg!(debug_assertions) {
            return Err("Development trust is unavailable in release".into());
        }
        let store = if config.manual_trust_first_client {
            None
        } else {
            Some(tokio::task::spawn_blocking(move || PairingStore::open(directory)).await??)
        };
        Self::from_store(config, store, discovery).await
    }
    pub(crate) async fn from_store(
        config: Config,
        store: Option<(PairingStore, PairingRecord)>,
        discovery: bool,
    ) -> Result<(Self, ApplicationHandle, CodeReceiver), Error> {
        if config.manual_trust_first_client && !cfg!(debug_assertions) {
            return Err("Development trust is unavailable in release".into());
        }
        let socket = UdpSocket::bind(config.bind_addr()).await?;
        let bound = socket.local_addr()?;
        let addresses = tokio::task::spawn_blocking(move || {
            let ips = if_addrs::get_if_addrs()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|interface| match interface.ip() {
                    std::net::IpAddr::V4(ip) => Some(ip),
                    _ => None,
                })
                .collect();
            crate::discovery::allowed_addresses(ips, config.bind_addr())
        })
        .await?;
        let (status, status_rx) = watch::channel(ServiceStatus {
            lifecycle: Lifecycle::Starting,
            bound,
            addresses,
            development: config.manual_trust_first_client,
            backend: BackendStatus::default(),
            counters: Default::default(),
        });
        let (commands_tx, commands) = mpsc::sync_channel(8);
        let (codes, codes_rx) = mpsc::sync_channel(8);
        let (shutdown_tx, shutdown) = watch::channel(false);
        let (pairing, pairing_rx) = watch::channel(Default::default());
        Ok((
            Self {
                config,
                socket,
                store,
                commands,
                codes,
                shutdown,
                status,
                discovery,
                pairing,
            },
            ApplicationHandle {
                commands: commands_tx,
                shutdown: shutdown_tx,
                status: status_rx,
                pairing: pairing_rx,
            },
            codes_rx,
        ))
    }
    /// Own the backend, socket and workers until all ordinary cleanup has completed.
    pub async fn run<B: VirtualGamepadBackend + Send + 'static>(
        self,
        backend: B,
        signal: impl Future<Output = std::io::Result<()>>,
    ) -> Result<(), Error> {
        let status = self.status.clone();
        let result = self.run_owned(backend, signal).await;
        status.send_modify(|status| {
            status.lifecycle = if result.is_ok() {
                Lifecycle::Stopped
            } else {
                Lifecycle::Failed
            }
        });
        result
    }
    async fn run_owned<B: VirtualGamepadBackend + Send + 'static>(
        self,
        backend: B,
        signal: impl Future<Output = std::io::Result<()>>,
    ) -> Result<(), Error> {
        let Self {
            config,
            socket,
            store,
            commands,
            codes,
            shutdown,
            status,
            discovery,
            pairing,
        } = self;
        // No fallible presentation setup or storage initialization after output starts.
        let (storage, record) = match store {
            Some((store, record)) => (Some(crate::pairing_store::worker(store)?), Some(record)),
            None => (None, None),
        };
        let advertisement =
            record.filter(|_| discovery).and_then(|record| {
                match Discovery::start(
                    std::env::var("COMPUTERNAME").unwrap_or_else(|_| "Virtual Gamepad".into()),
                    record.server.0.get(),
                    config.bind_addr(),
                ) {
                    Ok(discovery) => Some(discovery),
                    Err(_) => {
                        tracing::warn!("Discovery worker unavailable; manual IP remains available");
                        None
                    }
                }
            });
        let (latest, state) = LatestState::channel();
        let (requests, request_rx) = watch::channel(None);
        let (health_tx, health_rx) = watch::channel(OutputHealth::default());
        let final_health = health_rx.clone();
        let runtime = tokio::runtime::Handle::current();
        let output = tokio::task::spawn_blocking(move || {
            runtime.block_on(crate::input::run_output_with_timeout(
                backend,
                request_rx,
                state,
                health_tx,
                config.input_timeout,
            ))
        });
        let stop = shutdown_or_signal(shutdown, signal);
        status.send_modify(|status| status.lifecycle = Lifecycle::Running);
        let result = if let Some(record) = record {
            let handler = PairedSession::new(
                record,
                storage.as_ref().expect("production storage").0.clone(),
                commands,
                requests,
                health_rx.clone(),
                config.input_timeout,
                codes,
            )
            .with_view(pairing);
            let (receiver, metrics) = Receiver::new(socket, handler, latest);
            serve(receiver, output, stop, health_rx, &status, metrics).await
        } else {
            drop((commands, codes));
            let handler = DevelopmentTrust::with_output(true, requests, health_rx.clone())
                .with_timeout(config.input_timeout);
            let (receiver, metrics) = Receiver::new(socket, handler, latest);
            serve(receiver, output, stop, health_rx, &status, metrics).await
        };
        status.send_modify(|status| status.backend = (*final_health.borrow()).into());
        // Receiver and output are gone. Join discovery and disk outside async workers.
        let cleanup = tokio::task::spawn_blocking(move || -> Result<(), Error> {
            drop(advertisement);
            if let Some((sender, worker)) = storage {
                drop(sender);
                worker
                    .join()
                    .map_err(|_| "Pairing storage worker panicked")?;
            }
            Ok(())
        })
        .await?;
        if let Err(cleanup) = cleanup {
            return Err(
                format!("Service cleanup failed: {cleanup}; run result: {result:?}").into(),
            );
        }
        result
    }
}

async fn shutdown_or_signal(
    mut shutdown: watch::Receiver<bool>,
    signal: impl Future<Output = std::io::Result<()>>,
) -> std::io::Result<()> {
    tokio::select! {
        biased;
        _ = async {
            loop {
                if *shutdown.borrow_and_update() || shutdown.changed().await.is_err() { return; }
            }
        } => Ok(()),
        result = signal => result,
    }
}

async fn serve<H: Handler>(
    receiver: Receiver<H>,
    mut output: tokio::task::JoinHandle<Result<(), String>>,
    shutdown: impl Future<Output = std::io::Result<()>>,
    mut health: watch::Receiver<OutputHealth>,
    status: &watch::Sender<ServiceStatus>,
    metrics: watch::Receiver<crate::telemetry::Counters>,
) -> Result<(), Error> {
    let mut signal_result = Ok(());
    let mut output_result = None;
    let receive_result = receiver
        .run(async {
            tokio::pin!(shutdown);
            let mut health_open = true;
            let mut report = tokio::time::interval(std::time::Duration::from_millis(500));
            report.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    biased;
                    result = &mut shutdown => { signal_result = result; break; }
                    result = &mut output => { output_result = Some(result); break; }
                    _ = report.tick() => {
                        let counters = *metrics.borrow();
                        status.send_if_modified(|s| {if s.counters==counters {false} else {s.counters=counters;true}});
                    }
                    result = health.changed(), if health_open => {
                        if result.is_err() { health_open = false; } else {
                            let backend = (*health.borrow_and_update()).into();
                            status.send_if_modified(|status| {
                                if status.backend == backend { return false; }
                                status.backend = backend;
                                true
                            });
                        }
                    }
                }
            }
        })
        .await;
    status.send_modify(|status| {
        status.lifecycle = Lifecycle::Stopping;
        status.counters = *metrics.borrow();
    });
    // Receiver destruction closes requests. Never return before output cleanup.
    if let Some(result) = output_result {
        result??;
        return Err("Output task stopped unexpectedly; receiver closed".into());
    }
    output.await??;
    receive_result?;
    signal_result?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        protocol::*,
        session::DenyAll,
        virtual_gamepad::fake::{Call, RecordingBackend},
    };
    use std::{
        num::NonZeroU64,
        sync::{Arc, Mutex},
        time::Duration,
    };

    struct TestDirectory(PathBuf);
    impl TestDirectory {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!(
                "vg-service-test-{}",
                crate::session::random_id().unwrap()
            )))
        }
    }
    impl Drop for TestDirectory {
        fn drop(&mut self) {
            if self.0.exists() {
                std::fs::remove_dir_all(&self.0).unwrap();
            }
        }
    }
    fn config() -> Config {
        let reserve = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        Config::from_args(["--bind".into(), reserve.local_addr().unwrap().to_string()]).unwrap()
    }
    fn status(bound: SocketAddr) -> watch::Sender<ServiceStatus> {
        watch::channel(ServiceStatus {
            lifecycle: Lifecycle::Running,
            bound,
            addresses: vec![],
            development: false,
            backend: BackendStatus::default(),
            counters: Default::default(),
        })
        .0
    }
    async fn receive(socket: &UdpSocket) -> Message {
        let mut bytes = [0; PACKET_LEN];
        let len = tokio::time::timeout(Duration::from_secs(2), socket.recv(&mut bytes))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(len, PACKET_LEN);
        crate::protocol::decode(&bytes).unwrap()
    }
    async fn send(socket: &UdpSocket, message: Message) {
        socket.send(&message.encode().unwrap()).await.unwrap();
    }
    async fn running(handle: &ApplicationHandle) {
        let mut status = handle.subscribe();
        tokio::time::timeout(Duration::from_secs(2), async {
            while status.borrow_and_update().lifecycle != Lifecycle::Running {
                status.changed().await.unwrap();
            }
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn delayed_popup_close_cannot_cancel_a_new_pairing_window() {
        let directory = TestDirectory::new();
        let (app, handle, _codes) = Application::prepare_at(config(), directory.0.clone(), false)
            .await
            .unwrap();
        let task = tokio::spawn(app.run(RecordingBackend::default(), std::future::pending()));
        running(&handle).await;
        handle.command(PairingCommand::Open).unwrap();
        let first = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if let Some((_, deadline)) = handle.pairing().window {
                    break deadline;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        // These commands may be consumed in the same owner tick. Checking the
        // snapshot only in the desktop supervisor would leave this race open.
        handle.command(PairingCommand::Open).unwrap();
        handle.command(PairingCommand::CloseIf(first)).unwrap();
        let second = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if let Some((_, deadline)) = handle.pairing().window
                    && deadline != first
                {
                    break deadline;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        handle.command(PairingCommand::CloseIf(second)).unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            while handle.pairing().window.is_some() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        handle.exit();
        task.await.unwrap().unwrap();
    }
    #[tokio::test]
    async fn live_pairing_view_clears_on_close_and_shutdown() {
        let directory = TestDirectory::new();
        let (app, handle, _codes) = Application::prepare_at(config(), directory.0.clone(), false)
            .await
            .unwrap();
        let task = tokio::spawn(app.run(RecordingBackend::default(), std::future::pending()));
        running(&handle).await;
        handle.command(PairingCommand::Open).unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            while handle.pairing().window.is_none() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        handle.command(PairingCommand::Close).unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            while handle.pairing().window.is_some() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        handle.exit();
        task.await.unwrap().unwrap();
        assert!(handle.pairing().window.is_none());
    }
    #[tokio::test]
    async fn application_pairs_through_shared_service_and_exit_joins_neutral_and_storage() {
        let directory = TestDirectory::new();
        let (app, handle, codes) = Application::prepare_at(config(), directory.0.clone(), false)
            .await
            .unwrap();
        assert_eq!(handle.status().lifecycle, Lifecycle::Starting);
        assert_eq!(
            handle.command(PairingCommand::Open),
            Err(CommandError::Unavailable)
        );
        let bound = handle.status().bound;
        let backend = Arc::new(Mutex::new(RecordingBackend::default()));
        let task = tokio::spawn(app.run(backend.clone(), std::future::pending()));
        running(&handle).await;
        handle.command(PairingCommand::Open).unwrap();
        let code = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if let Ok((code, _)) = codes.try_recv() {
                    break code;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        socket.connect(bound).await.unwrap();
        let client = InstanceId(NonZeroU64::new(7).unwrap());
        send(
            &socket,
            Message::Hello(Hello {
                client_nonce: Nonce(NonZeroU64::new(8).unwrap()),
                client_instance: client,
                state_rate: 60,
                capabilities: 3,
                app_build: 1,
                api_level: 34,
            }),
        )
        .await;
        let Message::HelloAck(hello) = receive(&socket).await else {
            panic!("Expected HelloAck");
        };
        assert!(hello.session.is_none());
        send(
            &socket,
            Message::PairRequest(PairRequest {
                client_nonce: hello.client_nonce,
                server_nonce: hello.server_nonce,
                client_instance: client,
                code,
                slot: 255,
            }),
        )
        .await;
        let accept = loop {
            match receive(&socket).await {
                Message::PairAccept(accept) => break accept,
                Message::Status(_) => {}
                _ => panic!("Expected PairAccept"),
            }
        };
        send(
            &socket,
            Message::ControllerState(ControllerState {
                session: accept.session,
                sequence: 1,
                client_ms: 0,
                buttons: 1,
                left_x: 0,
                left_y: 0,
                right_x: 0,
                right_y: 0,
                left_trigger: 0,
                right_trigger: 0,
            }),
        )
        .await;
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if backend
                    .lock()
                    .unwrap()
                    .calls
                    .iter()
                    .any(|call| matches!(call, Call::Submit(report) if report.buttons == 0x1000))
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        handle.exit();
        tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(handle.status().lifecycle, Lifecycle::Stopped);
        assert!(!handle.status().backend.attached);
        assert!(
            backend
                .lock()
                .unwrap()
                .calls
                .ends_with(&[Call::Neutral, Call::Detach])
        );
        assert!(UdpSocket::bind(bound).await.is_ok());
        let (store, record) = PairingStore::open(directory.0.clone()).unwrap();
        assert_eq!(record.client, Some(client));
        drop(store);
    }

    #[tokio::test]
    async fn full_commands_cannot_block_exit_and_drop_of_last_handle_stops() {
        for explicit in [true, false] {
            let directory = TestDirectory::new();
            let (app, handle, _codes) =
                Application::prepare_at(config(), directory.0.clone(), false)
                    .await
                    .unwrap();
            let observed = handle.subscribe();
            // Fill without polling the receiver to model a saturated presentation queue.
            app.status
                .send_modify(|status| status.lifecycle = Lifecycle::Running);
            for _ in 0..8 {
                handle.command(PairingCommand::Open).unwrap();
            }
            assert_eq!(
                handle.command(PairingCommand::Open),
                Err(CommandError::Busy)
            );
            if explicit {
                handle.exit();
                assert_eq!(
                    handle.command(PairingCommand::Open),
                    Err(CommandError::Unavailable)
                );
            }
            drop(handle);
            let backend = Arc::new(Mutex::new(RecordingBackend::default()));
            tokio::time::timeout(
                Duration::from_secs(2),
                app.run(backend.clone(), std::future::pending()),
            )
            .await
            .unwrap()
            .unwrap();
            assert_eq!(observed.borrow().lifecycle, Lifecycle::Stopped);
            assert!(backend.lock().unwrap().calls.is_empty());
        }
    }

    #[tokio::test]
    async fn preparation_preserves_corrupt_storage_and_releases_lock_after_bind_failure() {
        let directory = TestDirectory::new();
        let (store, _) = PairingStore::open(directory.0.clone()).unwrap();
        drop(store);
        let path = directory.0.join("pairing.bin");
        let original = std::fs::read(&path).unwrap();
        std::fs::write(&path, b"corrupt").unwrap();
        assert!(
            Application::prepare_at(config(), directory.0.clone(), false)
                .await
                .is_err()
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"corrupt");
        std::fs::write(&path, original).unwrap();
        let occupied = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let config =
            Config::from_args(["--bind".into(), occupied.local_addr().unwrap().to_string()])
                .unwrap();
        assert!(
            Application::prepare_at(config, directory.0.clone(), false)
                .await
                .is_err()
        );
        assert!(PairingStore::open(directory.0.clone()).is_ok());
    }

    #[tokio::test]
    async fn signal_failure_still_joins_service_and_releases_storage() {
        let directory = TestDirectory::new();
        let (app, handle, _codes) = Application::prepare_at(config(), directory.0.clone(), false)
            .await
            .unwrap();
        let result = app
            .run(RecordingBackend::default(), async {
                Err(std::io::Error::other("Injected signal failure"))
            })
            .await;
        assert!(result.is_err());
        assert_eq!(handle.status().lifecycle, Lifecycle::Failed);
        assert!(UdpSocket::bind(handle.status().bound).await.is_ok());
        assert!(PairingStore::open(directory.0.clone()).is_ok());
    }

    // These two supervisor regressions moved from main.rs with the service owner.
    #[tokio::test]
    async fn output_task_error_or_panic_closes_udp_without_waiting_for_signal() {
        for panic in [false, true] {
            let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
            let bound = socket.local_addr().unwrap();
            let (latest, state) = LatestState::channel();
            let (receiver, metrics) = Receiver::new(socket, DenyAll, latest);
            let output = tokio::spawn(async move {
                assert!(!panic, "Injected output task panic");
                Err("Injected output task error".into())
            });
            let (_health, health_rx) = watch::channel(OutputHealth::default());
            let result = tokio::time::timeout(
                Duration::from_secs(1),
                serve(
                    receiver,
                    output,
                    std::future::pending(),
                    health_rx,
                    &status(bound),
                    metrics,
                ),
            )
            .await
            .unwrap();
            assert!(result.is_err());
            assert!(state.borrow().is_none());
            assert!(UdpSocket::bind(bound).await.is_ok());
        }
    }
    #[tokio::test]
    async fn signal_error_still_joins_output_cleanup() {
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let bound = socket.local_addr().unwrap();
        let (latest, mut state) = LatestState::channel();
        let (receiver, metrics) = Receiver::new(socket, DenyAll, latest);
        let finished = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let observed = finished.clone();
        let output = tokio::spawn(async move {
            while state.changed().await.is_ok() {}
            observed.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        });
        let (_health, health_rx) = watch::channel(OutputHealth::default());
        let result = serve(
            receiver,
            output,
            async { Err(std::io::Error::other("Injected signal failure")) },
            health_rx,
            &status(bound),
            metrics,
        )
        .await;
        assert!(result.is_err());
        assert!(finished.load(std::sync::atomic::Ordering::SeqCst));
    }
    #[cfg(not(debug_assertions))]
    #[tokio::test]
    async fn release_service_rejects_directly_constructed_development_config() {
        let mut config = Config::default();
        config.manual_trust_first_client = true;
        assert!(
            Application::prepare_at(config, PathBuf::new(), false)
                .await
                .is_err()
        );
    }
}
