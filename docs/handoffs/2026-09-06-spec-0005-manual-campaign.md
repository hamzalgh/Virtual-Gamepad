# Spec 0005 physical reliability campaign

Spec 0004 remains accepted. This campaign verifies the new diagnostics and repeated reliability cases. It does not repeat the old prerequisite gate. Spec 0005 remains In Progress until its physical evidence is recorded.

## Start the updated applications

1. In Android Studio open `mobile_controller`, sync, and Run the debug app on the API 34 phone. Record sync, install and launch results. The new Diagnostics button opens detailed statistics and releases A. Closing it requires a new touch to press A.
2. From `pc_agent`, run `cargo run --offline -- --debug --manual-trust-first-client`. Keep the default 250 ms timeout for acceptance. The optional `--input-timeout-ms` accepts only 100 through 1000 and is advertised consistently to the phone. Do not run the isolated probe at the same time.
3. PC Wi-Fi IPv4 was rechecked with `ipconfig` as `[redacted PC private address]` during implementation. Recheck if the network changes. Connect Android to that address, port `8765`, using the existing Private network and program scoped UDP rule. No firewall change or driver installation is needed.
4. Open `joy.cpl` and the Xbox controller Test tab. Confirm one neutral controller, then hold and release Android A. Only A should change. Very short taps/releases between strict 60 Hz samples may still be missed, as explicitly accepted.
5. Open Diagnostics. Observe accepted sequence, accepted/rejected/lost counts, observed/negotiated rate, echo age, backend/attached/timeout state, successful scheduled send count, skipped slots, and the most recent interval minimum/mean/maximum. Zero interval data during the first half second is an unmeasured window. Close restores controller interaction. Confirm the button and dialog fit the physical landscape screen and enlarged text setting.

## Interpret the measurements

- Echo age uses wrapping phone monotonic milliseconds. It includes the age of the last accepted state and status scheduling, so it is not pure network RTT or measured touch latency.
- Cadence records successful send completion intervals, excludes neutral/disconnect cleanup, and publishes at most twice per second. Counters are cumulative per sender; interval statistics cover the most recently completed window of at least 500 ms. Skipped slots are deliberately not replayed.
- A low observed rate below 45 Hz, no accepted state, a PC degraded flag, or one second without admitted status produces Degraded. Three seconds without status ends the phone session. PC timeout/backend failure status ends it sooner. Reconnect is always explicit.
- PC `UDP diagnostics` lists accepted/stale/refused counts, source/session/unsupported rejections, each codec error category, estimated sequence loss and log drops. `Output diagnostics` lists phase, successful updates, fault transitions and timeout transitions. Local totals saturate. Wire accepted/rejected counts wrap and wire loss saturates.
- Timeout events include `deadline_lateness_ms` and `cleanup_ms`. These measure the output check lateness and synchronous cleanup API duration. They do not prove when a physical tester or game displayed release. Whole millisecond measurements round down. Add independent visual evidence for the physical gate.
- At ordinary load, periodic logging is at most one UDP and one output record per second, plus finite state transition messages. The terminal writer holds at most 64 events of 2048 bytes, drops full/error/oversized events and reports drops later when possible. No automatic growing log file is created. A blocked console may lose final records; input safety does not wait for it.

## Held A loss matrix

Run each case separately, reconnecting explicitly between cases. Record the action, release/removal result, PC timeout count change, phone phase and recovery result.

1. Exit controller and system Back. Expect local release, best effort neutral/disconnect, then PC neutral/detach.
2. Home/background, screen lock, then Android force stop. Expect no resumed sender merely from returning to the app. Force stop must be safe without disconnect delivery.
3. Disable Wi-Fi, then restore it. Separately interrupt the router briefly when doing so will not disturb other work. Expect PC neutral/detach after valid input stops. Android detects unavailable transport through socket errors or missing status; it does not promise an immediate network callback. Returning Wi-Fi does not reconnect automatically.
4. Ctrl+C in the integrated PC terminal while A is held. Expect controller removal and clean shutdown, without the former false timeout warning caused by clearing latest state. Restart the command, Exit controller if the phone still shows the old session, then Connect. Expect exactly one working controller.
5. Close/terminate the agent as an additional recovery cycle, with games closed. The original forced termination prerequisite already passed. Record this new campaign result separately; forced termination cannot run graceful cleanup.
6. Backend failure is safely injected in the automated fake backend campaign. Do not uninstall/disable the driver or provoke a kernel fault merely to get a physical error. If a real backend failure occurs naturally, retain its error and recovery evidence. Faulted input remains refused until PC process restart.
7. Leave controller mode and wait for the normal system display timeout. The phone must no longer be held awake by controller mode. Permission revocation must result in a closed safe session; API 37 execution requires an appropriate device and remains a separate platform gate.

For the 250 ms target, record a high frame rate video or an instrumented observation of traffic cutoff and tester release, including frame rate and uncertainty. A glance at `joy.cpl` cannot establish an exact duration. Compare the evidence with the last valid input cutoff, configured timeout, logged deadline lateness and cleanup duration. A synchronous kernel call can stall indefinitely and cannot be cancelled by Tokio; report that limitation rather than marking timing passed.

## Repeated campaigns and soak

- Run at least 20 background/foreground cycles, each with held A before leaving, explicit Connect afterward and a new press/release. Record every failure, not only the final success.
- Run 50 explicit connect/disconnect/reconnect cycles and 20 Wi-Fi off/on cycles, as required by `context/testing-and-release.md`. Check that exactly one target appears after each reconnect and that no held input or old session survives.
- Run 30 actual minutes on the phone/Windows driver path, with periodic A presses/releases. At 0, 5, 10, 15, 20, 25 and 30 minutes record PC Task Manager memory and CPU, Android Studio profiler memory/CPU if available, phone cadence/echo-age/rate/counters, PC output counters and any log drops. Record warmup separately. Expect no persistent memory growth, backlog, stuck A, unexpected timeout or uncontrolled logging. Document environmental packet loss rather than assuming Wi-Fi is perfect.
- Keep terminal scrollback bounded. If capturing console output, capture only the finite campaign and record its duration/size. Do not introduce an unattended logging service.

## Repeatable automated fault helper

From `pc_agent`, run `cargo test --offline --test reliability`. It binds ephemeral loopback sockets only and uses a fake backend. The fixed 16 slot delay window, dropped datagram, duplicate/reorder/wrap, corrupt CRC/reserved fields, alternate source/session, delayed consumer burst, silent loss, transition status and backend failure cases are named in the test file. It is excluded from release and cannot be exposed as a service. It does not feed a real virtual controller.

`cargo test --offline --all-targets` also runs 50 simulated explicit reconnects and 30 minutes of simulated monotonic input. Android JVM tests run 20 simulated lifecycle cycles. These are regression evidence, not the physical repetition or soak results above.

## Evidence to return

Record phone model/API, Windows version, application build/install result, network band, scenario/cycle counts, measured release timing with uncertainty, interval/rate/age samples, memory/CPU samples, timeout/failure/log-drop totals and recovery result. Mark anything not performed as unverified. API 37, Windows 11, connected instrumentation and broader physical layout/game coverage are not inferred from API 34 / Windows 10 results. No Spec 0006 work begins here.
