# Spec 0001 — Foundation and Tooling

Status: **Accepted**  
Milestone: M0  
Depends on: planning context only  
Changes allowed: repository root, `mobile_controller/`, `pc_agent/`

## Outcome

Create two independent, reproducible project skeletons that build on the user's Windows 10 x64 machine and establish the verification/secret rules needed by later slices. Do not implement controller input or networking beyond health placeholders.

## Decisions to confirm at start

- Final Android application ID/publisher namespace. Use a clearly marked development ID only if the user cannot decide yet; record that it must be fixed before signing/distribution.
- Display product name: default `Virtual Gamepad` and `Virtual Gamepad Agent` unless changed.
- Whether to initialize local Git now. Recommended: yes, with no remote required.

These choices do not justify combining Gradle and Cargo.

## Android scaffold

- Create a standard single-activity Kotlin/Compose application under `mobile_controller/`.
- Planning baseline: min SDK 26, compile/target SDK 37, JDK 17, current compatible pinned AGP/Kotlin/Compose BOM from `context/library-docs.md` after refresh.
- Use Gradle Kotlin DSL, Gradle wrapper, and version catalog.
- Add Debug and Release build types with no release signing secret in the repository.
- Add a minimal dark setup placeholder that can render in previews and on a device.
- Establish package directories for `app`, `controller`, `network`, `protocol`, `settings`, and `ui` without filling them with speculative abstractions.
- Add unit/UI test dependencies and one smoke test proving the harness runs.
- Add formatter/linter only if it is maintained and integrates cleanly; record the chosen task name in package instructions.

## Rust scaffold

- Create an application crate under `pc_agent/`, edition 2024.
- Pin Rust 1.98.1 or the refreshed stable decision in `rust-toolchain.toml` with `rustfmt` and `clippy` components.
- Target `x86_64-pc-windows-msvc`; document required Visual Studio 2022 Build Tools/Windows SDK.
- Keep `main.rs` limited to startup composition and a clean health message.
- Create empty/focused module boundaries only where the next specs need them: `config`, `protocol`, `network`, `session`, `input`, `virtual_gamepad`, `telemetry`.
- Add a `VirtualGamepadBackend` domain trait plus a recording fake only if it can be done without introducing the real Windows dependency; otherwise defer the trait body to 0004.
- Add one smoke unit test.

## Repository tooling

- Add `.gitignore` rules for Android/Gradle, Rust target output, IDE local state, signing material, local config/logs, and Windows packaging output.
- Add `.editorconfig` for line endings/whitespace without overriding language formatters.
- Create `protocol/fixtures/v1/README.md` as the future shared-fixture contract; no fake fixtures yet.
- Add a root `README.md` with project purpose, independent build entry points, current non-release status, and links to context/specs.
- Do not add monorepo build orchestration, containers, CI, release workflows, or package managers unless needed to make the two native skeletons reproducible.
- Do not create or commit keystores, certificate files, API keys, pairing values, or machine-specific SDK paths.

## Environment inventory

Record actual versions/availability in the progress tracker or a local non-secret report:

- Windows edition/build/architecture.
- Android Studio, embedded JDK, installed SDK platforms/build tools.
- Physical phone model, Android/API version, navigation mode, display cutout, and landscape resolution/density.
- Rustup, Rust/Cargo, MSVC Build Tools, Windows SDK.
- Git.
- Node/npm only if the local Context7 server is chosen; not required for the remote MCP.
- Context7 availability in the restarted/new Codex task.

Do not install or upgrade unrelated software silently. Explain any prerequisite before requesting installation.

## Automated verification

Run and record:

```text
mobile_controller\gradlew.bat testDebugUnitTest
mobile_controller\gradlew.bat lintDebug
mobile_controller\gradlew.bat assembleDebug
```

Plus the configured Android formatting task if one was added.

Run and record from `pc_agent/`:

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo check --all-targets
cargo test --all-targets
```

## Manual verification

1. Open only `mobile_controller/` in Android Studio.
2. Complete Gradle sync with the recorded JDK.
3. Run the placeholder on the physical phone or an emulator.
4. Confirm the screen launches in the intended orientation baseline without runtime errors.
5. Run the Rust binary in a terminal and confirm its health/version output and clean exit.

## Exit criteria

- Both skeletons compile/test with exact evidence.
- Android Studio can sync and launch the placeholder, or the unmet physical/manual evidence is explicitly marked `[~]` and no dependent claim is made.
- Tool versions are pinned/recorded and compatible.
- Build outputs/secrets are ignored.
- Root and package documentation command names match reality.
- No controller/network feature was prematurely added.

## Out of scope

- UDP, protocol bytes, touch handling, ViGEm, permissions, discovery, pairing, tray, installer, and release signing.
