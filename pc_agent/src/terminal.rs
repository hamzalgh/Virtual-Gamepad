//! Terminal presentation only. These threads own no socket, backend or storage.
use crate::{
    application::{ApplicationHandle, CodeReceiver, Lifecycle, ServiceStatus},
    paired_session::PairingCommand,
};
use std::io::BufRead;

pub const HELP: &str =
    "Commands: pair | forget | replace | status | exit. Ctrl+C exits. Trusted private LAN only.";

pub fn summary(status: &ServiceStatus) -> String {
    let addresses = if status.bound.ip().is_loopback() {
        format!("{} (local testing only)", status.bound.ip())
    } else if status.addresses.is_empty() {
        "unavailable. Check your private Wi-Fi connection.".into()
    } else {
        status
            .addresses
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "{:?} | Virtual Gamepad {} | UDP port {}\nPC IPv4: {}",
        status.lifecycle,
        env!("CARGO_PKG_VERSION"),
        status.bound.port(),
        addresses
    )
}

/// One startup notice; immutable service changes otherwise stay silent.
async fn initial_status(
    mut status: tokio::sync::watch::Receiver<ServiceStatus>,
) -> Option<ServiceStatus> {
    loop {
        let snapshot = status.borrow_and_update().clone();
        if snapshot.lifecycle == Lifecycle::Running {
            return Some(snapshot);
        }
        if matches!(snapshot.lifecycle, Lifecycle::Stopped | Lifecycle::Failed)
            || status.changed().await.is_err()
        {
            return None;
        }
    }
}

/// Called before run so thread creation failures cannot strand active output.
pub fn start(handle: ApplicationHandle, codes: CodeReceiver) -> std::io::Result<()> {
    let status = handle.subscribe();
    let runtime = tokio::runtime::Handle::current();
    std::thread::Builder::new()
        .name("pairing-code-display".into())
        .spawn(move || {
            let Some(snapshot) = runtime.block_on(initial_status(status)) else {
                return;
            };
            println!("{}", summary(&snapshot));
            if snapshot.development {
                println!("UNSAFE development bypass: pairing and discovery are disabled.");
            } else {
                println!("Waiting for a phone. Every connection requires a fresh six digit code.");
                println!(
                    "Type pair, then Enter, to open a code window (ends any current session)."
                );
            }
            println!("{HELP}");
            for (code, expires) in codes {
                let remaining = expires
                    .saturating_duration_since(tokio::time::Instant::now())
                    .as_secs();
                if remaining > 0 {
                    println!("Pairing code: {code:06} ({remaining} seconds remaining)");
                }
            }
        })?;
    std::thread::Builder::new().name("terminal-commands".into()).spawn(move || {
        // EOF ends this reader. It never ends the service while main owns a handle.
        for line in std::io::stdin().lock().lines() {
            let Ok(line) = line else { break; };
            match line.trim() {
                "exit" => { handle.exit(); break; }
                "status" => {
                    let status = handle.status();
                    println!("{}\nBackend: ready={} attached={} timed_out={} fault={} (driver/firewall preflight not verified)", summary(&status), status.backend.ready, status.backend.attached, status.backend.timed_out, status.backend.fault);
                    if let Some(diagnostic) = status.backend.diagnostic {
                        println!("{}", diagnostic.message());
                    }
                }
                command => {
                    let command = match command {
                        "pair" => PairingCommand::Open, "forget" => PairingCommand::Forget, "replace" => PairingCommand::Replace,
                        _ => { println!("{HELP}"); continue; }
                    };
                    if handle.command(command).is_err() { println!("Command unavailable; retry after pending work finishes."); }
                }
            }
        }
    })?;
    Ok(())
}
