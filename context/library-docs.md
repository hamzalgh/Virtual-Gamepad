# Library and Platform Documentation

## Spec 0010 Windows startup failure research (2026-09-14)

The exact pinned vigem-client 0.1.4 source was inspected in Cargo's registry: x360.rs, bus.rs, event.rs and the client lifecycle. wait_ready constructs a fresh IOCTL/OVERLAPPED and waits synchronously with the target's auto reset completion event. plugin scans serial slots internally; unplug clears the recorded serial only on success. Target Drop retries unplug and Client Drop closes the bus handle. A Tokio timeout cannot cancel these calls. No version, feature or dependency change is made.

GitHub release tree/blob reads confirm identical EmulationTargetPDO.cpp and XusbPdo.cpp source at driver v1.21.442.0 and setup v1.22.0. The former uses a one-second boot wait, reports STATUS_DEVICE_HARDWARE_ERROR on timeout and explicitly clears its boot notification event afterward. The latter signals boot during XUSB initialization and can accept an unchanged neutral report without delivering input. This supports one fresh target retry after cleanup for wait-ready WinError(483), not repeated waits on one target or neutral polling as readiness evidence. Exact blob IDs and the recovery contract are in Spec 0010. The friend's underlying Windows 11 failure remains unproven.

Context7 resolved only /nefarius/vigemclient (C++ SDK); its query is not evidence for the Rust wrapper. Authoritative sources: [Rust 0.1.4](https://docs.rs/vigem-client/0.1.4/src/vigem_client/x360.rs.html), [driver release source](https://github.com/nefarius/ViGEmBus/blob/v1.21.442.0/sys/EmulationTargetPDO.cpp), [XUSB release source](https://github.com/nefarius/ViGEmBus/blob/v1.21.442.0/sys/XusbPdo.cpp), and [setup release retaining the driver version](https://github.com/nefarius/ViGEmBus/releases/tag/v1.22.0).

Failure notifications use existing tray-icon 0.25.0 window_handle/with_guid and windows-sys 0.61.2 Shell_NotifyIconW, with a generated per-run icon GUID and fixed zero-terminated buffers. Native UI ownership remains on the tray thread. [Microsoft Shell_NotifyIconW](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shell_notifyiconw) and [NOTIFYICONDATAW](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/ns-shellapi-notifyicondataw) govern GUID identity and NIF_INFO; Windows may suppress the banner, so persistent details remain necessary. No new package, license or redistribution boundary is introduced.

## Spec 0010 desktop and packaging research (2026-09-13)

Inno Setup 6.7.3 review compilation is now verified. Official [release assets](https://github.com/jrsoftware/issrc/releases/tag/is-6_7_3), the pinned [license](https://github.com/jrsoftware/issrc/blob/is-6_7_3/license.txt) and bundled license were checked before authorized portable compiler installation. Distribution and ISCC hashes are in packaging/README.md and release-prerequisites.json. Compiler PE metadata is 0.0.0.0, so use the documented [Ver constant](https://jrsoftware.org/ishelp/topic_predefinedvars.htm). The pinned [uninstaller source](https://github.com/jrsoftware/issrc/blob/is-6_7_3/Projects/Src/Setup.Uninstall.pas) confirms usUninstall runs after confirmation and before file removal, with an exception aborting that path. Native compilation is evidence of script validity, not real installation/cleanup acceptance. Earlier compiler-unavailable notes below describe the previous preparation stage.

The pairing popup uses the already pinned windows-sys 0.61.2 with Gdi, LibraryLoader and KeyboardAndMouse feature modules for fonts, native window registration and local control focus only. No input injection, new package/version or redistribution license is introduced. Context7 /microsoft/windows-rs and the downloaded 0.61.2 signatures were checked alongside Microsoft [window creation](https://learn.microsoft.com/en-us/windows/win32/winmsg/using-windows), [WM_CLOSE](https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-close) and [application manifests](https://learn.microsoft.com/en-us/windows/win32/sbscs/application-manifests). Native controls and fonts stay in windows_pairing_window and are destroyed on their UI thread; WM_NCDESTROY clears the live handle before reuse. Native rendering, keyboard and display scaling still need user evidence.

Windows now pins tray-icon 0.25.0, with muda 0.20.0 locked transitively. Both are maintained upstream native tray/menu projects under MIT/Apache-2.0 choices; dpi declares Apache-2.0 AND MIT. Their sole removal boundary is Windows presentation, independent of protocol and backend policy. The Windows-filtered Cargo graph also includes Unicode-3.0 attribution. Full discovered license/notice texts are retained in pc_agent/packaging/THIRD_PARTY_NOTICES.txt. This is dependency metadata/notice review, not an advisory clearance or platform acceptance claim.

Context7 tray-icon guidance was checked against the downloaded versioned source. In particular muda stores its event handler in a OnceCell: install the bounded handler before user events, never rely on replacing it at shutdown. Microsoft SetTimer/GetMessage guidance governs the main-thread native pump and nested-menu expiry callback. Windows SDK rc.exe embeds version/asInvoker resources without a new build dependency. Inno Setup 6.7.3 is the selected compiler; its installation, trusted binary checksum and applicable license terms remain prerequisites. AppMutex/SetupMutex and CreateMutex behavior was checked in the official Inno reference. No compiler/setup was run.

Android signing follows the official app-signing guidance and keeps the existing ID/version. Resolved release POMs predominantly declare Apache-2.0; the repackaged protobuf component declares BSD-3-Clause, and Guava ListenableFuture inherits its parent license. Bundled notices preserve extracted license/notice entries and the upstream protobuf text. Gradle inventory reports remain ignored build output, with no local dependency paths copied into distributed notices. Release assets contain LICENSE, NOTICE and THIRD_PARTY_NOTICES.

References: https://docs.rs/tray-icon/0.25.0/tray_icon/ ; https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-settimer ; https://jrsoftware.org/ishelp/topic_setup_appmutex.htm ; https://jrsoftware.org/ishelp/topic_setup_setupmutex.htm ; https://developer.android.com/studio/publish/app-signing ; https://raw.githubusercontent.com/protocolbuffers/protobuf/v3.25.5/LICENSE .

Last researched: **2026-09-04**. Versions change; this file records the planning baseline, not permission to update blindly. Recheck the applicable official release notes and Context7 immediately before scaffolding or dependency changes, then pin exact versions in build files and lockfiles.

## Selection policy

For each dependency:

1. State the capability it provides.
2. Check whether the platform standard library already provides it clearly.
3. Review current official/upstream docs, release activity, supported platforms, license, advisories, and transitive cost.
4. Use Context7 for current third-party API examples when available.
5. Isolate dependencies with high platform or maintenance risk.
6. Pin a compatible version and commit the generated lockfile/version catalog.
7. Add tests at the boundary and record removal/replacement cost.

Do not add a library merely because it is popular. Do not let Context7 or a tutorial override official platform behavior.

## Android baseline

### Planned toolchain

| Item | Planning baseline | Notes |
|---|---|---|
| Android Studio | Current stable at scaffold time | User builds/runs Android here |
| Android Gradle Plugin | 9.4.x | Official 9.4 notes list API 37 support, Gradle 9.6, Build Tools 36, JDK 17 |
| Gradle | 9.6 wrapper | Pin wrapper checksum/version |
| Gradle daemon JDK | 25 | Generated daemon criteria are pinned to Java 25; AGP 9.4 requires at least 17. This is separate from Java source compatibility. |
| Kotlin Compose plugin | 2.2.10 | Generated, pinned, and command-line build verified on 2026-09-04; keep until a deliberate compatibility refresh. |
| Kotlin language | JVM/Android | Kotlin only; no Java application layer unless platform interop requires it |
| Compose BOM | 2026.02.01 | Generated, pinned, and command-line build verified on 2026-09-04; a newer BOM is available but was not substituted into the known-working scaffold. |
| `compileSdk` / `targetSdk` | 37 | Recheck Android 17 requirements immediately before scaffold/release |
| `minSdk` | 26 | Practical Android 8.0+ floor; revise only with a named device/requirement |

Use the Kotlin Compose compiler plugin appropriate for Kotlin 2.x. Keep plugin/library versions centralized in `gradle/libs.versions.toml` unless Android Studio's generated structure has a strong reason otherwise.

Spec 0001 preserves the Android Studio generated dependency set because it already synced, installed, and launched on the physical test phone. Android lint reports newer AndroidX, Compose BOM, and Kotlin Compose plugin versions. Those notices are recorded rather than mixed into the foundation slice without a separate compatibility check. Gradle daemon JVM criteria request Java 25 and take precedence over `JAVA_HOME`; the app's Java source and target compatibility remain Java 11.

### Initial Android dependencies

Pin current compatible versions during Spec 0001:

- `androidx.activity:activity-compose` — activity/Compose integration.
- Compose BOM plus UI, foundation, Material 3, tooling preview, and test modules — UI only.
- `androidx.lifecycle:lifecycle-runtime-compose` and ViewModel Compose — lifecycle-aware state collection and ownership.
- `androidx.datastore:datastore-preferences` — small non-sensitive persisted settings.
- Kotlin coroutines Android/test — lifecycle-owned UDP work and deterministic coroutine tests.
- AndroidX Core — window/system integration through supported APIs.

Use platform APIs for:

- UDP: `DatagramSocket`/`DatagramPacket` on an I/O dispatcher.
- Binary layout: explicit byte-array operations or `ByteBuffer.order(LITTLE_ENDIAN)` with strict offsets.
- CRC: `java.util.zip.CRC32`.
- Random IDs/nonces: `SecureRandom`.
- Network state/capabilities: Android connectivity APIs.
- Immersive/system bars: AndroidX window compatibility APIs.

### Dependencies not justified initially

- Retrofit/OkHttp/Ktor: continuous transport is UDP, not HTTP.
- Protobuf/JSON/Postcard: the wire format is a fixed 40-byte manual binary protocol.
- Room/SQL database: preferences are sufficient.
- Hilt/Koin: constructor composition is small at the start.
- Game engine/custom rendering framework: Compose plus low-level pointer input is adequate.
- Image-loading libraries: no remote image product requirement.

Add one only when a spec demonstrates a concrete need and records the tradeoff.

### Android APIs requiring special attention

#### Local network access

Android 17/API 37 introduces local-network permission enforcement for apps targeting API 37. Outgoing and incoming UDP/raw-socket access is in scope. The app needs an explicit permission flow and must not treat denial as a generic network failure. Recheck the final manifest/runtime API against the current Android documentation.

#### Insets and immersive mode

Modern Android enforces edge-to-edge behavior on recent target levels. Compose layout must use current safe inset APIs even when status/navigation bars are hidden. Avoid deprecated full-screen flag recipes copied from old tutorials.

#### Network service discovery

Later discovery uses Android NSD/DNS-SD, not a custom broadcast scan. Recheck current `NsdManager` callbacks, lifecycle, Android 17 local-network picker/permission behavior, and whether older devices require a carefully scoped multicast lock. Manual IP remains mandatory fallback.

### Android official sources

- [Jetpack Compose setup](https://developer.android.com/develop/ui/compose/setup)
- [Compose BOM](https://developer.android.com/develop/ui/compose/bom)
- [Android Gradle Plugin 9.4 release notes](https://developer.android.com/build/releases/agp-9-4-0-release-notes)
- [Kotlin releases](https://kotlinlang.org/docs/releases.html)
- [Android architecture recommendations](https://developer.android.com/topic/architecture/recommendations)
- [Compose pointer input](https://developer.android.com/develop/ui/compose/touch-input/pointer-input/understand-gestures)
- [Compose window insets](https://developer.android.com/develop/ui/compose/system/insets)
- [Android local-network permission](https://developer.android.com/privacy-and-security/local-network-permission)
- [Android NSD guide](https://developer.android.com/develop/connectivity/wifi/use-nsd)
- [DataStore API](https://developer.android.com/reference/kotlin/androidx/datastore/preferences/core/package-summary)
- [App signing](https://developer.android.com/studio/publish/app-signing)

## Rust and Windows baseline

### Planned toolchain

| Item | Planning baseline | Notes |
|---|---|---|
| Rust | 1.98.1 | Current stable observed 2026-09-03; pin in `rust-toolchain.toml` |
| Edition | 2024 | Use stable language/tooling |
| Target | `x86_64-pc-windows-msvc` | Windows 10/11 x64 |
| C/C++ tools | Visual Studio 2022 Build Tools + Windows SDK | Required by the MSVC target/native dependencies |
| Package lock | `Cargo.lock` committed | PC application, reproducible builds |

Run Rustup with the MSVC toolchain on Windows. Do not target GNU by accident when native Windows/driver-facing crates expect MSVC.

### Initial Rust crates

Exact compatible versions are selected in Spec 0001/0002 after Context7/upstream review:

- `tokio` — UDP, timers, signals, bounded/latest-value coordination. Enable only needed features such as `rt-multi-thread`, `macros`, `net`, `time`, `sync`, and `signal`.
- `crc32fast` — CRC-32/ISO-HDLC for protocol v1.
- `thiserror` — typed library/domain errors without stringly parsing.
- `tracing` and `tracing-subscriber` — structured local diagnostics and rate-aware filtering.
- `clap` — validated terminal configuration once command options exceed trivial parsing; omit if unnecessary at first.
- `serde` plus `toml` — only for persisted human-readable configuration/tray settings; not for the wire protocol.
- OS randomness via a small maintained crate/current standard facility — session IDs, instance IDs, nonces, and pairing codes.
- `vigem-client` under `cfg(windows)` — isolated v1 Xbox 360 backend after the driver gate.

Tokio `watch` is the preferred starting primitive for continuous controller state because it retains only the latest value. Infrequent control commands use bounded channels.

### Later, spec-scoped crates

- `mdns-sd` (0.21.1 observed during planning) — DNS-SD/mDNS advertise/discover after reliability.
- `tray-icon` plus its required event-loop/window integration — Windows tray phase only; verify current Windows support and lifecycle before selection.
- `windows`/`windows-sys` — narrow Win32 APIs such as `SendInput` in the later keyboard/mouse backend.
- A secure credential/config storage solution only if the future threat model requires a secret stronger than v1's trusted-LAN paired ID.

Do not pull these into the initial controller slice.

### ViGEm decision and risk

ViGEmBus was retired and its repository archived. The final release listed by the project is 1.22.0, and `vigem-client` 0.1.4 was the current Rust wrapper found during research. The driver supports Xbox 360/DS4 emulation on Windows 10/11, but its retirement creates maintenance, signing, future-Windows, and redistribution risk.

Rules:

- Prove it on the user's Windows 10 machine in Spec 0004.
- Pin driver/client versions that were actually tested.
- Link to/check the official binary and checksum; do not silently rehost it.
- Keep all wrapper types behind `VirtualGamepadBackend`.
- Record Windows 11 evidence separately.
- Treat driver failure on a current target as a product architecture decision, not a UI bug.

Microsoft Virtual HID Framework is not a drop-in library replacement. It is a kernel-driver framework with signing and driver-development implications, suitable only for a separately approved long-term backend project.

### Windows platform APIs

- XInput/XUSB mapping defines the primary compatibility shape.
- Windows Firewall installer rule is program scoped, UDP inbound, Private profile only.
- Later `SendInput` keyboard/mouse injection is constrained by User Interface Privilege Isolation and can inject only into equal/lower integrity processes; anti-cheat/game policy may also block it.
- The runtime agent should be ordinary-user software after setup. Elevation belongs only to explicit driver/firewall/install work.

### Rust/Windows official and upstream sources

- [Rust 1.98.1 release](https://blog.rust-lang.org/2026/09/03/Rust-1.98.1/)
- [Rust Windows MSVC target](https://doc.rust-lang.org/rustc/platform-support/windows-msvc.html)
- [Tokio `UdpSocket`](https://docs.rs/tokio/latest/tokio/net/struct.UdpSocket.html)
- [Tokio `watch`](https://docs.rs/tokio/latest/tokio/sync/watch/)
- [`crc32fast`](https://docs.rs/crc32fast/latest/crc32fast/)
- [`vigem-client`](https://docs.rs/vigem-client/latest/vigem_client/)
- [ViGEmBus repository](https://github.com/nefarius/ViGEmBus)
- [ViGEm end of life](https://docs.nefarius.at/projects/ViGEm/End-of-Life/)
- [Microsoft XInput/XUSB mapping](https://learn.microsoft.com/en-us/windows/win32/xinput/directinput-and-xusb-devices)
- [Microsoft Virtual HID Framework](https://learn.microsoft.com/en-us/windows-hardware/drivers/hid/virtual-hid-framework--vhf-)
- [Windows Firewall rules](https://learn.microsoft.com/en-us/windows/security/operating-system-security/network-security/windows-firewall/rules)
- [Microsoft `SendInput`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput)

## Web-game test dependency

The browser test harness uses the standards-based Gamepad API and can be a small local static page; it needs no production web framework. Browser support and user-gesture behavior are rechecked at test time. The harness verifies axes/buttons exposed by the virtual controller but is not proof that an arbitrary web game supports gamepads.

- [W3C Gamepad API](https://www.w3.org/TR/gamepad/)

## Packaging tools

### Android

Use Gradle/Android Studio signing and release tasks. Keep keystore/passwords outside the repository. Recheck Android developer-verification/registration requirements near release because rollout dates and regional requirements can change.

### Windows

The recommended starting installer is Inno Setup because the product needs a clear x64 preflight, optional startup task, firewall setup, driver guidance, shortcuts, and owned uninstallation. Re-evaluate against WiX/MSI in Spec 0010 if enterprise deployment, repair semantics, or driver packaging makes MSI materially better.

Installer/compiler versions are selected only in that phase and pinned/documented. Authenticode signing is a separate credential requirement.

## Dependency record template

When adding a dependency, append or update an implementation record in the active spec or a future lockfile audit:

```text
Name/version:
Used by:
Purpose:
Why platform API is insufficient:
Official/upstream docs checked:
Context7 library ID/query checked:
License/redistribution:
Maintenance/advisory status:
Feature flags:
Isolation/removal boundary:
Verification:
```

## Spec 0009 dependency selection (2026-09-11)

The discovery adapter selects `mdns-sd = "=0.21.1"`, the deliberately pinned planning baseline, not a latest version claim. It supplies DNS-SD registration without a custom DNS implementation or Bonjour installation. Context7 returned an unrelated Go library; downloaded 0.21.1 source confirmed explicit interface controls, disabled automatic IP selection, registration and shutdown APIs. It is Apache 2.0 or MIT licensed; replacement is confined to discovery. This is an actively published upstream crate, but no full advisory clearance is claimed. DNS metadata carries no secret or full identity.

Windows adds Microsoft's `windows = "=0.62.2"` for COM Network List Manager and directly pins `windows-sys = "=0.61.2"` for the narrow DACL adapter. Both use MIT or Apache 2.0 licensing. Windows network categories and directory protection are absent from Rust's standard library; native adapters avoid shell commands and administrator requirements. Feature flags are limited to their named Windows APIs. Unsafe code stays in the documented Windows adapters and never handles UDP input. Removing discovery or replacing local storage removes these adapter uses. Distribution must retain the selected licenses and notices. Platform execution and advisory coverage are reported separately from compilation and source review.

`if-addrs = "=0.15.0"` is directly pinned from the mDNS dependency graph to join IPv4 addresses to Windows adapter GUIDs without another handwritten IP Helper adapter. Its downloaded source confirms `adapter_name` and `ip()` behavior. It is MIT or Apache 2.0 licensed, maintained upstream at [if-addrs](https://github.com/messense/if-addrs), and confined to the Windows discovery inventory. No Android dependency was added. Platform NsdManager uses API 34 callbacks and the older resolver at the API floor; native execution, multicast delivery, profile transitions and API 37 permissions remain unverified.

References: [mdns-sd upstream](https://github.com/keepsimple1/mdns-sd), [Microsoft Rust Windows bindings](https://github.com/microsoft/windows-rs), [Network List Manager](https://learn.microsoft.com/en-us/windows/win32/api/netlistmgr/nn-netlistmgr-inetworklistmanager), [SetFileSecurityW](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-setfilesecurityw).

## Documentation refresh policy

Re-run research when:

- Scaffolding begins after a delay.
- Target/compile SDK changes.
- Android local-network or distribution policy changes.
- Rust stable/toolchain is upgraded.
- Tokio, Compose, DataStore, mDNS, tray, installer, or virtual backend is added/upgraded.
- Windows 11 changes driver behavior.
- A dependency is archived, has a security advisory, or cannot reproduce the tested build.
- A release candidate is cut.

## Spec 0002 dependency record (2026-09-05)

Exact direct pins are Tokio `1.53.1`, crc32fast `1.5.1`, tracing `0.1.44`, and tracing-subscriber `0.3.23`. Cargo.lock records the transitive graph. Rust remains `1.98.1` and the Windows MSVC target is unchanged. No clap, serde, thiserror, interface enumeration crate, driver crate, or production randomness dependency was needed.

Tokio supplies asynchronous UDP, cancellation, signals, timers, and bounded watch storage, which the standard library does not provide as an integrated asynchronous runtime. Enabled features are `rt-multi-thread`, `macros`, `net`, `time`, `sync`, and `signal`. It is MIT licensed and maintained by the Tokio project. Its boundary is transport, latest state storage, and binary runtime composition; replacing it does not affect the protocol wire format.

crc32fast supplies the specified IEEE CRC algorithm, absent from the Rust standard library. Version 1.5.1 is published upstream and provides a maintained optimized implementation. It is licensed `MIT OR Apache-2.0`. Only the protocol checksum calls depend on it; canonical and independent fixture tests guard replacement.

tracing supplies structured events and tracing-subscriber formats local terminal diagnostics. Both are MIT licensed and maintained in the Tokio tracing project. tracing enables only `std`; the subscriber enables `fmt` and `ansi` with defaults disabled. Their removal boundary is diagnostics and startup composition. No external collector or telemetry upload is configured. Distribution must retain applicable license and copyright notices for these crates and their transitive dependencies; installer notices remain a later packaging task.

Context7 resolved and queried `/tokio-rs/tokio` for UDP, select cancellation and watch semantics, and `/websites/rs_crc32fast` for IEEE CRC and the hash API. Versioned upstream documentation was checked for [Tokio UDP](https://docs.rs/tokio/1.53.1/tokio/net/struct.UdpSocket.html), [watch send_replace](https://docs.rs/tokio/1.53.1/tokio/sync/watch/struct.Sender.html), [crc32fast](https://docs.rs/crc32fast/1.5.1/crc32fast/), [tracing](https://docs.rs/tracing/0.1.44/tracing/), and [the subscriber](https://docs.rs/tracing-subscriber/0.3.23/tracing_subscriber/). Microsoft documents UDP truncation and ICMP error reporting in [recvfrom](https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-recvfrom). The buffer covers the maximum IPv4 datagram payload so oversize rejection does not rely on platform truncation behavior.

Cargo metadata with `--locked --offline --filter-platform x86_64-pc-windows-msvc` confirmed the versions, repositories and license expressions from local package metadata. An initial metadata query for all platforms attempted to download absent Unix dependencies and failed under restricted network access; the Windows filtered query passed. Windows compilation, Clippy, tests and release build passed using cached dependencies. A full security advisory database audit was not performed; these observations are not an advisory clearance or a claim about other targets.

## Spec 0003 dependency record (2026-09-05)

Android adds DataStore Preferences `1.1.7`, Lifecycle runtime Compose and ViewModel KTX `2.6.1` (matching the existing Lifecycle catalog baseline), and kotlinx.coroutines Android/test `1.10.2`. These are deliberate compatible pins, not claims to be the latest releases. Existing AGP, SDK, Compose, Activity and Kotlin plugin pins stay unchanged. Gradle builds and JVM tests pass; lint reports newer versions, recorded without a broad upgrade. DataStore provides transactional asynchronous preferences; Lifecycle owns ViewModel cancellation and lifecycle aware StateFlow collection; coroutines supplies I/O dispatch and a virtual test clock. Platform UDP, CRC32 and SecureRandom remain the transport/codec primitives. No networking, serialization or dependency injection framework is added.

AndroidX and kotlinx.coroutines use Apache 2.0 licensing and are maintained upstream. Retain applicable notices when packaging; a full transitive advisory audit was not performed. DataStore is isolated in `settings/`, Lifecycle in app/UI ownership, and coroutine transport orchestration in `network/`. The pure codec does not depend on Android or these libraries. The test coroutine module is not a runtime dependency. The DataStore preferences file is excluded from backup/transfer to avoid copying a client identity to another installation.

Checked [DataStore usage](https://developer.android.com/topic/libraries/architecture/datastore), [DataStore 1.1.7 release notes](https://developer.android.com/jetpack/androidx/releases/datastore#1.1.7), [Lifecycle releases](https://developer.android.com/jetpack/androidx/releases/lifecycle#2.6.1), [Compose state collection](https://developer.android.com/develop/ui/compose/state), and [coroutines 1.10.2](https://github.com/Kotlin/kotlinx.coroutines/releases/tag/1.10.2). Context7 `/websites/developer_android` queries returned only loosely related snippets, so official documentation and compilation were used to verify the actual APIs. Official [local network permission guidance](https://developer.android.com/privacy-and-security/local-network-permission) confirms `ACCESS_LOCAL_NETWORK` declaration/request for target/API 37 and UDP coverage. No network state permission is declared because no connectivity observer is used; loss is detected through socket errors or missing acknowledgements/status, without guessing a router/firewall cause.

The Windows JVM DataStore disk test encountered `Unable to rename ...preferences_pb.tmp to ...preferences_pb` on its second write. Local tests now exercise the repository persistence mapping with an in-memory DataStore, and `SettingsPersistenceTest` covers real disk recreation on Android. That instrumentation test was compiled, not run; physical persistence remains a manual gate. This is not recorded as a passing disk test or proof about Android file I/O.

Rust adds `getrandom = "=0.3.4"` with default features for OS random handshake nonces. It is a small maintained Rust Random project crate, licensed `MIT OR Apache-2.0`; Windows filtered Cargo metadata confirmed its version, license and upstream repository. The standard library does not provide the required stable OS cryptographic random fill API at this boundary. Its removal boundary is nonce creation in the development session handler. OS failure withholds acknowledgement rather than substituting predictable data. Checked Context7 `/rust-random/getrandom` and the [versioned fill API](https://docs.rs/getrandom/0.3.4/getrandom/fn.fill.html). No new unsafe application code or driver dependency is introduced. `Cargo.lock` records the transitive graph; standard checks and release compilation pass. A full advisory audit remains unperformed.

## Spec 0004 driver probe dependency record (2026-09-05)

The Windows target now pins `vigem-client = "=0.1.4"` with default features disabled. It provides Xbox 360 device creation and report submission, which Rust's standard library does not provide. The crate is MIT licensed, published 2022-08-01, and depends on the retired ViGEmBus ecosystem; it is a deliberately isolated legacy dependency, not a claim of current maintenance. Windows filtered, locked, offline Cargo metadata confirms its [upstream repository](https://github.com/CasualX/vigem-client), version and license. The official crate index records checksum `b857e6f99efe1e1eb1e4dfb035de8ae7ec8ec56bd1928edcbd7c6e4427634d52`, also captured by Cargo.lock. No unstable DS4 or notification feature is enabled.

The resolved runtime dependency is `winapi 0.3.9`, licensed MIT/Apache-2.0. Cargo.lock also records its two GNU target import library packages at `0.4.0`; they are not used by this MSVC build. The Rust client implements the driver protocol directly, without introducing the C/C++ ViGEmClient SDK or a separate native client DLL. Retain the applicable copyright and license notices in future distributions. Replacement should affect `virtual_gamepad/vigem.rs` and the Windows Cargo dependency only; domain reports, networking and session types must not import either crate. A full advisory database audit has not been performed.

Context7 could resolve only the C/C++ `/nefarius/vigemclient` library. Its lifecycle query supplied background, not the Rust API contract. Reviewed the [versioned Rust client documentation](https://docs.rs/vigem-client/0.1.4/vigem_client/) and downloaded source `client.rs`, `event.rs`, `bus.rs` and the Xbox target methods in `x360.rs`. Resource ownership stays in one adapter, with explicit cleanup of partially attached targets. The raw handle conversion APIs and unstable notification paths are not used. The application introduces no unsafe blocks.

The client uses blocking `GetOverlappedResult` calls for driver operations; a Tokio timeout cannot cancel them. `plugin` scans serial numbers on failure, and `wait_ready` has no caller supplied deadline. The adapter therefore cannot promise a bounded response if the driver itself stalls. The upstream event constructor also has a debug assertion on event creation failure. These limits remain visible at the gate; there is no silent fallback to keyboard/mouse or a claim of protection against an unresponsive kernel driver. Force killing a process can skip destructors and is explicitly warned about by the Rust client; actual host cleanup must be observed separately.

The user installed the verified official ViGEmBus setup `1.22.0`, which retains driver `1.21.442.0`. Windows inspection confirms that signed driver and a running ViGEmBus service on Windows 10 Pro 22H2 x64 build 19045.6466. Exact setup URL, SHA-256, signature and authorization evidence are recorded in `progress-tracker.md`. The driver source uses BSD-3-Clause licensing. No installer is bundled in this repository. Other operating systems, forced termination recovery and game compatibility remain separate manual gates.

## Spec 0004 integration APIs (2026-09-06)

No Android runtime dependency was added. Existing Compose pointer APIs route stable IDs at the surface boundary; existing AndroidX window insets APIs manage immersive mode and the platform window flag keeps the screen awake only in controller mode. Checked official [pointer handling](https://developer.android.com/develop/ui/compose/touch-input/pointer-input/understand-gestures), [immersive mode](https://developer.android.com/develop/ui/views/layout/immersive) and [screen awake guidance](https://developer.android.com/develop/background-work/background-tasks/awake/screen-on). Context7 Android results included older system UI flags; those were not adopted. Compilation verifies API availability, not physical gesture, focus or inset behavior.

Tokio remains pinned at 1.53.1. Its `test-util` feature is added only to dev dependencies for paused time timeout tests. Context7 supplied watch/cancellation background, checked against [versioned spawn_blocking documentation](https://docs.rs/tokio/1.53.1/tokio/task/fn.spawn_blocking.html). One blocking worker owns the synchronous adapter and is joined during shutdown. Watch channels hold only current requests, accepted input and health. A kernel driver call in progress cannot be aborted by a Tokio timeout; this remains a driver limitation, including during shutdown.

Android lint has nine dependency update notices and one `DiscouragedApi` warning for the spec's landscape lock. Newer Android large screen behavior can ignore fixed orientation; no compatibility claim is made from the manifest alone. Phone, large screen, enlarged text and API 37 execution remain manual verification work.

## Spec 0005 API verification (2026-09-06)

No dependency or version change. Context7 queries covered Tokio watch/select/cancellation and Compose AlertDialog. The versioned docs.rs requests were unavailable in this session, so the installed Tokio 1.53.1 UDP source and tracing-subscriber 0.3.23 MakeWriter source were checked directly. UDP send/receive remain cancellable asynchronous operations. Reply waits are bounded; synchronous driver operations still are not cancellable.

The terminal adapter uses Rust's standard bounded channel and nonblocking [SyncSender::try_send](https://doc.rust-lang.org/std/sync/mpsc/struct.SyncSender.html#method.try_send), with fixed event storage and no new logging dependency. The existing Material3 dialog API follows [Android's AlertDialog documentation](https://developer.android.com/develop/ui/compose/components/dialog). No runtime dependency, license or redistribution boundary changed. The targeted UI/UX skill searches had no relevant verified database match; the existing project UI guidelines and official Compose component were the fallback, with physical layout and accessibility execution still pending.
