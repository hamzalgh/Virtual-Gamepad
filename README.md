# Virtual Gamepad

Use your Android phone as a wireless Xbox-style controller for PC games. Virtual Gamepad connects over your home Wi-Fi and provides sticks, buttons, a D-pad and triggers on your phone's screen.

Currently a preview for one phone at a time, intended for games that accept Xbox controllers.

## Downloads

There is no public app download attached to this source repository yet. An upload-ready preview package is prepared locally at `dist/preview/VirtualGamepad-Preview.zip`; generated binaries are excluded from Git. To let testers download it, attach that ZIP to a GitHub Release.

The preview contains a Windows x64 installer and Android APK for testing. The Windows installer is unsigned, and the APK uses the existing Android development certificate. These are preview builds, not official release downloads.

- [Required ViGEmBus driver 1.22.0](https://github.com/nefarius/ViGEmBus/releases/download/v1.22.0/ViGEmBus_1.22.0_x64_x86_arm64.exe)

The driver is a separate prerequisite and is not bundled in the preview ZIP. Public app downloads and official signing are not available yet.

## Setup

You need **Android 8 or newer**, a **Windows 10 or 11 PC (64-bit)**, and a trusted private network shared by both devices. A focused Windows 11 startup retest was reported for a prior preview; retest the current preview before relying on it. Broader Windows coverage remains a release gate.

1. **Install the PC driver.** Run the ViGEmBus installer above and restart if requested. It lets Windows recognise a virtual Xbox controller. Skip this step if the supported driver already works on your PC. ViGEmBus is retired; use its [official archived release](https://github.com/nefarius/ViGEmBus/releases/tag/v1.22.0).
2. **Install the Windows app.** Run the Virtual Gamepad installer and select its optional Private-network UDP allowance so your phone can connect. Do not enable Public-network access. The driver is installed separately.
3. **Install the Android app.** Copy the APK to your phone and open it. Allow installation from that browser or file manager if prompted. Update an existing installation without uninstalling it; if Android refuses, keep your current app and data.
4. **Pair the devices.** Open Virtual Gamepad on your PC. Right-click its icon near the clock and choose **Pair** to display a six-digit code. On your phone, open the app, choose **Find PCs**, select your PC, then **Connect** and enter the code. Allow local-network permission if requested.
5. **Play.** Once connected, open your game and select controller input if needed. Use **Stop listening** or **Exit** in the PC tray when finished.

Every new connection needs a fresh code. Choosing Pair again ends the previous connection. Starting with Windows is optional and off by default.

**PC not found?** Enter its current Wi-Fi IPv4 address manually, using port **8765**. Find the address in Windows Settings > Network & Internet > Wi-Fi > your connected network's properties. Avoid guest/public Wi-Fi and do not forward the port on your router. More help is in [troubleshooting](pc_agent/packaging/troubleshooting.txt).

## For developers

The two applications build independently:

- **Android:** open `mobile_controller/` in Android Studio. Install Android SDK 37 and use JDK 25 as requested by the Gradle configuration.
- **Windows:** work in `pc_agent/`. Install Rust through rustup, Visual Studio's C++ build tools and the Windows SDK. The repository pins the Rust toolchain.

From `mobile_controller/`, build the Android debug APK:

```powershell
.\gradlew.bat assembleDebug
```

Output: `app/build/outputs/apk/debug/app-debug.apk`.

From `pc_agent/`, build and start the Windows tray app:

```powershell
cargo build --release --locked
.\target\release\virtual-gamepad-agent.exe --tray
```

Omit `--tray` for terminal mode, with `pair`, `status` and `exit` commands. Controller output requires the driver.

Run Android checks with `gradlew.bat testDebugUnitTest lintDebug`. For Rust, use `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings` and `cargo test --locked --all-targets`.

See [architecture](context/architecture.md), the shared [UDP protocol](context/protocol-spec.md) and [packaging instructions](pc_agent/packaging/README.md) for further details. Keep generated builds and signing credentials out of version control.

## Licence

Licensed under the [Apache License 2.0](LICENSE). See [NOTICE](NOTICE) for attribution and [third-party notices](pc_agent/packaging/THIRD_PARTY_NOTICES.txt) for dependency licences. ViGEmBus is a separate project with its own licence.
