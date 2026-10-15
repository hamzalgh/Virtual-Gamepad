# Build Plan

## Planning principle

Build in end-to-end, acceptance-testable slices. Each slice either moves a real controller signal farther through the system or hardens a proven path. Discovery, multiple phones, layout customization, tray polish, and keyboard/mouse compatibility cannot outrun the reliable single-controller core.

User-approved sequencing exception (2026-09-08): continue to Spec 0006 using the implemented Spec 0005 safety foundation. Finish the outstanding physical reliability campaigns and delivery-stall investigation at Spec 0007's existing checkpoint, preserving automated safety checks and focused physical tests for each new control. This is development progression, not acceptance of unperformed tests or permission to bypass the later G2/release gates.

## Dependency graph

```text
Foundation
  ├─ Protocol v1 + Rust UDP
  │    └─ Android hello/settings
  │          └─ A-button vertical slice + ViGEm gate
  │                └─ Reliability/safety
  │                      ├─ Left stick + multi-touch
  │                      │    └─ Full controller
  │                      └─ Adaptive controller shell/settings
  │                               └─ Discovery + pairing
  │                                      └─ Tray + packaging (0010)
  │                                             └─ Settings + layout customization (0010a)
  │                                                    └─ Multiple controllers (0011)
  │                                                           └─ Keyboard/mouse mode
  └──────────────────────────────────────────────────└─ Final certification
```

Specs are ordered by dependency under `docs/specs/`. The user inserted Spec 0010a after 0010 and before 0011 on 2026-09-11, preserving existing spec numbers.

## 10-day focused execution plan

The 10-day plan is an aggressive AI-assisted target for a **reliable single-controller development build**, not a promise of the final multi-controller, discovery, tray, installer, and keyboard/mouse product. It assumes prompt access to the Windows 10 PC, a physical Android phone, Android Studio, driver installation permission, and short manual test feedback each day.

### Day 1 — Foundation and machine readiness

- Scaffold independent Android and Rust projects.
- Pin toolchains and establish formatter/lint/test commands.
- Add a Rust fake virtual backend and basic app health output.
- Install/detect the virtual-controller prerequisite using the official source.
- Record actual phone Android version, screen characteristics, PC local IP, and router client-isolation result.

Gate: both projects compile; Windows prerequisite state is known.

### Day 2 — Protocol v1 and Rust receiver

- Implement the 40-byte protocol types/CRC and exhaustive Rust parser tests.
- Add shared golden fixtures.
- Build the UDP listener, structured rejection reasons, and latest-value state holder.
- Use a local packet sender/test to prove valid and malformed behavior without Android.

Gate: Rust passes codec, invalid-input, sequence wrap, and UDP smoke tests.

### Day 3 — Android setup, permissions, and handshake

- Implement address/port validation and DataStore persistence.
- Implement v1 Kotlin codec against the same fixtures.
- Add Android local-network/internet permissions and clear permission UX.
- Send hello and display acknowledgement/backend state.

Gate: physical phone and PC exchange valid hello/status over the real 2.4 GHz LAN.

### Day 4 — First real Xbox input and backend viability

- Implement the Android A control and current-state sender.
- Complete the isolated ViGEm adapter.
- Map A press/release and verify in `joy.cpl` or a trusted controller tester.
- Exercise graceful detach and agent relaunch.

Hard gate: if the virtual Xbox device cannot be installed, attached, updated, neutralized, detached, and rediscovered reliably, stop feature expansion and decide the backend path.

### Day 5 — Reliability foundation

- Enforce full-state 60 Hz cadence and latest-state-only application.
- Add source/session admission in temporary manual-development trust mode.
- Implement 250 ms timeout neutralization, lifecycle neutral/disconnect, and backend-fault handling.
- Add loss/reorder/stale/malformed test cases and local counters.

Gate: no held A input survives timeout, app backgrounding, Wi-Fi loss, or normal shutdown.

### Day 6 — Left stick and coordinated multi-touch

- Implement the root pointer router, fixed left stick, radial dead zone, sensitivity transform, and Y inversion.
- Add pure geometry/encoding tests.
- Verify simultaneous left-stick movement plus A press/release with separate fingers.
- Verify pointer cancellation and off-control movement.

Gate: stick centers correctly and no pointer stealing/stuck state occurs.

### Day 7 — Full controller state and mapping

- Add right stick, L3/R3, D-pad, A/B/X/Y, LB/RB, Back, Start, and analog LT/RT.
- Add button-bit and Xbox-report mapping tests.
- Test representative multi-finger combinations and D-pad diagonals.

Gate: every planned control appears with the correct value in a Windows controller tester.

### Day 8 — Adaptive controller experience

- Apply the fixed responsive landscape layout, safe insets, immersive mode, and active-only keep-screen-on behavior.
- Add connection health/status overlay and recoverable error states.
- Expose and persist dead-zone/sensitivity settings.
- Exercise small/large aspect ratios in Compose previews/emulators and one physical phone.

Gate: no planned control is obstructed or overlaps on the tested size matrix.

### Day 9 — Reliability campaign

- Run 30-minute soak, repeated background/foreground, Wi-Fi toggle, malformed flood at a safe local rate, and reconnect loops.
- Measure observed packet rate, latest-state age, status RTT, timeout response, and UI responsiveness.
- Fix only release-blocking reliability defects; avoid starting discovery or tray work.
- Test at least one representative installed Windows game.

Gate: the M2 single-controller checklist passes or remaining blockers are precisely recorded.

### Day 10 — Consolidation and handoff

- Run complete Android/Rust checks and cross-language fixtures.
- Perform a clean debug-install/run walkthrough.
- Update every context/spec status with evidence.
- Produce a development APK/PC binary if safe and clearly label them non-release artifacts.
- Prioritize the post-10-day queue: M2 blockers first, then pairing/discovery, packaging, and Win11 validation.

Target outcome: a dependable single-phone development controller on the user's Windows 10 PC, with honest evidence and a continuation-ready repository. If hardware or driver issues consume time, the target contracts to the deepest fully verified vertical slice rather than falsely marking later work complete.

## Full project phases

### Phase 0 — Planning and repository foundation

Spec: [`0001-foundation-and-tooling.md`](../docs/specs/0001-foundation-and-tooling.md)

Deliver independent projects, pinned toolchains, root documentation, command conventions, secret exclusions, shared fixture location, and baseline CI/local checks. No controller features beyond health checks.

Exit: both skeletons compile on their intended toolchains and the progress tracker contains evidence.

### Phase 1 — Protocol and receive path

Specs:

- [`0002-protocol-v1-and-rust-udp.md`](../docs/specs/0002-protocol-v1-and-rust-udp.md)
- [`0003-android-connection-and-hello.md`](../docs/specs/0003-android-connection-and-hello.md)

Implement codecs, fixtures, Rust UDP validation/latest-state infrastructure, Android manual endpoint UI, permissions, persistence, and bidirectional hello/status.

Exit: real phone-to-PC hello succeeds and invalid datagrams cannot alter state.

### Phase 2 — End-to-end backend proof

Spec: [`0004-one-button-vertical-slice.md`](../docs/specs/0004-one-button-vertical-slice.md)

Complete A press/release through the virtual device. This is also the ViGEm go/no-go gate.

Exit: Windows controller tester sees A press and release, neutral and detach behave, and relaunch recovers.

### Phase 3 — Safety and transport reliability

Spec: [`0005-reliability-and-safety.md`](../docs/specs/0005-reliability-and-safety.md)

Complete 60 Hz scheduling, sequence/session/source rules, latest-state storage, timeout neutralization, Android lifecycle behavior, shutdown, backend-error behavior, counters, and deterministic fault tests.

Exit: safety matrix passes before more controls are added.

### Phase 4 — Input system and complete controller

Specs:

- [`0006-left-stick-and-multitouch.md`](../docs/specs/0006-left-stick-and-multitouch.md)
- [`0007-full-single-controller.md`](../docs/specs/0007-full-single-controller.md)
- [`0008-adaptive-controller-ui-and-settings.md`](../docs/specs/0008-adaptive-controller-ui-and-settings.md)

Build the coordinated pointer router and pure transforms, then all controller inputs, then responsive visual polish/settings without changing the protocol.

Exit: M2 reliable single-controller gate passes.

### Phase 5 — Discovery and pairing

Spec: [`0009-discovery-and-pairing.md`](../docs/specs/0009-discovery-and-pairing.md)

Add DNS-SD/mDNS discovery, manual fallback, six-digit pairing window, persistent paired client identity, session/source rotation, revocation, and Android 17 local-network handling. Remove manual trust from normal builds.

Exit: only the paired phone can control; restart/reconnect does not require re-pairing; replacement requires explicit PC action.

### Phase 6 — Desktop shell and distributable packaging

Spec: [`0010-desktop-tray-and-packaging.md`](../docs/specs/0010-desktop-tray-and-packaging.md)

Add tray UX, driver/firewall health, settings, local logs, optional disabled-by-default startup, signed APK flow, and Windows installer/uninstaller. Decide driver redistribution/linking only after license and checksum review.

Exit: clean install/use/uninstall walkthrough passes and no runtime admin is needed after setup.

### Phase 6a — Settings and layout customization

Spec: [`0010a-settings-and-layout-customization.md`](../docs/specs/0010a-settings-and-layout-customization.md)

Simplify Settings and provide user drag/rearrange controls plus saved layout profiles. Complete the detailed editing and persistence design before implementation. Preserve safe touch regions, neutralization and the default reference layout.

Exit: the settings and layout acceptance criteria pass before starting multiple controllers. This ordering is explicitly requested by the user on 2026-09-11.

### Phase 7 — Multiple controllers

Spec: [`0011-multiple-controllers.md`](../docs/specs/0011-multiple-controllers.md)

Generalize bounded session/slot management for up to four phones with independent state, timeout, counters, pairing, and virtual devices.

Exit: two phones can play simultaneously and disconnecting one never affects the other; expand to four as test hardware permits.

### Phase 8 — Keyboard/mouse compatibility

Spec: [`0012-keyboard-mouse-mode.md`](../docs/specs/0012-keyboard-mouse-mode.md)

Add explicit profiles that map controller state to Windows keyboard/mouse input, with all-key release safety and prominent limitations. Controller mode remains default.

Exit: a local browser harness and selected keyboard-only game respond, while timeout/profile exit releases every injected input.

### Phase 9 — Release certification

Spec: [`0013-release-certification.md`](../docs/specs/0013-release-certification.md)

Run full regression, performance/soak, compatibility, security review, signing, clean Windows 10/11 install, Android device matrix, documentation, and recovery tests.

Exit: release checklist is evidence-complete and remaining limitations are published.

## Decision gates

### G1 — Virtual backend viability

Pass before Phase 3/4 expansion. If ViGEm fails:

1. Confirm driver/client versions and official prerequisite.
2. Test a minimal upstream-style sample independent of project networking.
3. Decide between a maintained compatible alternative, accepting a retired pinned dependency, or funding a VHF/driver track.
4. Update architecture and release risk before continuing.

Do not casually choose Microsoft Virtual HID Framework: it moves the project into kernel-driver development, signing, and a substantially different risk/cost class.

### G2 — Single-controller reliability

Pass before discovery, multiple phones, customization, or keyboard/mouse. Requires full input, timeout/lifecycle safety, 30-minute soak, repeated reconnect, and at least one real game.

### G3 — Pairing/reconnect usability

Pass before installer polish. Requires manual-IP recovery, paired identity persistence, source/session rotation, revocation, rate-limiting, and Android permission UX.

### G4 — Distribution readiness

Pass before calling an artifact “release.” Requires signed APK, Windows installer behavior, driver/firewall review, clean uninstallation, Windows 11 evidence, and published compatibility limitations.

## Change discipline

- Each work session selects one numbered spec.
- A protocol change is never hidden inside a UI or backend task.
- A failing gate blocks dependent scope but does not block documentation, isolated tests, or research that can resolve it.
- Any scope cut is reflected in the tracker and release notes.
- User manual tests are short, explicit, and requested as soon as their result affects the next decision.
