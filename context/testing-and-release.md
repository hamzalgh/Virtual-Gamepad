# Testing and Release

## Quality policy

A feature is complete only when its behavior, failure behavior, and evidence are present. Compilation alone does not prove multi-touch, Wi-Fi, virtual-driver, game, lifecycle, installer, or operating-system compatibility.

Tests are layered so most defects are found without a phone or driver, while platform behavior is still verified on real hardware before release.

## Automated test layers

### Android/JVM unit tests

Keep domain logic outside Android UI types where practical. Cover:

- Neutral `ControllerState` and immutable updates.
- Axis clamp and asymmetric full-scale conversion (`-32768`/`32767`).
- Positive-up Y inversion from screen coordinates.
- Radial dead-zone boundaries and rescaling.
- Sensitivity-curve boundaries and monotonicity.
- Circular stick clamp and diagonal normalization.
- Trigger mapping to `0..255`.
- Every button bit, with no Guide bit and reserved bits zero.
- D-pad diagonals and prevention of opposing directions.
- Pointer ID assignment, ownership, release, cancel, and global reset.
- Protocol encode/decode for every v1 message.
- CRC success/failure and canonical check value.
- Exact 40-byte output and zeroed reserved fields.
- Endpoint/port validation and safe preference defaults.
- Connection/session state-machine transitions.

### Compose UI tests

Cover behavior that depends on semantics/layout without claiming physical multi-touch equivalence:

- Setup validation, permission rationale, retry, and persisted endpoint.
- Connected/degraded/disconnected status rendering.
- Settings bounds and reset-to-default.
- Landscape controller surface contains every expected control.
- Large-font setup screens remain usable.
- Controller capture regions stay inside computed safe bounds for the layout fixture matrix.

Use screenshot/golden tests only if their maintenance cost is justified; geometry assertions and previews are the first baseline.

### Android instrumentation/device tests

Automate where stable:

- App background/foreground stops and restarts the session safely.
- Immersive/system-bar restoration.
- Keep-screen-on flag active only in controller mode.
- Local-network permission denial/retry.
- DataStore persistence across process recreation.

Physical manual multi-touch remains required because emulator/macro behavior does not represent every touchscreen.

The debug APK also contains an explicit foreground connection probe using Android's platform `Instrumentation`. It sends ordinary neutral input by default; optional `-e inputPattern mixed` holds scripted A+B, full positive LX, full negative RY and LT/RT at 128/255 through the real ViewModel/sender. `-e inputPattern exercise` alternates full button/diagonal combinations, opposite LX/RY and independent triggers with neutral, once per second; `durationSeconds` is bounded to 10–1800 for the physical soak. Scripted input verifies the physical network/backend, not touch ownership. Its normal cleanup neutralizes/disconnects, and the probe is absent from release. Install `app-debug.apk`, start one PC agent with `--debug --manual-trust-first-client`, and substitute its current private IPv4 address below:

```text
adb shell am instrument -w -r -e pcAddress PC_IPV4 -e durationSeconds 60 com.hamzalgh.virtualgamepad/com.hamzalgh.virtualgamepad.debug.ConnectionProbe
```

The probe opens the real Activity, saves the supplied endpoint on port 8765, connects through its ViewModel, observes the selected input pattern, and disconnects/finishes on completion or failure. Duration is bounded to 10–1800 seconds. It reports diagnostics every five seconds. Read `INSTRUMENTATION_RESULT`: `PASS` with final code `-1` means the requested foreground interval completed; `FAIL` with code `0` means it did not. The ADB process exit code alone is not a verdict. This probe is absent from release, adds no dependency or Wi-Fi lock, and does not exercise physical touch ownership or background loss by itself. A larger supported PC `--input-timeout-ms` is advertised to the phone; success at that value does not satisfy the default 250 ms physical gate and delays safety cleanup after packet loss.

### Rust unit tests

Cover:

- Every v1 message encode/decode and byte offset.
- Invalid length, magic, version, type, CRC, reserved data, and payload ranges.
- D-pad conflicts and reserved button bits.
- Sequence equality, normal ordering, reordering, duplicate, and `u32` wraparound.
- Hello nonce and pairing-state transitions.
- Pairing code expiry, five-attempt limit, cooldown, replacement, and revocation.
- Session ID and bound source IP/port admission.
- Invalid packets not refreshing the valid-state deadline.
- Latest-value overwrite behavior under a slow consumer.
- Timeout neutralization using Tokio paused/fake time.
- Backend failure, graceful shutdown, and disconnect neutralization with a recording fake.
- Every Xbox report mapping and neutral report.
- Configuration bounds and safe defaults.

### Rust integration tests

- Bind on loopback and exchange hello/state/status through real UDP sockets.
- Send short, exact, and oversized datagrams.
- Send bursts, duplicates, reordered states, and datagrams from a second source socket.
- Delay the consumer and prove only the newest accepted state is applied.
- Drop all state and prove neutral is submitted within timeout plus one scheduling tolerance.
- Fuzz/property-test the decoder with bounded random 40-byte and arbitrary-length inputs; it must never panic.

### Cross-language golden fixtures

`protocol/fixtures/v1/` will contain canonical binary datagrams and a readable manifest. Kotlin and Rust must both:

- Decode each valid fixture to the same fields.
- Encode the manifest values to byte-identical output.
- Reject the marked invalid fixtures for the same category.

Required fixtures include neutral, all-buttons, axis extremes, mixed state, sequence wrap, every control message, bad CRC, nonzero reserved fields, wrong magic/version/type, and D-pad conflicts.

Fixtures are the strongest defense against two locally passing but mutually incompatible codecs.

## Standard verification commands

The exact tasks are finalized during project scaffolding.

### Android

From `mobile_controller/` on Windows:

```text
gradlew.bat testDebugUnitTest
gradlew.bat lintDebug
gradlew.bat assembleDebug
```

Spec 0001 configures no separate Kotlin formatter task. A future spec may add one only after selecting and pinning a maintained formatter.

Before release also run the release lint/tests/bundle or APK tasks configured by the project. Android Studio sync and an actual device run are reported separately.

### Rust

From `pc_agent/`:

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo check --all-targets
cargo test --all-targets
cargo build --release
```

When cross-target/build CI exists, Windows integration still requires an actual Windows runner or machine; a non-Windows compile is not a driver test.

## Manual Android matrix

### Minimum device coverage

Before single-controller milestone:

- User's primary physical phone on the home 2.4 GHz network.
- At least one emulator/preview with a compact landscape size.
- At least one wide/tall-aspect landscape fixture.

Before public release, expand to:

- Oldest supported API behavior (API 26 or the agreed revised floor).
- A current mainstream API level.
- Android 17/API 37 local-network permission behavior.
- Cutout/rounded-corner and three-button/gesture-navigation configurations.
- Small and large physical screens when hardware is available.

### Touch cases

- One stick through full circle, edge clamp, center release, and L3.
- Both sticks simultaneously.
- Stick held while pressing/releasing each face/shoulder/D-pad control.
- Two face buttons and a stick together.
- Both analog triggers independently and together.
- D-pad cardinal and diagonal transitions.
- Pointer leaves its visual control, then releases.
- Rapid tap and rapid pointer replacement.
- System gesture attempted near every edge.
- Incoming notification/system overlay, if applicable.
- All fingers cancelled by app background, screen lock, or task switch.

### Lifecycle/network cases

- Home, Recent Apps, screen off/lock, process kill, app reopen.
- Wi-Fi off/on, router temporarily unavailable, PC agent stop/start.
- Wrong IP, invalid port, host unreachable, firewall blocked, permission denied.
- PC address changes and manual fallback is used.
- Pairing code correct, wrong, expired, rate-limited, and revoked.
- Screen does not dim while active and returns to ordinary policy after exit.

## Manual Windows matrix

### Driver/backend

- Driver absent: exact, recoverable prerequisite message.
- Driver present: attach one virtual Xbox controller.
- A press/release, all buttons, axes, and triggers visible in `joy.cpl` or an approved tester.
- Timeout and disconnect return every field to neutral.
- Normal exit detaches; relaunch recovers.
- Backend update failure enters safe state.
- Agent process forcibly terminated, then driver/agent recovery documented and tested.

### OS targets

- Windows 10 x64: full primary validation.
- Windows 11 x64: clean install, firewall, driver, tray, controller, game, and uninstall validation before supported release.

Do not infer Windows 11 success from Windows 10 or documentation alone.

### Games and browser

Record each test with OS, game/browser version, controller mode, result, and notes:

- PES 2017 as one native-game sample.
- One available Need for Speed title as a racing/analog sample.
- At least one additional modern XInput game if available.
- A local Gamepad API test page in Chrome/Edge.
- One representative web game that explicitly supports gamepads.
- Later, one keyboard-only browser/game path for the opt-in mapping backend.

No result is generalized to “all games.”

## Reliability and performance campaign

### Required scenarios

- 30-minute active-input soak.
- 50 connect/disconnect/reconnect cycles.
- 20 Android background/foreground cycles.
- 20 Wi-Fi disable/enable cycles or a safe repeatable equivalent.
- Reordered/duplicate/lost packet simulation.
- Burst traffic and slow backend consumer simulation.
- Malformed traffic at a controlled local rate with rate-limited logs.

### Acceptance targets

- Sender targets 60 Hz without blocking the Compose main thread.
- Receiver never applies a stale state after a newer accepted sequence.
- Receiver memory/channel depth remains bounded during bursts.
- Output becomes neutral no later than configured timeout plus one reasonable scheduler/backend update interval.
- No held control survives pause, Wi-Fi loss, disconnect, backend fault, or normal exit.
- No parser panic or uncontrolled log growth.
- Observed state rate, status round-trip age, loss, and scheduling jitter are recorded; environmental latency is reported rather than fabricated.
- User-visible control feedback begins in the same UI frame where practical and does not wait for network acknowledgement.

Network RTT targets are diagnostic, not universal guarantees on congested 2.4 GHz. Optimize regressions and state age, not a misleading single ping number.

## Security review checklist

- UDP bound to configured local interfaces/port and never advertised as internet-safe.
- Windows firewall rule is program-scoped, UDP inbound, Private profile only.
- Exact packet validation precedes state mutation.
- Pairing code is random, short-lived, rate-limited, and not normally logged.
- Paired instance/session IDs are generated with OS randomness.
- Wrong source/session cannot refresh timeout or control output.
- No secrets, signing material, full session tokens, or unnecessary personal data in repository/logs.
- Android exported components and intents are minimized.
- Dependency versions, licenses, advisories, and upstream status are reviewed.
- Keyboard/mouse profiles cannot run without explicit user selection and release all inputs on every exit.
- Security review is rerun for the release diff.

## Android distribution

### Build/signing

- Use a stable application ID chosen during scaffolding.
- Create a release keystore outside the repository; back it up securely.
- Inject signing values through user-local Gradle properties/environment, never committed files.
- Produce and archive the signed APK, version metadata, checksum, and release notes.
- Verify install/upgrade behavior with the same signing identity.

### Permissions and policy

- Explain why local-network access is needed and request it only in the connection flow.
- Keep the permission list minimal.
- Document direct APK installation and the Android developer-verification/registration path applicable at release time.
- Recheck target-SDK requirements and Android 17 behavior immediately before release.

## Windows distribution

### Recommended installer direction

Use an Inno Setup-based x64 installer unless the packaging spike identifies a concrete need for MSI/WiX. Inno Setup is recommended for straightforward preflight, optional tasks, firewall commands, shortcuts, and uninstall behavior. The decision is revisited before implementation, with installer source committed and reproducible.

The installer should:

1. Verify supported x64 Windows version.
2. Install the agent under a standard per-machine or per-user location chosen in the spec.
3. Detect the virtual-controller driver and show an exact official prerequisite/recovery path.
4. Add a program-scoped UDP inbound rule for Private networks only, with explicit user consent/elevation.
5. Offer start-with-Windows as an unchecked option.
6. Create Start Menu/uninstall entries and a log/config location.
7. Preserve or remove user settings according to a clear uninstall choice.
8. Remove only rules/files it owns.

Do not silently bundle or redistribute the retired driver until its license, official distribution path, version, checksum, and unattended-install behavior have been reviewed. Linking to or detecting an official driver release is safer than casually repackaging it.

### Signing and reputation

- Prefer Authenticode signing for the executable and installer.
- If no code-signing certificate is available, document Windows SmartScreen warnings as a release limitation rather than claiming a polished warning-free install.
- Generate SHA-256 checksums for release artifacts.

## Release artifact set

- Signed Android APK and SHA-256 checksum.
- Windows x64 installer and SHA-256 checksum.
- Versioned protocol/spec documentation.
- Install, pair, use, troubleshoot, update, and uninstall guide.
- Driver/firewall prerequisite and recovery guide.
- Compatibility matrix and known limitations.
- License/third-party notices.
- Changelog and reproducible build identifiers.

## Release gates

### Development alpha

- One-button vertical slice and backend gate pass.
- Timeout neutral proven.
- Artifacts clearly marked development-only.

### Single-controller beta

- Full control surface and M2 reliability matrix pass.
- Manual IP flow works on the user's network.
- At least one native game and local Gamepad API tester pass.

### Local-product release candidate

- Discovery/pairing, tray, installer, signed APK, clean install/uninstall, and security review pass.
- Windows 10 and Windows 11 have explicit evidence.

### Final release

- Multiple-controller and keyboard/mouse features are included only if their specs and safety tests pass; otherwise release notes mark them deferred.
- All required documents and checksums are present.
- Known limitations prominently include ViGEm retirement, trusted-LAN protocol, game/anti-cheat variability, and verified device/OS matrix.

## Evidence record

Every manual run should capture:

- Date, artifact commit/build ID, device/OS, network, and test case.
- Exact observed result, not only pass/fail.
- Logs/screenshots when they materially prove the result.
- Any workaround.
- Tester and follow-up issue/spec.

Summaries go in `context/progress-tracker.md`; bulky evidence belongs in a future `docs/test-reports/` directory.

## References

- [Android app signing](https://developer.android.com/studio/publish/app-signing)
- [Android developer verification](https://developer.android.com/developer-verification/guides)
- [Windows Firewall rule guidance](https://learn.microsoft.com/en-us/windows/security/operating-system-security/network-security/windows-firewall/rules)
- [Microsoft `SendInput` limitations](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput)
