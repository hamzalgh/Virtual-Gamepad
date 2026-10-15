# Spec 0004 — One-Button Vertical Slice and ViGEm Gate

Status: **Accepted**
Milestone: M1 / hard decision gate G1  
Depends on: 0003  
Primary changes: `mobile_controller/`, `pc_agent/`

## Outcome

Pressing and releasing one Android `A` control changes the A button on one virtual Xbox 360 controller in Windows, with clean neutral/detach/relaunch behavior. This proves the riskiest external dependency before broad controller UI work.

## Driver prerequisite gate

Before integrating:

1. Obtain the exact ViGEmBus prerequisite only from the official project/release path.
2. Record driver version, release checksum/source, license/redistribution note, install result, reboot requirement, and Windows build.
3. Confirm Device Manager/service health as documented upstream.
4. Pin/review the `vigem-client` Rust dependency and its lifecycle warnings.

Do not silently download/repackage/install a driver. The user must knowingly perform/admin-authorize the prerequisite.

If the minimal upstream-style attach/update test fails, stop dependent feature work and follow Gate G1 in `context/build-plan.md`.

## Android slice

- Add a minimal landscape `GamepadSurface` with one large labelled `A` button inside safe bounds.
- Use the future root pointer-router boundary even though only one control exists; track stable pointer ID and handle down/up/cancel.
- Update immutable local `ControllerState` immediately.
- Start the negotiated 60 Hz full-state sender only when the session is ready and controller mode is active.
- Increment wrapping sequence for every state datagram and include current provisional session.
- On A release/cancel, the next state is released.
- On exit/pause/stop/socket failure, set local neutral, send one best-effort neutral plus documented disconnect attempts, then stop.
- Local visual feedback is immediate and independent of PC status.

## Rust slice

- Define `XboxReport` and `VirtualGamepadBackend` independent of ViGEm.
- Implement a recording fake used by all state/timeout tests.
- Implement Windows-only ViGEm adapter with attach, submit, neutral, health, and detach.
- Contain all ViGEm/platform types and any unsafe boundary in that module.
- Admit only current manual-trust session/source/fresh sequence.
- Map protocol A bit to Xbox A; all other report fields remain neutral.
- Apply newest state through the bounded/latest-value path.
- On Ctrl+C and all ordinary errors, submit neutral and detach.
- Surface actionable “driver missing/unavailable” health to hello/status and terminal output.

## Execution policy for this slice

- The user confirmed the isolated Windows tester gate: A press/release, Ctrl+C while A was pressed, detach and relaunch passed with driver 1.21.442.0. Setup 1.22.0 requested no reboot. This authorizes the dependent Android/session implementation, not acceptance of the full slice.
- Connect transitions directly into controller mode after acknowledgement of a ready source bound session. Until then Android sends hello only. The acknowledged rate must be 60 Hz for this build; another rate is a recoverable compatibility error.
- On 2026-09-06 the user explicitly chose to retain strict 60 Hz after observing rapid double taps as one press in joy.cpl. A press or release entirely between sender samples can be omitted; two presses separated by such a release can merge. This is an accepted sampling limitation, not guaranteed delivery of every tap. Do not add immediate change packets, delayed releases, synthetic pulses or historical input replay. A deterministic 6 ms release reproduction and a 25 ms release comparison document the boundary; these are simulated timings, not measured phone or tester timings.
- The PC first prepares a neutral target through its separate output owner. Hello retries can then obtain the provisional session; preparing a target does not itself grant input admission. Pending preparation expires after five seconds. Once a session is granted, the 250 ms deadline starts and only fresh admitted state extends it.
- Repeated hello with the same nonce/source can repeat the same acknowledgement without refreshing the input deadline. A new hello after a granted session, or a source change for the claimed instance, invalidates the old session and prepares a fresh one. The first instance claim lasts until process exit, including after disconnect. Other instances cannot take it over.
- Timeout and admitted disconnect neutralize and detach. Timed out sessions refuse further input until a fresh hello; backend faults refuse reattachment until process restart. Status retains the old session identity long enough to report timeout/fault truthfully. Manual development trust is not labelled paired.
- Valid v1 fields for other controls remain part of the unchanged normative codec and fixtures. This slice's admission policy additionally refuses any nonzero non-A button, axis or trigger, without advancing sequence/deadline. This is not a protocol reserved-bit change.
- Android cleanup first makes local state neutral and stops cadence, then attempts one neutral packet with the next wrapping sequence and three disconnects 5 ms apart on its I/O owner. Cleanup never waits on the main lifecycle callback; the PC deadline remains authoritative. A replacement connection waits for the preceding owner's bounded cleanup before opening another socket.
- The output owner consumes bounded latest state and session requests on one blocking worker. Driver operations never execute in the receiver. Already executing kernel calls cannot be cancelled; the pinned driver's stall and forced termination limits remain documented.

## Automated tests

- Android A down/up/cancel produces correct full-state bit across sequences.
- Android pause/session close produces neutral/disconnect and stops cadence using fake transport/time.
- Rust mapping produces A only and neutral releases it.
- Wrong session/source, duplicate/stale sequence, bad packet, and non-A reserved state never press A.
- Backend attach/update/neutral/detach order with recording fake.
- Backend update failure triggers neutral/detach-safe state and error status.
- Graceful shutdown is idempotent.

## Manual Windows/phone test

1. Start agent with driver present and manual trust enabled.
2. Connect physical phone over Wi-Fi and enter controller mode.
3. Open Windows `joy.cpl` (or approved controller tester) and identify the new Xbox controller.
4. Press/hold/release A and observe matching sampled state. Try rapid taps and record the accepted limitation: transitions entirely between 60 Hz samples may be omitted or merge presses. This test does not require guaranteed delivery of every tap.
5. Hold A, then background Android, turn off Wi-Fi, stop the agent normally, and repeat with socket interruption. A must return to released within the current safety path.
6. Exit/relaunch the agent and reconnect; confirm no stale extra controller/input and A still works.
7. Move to one representative game only after the controller tester passes; record whether the game sees the device, without claiming general compatibility.

## Hard exit criteria

- Exact driver/client versions and official source are recorded.
- Virtual Xbox device appears on Windows 10 x64.
- Physical Android A press and release traverse the real UDP/driver path.
- Normal lifecycle/shutdown leaves A released and device cleanly detached.
- Relaunch recovery passes.
- Fake-backend and standard Rust/Android checks pass.
- Any forced-kill behavior is observed/documented.

If any hard criterion fails, mark `[!]`, preserve the minimal reproduction, and do not start Specs 0006–0008. Reliability work that helps diagnose the gate may continue in isolation.

## Out of scope

- Other controls, polished layout, real pairing, discovery, tray, installer, multiple phones, and keyboard/mouse.
