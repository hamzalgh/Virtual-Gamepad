# Project Overview

## Product statement

Virtual Gamepad turns an Android phone into a responsive Xbox-style controller for a Windows PC on the same private Wi-Fi network. The Android app captures multi-touch controller state; the Windows agent validates the state and exposes it as a virtual Xbox 360/XInput-compatible controller.

The product succeeds when a user can install both applications, pair one phone, open a game or controller tester, and play without stuck inputs, confusing network setup, or perceptible state backlog.

## Confirmed product decisions

### Controller surface

The default controller includes:

- Left and right analog sticks.
- L3 and R3 stick clicks.
- A, B, X, and Y.
- D-pad with cardinal directions and supported diagonals.
- LB and RB.
- Analog LT and RT touch controls.
- Back and Start.

There is no Guide/Xbox button. There is no phone vibration, rumble, or haptic feedback in any planned version.

### Platforms

- Phone: Android 8.0 or newer is the practical baseline. The UI must adapt to different landscape sizes, aspect ratios, cutouts, rounded corners, and gesture insets rather than target a phone model.
- PC: Windows 10 x64 is the primary development and validation system. Windows 11 x64 is a supported target and requires explicit release validation.
- Network: phone and PC are on the same private home LAN. The initial network is 2.4 GHz Wi-Fi, so reliability under interference matters more than laboratory throughput.
- Language: source, comments, documentation, logs, and UI copy are English.

“Any Android phone” means a responsive layout and documented Android-version floor. It cannot mean every historic Android device, vendor bug, or unsupported form factor.

### Experience

- Controller mode is landscape-only and must not rotate when the user moves the phone.
- Controller mode is immersive, with system bars hidden, while still respecting safe insets.
- The screen stays awake only while controller mode is active.
- The first layout is fixed and polished. User layout customization is deferred until controller reliability is proven.
- IP address, port, sensitivity, dead zone, and ordinary preferences persist locally.
- Auto-start on Windows is optional and off by default.

### Connectivity and control ownership

- Manual IPv4 address and UDP port are implemented first.
- Automatic local discovery and a short pairing code follow reliability.
- Exactly one paired phone controls the first release-quality controller.
- Multiple simultaneous phones/controllers are implemented only after the single-controller reliability gate.
- When Android pauses, stops, locks, loses Wi-Fi, or exits controller mode, input must stop. The PC independently returns the virtual controller to neutral after timeout.

### Distribution

The intended release includes:

- A signed Android APK with installation and local-network permission guidance.
- A polished Windows x64 installer with virtual-driver detection, Private-network firewall configuration, configuration defaults, uninstall behavior, and recovery documentation.
- A tray-based Windows experience after the terminal agent is reliable.

### Publication and license decision (2026-09-12)

The user chooses public GitHub source plus official Android APK and Windows executable/installer downloads with setup instructions. Instructions must cover driver prerequisites, Private network firewall access, pairing and known compatibility limits. LinkedIn demos will link to the official project and release pages when available.

Apache License 2.0 is the selected project license, subject to dependency compatibility and third party notice review before applying it. Prepare the standard LICENSE and an appropriate NOTICE with the user's confirmed copyright attribution during release preparation. The exact copyright holder name remains to be confirmed; do not infer it from a package name or account. Recording this decision does not itself apply a license or replace dependency licenses.

On 2026-09-13 the user authorized a suggested provisional public credit, **Virtual Gamepad contributors**, which may be refined before publication. This supersedes the unresolved name above. LICENSE/NOTICE are now prepared after reviewing the locked Windows Cargo license declarations/texts and resolved Android release POMs/notices, including protobuf BSD-3-Clause and Unicode-3.0 components. Third-party licenses remain separate. No signing credentials are available or created, and no official repository/release identity is yet confirmed.

The attribution intent is to preserve the project's origin when others distribute modifications. Apache 2.0 requires relevant notices to be retained and changed files to be identified under its terms; it does not require prominent credit on every screen or social post. Do not add incompatible extra restrictions or claim that the license prevents copying.

Community forks remain separate. Only changes approved and merged by the maintainer enter official source; unofficial builds must not be delivered automatically to users of official releases. Publishing source grants no write access to the official repository. Preserve the absence of automatic updates unless a later explicit decision changes it.

Spec 0010 should carry this decision into packaging and distribution planning, and Spec 0013 should verify licenses, notices and official release identity. Publication, external posting, signing key creation, installation and release certification require their own authorized work; none occurred when this decision was recorded.

## Compatibility promise

### Primary mode

The primary output is a virtual Xbox 360/XInput-compatible controller. This targets broad compatibility with Windows games such as football, racing, action, and local multiplayer games. PES 2017 and Need for Speed are representative manual tests, not product-specific integrations.

### Browser games

A browser game works in controller mode only if it consumes the browser Gamepad API or another compatible gamepad input path. A later, opt-in keyboard/mouse mapping mode can cover some browser and legacy games that do not accept controllers.

### Honest limits

The project cannot promise every game. Compatibility may be blocked by:

- A game that supports only keyboard/mouse or an incompatible controller API.
- Anti-cheat or security software that rejects virtual devices or injected input.
- Privilege boundaries for later keyboard/mouse injection.
- A browser page that does not use the Gamepad API.
- Driver incompatibility on a future Windows update.

These are documented limits, not failures to disguise.

## Product priorities

In order:

1. No stuck buttons, triggers, or axes.
2. Correct multi-touch and controller mapping.
3. No stale-state backlog under packet loss or scheduling delay.
4. Simple install, pairing, status, and recovery.
5. Low and stable latency at 60 Hz on ordinary home Wi-Fi.
6. Broad XInput game compatibility.
7. Responsive visual polish across supported phones.
8. Discovery, multi-controller support, customization, and keyboard/mouse compatibility.

## Milestones

### M0 — Architecture and risk proof

- Both projects build on the target machine.
- Protocol v1 golden vectors pass on both platforms.
- ViGEm driver/client viability is demonstrated on Windows 10 x64.

### M1 — First end-to-end input

```text
Android A press
→ v1 UDP state packet
→ Rust validation/session
→ virtual Xbox 360 A
→ visible in Windows controller tester
```

### M2 — Reliable single controller

- Full controls work simultaneously.
- 60 Hz full-state transmission has no unbounded queue.
- Lost network/app lifecycle/backend failures neutralize safely.
- A 30-minute soak and repeated disconnect test pass.
- At least one representative native Windows game passes.

### M3 — Usable local product

- Adaptive, safe-inset-aware controller UI.
- Persisted tuning settings and clear status/error UI.
- Automatic discovery with manual fallback.
- Short-code pairing and single-controller admission.
- Tray application and install/uninstall path.

### M4 — Expanded compatibility

- Multiple paired phones/controllers with isolated state and timeouts.
- Opt-in keyboard/mouse profiles for games that lack controller support.
- Signed, documented release validated on Windows 10 and Windows 11.

## Out of scope

- Internet/WAN control, relay servers, accounts, or cloud storage.
- Bluetooth transport.
- iOS, macOS, Linux, consoles, or smart-TV clients.
- Public-LAN or zero-trust cryptographic security in v1.
- Rumble/vibration, audio, motion controls, touchpad, or Guide button.
- Per-game hacks or anti-cheat bypasses.
- Multi-phone support before M2.

## Success measures

Release decisions use observable evidence:

- A physical Android device can hold a stick direction while pressing/releasing multiple buttons without pointer stealing.
- Neutral state is applied within the configured timeout after valid packets stop.
- Reordered, stale, malformed, unpaired, and wrong-source datagrams never alter controller output.
- The receiver applies the newest valid state rather than replaying queued historical states.
- Windows reports the expected controller and mappings through `joy.cpl` or a known gamepad tester.
- Manual game tests record title, version, input mode, result, and any workaround.
- Installer and APK instructions work from a clean user perspective.

## Known strategic risk

The v1 Windows backend uses ViGEm because it provides the desired Xbox 360 compatibility and has a Rust client, but the project is retired. The backend must therefore be proven early, isolated behind a trait, pinned, and treated as replaceable. Failure of the driver spike stops feature expansion until a supported fallback decision is made.

## References

- [Android app architecture recommendations](https://developer.android.com/topic/architecture/recommendations)
- [Microsoft: XInput and XUSB devices](https://learn.microsoft.com/en-us/windows/win32/xinput/directinput-and-xusb-devices)
- [W3C Gamepad API](https://www.w3.org/TR/gamepad/)
- [ViGEmBus repository and retirement notice](https://github.com/nefarius/ViGEmBus)
