# Spec 0004 implementation self review

Date: 2026-09-06

Result: Spec 0004 accepted for the tested API 34 / Windows 10 development path after the final user evidence below. This is the implementing agent's own review; no other model or subagent was used. No commit, push or later spec was started.

Final user evidence: holding Android A and ending `virtual-gamepad-agent.exe` through Task Manager removed the controller from joy.cpl. Restarting the agent and explicitly reconnecting produced exactly one virtual Xbox controller. This closes the forced termination/recovery gate, alongside the already recorded Stardew Valley A result. Earlier pending statements in this report describe the evidence available at their respective stages. Exact physical timing, API 37, Windows 11 and broader device coverage remain unverified; the accepted strict 60 Hz sampling limitation is unchanged.

## Changes reviewed

- `pc_agent/src/virtual_gamepad.rs`, `virtual_gamepad/fake.rs`, `virtual_gamepad/vigem.rs` and `examples/vigem_probe.rs`: isolated A report, resource ownership, recording failure injection, initial neutral, independent neutral/detach attempts, sticky failure and isolated real driver probe. ViGEm types stay in the Windows adapter. No application unsafe block was added.
- `pc_agent/src/input.rs`: bounded current snapshot and session requests, separate output worker, A mapping, monotonic 250 ms valid input deadline, timeout neutral/detach and shutdown cleanup. Driver calls do not block the UDP receiver. Watched values are copied before backend work.
- `pc_agent/src/session.rs`, `network.rs`, `main.rs`: first development instance claim, readiness before session grant, source/session/freshness checks, rotation invalidation, disconnect clearing, counters and runtime shutdown composition. Other inputs are refused at admission without changing the v1 codec. The unsafe development route remains disabled by default and refused in release builds.
- Android `controller/ControllerState.kt`, `ui/gamepad/GamepadSurface.kt`, `app/MainActivity.kt` and manifest: immutable A state, stable pointer ownership, release/cancel/leave/reentry, focus/resize cancellation, visible Exit and system Back, landscape safe bounds, immersive mode and active-only awake flag.
- Android `network/StateSender.kt`, `HelloSession.kt`, `ConnectionState.kt`, `app/ConnectionViewModel.kt` and setup copy: ready-session transition, negotiated 60 Hz snapshots, wrapping sequence, no catch-up burst, off-main-thread I/O, local neutral before cleanup, one neutral plus three disconnect attempts, cleanup join before replacement and explicit reconnect after lifecycle loss.
- Rust `tests/one_button.rs`, backend/owner unit tests, Android `OneButtonTest.kt` and device test sources: domain, transport, timing, failure and UI cancellation coverage. Existing protocol and diagnostic tests are retained. Shared fixtures and normative protocol documentation are unchanged.
- Cargo pins, dependency evidence, governing execution policy and progress tracker document the implementation and remaining gates.

## Review findings resolved

The Android active-session timeout initially risked waiting on a still-running cadence child. Terminal paths now close the owner before leaving its coroutine scope. Fake transport tests cover both missing status and explicit timeout status. Cleanup joins cadence before sending neutral/disconnect, including sender socket failure.

Focus loss must discard pointer ownership as well as release the bit. The view model increments an input epoch and the root pointer scope restarts on that epoch or surface size change. Pure routing tests and compiled UI cancel tests cover the boundary; actual window focus and resizing are still manual tests.

Clippy findings about test module placement and collapsible conditions were corrected. Backend recovery copy points to the PC terminal, ViGEmBus device status and agent restart. No unresolved code blocker was found in the reviewed automated paths.

## Automated evidence

Commands ran from the relevant application directory on Windows 10 x64:

```text
cargo fmt --check
cargo check --offline --all-targets
cargo clippy --offline --all-targets --all-features -- -D warnings
cargo test --offline --all-targets
cargo build --offline --release
cargo test --offline --release --test development_trust --test one_button
```

All passed. Debug test count: 37 (12 library, 4 development trust, 3 one button integration, 8 protocol, 10 UDP). Release ran four development trust tests and the output constructor refusal assertion in the one button suite; two other one button tests return early in release and are not evidence of release input execution. The real UDP regression uses the recording fake and verifies held A, invalid traffic that cannot refresh the deadline, detach, fresh-session recovery and disconnect clearing. It is not physical driver evidence.

```text
.\gradlew.bat testDebugUnitTest lintDebug assembleDebug compileDebugAndroidTestKotlin --offline
```

Passed: 25 JVM tests, zero failures/errors; lint zero errors and 10 warnings; debug APK assembled; Android test sources compiled. Nine warnings are dependency update notices. One is the fixed orientation warning for newer Android behavior. The declared landscape policy follows this spec; this is not evidence that every Android form factor honors it. No Android Studio sync, connected instrumentation or new physical app run was performed by the agent.

`git diff --check` passed. Rust formatting/check/Clippy passed without diagnostics. All ViGEm client API use is isolated in the Windows adapter. There are no protocol fixture edits, installer files, new controls or production pairing changes.

## Driver and network evidence

Official setup 1.22.0 was downloaded only after explicit user authorization, with expected SHA-256 and valid publisher signature verified. The user installed it and reported driver 1.21.442.0 healthy. Windows inspection corroborated the signed driver, running service and PnP OK status on Windows 10 Pro 22H2 x64 build 19045.6466. The earlier driver report and progress tracker retain exact source/hash/license evidence.

The agent's isolated real API smoke passed. The user then confirmed the visual A behavior, Ctrl+C while A was pressed and clean relaunch. Both supplied observe-mode logs finish with `NEUTRAL and DETACH succeeded`. No reboot was requested. Android was closed, as intended for this isolated gate. That passes the prerequisite gate, not the full physical phone slice.

The current Wi-Fi IPv4 was rechecked as `[redacted PC private address]`. The debug executable has a program scoped Private inbound UDP 8765 Allow rule from LocalSubnet. No firewall rule was changed in this slice. Address and firewall facts are host observations, not portable defaults.

## Next manual gate

1. In Android Studio open `mobile_controller`, sync and run the updated debug app on the API 34 phone. Record sync/install result. Leave the isolated `vigem_probe` closed.
2. In a terminal at `pc_agent`, run `cargo run -- --debug --manual-trust-first-client`. Expect UDP listening on 8765 and the unsafe development warning. Use the same private Wi-Fi on both devices. If the PC address changes, use the current IPv4 from `ipconfig`.
3. On Android enter PC `[redacted PC private address]`, port `8765`, then Connect. Backend preparation can require another hello retry. Expect controller mode to open automatically with one A control, no pairing claim, safe landscape content and an Exit controller button.
4. Open `joy.cpl`, select the newly attached Xbox 360 controller and open Properties/Test. Expect all controls neutral. Press and hold Android A, then release; only the A indicator should change. Repeat short taps. Move the held finger outside the circle, then inside; expect release then press. Lift outside; expect no held input. A second finger must not take over an existing held pointer.
5. While holding A, separately test Exit controller, system Back, Home/background, screen lock and Wi-Fi off. Reconnect explicitly between cases. Expect local release on UI/lifecycle loss, no resumed sender merely from returning to the app, and PC neutral/detach on admitted disconnect or after 250 ms without valid input plus scheduling/driver latency. Record observed timing; a visual glance alone cannot prove a precise deadline. Focus loss such as a system overlay must cancel the old touch; a new touch is needed to press again.
6. With A held through the Android app, press Ctrl+C in the agent terminal. Expect clean shutdown and controller removal. Restart the same command, return to setup and Connect again. Expect one neutral controller and working A, with no stale extra device. Also force stop the Android application while A is held to exercise loss without a graceful phone disconnect; the PC timeout must neutralize/detach. Relaunch the app and explicitly reconnect.
7. With games closed, separately observe hard PC termination: identify `virtual-gamepad-agent.exe` in Task Manager and End task while Android holds A. Record whether the device disappears or remains stuck, then release the phone touch, restart the agent and reconnect. No graceful cleanup is promised for this case. A stuck device, duplicate controller or failed recovery blocks acceptance; preserve the observation before attempting recovery.
8. Only after the tester and recovery cases pass, try one named Windows game and record controller detection and A press/release. This proves only that game/host combination. Check enlarged text, system insets, immersive exit and awake flag clearing on the physical phone. Run compiled device tests separately if available; compilation is not execution. API 37 local-network permission execution remains a separate unverified platform gate.

## Remaining limits

### Later shutdown warning fix and game result

The supplied log places the warning after receiver `stopped`, followed immediately by clean shutdown, rather than during normal input. A deterministic regression reproduced the output owner reading a cleared latest slot while its session request still contained the old ID. The fallback grant deadline was already expired even though the most recent input was fresh. The fix treats explicit clearing after applied state as revocation: neutral/detach without a timeout flag. It does not relax the valid-input deadline or suppress real timeouts. The test failed before the fix and passes afterward, checking neutral/detach order and idempotent shutdown.

All Rust formatting/check/Clippy checks, 38 debug tests, release build and release trust checks passed after this fix. No Android runtime change was needed. Self review confirms new sessions reset previous output state, cleared sessions do not resume input, and stale snapshots still exercise the existing timeout tests. Physical retesting of this fix is pending.

The user clarified that the apparent reconnect was restarting only the PC command. Its log correctly refuses old-session packets until a new hello; the phone's separate three-second status timeout does not grant a new session. Earlier explicit reconnection is Exit controller then Connect. No automatic reconnect policy was added.

The user confirmed A works in Stardew Valley, with game version unspecified. The named game observation is now recorded, without general compatibility claims. Task Manager forced termination and recovery remain unverified; both supplied log runs finish with clean shutdown. The tracker and handoff reflect this remaining gate.

### Later user feedback and accepted sampling limit

After the implementation handoff, the user confirmed ordinary physical input in joy.cpl, background disconnect and integrated PC Ctrl+C disconnect. This supersedes the earlier report's pending status for those cases only. Forced PC termination/relaunch and a named game still have no evidence.

The user observed rapid double taps as one press in joy.cpl. A fake-time reproduction using the real router, sender and codec failed an initial assertion expecting two presses: a 6 ms release at 40 ms fell between sender samples at 34 and 50 ms. This identifies a possible source of the observed loss but does not measure physical phone or joy.cpl timing. The user explicitly chose to keep strict 60 Hz and accept/document this limitation. No runtime fix is claimed or made.

Self review checked two characterization tests: the short release is omitted and final release is transmitted; a 25 ms release separates two transmitted presses. All 27 JVM tests, lint (0 errors, 10 warnings), debug assembly and Android test compilation passed after these additions. `git diff --check` passed. The governing spec and Android AGENTS instruction now record the decision. The temporary next-session handoff for the remaining Spec 0005 gate was retired after the work moved on. No other model, commit or push was used.

### Original implementation limitations

The retired driver's synchronous kernel calls can stall and cannot be cancelled by Tokio; shutdown has no guaranteed bound under a stalled driver. Forced termination bypasses destructors and remains unobserved. No Windows 11, new physical Android control path, target game, installer or connected instrumentation verification is claimed. TalkBack gameplay operation, broader form factors and enlarged text remain unverified. Production pairing, discovery, other controls, tray, installer, multiple phones and keyboard/mouse are outside this change.
