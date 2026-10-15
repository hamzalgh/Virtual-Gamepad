# Spec 0013 — Release Certification

Status: **Ready**  
Milestone: Final release  
Depends on: all features selected for the release have passed their own specs  
Primary changes: defects found by certification, release artifacts, test reports, documentation

## Outcome

Produce an evidence-backed release for supported Android devices and Windows 10/11 x64, with signed/checksummed artifacts, clean install/upgrade/uninstall behavior, published compatibility limits, and no unresolved safety blocker.

Optional features that have not passed their specs are removed/disabled and documented as deferred rather than rushed into the release.

## Release freeze inputs

- Named commit/tag and clean source state.
- Frozen protocol v1 and golden fixtures.
- Pinned Android/Rust/installer/dependency versions and lockfiles.
- Exact supported Android floor/target and Windows versions.
- Exact virtual-driver/client prerequisite versions/source/checksum.
- Final application IDs, version numbers, signing identities, licenses, and notices.
- Release feature list: single/multiple controllers and keyboard/mouse included only if certified.

## Automated gate

Run from clean checkouts/environments where practical:

- Full Android formatting, unit, UI/instrumentation where configured, lint, debug and signed-release builds.
- Full Rust fmt, Clippy with warnings denied, check/test all targets/features, release build.
- Cross-language fixture verification.
- Parser property/fuzz corpus and malformed-input regression suite.
- Installer build/static checks and artifact checksum generation.
- Dependency/license/advisory review.
- Bug review and security review of release diff.

Warnings require explicit triage; do not broadly suppress them to make a gate green.

## Safety certification

For each supported output mode/slot:

- Exact neutral at startup before session.
- Neutral after no valid state within configured 250 ms default plus measured scheduler tolerance.
- Neutral after Android pause/stop/lock/process kill/Wi-Fi loss.
- Neutral after PC network/backend/status/task error.
- Neutral/detach after tray exit, console Ctrl+C, service/app shutdown, uninstall.
- No timeout refresh from malformed, stale, unpaired, wrong-session, or wrong-source traffic.
- No stale state replay under burst/slow consumer.
- One slot failure does not affect others when multi-controller is included.
- All injected keys/mouse buttons release when compatibility mode is included.

Any stuck input is a release blocker.

## Performance/reliability certification

- 30-minute minimum active soak; prefer multi-hour final soak.
- 50 connect/disconnect/reconnect cycles.
- 20 background/foreground and 20 Wi-Fi interruption cycles.
- Loss/reorder/duplicate/burst/malformed controlled tests.
- Monitor CPU, memory, handle/task count, log size, state rate, state age/RTT, timeout count, and UI responsiveness.
- Test on the user's 2.4 GHz network during ordinary interference; record environment rather than promise universal latency.
- Confirm 60 Hz baseline and protocol-negotiated upgrade fields do not imply an untested 90/120 Hz feature.

## Android matrix

- Oldest supported API/device/emulator evidence.
- Current mainstream physical Android device.
- Android 17/API 37 local-network permission behavior.
- Compact and wide landscape sizes; cutout/rounded corner; gesture and three-button navigation where available.
- Both landscape directions where the environment can present them.
- Large font setup/settings and basic TalkBack navigation.
- Immersive bars, explicit exit, keep-screen-on only while active, policy restoration.
- Fresh signed APK install and same-signature upgrade.
- Android developer verification/registration status documented for intended distribution regions/date.

## Windows matrix

On clean/snapshot Windows 10 x64 and Windows 11 x64:

- Driver absent/present/broken recovery.
- Installer elevation, Private firewall rule, Public profile behavior.
- First run, manual IP, discovery, pairing, reconnect, revoke/replace.
- Tray start/stop/exit and startup disabled by default; optional startup after reboot.
- Controller tester full mapping and neutralization.
- Upgrade preserving compatible settings/pairing.
- Uninstall preserving/removing data choice and owned cleanup.
- Forced process termination and documented recovery.
- Authenticode/SmartScreen behavior recorded.

Windows 11 support is not certified until this matrix is actually run there.

## Compatibility matrix

At minimum record:

- PES 2017 or an available equivalent football title.
- One available Need for Speed title or equivalent racing title for analog axes/triggers.
- One additional modern XInput game if available.
- Local Chrome/Edge Gamepad API harness.
- One web game known to support gamepads.
- If keyboard/mouse mode ships: one browser/legacy target at ordinary integrity plus expected elevated-target limitation.
- If multiple controllers ship: one local multiplayer target with at least two phones/clients.

For each: OS, version, mode, controller count, result, mapping issue, workaround, and evidence. Publish a sample matrix, not “works with all games.”

## Security/privacy gate

- Service remains private-LAN only; no port-forwarding/public exposure guidance.
- Firewall is program-scoped UDP Private only.
- Pairing code/IDs/session data are random, appropriately stored/redacted, and rate-limited.
- Trusted-LAN/no-encryption limitation is visible.
- Android components/permissions are minimal.
- No keys, certificates, API tokens, personal data, or machine-specific secrets in source/artifacts/log bundle.
- Manual trust/debug overlay/verbose packet logging disabled in release.
- Keyboard/mouse mode has explicit consent and cannot silently activate.
- Third-party retirement/license/advisory risks documented, especially ViGEm.

## Documentation set

- Quick start: install PC prerequisite/agent, allow Private network, install APK, pair, play.
- Detailed install, upgrade, uninstall, and clean removal.
- Find PC IP/manual connection and router isolation troubleshooting.
- Permissions, discovery, pairing, revoke/replace.
- Controller layout and tuning.
- Driver missing/broken and stuck-device recovery.
- Windows 10/11 and Android support matrix.
- Game/browser compatibility and keyboard/mouse limitations.
- Trusted-LAN security statement.
- Changelog, licenses/notices, checksums, and support log collection.

## Artifact verification

- Signed APK signature verified; install/upgrade tested.
- Windows executable/installer signature verified when certificate exists.
- SHA-256 checksums recomputed after final signing and match published values.
- Artifact version/build ID matches UI/logs/docs/tag.
- Release package contains no debug symbol/log/secret unintentionally; symbols may be archived privately for debugging.
- Reproduce release from documented commands as far as signing credentials allow.

## Exit criteria

- Every selected feature spec is complete with evidence.
- Full automated gate passes.
- Safety matrix has no blocker.
- Windows 10 and Windows 11 clean-machine matrices pass.
- Android/API/device matrix passes at the claimed support floor/target.
- Signed/checksummed artifacts and documentation are complete.
- Compatibility and known limitations are published prominently.
- Progress tracker and release report link to evidence.

## Stop-ship conditions

- Any stuck input or stale-state replay.
- Unpaired/wrong-source device can control.
- Installer opens Public firewall or cannot clean owned state.
- Release artifact contains secret/debug bypass.
- Driver behavior is unreliable on a claimed OS.
- APK/installer signature or update identity is wrong.
- Required Windows 11/Android permission evidence is missing while still claimed.
- Documentation materially overpromises game/security/platform compatibility.
