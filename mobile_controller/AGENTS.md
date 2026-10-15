# Android Package Instructions

These rules apply under `mobile_controller/` in addition to the repository instructions.

## Toolchain and ownership

- Kotlin and Jetpack Compose only; preserve a standard Android Studio/Gradle project.
- Use Android Studio for sync, run, Logcat, profiling, and device debugging.
- Baseline: `minSdk 26`; compile/target SDK and build tools are pinned in the project according to `context/library-docs.md`.
- Do not replace the project with Flutter, React Native, a game engine, or a custom VS Code build flow.

## Architecture

- Use a single-activity Compose app and unidirectional data flow.
- ViewModels expose immutable `StateFlow`; UI collects lifecycle-aware state.
- Keep these concerns separate: UI/setup, touch routing, pure controller model, protocol codec, UDP session, and preferences.
- The gamepad screen uses one coordinated multi-touch router. Track stable pointer IDs, not pointer indexes, and assign each pointer to at most one control until up/cancel.
- Touch geometry, dead zone, sensitivity, normalization, bitmasks, and encoding must be pure/testable where practical.
- `GamepadRouter` is the sole gameplay owner for all controls, with at most one pointer per control. Geometry/tuning/reset changes cancel ownership; a held ignored pointer cannot capture by moving. The sender samples one immutable full controller state.
- `StickTuning` validates dead zone `0.00..0.30` (default `0.15`) and sensitivity `0.5..2.0` (default `1.0`). Apply magnitude to the power `1/sensitivity` after radial rescaling; the visual knob uses untuned displacement. Persisted invalid values fall back to defaults.
- Spec 0008 tuning uses a local draft and preview. Apply saves both values in one DataStore transaction before publishing an immutable tuning pair with neutral state and a new input epoch. Reset changes the draft; Apply persists it. Storage failure retains the previous pair and permits retry without reconnecting.
- Use `GamepadGeometry.stickTravel` / `rightStickTravel` for input normalization and `stickKnob` / `rightStickKnob` for rendering. Full magnitude occurs when the knob contacts the base boundary; using the larger base radius for input makes the knob lag behind the finger.
- Spec 0007 placement follows the supplied reference with the left stick outside and D-pad inward. `GamepadGeometry.regions` is shared by capture, passive visuals and debug overlay; reject insufficient space or any gap below 8 dp. D-pad has one owner, eight sectors and a neutral center; trigger rails use mirrored absolute horizontal position, clamped during drag and zero on release. L3/R3 have dedicated regions.
- The updated reference already contains the requested left stick/D-pad swap. Spec 0008 retains compressed main-control sizes below 320 usable dp height; larger bounds scale the reference controls up to 1.25 times the 640 by 320 design. Only subpixel Float rounding is snapped at boundaries; containment and gap rejection remain active for every build.
- Network I/O and the fixed-rate sender run off the main thread and are cancelled with their lifecycle owner.
- Cadence snapshots exclude cleanup sends and publish at most twice per second. Keep detailed diagnostics behind the Diagnostics action so changing counters cannot resize the controller surface; opening diagnostics cancels held touch ownership.
- Gameplay exposes only the integrated gear/Back/Start/X navbar; status and development trust text live inside Settings. Diagnostics and the debug-only touch switch live inside Settings. Opening Settings neutralizes input and restores system UI; the existing sender continues neutral snapshots. Unsupported geometry also clears keep-awake/immersive policy. Setup/settings use full system text scale; fixed gameplay labels cap visual scale at 1.3.
- Persist Show L3/R3 independently of tuning. Hiding removes both captures and visuals, clears ownership, and masks L3/R3 bits at the ViewModel boundary; other geometry stays fixed.
- Session health uses admitted PC status, never local send success: one second of silence is degraded, three seconds closes the session, and reconnect remains explicit. Echo age is diagnostic state age, not pure network RTT.
- Spec 0007 timeout status with backend ready and controller attached means recoverable Degraded input. Keep the same sender/session and touch surface active; terminal timeout, fault or status silence still closes it. Do not reset sequence or negotiate a new session for a brief input gap.
- Strict 60 Hz samples only the current state. Very short presses/releases between ticks can be missed; the user accepted this limitation in Spec 0004. Do not introduce change-triggered sends or pulse stretching without a new explicit decision.
- Keep the single `connectionDataStore` delegate as the preferences owner. Preserve client identity across local restarts, exclude it from backup/transfer, and never persist sessions or handshake nonces.

- Spec 0009 persists accepted server identity/name in that same DataStore before starting snapshots after PairAccept. Codes remain memory only. Source, nonce and deadline checks precede acceptance; pairing rejection never refreshes session liveness.
- `PcDiscovery` owns platform NSD on the main thread; `DiscoveryCatalog` bounds services to 32 and rejects stale lookup tickets. Discovery is explicit and stops on connect, cancel, background or local network change. Old Android permits only one uncancellable lookup until its callback returns; manual IP remains available. Never infer native NSD or API 37 permission acceptance from JVM tests.
- Kotlin protocol tests consume `../../protocol/fixtures/v1/manifest.txt` from the app test working directory; keep fixtures shared with Rust rather than copying them into Android resources.
- Pre-session status has no nonce and must not refresh reachability. Only a matching fresh hello acknowledgement or admitted active-session status may extend the connection deadline.

## Lifecycle and safety

- Lock the controller activity to landscape without sensor-driven rotation.
- Use immersive edge-to-edge UI and consume safe drawing/gesture insets. Never hard-code around a particular notch or phone.
- Keep the screen awake only while controller mode is active; clear the flag on exit, pause, error, or disconnect.
- On pause/stop, lock, touch cancellation, socket failure, or Wi-Fi loss: neutralize local state, stop the sender, and best-effort send the documented disconnect/neutral sequence.
- The PC timeout remains the final safety authority.
- Request local-network permission where the target Android version requires it and explain why in UI.

## UI rules

- Follow `context/ui-guidelines.md`; do not invent a parallel token system.
- Prioritize simultaneous touch reliability, reachable controls, high contrast, and immediate pressed feedback over decoration.
- No vibration/haptics. No Guide button.
- Store only non-sensitive settings locally: PC address/port, dead zone, sensitivity, and user preferences.

## Dependencies

- Prefer Android/Java UDP and CRC APIs.
- AndroidX Lifecycle, DataStore, Activity, and Compose are allowed when their role is explicit.
- Do not add DI, database, serialization, or networking frameworks for functionality the platform already handles clearly.
- Check the current API in Context7 when available and confirm platform behavior in official Android documentation.

## Verification

- `assembleRelease` remains unsigned unless explicitly invoked with `-PofficialRelease=true` and all VG_ANDROID_* signing environment values. Official signing uses a keystore outside the repository and never defaults to the debug key. Use `--no-configuration-cache` for credentialed builds. `exportReleaseDependencyInventory --no-configuration-cache` writes ignored release dependency metadata for notice preparation; keep bundled license assets current.
- An explicitly authorized local preview may sign a copy of the release APK with the existing development key using SDK apksigner. Keep it clearly labelled Preview, verify its signature against the existing debug APK and retain official-release rejection of debug certificates. Do not create a new key, uninstall the phone app or assume its installed signer from a local artifact.

- Do not operate phone gameplay controls, inject touches, or run scripted controller-input probes unless the user explicitly authorizes that interaction. The user performs play testing by default; build/test authorization alone does not authorize phone control.

For touched code, run the applicable Gradle tasks when the wrapper exists, such as:

```text
gradlew.bat testDebugUnitTest
gradlew.bat lintDebug
gradlew.bat assembleDebug
```

Spec 0001 does not configure a separate Kotlin formatter task. Add and document one only in a spec that deliberately selects and pins the formatter.

- Compile device tests with `gradlew.bat compileDebugAndroidTestKotlin`; compilation is not execution. Real DataStore disk recreation is covered by `SettingsPersistenceTest` on Android because the Windows JVM file replacement test failed; local settings tests cover persistence mapping with an in-memory store.
- The debug APK includes an explicit ADB `ConnectionProbe` for foreground input through the real Activity/ViewModel/session: neutral by default, with opt-in scripted mixed/exercise patterns and a 10–1800 second duration. It needs no separate test APK and is absent from release. Invocation and result interpretation are in `../context/testing-and-release.md`; it does not verify physical touch ownership or the full lifecycle matrix.

Do not claim physical multi-touch, lifecycle, immersive mode, local-network permission, or Wi-Fi behavior is verified without running it on an appropriate device. Report when Android Studio sync or a physical-device test remains.
