//! Isolated driver gate. No socket, phone session or release input admission.
#[cfg(windows)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::Duration;
    use virtual_gamepad_agent::virtual_gamepad::{Gamepad, XboxReport, vigem::VigemBackend};

    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--help"] || args.is_empty() {
        println!("usage: cargo run --example vigem_probe -- --smoke|--observe");
        println!(
            "Debug only. Creates one controller. Observe: 10 seconds neutral, then A alternates every 3 seconds for 60 seconds. Ctrl+C cleans up."
        );
        return Ok(());
    }
    if !cfg!(debug_assertions)
        || args.len() != 1
        || !matches!(args[0].as_str(), "--smoke" | "--observe")
    {
        return Err(
            "Use a debug build with exactly --smoke or --observe; release feeding is disabled."
                .into(),
        );
    }
    let observe = args[0] == "--observe";
    println!(
        "Isolated ViGEm probe, no UDP. Driver calls are synchronous; a stalled driver call cannot be cancelled. Close games before testing."
    );
    let mut gamepad = Gamepad::new(VigemBackend::default());
    let result: Result<(), Box<dyn std::error::Error>> = async {
        gamepad.attach()?;
        println!(
            "ATTACHED and initial NEUTRAL submitted. Backend {:?}.",
            gamepad.health()
        );
        let sequence = async {
            tokio::time::sleep(if observe {
                Duration::from_secs(10)
            } else {
                Duration::from_millis(250)
            })
            .await;
            for cycle in 1..=if observe { 10 } else { 2 } {
                gamepad.submit(XboxReport {
                    buttons: 0x1000,
                    ..XboxReport::NEUTRAL
                })?;
                println!("Cycle {cycle}: A PRESSED, all other fields neutral.");
                tokio::time::sleep(if observe {
                    Duration::from_secs(3)
                } else {
                    Duration::from_millis(250)
                })
                .await;
                gamepad.submit(XboxReport::NEUTRAL)?;
                println!("Cycle {cycle}: A RELEASED, all fields neutral.");
                tokio::time::sleep(if observe {
                    Duration::from_secs(3)
                } else {
                    Duration::from_millis(250)
                })
                .await;
            }
            Ok::<(), Box<dyn std::error::Error>>(())
        };
        tokio::select! {
            biased;
            signal = tokio::signal::ctrl_c() => {
                signal?;
                println!("Ctrl+C received.");
                Ok(())
            }
            result = sequence => result,
        }
    }
    .await;
    gamepad.shutdown();
    for error in gamepad.cleanup_errors() {
        eprintln!("Cleanup error: {error}");
    }
    result?;
    if !gamepad.cleanup_errors().is_empty() {
        return Err("Backend cleanup failed; stop the viability gate.".into());
    }
    println!(
        "NEUTRAL and DETACH succeeded. Probe complete. Tester visibility and forced termination require separate evidence."
    );
    Ok(())
}

#[cfg(not(windows))]
fn main() {
    eprintln!("The ViGEm viability probe requires Windows.");
    std::process::exit(1);
}
