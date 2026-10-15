# Spec 0010 — Desktop Tray and Packaging

Status: **Accepted**
Milestone: M3  
Depends on: 0009  
Primary changes: PC tray/application shell, installer, Android release config, user docs

## Outcome

Deliver an understandable local-product experience: a Windows tray agent with health/pairing controls, a repeatable x64 installer/uninstaller, and a signed Android APK pipeline. Runtime remains ordinary-user software after explicit setup elevation.

## Completion implementation decision

The user authorizes an installable local Android preview using the existing development signing setup. Sign a copy of the verified release APK with that existing key, preserving the release payload and comparing its certificate to the local debug APK. This does not create a key, alter official signing policy, prove the phone's installed identity or confer public release acceptance. Deliver only clearly named preview files under dist/preview. The user also authorizes removing this spec's redundant isolated build caches, duplicate review bundles and compiler download/probe files after preserving the final deliverables and checking process/startup references. Keep all source, notices, reusable compiler and pre-existing unrelated outputs.

The user authorizes compiler acquisition and review installer preparation. Compiler 6.7.3 is verified against its official distribution digest and signature, then installed using upstream portable mode in ignored build storage. Enforce its version with the preprocessor Ver constant because ISCC PE metadata is 0.0.0.0. Uninstall initialization may check ownership but must not remove anything; owned cleanup occurs only at usUninstall after confirmation. Cancelling the normal uninstall prompt preserves files, data, firewall and startup entries. Actual app installation, signing and physical acceptance remain separate from preparing the review package.

The tested preview implementation is accepted on 2026-10-14. This acceptance covers the user-reported tray, pairing, startup recovery, Stop/Start, installer ownership, and read only setup checks on Windows 10 and Windows 11 machines with ViGEmBus installed. It does not certify an official release. Clean-install variants, data-removal consent, cross-version upgrade, driver-absent coverage, settings persistence, signing, and broader platform/reliability checks remain named release gates below. Settings persistence is deferred to the later settings work as the user requested.

Setup uses Global\VirtualGamepad.Desktop.Setup and the runtime retains Global\VirtualGamepad.Desktop.Runtime until cleanup finishes. This conservatively blocks setup while any current runtime copy is present; the exact executable process check also protects upgrades from older copies without that marker. Runtime refuses to start during setup. The unchecked postinstall launch uses --tray-after-setup, waits at most 120 seconds for setup to close, then starts normally. Setup never force kills a controller owner. Inno's native mutex checks and the custom cleanup still need real elevated/cross-account testing.

Uninstall identifies the account and exact LocalAppData path in its default-No data removal prompt. It only removes that account's owned startup value after exact path validation. Other accounts retain their data and startup entries; those users should disable startup in their own session before uninstall. This intentionally avoids scanning other profiles. The compiler is pinned to 6.7.3 and its official distribution digest, valid signature and bundled license were verified before portable use. The build script requires an explicitly supplied verified-origin compiler SHA-256 and the script checks the exact compiler Ver.

Read only preflight inventories Windows, ViGEmBus service/file version and the owned firewall rule, with a five second child-process bound. It cannot prove driver usability or actual packet reachability. The existing backend supplies usability evidence on pairing. The tray reports XInput slot as unavailable because the existing backend exposes no slot index; this avoids adding a new driver request to the approved attachment path. Network counters are shown, with packet loss labelled as recorded evidence rather than a latency diagnosis.

The user now authorizes completing all remaining implementation before presenting one consolidated manual checklist. Their supplied terminal status before/after pairing and clean exit are accepted as user evidence. Three input timeouts in that run remain unresolved. Do not repeat accepted phone restart, background or touch checks merely to close this spec. New tray stop/start, setup ownership and signed artifact upgrade behavior still require their own evidence. Installation, key creation, publication and device campaigns remain separate actions.

The tray uses native labelled menu items and Windows dialogs, with keyboard navigation and confirmation for forget/replace. Settings are an explicitly opened, documented version 1 text file; Reload settings applies validated values only while stopped. This keeps settings usable without introducing a second UI framework. Values are port, timeout_ms, discovery and log_level; startup reflects the actual current user Run entry, not a second stale preference. Invalid or unknown settings are preserved and reported, never silently defaulted. Logs rotate at 1 MiB with three backups through the existing bounded writer.

The main thread owns tray-icon and a Win32 message loop. A 200 ms thread timer samples immutable status and pairing snapshots, with no per-packet UI event queue. Menu events use capacity eight, with a separate exit flag. Pairing snapshots are a distinct memory only type without Debug; expiry, reservation, closure and shutdown invalidate visible codes. Closing an unused pairing window does not revoke an established session; explicit pair/forget/replace retain their existing revocation policy.

User feedback on 2026-09-13 replaces the muted menu code with one native modeless pairing window. Use large bold six digit text, enabled system contrast colours, explanatory text and a seconds countdown rounded up until expiry. The existing UI timer owns its refresh even during nested menu loops. No second runtime or UI framework is needed; a static MessageBox cannot provide the required live countdown. Show the window on a new authoritative pairing snapshot. Close, Escape or Alt+F4 cancels that unused window through the bounded supervisor queue; retry a busy queue without reopening the dismissed code. Acceptance, reservation, expiry, Stop and Exit clear the window. No code enters the window title, ordinary status or logs. Native handles and fonts are owned and released on the main thread.

Stopped and Starting status shows the validated configured port, not the previous service's binding. Running status shows the current socket's actual port, published together with its Running transition. Reload remains stopped only and does not restart listening or change firewall rules. Invalid settings continue to block Start without changing the file. The reported successful rejection of timeout_ms=99 and recovery after restoring defaults remain accepted.

The user reports startup enabled launches after reboot and startup disabled correctly prevents launch. Their subsequent screenshot identifies the prompt as Windows Defender Firewall's network access alert for the new review executable, not UAC runtime elevation. Preserve asInvoker and the exact owned HKCU Run command. Popup and port display retests are user accepted on this host. Exact firewall scope and operation after scoped setup remain unverified; the alert's Private checkbox alone does not establish the required UDP port and LocalSubnet restrictions. Do not bypass Windows security or broaden firewall rules to suppress the prompt.

The desktop supervisor retains the pairing store ownership handle across Stop and creates only one application instance at a time. It duplicates that owned file handle for the joined storage worker, never reacquires a competing lock. A new run reloads the existing validated record; missing/corrupt records after initialization fail closed. CLI overrides remain available in terminal mode. No output, timeout or protocol algorithm changes are intended.

Installer source targets pinned Inno Setup 6.7.3, without executing its installer. The setup requires the owned runtime to be closed, offers an unchecked Private UDP firewall task, uses no driver bundle, and retains data by default. Ordinary user startup configuration stays in the tray. Compiler availability and release credentials may leave compilation/signing pending, but source and validation scripts must be complete and reviewable.

## Architecture

### Windows 11 startup recovery (2026-09-14)

The user authorizes this focused repair, using debug, architect, develop and sync on the current model only. The supplied Windows 11 x64 evidence shows working packet exchange and installed signed driver 1.21.442.0, but `wait_ready` fails with Windows error 483 and pairing is refused. Driver presence does not prove target readiness. The failing machine's underlying boot cause remains unknown.

Verified release source: [EmulationTargetPDO.cpp at v1.21.442.0](https://github.com/nefarius/ViGEmBus/blob/v1.21.442.0/sys/EmulationTargetPDO.cpp), blob `26e7919c53f03e0eea837eb6576cd1409d2aae55`, and [XusbPdo.cpp](https://github.com/nefarius/ViGEmBus/blob/v1.21.442.0/sys/XusbPdo.cpp), blob `1af3d296927d123e6877199a9fea11f756cd133c`, are identical at v1.22.0. The readiness worker waits one second and completes a timeout with STATUS_DEVICE_HARDWARE_ERROR, then clears the boot notification event. The pinned vigem-client 0.1.4 creates a new WaitDeviceReady request and OVERLAPPED for each call, reuses a target's auto reset completion event, and synchronously waits in GetOverlappedResult. Its plugin also internally scans serial slots. None of these calls has an application cancellation guarantee. Context7 supplied the C++ SDK, so the downloaded exact Rust crate source governs Rust behavior.

Select a fix inside the existing adapter: after the first wait-ready WinError(483) only, attempt neutral and explicit unplug, release the old target/client only after unplug succeeds, then create one fresh target/client and make one new readiness request. There are at most two application target attempts per attach. Do not retry connect, plugin, neutral or other readiness errors, or any cleanup failure. Preserve the first error and any cleanup errors. Repeated wait_ready on one target is rejected because the driver's event is cleared and is not a durable readiness flag. Neutral polling is rejected because unchanged neutral reports can return success without proving boot. A driver/client replacement would exceed this repair's evidence and scope.

The output owner supplies a current-request predicate, checking sender closure, session identity and the existing request deadline. Check it before and between startup calls and again before publishing readiness or applying input. Cancellation prevents subsequent startup/retry work and cleans the partial target after any in-flight native call returns. Initial neutral must succeed before activation. No live gameplay state may cross a startup cancellation or expired grant. Keep the existing joined blocking owner, single target and fail-closed fault lifecycle; Stop/Start creates a fresh service only after cleanup. No wall-clock startup or shutdown bound is claimed.

Diagnostics carry only operation, optional native error number, target attempt number and cleanup-failure fact into latest output/service status. Preserve the complete original/cleanup error in local logs. Error 483 at wait-ready explains a possible controller boot timeout, not a missing driver diagnosis. Terminal status and tray details show the diagnostic and Stop/Start plus fresh-code recovery guidance. The tray requests one nonmodal native notification per faulted service run, using its own per-run icon GUID and existing HWND through Shell_NotifyIconW. Persistent status remains available if Windows suppresses notifications. No notification loop, modal failure dialog, new dependency, settings mutation or wire field is needed.

Focused acceptance criteria:

1. Scripted adapter tests cover first success, 483 then fresh-target success, repeated 483, other failures, failed cleanup and cancellation between calls. No second target exists before successful old-target unplug/drop; no nonneutral report is used during startup.
2. Output tests prove neutral before readiness, no readiness/input after cancellation or expiry during a blocking attach, retained failure facts and joined Stop/Exit cleanup. Tests use fake calls and isolated storage/loopback only.
3. Diagnostic and notification reducers preserve operation/code/attempt/cleanup facts, notify once, and reset on a new service. Pairing secrets never enter these values. Protocol, fresh-code pairing, strict 60 Hz, default 250 ms neutral, brief-gap recovery and three-second detach stay unchanged.
4. Format, Clippy, all-target check/tests and focused release admission checks pass. The user reports the focused Windows 11 retest succeeded with the preview installer and pairing no longer failed with error 483. No driver installation, firewall/security changes, phone interaction or runtime campaign was performed by this work. Previously accepted checks and all open reliability/release gates stay as recorded. Spec 0010a remains Proposed.

### Decision and authorized boundary (2026-09-12)

The user accepted Spec 0009's focused checks and authorizes completing this design and its smallest coherent implementation slice. Spec 0010 remains in progress until its later tray, packaging and physical acceptance criteria pass. Formal Spec 0005/0007 campaigns, broader platform/accessibility evidence, delivery delays and the recorded neutral timeout remain unresolved. No automatic campaigns, device interaction, installation, firewall change, signing key creation or publication follows from this authorization. Spec 0010a follows this spec and remains unimplemented.

The implementation approach is a thin working path through the existing terminal and a shared service first. Keep Rust/Tokio and the existing backend. A Windows service under SCM would add privilege, session and IPC complexity without helping this one user tray application. A separate tray process with duplicate receiver logic would introduce competing ownership. Select one ordinary user process with interchangeable terminal and tray adapters instead.

### Service ownership and first slice

- `application::Application` exclusively owns the UDP socket, locked pairing store, discovery lifetime, admission handler, latest input channel and output task. `main.rs` selects the presentation and logging, then awaits the service. Terminal code owns only input/output and service handles.
- Preparation validates release trust before loading storage, loads the record on blocking work outside UDP, and binds the socket before reporting Running. Preparation failure never starts output or discovery. Corrupt, inaccessible, unknown version or locked pairing records fail closed and are preserved. There is no default identity fallback or automatic restore.
- `ApplicationHandle` submits existing pairing commands through a capacity eight nonblocking queue. Full/closed queues return a retryable presentation error. Exit uses a separate latest value shutdown signal so command saturation cannot prevent shutdown. EOF closes terminal input without stopping the service. Losing all service handles requests shutdown.
- Immutable service status uses a Tokio watch channel. It exposes lifecycle, bound endpoint, local IPv4 candidates, development mode, and redacted backend readiness/attachment/timeout/fault facts. No session ID, nonce, code or persisted client ID enters ordinary status or tracing. Running means the socket is served, not that the driver or firewall is verified. Addresses are candidates from local enumeration, not proof of reachability or Private profile.
- The terminal retains its bounded, memory only historical code output channel. The tray reads a separate authoritative PairingView watch snapshot with no Debug implementation; accepted, closed, expired and stopped windows clear the displayed code.
- Exit, signal errors, receive errors and unexpected output completion all close admission/socket first, join neutral/detach output cleanup, stop discovery and join pending storage work before reporting Stopped or Failed. Keep the Tokio runtime alive through cleanup; never abort or drop the running service future to exit. A blocked native driver call can still delay cleanup, so no fixed shutdown duration is promised and installers may never force kill it.
- Preserve `pair`, `forget`, `replace`, Ctrl+C, debug filtering and the quiet Running/IP/port summary. Add on demand `status` and `exit`. Terminal status is pulled only when requested, with no periodic display thread. No new dependency is needed for this slice.

### Tray event loop decision

Select `tray-icon` with a narrow native Win32 message loop on the process main thread. Context7 `/tauri-apps/tray-icon` and upstream documentation confirm that Windows icons need an event loop on their creating thread. Keep creation, menu mutation and destruction on that thread. A separate Tokio runtime thread owns the application; backend calls retain their existing blocking output owner. A full webview or another cross platform window framework is unnecessary for the initial tray menu.

Use `GetMessageW`, check its error return separately from quit, and dispatch messages on the icon thread. Isolate unsafe window procedures and handle lifetimes in a Windows adapter. Install menu event handlers that use `try_send` into the bounded application command channel; do not accumulate the library's default event queue. Coalesce service status into one latest snapshot. A 200 ms thread timer reads presentation snapshots; its TIMERPROC refreshes code expiry even inside native menu/dialog message loops. Counters are copied at 500 ms intervals. Do not post one window event per UDP packet. Closing a settings window keeps the tray alive; Exit keeps the message loop responsive until service cleanup completes, then removes the icon and quits. Event loop failure requests and awaits the same shutdown path.

The implementation pins `tray-icon` 0.25.0 and its transitive graph in Cargo.lock after version source, license and native build inspection. Upstream uses MIT OR Apache-2.0; required menu dependencies need their own notice inventory. Its removal boundary is tray presentation. No advisory clearance or Windows native execution is claimed by this research.

Before building tray Start/Stop, add a supervisor that retains the per user ownership lock while stopped and serializes restart after complete cleanup. Start during stopping is refused. A second instance fails clearly instead of stealing the port/store; no cross process control endpoint is introduced. Settings apply port/discovery changes only while stopped. Timeout remains validated at 100 to 1000 ms. Backend faults require a fresh service instance, without reviving the old session.

### Pairing and timing reconciliation

Every new production connection requires a fresh six digit code, including a saved phone and either application restarting. `pair` revokes the active session before opening a new 120 second window. Saved identity is informational metadata only. The tray labels it as the last paired phone and confirms forget/replace; network traffic never invokes these actions. Closing the pairing window cancels its challenge/code without creating admission. A code is cleared on acceptance, closure, expiry, stop and exit.

The state rate is fixed at strict 60 Hz with bounded full snapshots. No 90/120 Hz setting is part of Spec 0010. Preserve exactly 40 byte packets and fixtures, release trust, isolated ViGEm, the default 250 ms neutral deadline, same session brief gap recovery and terminal detach at three seconds. Android gameplay and pointer ownership do not change.

## Tray behavior

- Visible states: stopped, starting, ready/unpaired, pairing window open, connected/healthy, connected/degraded, timed out, driver issue, firewall/setup issue, shutting down.
- Menu/actions:
  - Start/stop listening.
  - Show local IPv4 addresses and UDP port.
  - Open/close pairing window and show current six-digit code/expiry.
  - Show last paired phone metadata and forget/replace action with confirmation; a fresh code is still required for every connection.
  - Show controller slot/status/counters.
  - Open settings/log folder/troubleshooting.
  - Toggle start with Windows; default off.
  - Exit, which neutralizes/detaches before termination.
- Notifications are sparse and actionable, not emitted per reconnect/packet.
- Closing any settings window does not accidentally exit the agent unless explicitly selected.

## Settings and logs

- Store validated port, timeout (100–1000 ms), discovery toggle, log level in a versioned per-user settings file; start-with-Windows is read from its separately owned HKCU Run entry. State rate stays fixed at 60 Hz. Keep the existing independently versioned pairing record separate.
- Safe defaults: UDP 8765, 250 ms timeout, 60 Hz, discovery on after permission/firewall setup, startup off.
- Write ordinary settings atomically. Preserve invalid settings and report the error; offer explicit restore/default recovery while stopped. Pairing records always fail closed on corruption or access failure and are never silently reset or replaced by settings recovery.
- Logs are local, bounded/rotated, redact pairing/session secrets, and include build/OS/driver context useful for support.

## Driver preflight

- Detect whether the exact supported ViGEmBus version/service is usable.
- If absent/broken, show the official source/version/checksum/recovery steps and keep the app neutral.
- Do not bundle the driver. Use detection plus official guided installation with separate authorization. Redistribution and unattended installation have not been cleared.
- Do not claim a supported driver replacement exists without a passed backend spike.

## Windows installer recommendation

Select Inno Setup for this consumer x64 installer. There is no enterprise MSI/repair requirement to justify WiX. Compiler version, license terms and checksum must be selected and recorded before installing or using the compiler. No compiler or installer is run in the first slice.

Use one stable installation ID `VirtualGamepad.Desktop`, `{autopf}\VirtualGamepad` for owned binaries/docs, and ordinary runtime data under the user's LocalAppData/VirtualGamepad. Setup is elevated; the runtime executable has an `asInvoker` manifest. Both terminal and tray presentation should use the same installed executable path, preserving one firewall target. Terminal remains the default; an explicit `--tray` shortcut/startup argument selects the native tray path. Upgrade preserves data and requires the user to exit the agent first. Never use force close, process name kill or broad process termination. Until graceful installer coordination is proven, setup/uninstall refuses to proceed while the owned executable is running.

The eventual owned firewall identifier is `VirtualGamepad.Desktop.UDP`, inbound Allow, UDP, Private only, explicit installed program, configured local port and `LocalSubnet` remote scope. Installation of the app does not imply consent to this rule. Skip leaves manual setup guidance. Upgrade modifies only that exact owned rule after validating its program path. No Public rule or driver bundling. Detect official ViGEmBus installer 1.22.0 and the previously observed driver 1.21.442.0 distinctly; service/version presence is informational, actual usability comes from the existing backend at connection time. Do not create a controller for preflight.

Startup is a current user Run entry named `VirtualGamepad.Desktop`, quoted full installed path with `--tray`, off by default. The ordinary user runtime owns this toggle, avoiding an elevated installer's different HKCU identity. Setup may explain it but does not silently enable it. Uninstall removes only owned files/rule/startup values after path validation, preserves user data by default, and offers data deletion only for the identified current user with explicit consent. Do not scan/delete other users' profiles or remove the shared driver.

Installer tasks:

1. Verify supported Windows 10/11 x64.
2. Install versioned binaries/resources/docs in a standard owned directory.
3. Perform driver preflight and open/run only explicitly approved prerequisite flow.
4. With user consent/elevation, add a program-scoped UDP inbound rule for Private networks only.
5. Explain the optional current user start-with-Windows toggle in the runtime; it remains off until explicitly enabled there.
6. Add Start Menu and uninstall entries; avoid unwanted desktop shortcut by default.
7. Launch agent optionally after install.
8. Log setup actions without secrets.

Uninstaller tasks:

- Require the owned agent to exit through its neutral/detach path before removing files; refuse to proceed while it remains running.
- Remove only files, startup entry, and firewall rule owned by this installation.
- Offer whether to remove the identified user's settings/pairings/logs, preserving them by default and validating exact owned paths.
- Do not remove a shared virtual driver automatically unless its ownership/reference policy is explicitly proven safe.

## Windows signing

Signing and publication are separate gates. Before packaging, confirm the maintainer's copyright attribution, stable official repository/release identity, dependency compatibility, required third party notices, Authenticode certificate availability/provider and release policy for unsigned artifacts. Apache License 2.0 is selected. On 2026-09-13 the user authorized a suggested provisional credit, Virtual Gamepad contributors. LICENSE/NOTICE and dependency notices are now prepared from Cargo metadata, resolved Android POMs and upstream license texts; official identity and final redistribution review remain publication gates. Community forks cannot update official artifacts; no automatic updater is included.

Prepare a local artifact inspection command that reads a candidate executable, computes SHA-256 and reports Authenticode state without signing, launching or installing it. An unsigned build is review material only. Later release preparation signs the versioned executable first, packages it, signs the installer with SHA-256 and a trusted timestamp, verifies both signatures, then computes final artifact checksums. Reproducible inputs do not imply byte identical timestamped signatures.

- Build reproducible release binary with version metadata.
- Prefer Authenticode signing for executable and installer.
- Keep certificate/private key outside repository/CI logs.
- If no code-signing certificate is available, document SmartScreen/reputation warnings as a release limitation and do not call the install warning-free.
- Publish SHA-256 checksums.

## Android release pipeline

- Finalize stable application ID and versioning.
- Obtain explicit authorization before creating a release keystore outside repository; confirm the owner and backup/recovery plan first. Existing application ID `com.hamzalgh.virtualgamepad` remains the baseline, not a copyright attribution.
- Inject signing values from user-local Gradle properties/environment.
- Build signed release APK, verify signature/install/upgrade, generate SHA-256 checksum.
- Provide clear permission, same-Wi-Fi, pairing, battery/keep-awake, and troubleshooting guidance.
- Recheck Android developer verification/registration requirements and target SDK policy at release time.

Missing signing values must fail an official release task rather than falling back to the debug key. A separately named unsigned review build remains possible. `assembleRelease` builds unsigned review material by default. `-PofficialRelease=true` requires VG_ANDROID_KEYSTORE, VG_ANDROID_STORE_PASSWORD, VG_ANDROID_KEY_ALIAS and VG_ANDROID_KEY_PASSWORD from the environment and rejects a keystore within the repository. Signing identity is verified separately with verify-apk.ps1.

## Build plan

1. First slice: extract the application owner, wire terminal pairing/status/exit, and test shared lifecycle with fake output, loopback and isolated temporary storage. Add reviewable packaging prerequisites and read only artifact inspection. Acceptance: terminal commands stay bounded, exit bypasses a full queue, output errors stop UDP, cleanup is joined, corrupt storage remains untouched and release cannot enable development trust.
2. Add authoritative pairing/status snapshots, stopped ownership and restart supervision, then native tray/menu event loop. Verify pure reducers and command failure states before requesting an authorized native tray check. Full tray status/menu requirements above belong to this slice.
3. Add validated ordinary settings, rotated secret free local logs, ordinary user startup ownership and read only driver/firewall preflight. Test migration/corruption boundaries and disabled startup defaults.
4. Prepare and statically check the installer source, owned identifiers, executable manifest/version metadata, dependency notices and signing configuration. Resolve the external identity/certificate/compiler prerequisites before executing any signed packaging or setup flow.
5. With specific authorization, execute clean install/upgrade/uninstall and physical signing/platform checks. Existing M2 and wider evidence gaps remain independent gates. Only then consider full Spec 0010 acceptance.

## Research basis (2026-09-12)

Context7 returned relevant tray thread and event handler guidance. Verified upstream [tray-icon documentation](https://docs.rs/tray-icon/0.25.0/tray_icon/) and [Windows message loop documentation](https://learn.microsoft.com/en-us/windows/win32/winmsg/using-messages-and-message-queues) govern the native design. Installer privilege and running application handling were checked against [Inno Setup privileges](https://jrsoftware.org/ishelp/topic_setup_privilegesrequired.htm) and [CloseApplications](https://jrsoftware.org/ishelp/topic_setup_closeapplications.htm). Signing prerequisites were checked against [Microsoft SignTool](https://learn.microsoft.com/en-us/windows/win32/seccrypto/signtool) and [Android app signing](https://developer.android.com/studio/publish/app-signing). This is design research, not runtime or legal release clearance.

## Automated tests

- Application service behavior is identical under terminal and tray adapters.
- Tray status reducer and every command/error state.
- Config defaults, bounds, atomic update, migration, and corruption recovery.
- Startup task disabled by default and idempotent when toggled.
- Installer script static checks and owned firewall/startup/uninstall identifiers.
- Release builds exclude manual trust, debug overlay, secret fixtures, and verbose packet logs.
- APK release manifest/permission/signing configuration validation without exposing keys.

## Manual clean-machine matrix

## Consolidated manual checklist

Existing accepted terminal status/exit, phone restart/background behavior and gameplay button/touch checks are not requested again. The reported terminal run had 2,014 updates, zero failures and three timeouts; that is clean shutdown evidence, not a clean network/reliability result. Do not run Spec 0005/0007 campaigns automatically.

The following is the complete remaining checklist for this spec. Items involving unavailable signed artifacts, installers or another Windows version are later release gates, not instructions to improvise keys or reinstall a driver now. Record failures in the existing tracker, not a new campaign/handoff file.

1. **Native tray and keyboard.** Exit any existing agent normally and check Task Manager before launching the new executable with `--tray`, as an ordinary user. Expect one icon, no persistent console, readable menu labels, keyboard navigation with Enter/Escape, and sensible labels at your normal and enlarged Windows text scale. Open/close settings, log folder, details and troubleshooting; the tray stays alive. Check setup inventory once; it reports facts or a bounded unavailable/error result without a UAC prompt or system changes. A controller slot may be unavailable; version, endpoint, backend status and counters must remain honest.
2. **Pairing window, without phone input.** Choose Pair, observe a popup with large six-digit text and a decreasing seconds countdown. Close with its button, X or Escape: the popup disappears without exiting the tray or reopening the same code. Open another window and leave the tray menu open through its 120-second expiry: the popup disappears there too. The earlier Cancel Forget/Replace checks remain accepted and need not repeat. Codes must not appear in agent.log or status details.
3. **One tray connection and Stop/Start.** Recheck the PC's current Wi-Fi IPv4 immediately before entering any phone address. Connect using the tray's fresh code; expect the code to clear and attached/ready status. Choose Stop: the phone loses admission and the virtual controller disappears after cleanup. While stopped, a second agent must refuse ownership; close its error and keep the original tray. Start: expect ready/unpaired, with no automatic saved-phone admission. Use a fresh code once more, then Exit: the tray and controller disappear after cleanup. No button presses, game campaign, phone backgrounding or phone restart are needed for this check.
4. **Settings and recovery.** While stopped, change settings.ini to an unused port, a valid timeout and discovery=false; Reload and Start must use the validated values. Stop and restore 8765/250/true (and log_level=info). Reload a deliberately invalid ordinary settings value: listening stays stopped, the error explains the problem and the file is unchanged. Restore the valid value and reload. Never modify or delete pairing.bin for this test; corruption preservation has automated coverage. Settings edits do not change firewall rules. Do not attempt phone connection on an unapproved alternative port.
5. **Startup, when you authorize this user setting.** Confirm off initially. Enable Start with Windows from the intended stable executable path, sign out/in or restart Windows, and expect one tray instance under that same ordinary account. Disable, repeat sign-in and confirm no automatic launch. This is the new Windows startup feature, not a repeat of phone restart testing. Disable startup before moving a portable build.
6. **Windows setup, once a reviewed installer exists and installation is authorized.** On Windows 10 and Windows 11 x64, cover driver-present and driver-absent machines without removing a working shared driver merely for testing. Missing driver guidance should identify the exact supported source/version/hash and stay neutral. Verify runtime starts without elevation. Decline the optional firewall task first: no owned allowance appears. In the consented case verify only the installed program, UDP 8765, Private profile and LocalSubnet are allowed; no Public allowance. Existing unrelated rules remain unchanged. Test the optional postinstall launch only after setup closes.
7. **Upgrade/uninstall, on those authorized test installations.** Setup/uninstall must refuse a running agent without force-killing it. Exit normally, upgrade, and confirm settings/last-phone metadata survive while a fresh code is still required. Uninstall with default No preserves data; in a separate disposable installation, explicit Yes removes only the displayed account's owned data. Owned files/rule and matching startup entry are removed, unrelated entries and the shared driver remain. A different elevated account must not delete the ordinary user's profile; disable startup in each relevant account beforehand.
8. **Signing and Android delivery, after credentials exist.** Verify expected Windows executable/installer signer, trusted timestamp and final SHA-256. Verify the APK's expected release certificate and that it is not debuggable. On an authorized physical Android installation, verify fresh signed installation and upgrade from an older APK signed by the same release key, with settings preserved. A debug-signed installation cannot serve as that upgrade baseline: do not uninstall or erase it to force success. Inspect permission/same-Wi-Fi/pairing guidance and bundled license notices; existing gameplay checks need not be repeated. Certificate availability, backup/ownership and official download identity must be settled before publication.

The tested preview path is accepted with the evidence and named gates recorded below. These items remain release gates where their evidence is still pending. Broader device/accessibility evidence and formal reliability campaigns remain separately tracked.

### Preview acceptance and remaining release gates (2026-10-14)

User supplied evidence confirms the current preview can pair and operate on Windows 11 x64 and Windows 10 x64 with ViGEmBus present. On the Windows 11 test PC, the reported error 483 startup failure was resolved by the preview recovery. On the Windows 10 PC, Status and setup details reported Windows 10 Pro 10.0.19045, ViGEmBus service Running, driver 1.21.442.0, and the owned inbound UDP 8765 firewall rule enabled for Private with RemoteAddress LocalSubnet and the installed executable path. The later ready but detached backend status is expected while awaiting a fresh pairing. The user also confirmed Stop/Start behavior, tray removal after Exit, setup and uninstall refusal while the app is running, and that choosing No at the uninstall confirmation leaves the app installed. The setup inventory is displayed when Status and setup details is opened after using the read only check command; there is no immediate separate notification from that command.

The following remain release gates and are not required to accept the tested preview implementation: settings persistence and invalid-settings recovery, deferred to separate settings work; clean install with firewall consent declined and accepted; postinstall launch; explicit Yes data removal and owned firewall/startup cleanup; a real version-to-version upgrade; driver-absent guidance and coverage; Windows signing and timestamp; official Android signing, fresh install and upgrade; broader accessibility/platform evidence; and the formal Spec 0005/0007 reliability campaigns. Do not reset pairing storage to test corruption. Keep discovery reliability tracked separately. Official publication and signing remain separate authorization steps.

### Platform coverage required for release

- Windows 10 x64 with driver absent and present.
- Windows 11 x64 with driver absent and present before supported release.
- Private network firewall consent/allow; Public profile does not get an open rule.
- Install, first run, pair, controller use, reboot with startup off, enable startup/reboot, disable startup.
- Upgrade over prior version with pair/config migration.
- Uninstall preserving/removing data choices; owned rule/startup/files removed.
- Signed APK fresh install and upgrade on physical Android device.

## Exit criteria

- Tray operates the proven core without duplicate logic/regression.
- Installer and uninstaller pass clean Windows 10 evidence; Windows 11 evidence is required before final release.
- Runtime works without admin after setup.
- Firewall rule is Private/program scoped and owned cleanup passes.
- Auto-start is off by default.
- APK is signed outside repository and verifies/installs/upgrades.
- Driver redistribution/signing/SmartScreen limitations are explicit.

## Out of scope

- Automatic self-update, cloud crash reporting, app stores, multiple controllers, keyboard/mouse mode, driver development.
