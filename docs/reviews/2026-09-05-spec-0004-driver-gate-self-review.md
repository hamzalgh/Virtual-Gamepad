# Spec 0004 driver gate self review

Date: 2026-09-05
Scope: backend abstraction, recording fake, Windows adapter and isolated probe only.
Reviewer: the implementing agent, without other models or subagents, as requested.
Result: automated checks and the short real driver probe pass. The visual tester gate remains pending; this is not acceptance of Spec 0004.

## Changes reviewed

- `pc_agent/src/virtual_gamepad.rs`: A only domain report, typed backend errors/health, backend trait and single resource owner. Shutdown attempts neutral and detach independently, remains idempotent, retains cleanup errors and prevents reuse after a fault.
- `pc_agent/src/virtual_gamepad/fake.rs`: records calls and injects operation failures for tests. It is not wired into runtime input or an unbounded receive queue.
- `pc_agent/src/virtual_gamepad/vigem.rs`: owns the client and target, contains all ViGEm types, submits neutral values for every field except A, preserves partial attach ownership and retries cleanup on adapter drop. No application unsafe code.
- `pc_agent/examples/vigem_probe.rs`: explicit debug only smoke/observe modes, finite cycles, explicit cleanup on completion, signal or ordinary error. It binds no UDP socket. No normal agent behavior changed.
- Windows dependency and lockfile pin, dependency/source notes, spec status and tracker evidence.

## Review findings and corrections

- Verified attach ownership precedes plugin/wait_ready calls, so failed readiness cannot discard the target without cleanup.
- Verified neutral failure does not skip detach, and original update failure remains returned while cleanup failures are separately available. Added a held A shutdown regression with simultaneous neutral/detach failures; it confirms both attempts and refusal of subsequent input/reattach.
- Verified normal attach begins with neutral and repeated shutdown adds no backend calls. Tests also cover update failure, partial attach failure, initial neutral failure, Drop cleanup and normal reattach.
- Verified the adapter conversion produces Xbox A bit `0x1000`; all axes, triggers and other buttons are zero. No protocol bit interpretation was added at this gate.
- Verified both the release probe and release agent reject their development feeding arguments with exit code 1. Existing strict protocol, source/session denial, shared fixture, latest value and UDP shutdown tests remain passing.
- Reviewed pinned client ownership and blocking driver calls. They cannot be cancelled by a Tokio timeout, and upstream event creation can assert in a debug build on resource failure. Forced termination can skip cleanup. These are documented upstream dependency limits, not silently declared safe by the wrapper.

## Verification evidence

From `pc_agent/`, passed `cargo fmt --check`, `cargo clippy --offline --all-targets --all-features -- -D warnings`, `cargo check --offline --all-targets`, and `cargo test --offline --all-targets` (30 tests, including six new backend tests).

Also passed `cargo build --offline --release`, `cargo test --offline --release --test development_trust` (4 tests), and `cargo build --offline --release --example vigem_probe`. Executing the release probe with `--smoke` and the release agent with `--manual-trust-first-client` produced the expected refusal and exit code 1.

`cargo run --offline --example vigem_probe -- --smoke` ran against the installed driver without a sandbox escalation. It returned success for attach, initial neutral, two A press/release cycles, final neutral and detach, then exited with code 0. No visual tester or XInput readback was used in this run, so successful submission is not recorded as observed button state.

A second `--observe` run attached successfully, but its tool session became unavailable before Ctrl+C could be sent. A subsequent process/device inventory found no remaining probe process or present Xbox controller. The termination cause and cleanup path are not established, so neither a Ctrl+C pass nor a forced termination pass is claimed.

From `mobile_controller/`, `gradlew.bat testDebugUnitTest lintDebug assembleDebug compileDebugAndroidTestKotlin --offline` passed in 53 seconds using approved access to existing Gradle caches. The unchanged unit tests and device test compilation were up to date. Existing XML records 16 JVM tests with zero failures/errors. Fresh lint reports zero errors and nine warnings. This was not Android Studio sync or a device test run.

`git diff --check` passed. The initial Cargo fetch failed DNS; a retry with multiplexing disabled fetched the exact dependency successfully. The initial sandbox Gradle invocation could not access the wrapper cache; the approved invocation passed. Neither environment issue is a driver failure.

## Pending gate and manual procedure

From a normal terminal in `pc_agent/`, run `cargo run --example vigem_probe -- --observe`. Open `joy.cpl`, select the new Xbox controller, then Properties and Test. Expect ten seconds neutral, followed by A pressed for three seconds and released for three seconds, repeated ten times. Other controls stay neutral. A is usually button 1 in the tester.

Press Ctrl+C while the terminal shows A PRESSED. Expect neutral/detach success and disappearance of that controller. Run the probe again; expect one controller and matching A behavior, without an extra stale device. Record installer reboot requirement, which the user has not yet supplied.

Do not conflate these checks with the remaining Spec 0004 work: provisional session/source admission, 60 Hz Android sender/A surface, timeout/lifecycle integration and fake clock safety tests have not been built. Physical phone input, Wi-Fi loss neutralization, forced termination, Windows 11, API 37 permission and game behavior remain unverified. No commit or push occurred.
