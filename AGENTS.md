# Agent Instructions — Virtual Gamepad

These rules apply to the whole repository. A nested `AGENTS.md` adds package-specific rules and wins for files in its subtree.

## Mission

Deliver a reliable Android-to-Windows virtual Xbox controller over a private local Wi-Fi network. Prefer a tested vertical slice over a broad unfinished implementation.

The two applications remain independent:

- `mobile_controller/`: Kotlin + Jetpack Compose, built and run in Android Studio.
- `pc_agent/`: Rust + Tokio, built and run from a Windows terminal or VS Code.

They share a documented binary UDP protocol, not source code or a build system.

## Required reading

Before changing code or build configuration, read:

1. `PROJECT_CONTEXT.md`.
2. `context/project-overview.md`.
3. `context/architecture.md`.
4. `context/protocol-spec.md` when networking, input state, discovery, pairing, or compatibility is involved.
5. The active spec in `docs/specs/`.
6. The closest nested `AGENTS.md` and the relevant source/tests.

Also consult:

- `context/ui-guidelines.md` for Android UI/input work.
- `context/testing-and-release.md` for verification or packaging.
- `context/library-docs.md` before adding/updating a dependency.
- `context/agent-workflow.md` for Context7 and agent tooling.
- `context/progress-tracker.md` before and after a work session.

## Conflict handling

Use the precedence in `PROJECT_CONTEXT.md`. When code and documentation materially disagree, do not guess or quietly edit one side. If the intended resolution is not obvious from an approved spec or test, explain the conflict and request a decision.

The user approved continuing to Spec 0006 on 2026-09-08 with Spec 0005 physical acceptance deferred to the Spec 0007 reliability checkpoint. Follow the exception recorded in Specs 0005–0007 and `context/progress-tracker.md`; do not reimpose the deferred gate before Spec 0006 or report it as passed. Automated safety checks and the new controls' immediate cancellation/neutral tests remain required.

The user authorized Spec 0008 on 2026-09-10 based on reported play in three native games. Formal Spec 0005/0007 campaigns remain unverified and must not restart automatically. This exception permits only Spec 0008 development and does not authorize Spec 0009 or release acceptance.

On 2026-09-11 the user accepted Spec 0008 for the focused tested development path and explicitly authorized completing Spec 0009 architecture and implementation. This supersedes the previous development entry restriction only for Spec 0009. Broader Spec 0008 evidence and formal Spec 0005/0007 campaigns remain unverified, M2 stays open, and accepting Spec 0009 requires revisiting those gaps. It does not authorize phone interaction, runtime campaigns, later features or release acceptance.

## Working method

The user reports that the Spec 0010 local preview APK updated the existing phone app successfully and worked with the PC agent. Preserve this focused acceptance alongside the accepted tray/settings checks and basic installer smoke report; do not restart those checks. Current local deliverables are in ignored dist/preview. Spec 0010a (Settings and Layout Customization) is the next planned architecture topic, not yet implemented or authorized by this documentation sync. Official release, broad firewall cleanup and previously unverified platform/reliability gates remain separate and pending.

On 2026-09-11 the user confirmed the final focused Spec 0009 checks as all good and requested the next session to work on Spec 0010. Spec 0009 is accepted for this tested development path and Spec 0010 development may proceed. This does not mark broader platform/accessibility coverage or formal Spec 0005/0007 reliability campaigns passed, or authorize release acceptance, installation/system changes or automatic device campaigns. Preserve the revised requirement for a fresh code on every connection and the quiet idle terminal.

For each feature:

1. Select one spec and confirm its prerequisites.
2. Inspect current code, tests, and uncommitted changes.
3. State the smallest coherent vertical slice.
4. Update the governing spec/protocol first if behavior changes.
5. Implement without unrelated rewrites.
6. Format, lint, compile, and test the touched application.
7. Perform or request the exact hardware/manual test the feature needs.
8. Update `context/progress-tracker.md` with evidence, remaining risks, and the next gate.

Never claim a build, device run, driver test, game test, or Windows-version test that was not actually performed.

## Non-negotiable architecture rules

- The Android app owns presentation, touch routing, controller-state calculation, packet encoding, settings, and network status.
- The PC agent owns UDP reception, validation, pairing/session policy, latest-state selection, timeout safety, mapping, and virtual-device integration.
- The wire format in `context/protocol-spec.md` is authoritative. Change it in documentation first, then update both implementations and golden vectors in the same slice.
- Continuous input is a full state snapshot, never raw touch events or deltas.
- No unbounded input queue. New valid state supersedes older state.
- Every loss-of-control path must converge to neutral: timeout, Android pause/stop, Wi-Fi loss, pairing loss, backend failure, graceful shutdown, and parse-failure storms.
- The PC must never expose the UDP service to the public internet. Firewall access is Private profile and program scoped.
- One paired phone is the first reliability target. Do not implement multiple controllers before the single-controller release gate passes.
- Do not add cloud services, accounts, telemetry upload, Bluetooth, iOS, rumble, or the Guide button.

## Protocol discipline

- Version 1 datagrams are exactly 40 bytes and little-endian.
- CRC-32 detects accidental corruption; it is not authentication or encryption.
- Sequence comparison must support `u32` wraparound.
- Session and source validation occurs before input application.
- Reserved bytes and bits must be zero when sent and rejected when required by the protocol.
- Invalid packets never refresh the valid-input timeout.
- Pairing provides local-network admission control, not protection from a hostile packet sniffer.

## Dependency and research discipline

- Prefer platform APIs and small maintained libraries.
- Before using a third-party API, consult the configured Context7 MCP when available, then verify platform-sensitive behavior against official documentation or upstream source.
- Official Android, Kotlin, Rust, Tokio, Microsoft, W3C, and upstream project documentation outrank summaries.
- Context7 being unavailable is not a blocker: use official sources and record the gap.
- Pin versions deliberately. Do not use floating versions in reproducible builds.
- Explain the purpose, maintenance state, license/redistribution impact, and removal boundary of each new dependency.
- ViGEm is retired. Keep it behind a `VirtualGamepadBackend` abstraction, complete the early driver spike, and do not spread its types through networking or domain code.
- No production Python implementation and no unsafe Rust outside a small documented Windows adapter when unavoidable.

## Security and secrets

- Never commit signing keys, keystores, passwords, pairing secrets, API keys, or machine-specific absolute paths.
- Do not put a Context7 API key in `.codex/config.toml`; use user-level environment or authentication configuration.
- Treat all UDP data as untrusted. Validate exact length, magic, version, message type, reserved fields, CRC, payload ranges, session, source, and sequence.
- Rate-limit malformed-packet and pairing-failure logs.
- Windows keyboard/mouse injection, when later implemented, is opt-in and disabled by default.

## Documentation maintenance

Keep the root README concise and reader facing: explain the product, app/driver downloads and nontechnical setup first, developer quick start second, and licence last. Keep agent workflow, campaign history and detailed acceptance gates in the governing context/spec files. Never invent public release URLs; label local preview links until actual hosted downloads exist.

For Spec 0010 and release work, follow the 2026-09-12 publication and Apache License 2.0 decision in `context/project-overview.md`. Confirm the copyright holder and review dependency compatibility before adding LICENSE/NOTICE; keep community forks separate from maintainer approved official releases. Recording the decision does not authorize publication or signing/system changes.

- `context/` contains stable project truth; avoid daily diary prose there.
- `docs/specs/` contains ordered, acceptance-testable implementation slices.
- Update `context/progress-tracker.md` after each completed or blocked slice.
- Record durable architecture decisions in the governing context/spec, not only in chat.
- Keep the entire project, UI copy, code, comments, and documentation in English.

## Completion report

After implementation, report:

### Changes made

- Files changed and the behavior each change adds.

### Verification

- Exact commands/tools run and their results.
- For Android, whether Gradle sync/build and a physical-device run actually occurred.
- For Windows, whether the driver, `joy.cpl`, installer, and target OS were actually exercised.

### Manual test steps

- Precise steps and expected results.

### Known limitations

- Anything incomplete, unverified, environment-dependent, or intentionally deferred.

## Context files

- [pc_agent/packaging/AGENTS.md](pc_agent/packaging/AGENTS.md): artifact inspection and packaging ownership rules.
