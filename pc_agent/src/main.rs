use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};
use virtual_gamepad_agent::{application::Application, config::Config, terminal};

fn main() -> Result<(), virtual_gamepad_agent::application::Error> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let after_setup = args == ["--tray-after-setup"];
    let tray = args == ["--tray"] || after_setup;
    let config = if tray {
        Config::default()
    } else {
        Config::from_args(args)?
    };
    #[cfg(windows)]
    let _runtime_guard = (|| {
        if after_setup {
            virtual_gamepad_agent::windows_tray::detach_console();
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
        loop {
            match virtual_gamepad_agent::windows_tray::RuntimeGuard::acquire() {
                Err(error)
                    if after_setup
                        && matches!(
                            error.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::PermissionDenied
                        )
                        && std::time::Instant::now() < deadline =>
                {
                    std::thread::sleep(std::time::Duration::from_millis(200))
                }
                result => return result,
            }
        }
    })()
    .inspect_err(|error| {
        if tray {
            virtual_gamepad_agent::windows_tray::report_error(&error.to_string());
        }
    })?;
    let (log_writer, _log_guard) = if tray {
        let root = std::path::PathBuf::from(
            std::env::var_os("LOCALAPPDATA").ok_or("LocalAppData unavailable")?,
        )
        .join("VirtualGamepad");
        virtual_gamepad_agent::telemetry::file_writer(&root)?
    } else {
        virtual_gamepad_agent::telemetry::terminal_writer()?
    };
    virtual_gamepad_agent::telemetry::set_debug(config.debug);
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(log_writer)
                .without_time()
                .with_target(false)
                .with_ansi(!tray)
                .with_filter(tracing_subscriber::filter::filter_fn(move |metadata| {
                    virtual_gamepad_agent::telemetry::terminal_event_enabled(
                        metadata.target(),
                        *metadata.level(),
                        virtual_gamepad_agent::telemetry::debug_enabled(),
                    )
                })),
        )
        .init();
    #[cfg(windows)]
    if tray {
        virtual_gamepad_agent::windows_tray::detach_console();
        let result = virtual_gamepad_agent::windows_tray::run();
        if let Err(e) = &result {
            virtual_gamepad_agent::windows_tray::report_error(&e.to_string());
        }
        return result;
    }
    tokio::runtime::Runtime::new()?.block_on(run_terminal(config))
}
async fn run_terminal(config: Config) -> Result<(), virtual_gamepad_agent::application::Error> {
    let (application, handle, codes) = Application::prepare(config).await?;
    terminal::start(handle.clone(), codes)?;
    #[cfg(windows)]
    {
        application
            .run(
                virtual_gamepad_agent::virtual_gamepad::vigem::VigemBackend::default(),
                tokio::signal::ctrl_c(),
            )
            .await?;
        tracing::info!("Clean shutdown complete");
        Ok(())
    }
    #[cfg(not(windows))]
    {
        drop((application, handle));
        Err("The runtime backend requires Windows".into())
    }
}
