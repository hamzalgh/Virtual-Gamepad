//! Desktop supervisor. One retained store lock, one running application, bounded UI work.
use crate::{
    application::{Application, ApplicationHandle, Error, ServiceStatus},
    desktop_settings::DesktopSettings,
    paired_session::{PairingCommand, PairingView},
    pairing_store::PairingStore,
};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::sync::{mpsc, watch};

#[derive(Clone)]
pub struct DesktopStatus {
    pub phase: &'static str,
    pub service: Option<ServiceStatus>,
    pub pairing: PairingView,
    pub error: String,
    pub setup: String,
    pub startup: bool,
    pub settings: DesktopSettings,
}
impl Default for DesktopStatus {
    fn default() -> Self {
        Self {
            phase: "Starting",
            service: None,
            pairing: Default::default(),
            error: String::new(),
            setup: "Setup has not been checked. Driver usability is checked when pairing.".into(),
            startup: false,
            settings: Default::default(),
        }
    }
}
pub enum DesktopCommand {
    Start,
    Stop,
    Pair(PairingCommand),
    // A dismissed popup must never cancel a subsequently opened window.
    ClosePairingWindow(tokio::time::Instant),
    Reload,
    Startup(bool),
    Preflight,
}
type ActiveService = (
    ApplicationHandle,
    tokio::task::JoinHandle<Result<(), Error>>,
    crate::application::CodeReceiver,
);
pub struct Desktop {
    pub commands: mpsc::Sender<DesktopCommand>,
    pub status: watch::Receiver<DesktopStatus>,
    pub exit: Arc<AtomicBool>,
    pub worker: Option<std::thread::JoinHandle<Result<(), Error>>>,
    pub root: PathBuf,
}
impl Desktop {
    pub fn spawn() -> Result<Self, Error> {
        let root =
            PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("LocalAppData unavailable")?)
                .join("VirtualGamepad");
        let (commands, rx) = mpsc::channel(8);
        let (tx, status) = watch::channel(DesktopStatus::default());
        let exit = Arc::new(AtomicBool::new(false));
        let stop = exit.clone();
        let directory = root.clone();
        let worker = std::thread::Builder::new()
            .name("desktop-service".into())
            .spawn(move || {
                let rt = tokio::runtime::Runtime::new()?;
                let result = rt.block_on(supervise(
                    directory,
                    rx,
                    stop,
                    &tx,
                    crate::virtual_gamepad::vigem::VigemBackend::default,
                ));
                tx.send_modify(|s| {
                    s.phase = "Exited";
                    s.pairing = Default::default();
                    if let Err(e) = &result {
                        s.error = e.to_string();
                    }
                });
                result
            })?;
        Ok(Self {
            commands,
            status,
            exit,
            worker: Some(worker),
            root,
        })
    }
    pub fn shutdown(&mut self) -> Result<(), Error> {
        self.exit.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            worker.join().map_err(|_| "Desktop worker panicked")??;
        }
        Ok(())
    }
}
impl Drop for Desktop {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

async fn supervise<B: crate::virtual_gamepad::VirtualGamepadBackend + Send + 'static>(
    root: PathBuf,
    mut commands: mpsc::Receiver<DesktopCommand>,
    exit: Arc<AtomicBool>,
    status: &watch::Sender<DesktopStatus>,
    backend: impl Fn() -> B,
) -> Result<(), Error> {
    let (owner, _) = PairingStore::open(root.clone())?;
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        os = std::env::consts::OS,
        arch = std::env::consts::ARCH,
        "Desktop owner acquired; driver/firewall inventory is available on request"
    );
    let loaded = DesktopSettings::load(&root);
    let mut valid_settings = loaded.is_ok();
    let mut settings = loaded.unwrap_or_else(|error| {
        status.send_modify(|s| {
            s.phase = "Stopped";
            s.error = error.to_string();
        });
        DesktopSettings::default()
    });
    crate::telemetry::set_debug(settings.debug);
    let mut active: Option<ActiveService> = None;
    let mut start = valid_settings;
    let mut tick = tokio::time::interval(Duration::from_millis(200));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    status.send_modify(|s| {
        s.settings = settings;
        s.startup = crate::windows_tray::startup_enabled().unwrap_or(false);
    });
    loop {
        if exit.load(Ordering::Acquire) {
            break;
        }
        if start && active.is_none() {
            start = false;
            status.send_modify(|s| {
                s.phase = "Starting";
                s.service = None;
                s.error.clear();
            });
            let prepared = match (owner.duplicate_owner(), owner.reload()) {
                (Ok(store), Ok(record)) => {
                    Application::from_store(
                        settings.config(),
                        Some((store, record)),
                        settings.discovery,
                    )
                    .await
                }
                (Err(e), _) | (_, Err(e)) => Err(e.into()),
            };
            match prepared {
                Ok((app, handle, codes)) => {
                    let task = tokio::spawn(app.run(backend(), std::future::pending()));
                    status.send_modify(|s| {
                        s.service = Some(handle.status());
                        s.phase = "Running";
                    });
                    active = Some((handle, task, codes));
                }
                Err(e) => status.send_modify(|s| {
                    s.phase = "Stopped";
                    s.error = e.to_string();
                }),
            }
        }
        if active
            .as_ref()
            .is_some_and(|(_, task, _)| task.is_finished())
        {
            let (handle, task, _) = active.take().unwrap();
            let result = task.await;
            status.send_modify(|s| {
                s.phase = "Stopped";
                s.service = Some(handle.status());
                s.pairing = Default::default();
                s.error = match result {
                    Ok(Ok(())) => String::new(),
                    Ok(Err(e)) => e.to_string(),
                    Err(e) => format!("Service task failed: {e}"),
                };
            });
        }
        tokio::select! {
            _=tick.tick()=> {
                if let Some((handle,_,codes))=&active {
                    for _ in 0..8 {if codes.try_recv().is_err(){break;}}
                    status.send_modify(|s| {s.service=Some(handle.status());s.pairing=if s.phase=="Running" {handle.pairing()} else {Default::default()};});
                }
            }
            command=commands.recv()=> match command {
                None=>break,
                Some(DesktopCommand::Start)=> {if active.is_none() && valid_settings {start=true;}},
                Some(DesktopCommand::Stop)=> {
                    if let Some((handle,_,_))=&active {handle.exit();status.send_modify(|s|{s.phase="Stopping";s.pairing=Default::default();});}
                }
                Some(DesktopCommand::Pair(command))=> {
                    if let Some((handle,_,_))=&active && handle.command(command).is_err(){status.send_modify(|s|s.error="Pairing is busy or unavailable. Retry when pending work finishes.".into());}
                }
                Some(DesktopCommand::ClosePairingWindow(deadline))=> {
                    if let Some((handle,_,_))=&active
                        && handle.pairing().window.is_some_and(|(_, current)| current == deadline)
                    {
                        // The UI retries while this same window is still published.
                        let _ = handle.command(PairingCommand::CloseIf(deadline));
                    }
                }
                Some(DesktopCommand::Reload)=> {
                    if active.is_none(){match DesktopSettings::load(&root) {Ok(value)=>{settings=value;valid_settings=true;crate::telemetry::set_debug(value.debug);status.send_modify(|s|{s.settings=value;s.error.clear();});},Err(e)=>{valid_settings=false;status.send_modify(|s|s.error=e.to_string());}}}
                    else {status.send_modify(|s|s.error="Stop listening before reloading settings.".into());}
                }
                Some(DesktopCommand::Startup(enabled))=> {
                    let result=tokio::task::spawn_blocking(move||crate::windows_tray::set_startup(enabled)).await;
                    status.send_modify(|s|match result {Ok(Ok(()))=>s.startup=enabled,Ok(Err(e))=>s.error=e.to_string(),Err(e)=>s.error=e.to_string()});
                }
                Some(DesktopCommand::Preflight)=> {
                    let result=tokio::task::spawn_blocking(crate::windows_tray::preflight).await;
                    status.send_modify(|s|s.setup=result.unwrap_or_else(|e|e.to_string()));
                }
            }
        }
    }
    status.send_modify(|s| {
        s.phase = "Stopping";
        s.pairing = Default::default();
    });
    if let Some((handle, task, _)) = active {
        handle.exit();
        task.await??;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    async fn wait(status: &mut watch::Receiver<DesktopStatus>, phase: &str) {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if status.borrow_and_update().phase == phase {
                    break;
                }
                status.changed().await.unwrap();
            }
        })
        .await
        .unwrap();
    }
    #[tokio::test]
    async fn stop_retains_owner_restart_serializes_and_invalid_reload_blocks_start() {
        let root = std::env::temp_dir().join(format!(
            "vg-desktop-test-{}",
            crate::session::random_id().unwrap()
        ));
        let (owner, _) = PairingStore::open(root.clone()).unwrap();
        let socket = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = socket.local_addr().unwrap().port();
        drop(socket);
        DesktopSettings {
            port,
            discovery: false,
            ..Default::default()
        }
        .save(&root)
        .unwrap();
        drop(owner);
        let (commands, rx) = mpsc::channel(8);
        let (tx, mut status) = watch::channel(Default::default());
        let exit = Arc::new(AtomicBool::new(false));
        let directory = root.clone();
        let stopped = exit.clone();
        let worker = tokio::spawn(async move {
            supervise(
                directory,
                rx,
                stopped,
                &tx,
                crate::virtual_gamepad::fake::RecordingBackend::default,
            )
            .await
        });
        wait(&mut status, "Running").await;
        commands.send(DesktopCommand::Stop).await.unwrap();
        wait(&mut status, "Stopped").await;
        assert!(PairingStore::open(root.clone()).is_err());
        let reserved = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        let replacement_port = reserved.local_addr().unwrap().port();
        DesktopSettings {
            port: replacement_port,
            discovery: false,
            ..Default::default()
        }
        .save(&root)
        .unwrap();
        commands.send(DesktopCommand::Reload).await.unwrap();
        tokio::time::timeout(Duration::from_secs(3), async {
            while status.borrow_and_update().settings.port != replacement_port {
                status.changed().await.unwrap();
            }
        })
        .await
        .unwrap();
        assert_eq!(status.borrow().phase, "Stopped");
        drop(reserved);
        commands.send(DesktopCommand::Start).await.unwrap();
        wait(&mut status, "Running").await;
        assert_eq!(
            status.borrow().service.as_ref().unwrap().bound.port(),
            replacement_port
        );
        assert!(
            std::net::UdpSocket::bind(status.borrow().service.as_ref().unwrap().bound).is_err()
        );
        commands.send(DesktopCommand::Stop).await.unwrap();
        wait(&mut status, "Stopped").await;
        std::fs::write(root.join("settings.ini"), b"version=99").unwrap();
        commands.send(DesktopCommand::Reload).await.unwrap();
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if !status.borrow_and_update().error.is_empty() {
                    break;
                }
                status.changed().await.unwrap();
            }
        })
        .await
        .unwrap();
        commands.send(DesktopCommand::Start).await.unwrap();
        tokio::time::sleep(Duration::from_millis(250)).await;
        assert_eq!(status.borrow().phase, "Stopped");
        exit.store(true, Ordering::Release);
        worker.await.unwrap().unwrap();
        assert_eq!(
            std::fs::read(root.join("settings.ini")).unwrap(),
            b"version=99"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
