# Spec 0005 implementation self review

Date: 2026-09-06. Reviewer: implementing agent only. No other model or subagent, commit or push was used. Initial working tree was clean; the accepted Spec 0004 code/evidence was retained.

Result: implementation and automated checks pass. Spec 0005 remains In Progress because physical timing, repetitions and actual phone/driver soak evidence are pending. Spec 0004 remains accepted, including forced termination/recovery, Stardew Valley A and the explicit strict 60 Hz sampling limitation.

## Changes reviewed

- `pc_agent/src/config.rs`: validated `InputTimeout`, default 250 ms and command option bounded to 100–1000 ms.
- `pc_agent/src/session.rs`: independent admission deadline and refusal of an expired repeated acknowledgement even if output health lags; common timeout in hello/status; transition status and rate window preservation. Wire accepted/rejected counters still wrap and loss saturates.
- `pc_agent/src/input.rs`: common configured output timeout, explicit phases, saturating update/fault/timeout counters, once per second output diagnostics, timeout check lateness and cleanup API duration. Latest state still has one slot. Driver API calls and backend types remain isolated as before.
- `pc_agent/src/network.rs`: per-category validation/admission counts, local estimated sequence loss, 5 ms transition checks plus periodic 2 Hz status, bounded 2 ms reply waits. Timer/cancellation branches precede receive traffic. No per-packet logging or input queue was added.
- `pc_agent/src/telemetry.rs`: fixed counters and a 64 event terminal channel. Each event is bounded to 2048 bytes. Full, oversized, failed or disconnected output drops diagnostics and increments a saturating loss count. Terminal writes run on their own thread. Final flush waits at most 100 ms after controller cleanup. A permanently blocked terminal may lose final logs but cannot hold input.
- `pc_agent/src/main.rs`: common timeout composition and coordinated cancellation when the output task returns an error or panics. Receiver destruction clears state and closes the socket; normal signal/socket errors still join output cleanup. The default and release trust refusal remain intact.
- Android `network/CadenceStats.kt` and `StateSender.kt`: interval minimum/mean/maximum, successful scheduled send total and skipped slot count. Immutable atomic cadence snapshots publish at most twice per second. The existing 60 Hz scheduler and accepted short tap behavior are unchanged; cleanup sends are excluded from measurements.
- Android `network/ConnectionState.kt` and `HelloSession.kt`: accepted sequence, counts, observed rate, wrap-safe echo age and backend/timeout flags; one second silence and low-rate degraded policy; bounded diagnostic UI publication; last report retained on silence timeout. Initial ready and terminal safety updates remain immediate. No automatic reconnect or session persistence was added.
- Android `app/MainActivity.kt`, `ui/gamepad/GamepadSurface.kt` and `ui/setup/ConnectionScreen.kt`: compact active status and an explicit Diagnostics dialog using existing Material3 styling. Opening the dialog cancels the held pointer and releases A. Detailed changing counters do not resize gameplay geometry. Terminal diagnostics appear on setup.
- Rust `tests/reliability.rs`, additions to `tests/one_button.rs` and module tests, Android `ReliabilityTest.kt` and `GamepadSurfaceTest.kt`: fault campaigns and regression checks described below. Device test compilation is not execution.
- Governing Spec 0005, package instructions, architecture/API notes, tracker and the [physical campaign](../handoffs/2026-09-06-spec-0005-manual-campaign.md): policy, evidence and remaining gates. No matching scope file exists; no later spec was advanced.

## Findings resolved

The deadline regression `expired_session_cannot_revive_while_output_health_lags` failed before the fix: after fake time advanced exactly 250 ms, admission accepted a newer state because output still reported ready. Admission now owns and checks the same monotonic deadline independently. The regression passes, and an expired repeated hello cannot advertise a ready session. Fresh hello recovery remains explicit and backend faults remain sticky.

Review also ensured transition reports do not reset the rate estimator more often than its measurement window, status floods cannot drive per-packet UI publication, detailed counters cannot resize the A surface, and terminal I/O cannot block timeout work. Formatting/compiler findings during implementation were corrected. No unresolved automated safety failure remains.

## Automated verification

From `pc_agent`, all commands passed:

```text
cargo fmt --check
cargo clippy --offline --all-targets --all-features -- -D warnings
cargo check --offline --all-targets
cargo test --offline --all-targets
cargo build --offline --release
cargo test --offline --release --test development_trust --test one_button --test reliability
```

Debug total: 52 passing tests, comprising 20 library, 2 binary composition, 4 development trust, 5 one button, 8 protocol, 3 reliability and 10 UDP tests. Important campaigns:

- The real loopback fault link covers selected drop, duplicate, bounded delay/reorder, sequence wrap, wrong source/session, CRC/reserved corruption, delayed consumer burst, silent loss and backend failure. Only the final release reaches a delayed consumer. No real driver is used.
- Fake owner time checks exact 100/250/1000 ms boundaries, idempotent cleanup and saturation. Existing invalid/stale/source/session storm tests preserve the latest timestamp and cannot extend timeout.
- Fifty simulated explicit reconnects produce exactly fifty attaches/detaches and reject old sessions. Thirty minutes of fake monotonic time processes 432,000 latest-value publications into 108,000 current releases with zero timeout or replay. This is not an actual wall-clock soak or a memory/CPU measurement.
- A 1,000 malformed packet test counts all invalid packets while asserting periodic logging. A full/disconnected logger test proves bounded storage and dropping behavior. Main composition tests close UDP on worker error/panic and join cleanup on signal failure.
- Existing shared protocol fixtures still pass byte exactly. No fixture, codec, dependency pin, lockfile, driver or firewall change was made.

The release run executes four development trust checks and the one button constructor refusal assertion. Other one button runtime cases return early in release, and the entire reliability helper is excluded there. They are not evidence of release input operation. Direct execution of `target/release/virtual-gamepad-agent.exe --manual-trust-first-client` also refused with expected exit code 1.

From `mobile_controller`, passed:

```text
.\gradlew.bat testDebugUnitTest lintDebug assembleDebug compileDebugAndroidTestKotlin --offline
```

Final Android run: BUILD SUCCESSFUL in 43 seconds. XML reports 32 JVM tests, zero failures/errors. The five new tests cover measured intervals/skips/cancellation, diagnostic clock wrap/thresholds, 20 simulated lifecycle cycles with exactly one cleanup, permission failure cleanup and publication under a status flood. Existing rapid tap characterization tests pass unchanged. Lint has zero errors and ten warnings. Debug APK assembled and all device test sources compiled, including diagnostics cancellation. No Android Studio sync, APK installation, connected instrumentation or physical UI execution was performed by the agent.

The first sandbox Gradle attempt could not use its cached distribution and failed network access. The approved cache-access invocation succeeded; no dependency version was changed. Read-only CIM queries were denied by the environment. `ipconfig` succeeded and confirmed Wi-Fi IPv4 `[redacted PC private address]`. The historical driver/Windows observations remain the accepted baseline, not a newly claimed device inspection. `git diff --check` passed after removing one introduced Markdown trailing space.

## Invariant coverage and remaining evidence

- Exact valid admitted fresh input only, latest boundedness, independent deadline and no invalid refresh: protocol/UDP/one button/reliability suites.
- Default/bounded timeout and one neutral/detach transition: configuration and owner fake-time tests.
- Local neutral before sender cleanup, error paths and explicit fresh hello: Android one button/connection/reliability tests; physical lifecycle/lock/network/awake flag cases remain in the campaign.
- Logging cannot hold input or grow a queue: fixed event queue, blocked writer and malformed flood tests. Actual terminal/CPU/physical timing still needs campaign observations.
- Graceful cleanup and ordinary task failure: backend/owner tests plus binary composition tests. The corrected Ctrl+C warning path still needs the requested physical log check.

Requested user follow-up: run the new Android build, inspect changing Diagnostics values, and retest integrated Ctrl+C without the prior false warning. The full campaign requires measured held A loss cases, 20 physical background/foreground cycles, 50 physical reconnects, 20 Wi-Fi cycles and 30 actual minutes on the phone/driver path with CPU/memory/cadence/age/log measurements. Pending evidence is not marked passed.

## Known limitations

Synchronous retired ViGEm kernel calls cannot be cancelled by Tokio, so a driver stall has no guaranteed shutdown deadline. The terminal worker is deliberately not joined without a bound if its output blocks. Echo age and timeout cleanup API timings are diagnostics, not physical latency certification. Wi-Fi loss uses transport errors or missing status rather than an immediate connectivity callback. Strict 60 Hz may omit very short presses/releases. The diagnostics dialog, enlarged text, actual cadence and awake policy need device testing. Windows 11, API 37 permission execution, connected instrumentation, installer, broader layout and new game tests remain unverified. No driver reinstall, Spec 0006 controls, pairing/discovery, tray, multiple phones or keyboard/mouse support was introduced.
