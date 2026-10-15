# Virtual Gamepad Project Context

This repository builds an Android phone into a low-latency controller for Windows games over a trusted local Wi-Fi network.

The maintained context is intentionally split by concern. This file is the entry point, not the full specification.

## Read first

1. [`AGENTS.md`](AGENTS.md) — repository workflow and non-negotiable rules.
2. [`context/project-overview.md`](context/project-overview.md) — product scope, decisions, and success criteria.
3. [`context/architecture.md`](context/architecture.md) — component boundaries and runtime flows.
4. [`context/protocol-spec.md`](context/protocol-spec.md) — authoritative UDP protocol v1.
5. The relevant implementation spec in [`docs/specs/`](docs/specs/).
6. The closest package instructions:
   - [`mobile_controller/AGENTS.md`](mobile_controller/AGENTS.md)
   - [`pc_agent/AGENTS.md`](pc_agent/AGENTS.md)

## Context map

- [`context/build-plan.md`](context/build-plan.md) — ordered roadmap, including the 10-day focused plan.
- [`context/testing-and-release.md`](context/testing-and-release.md) — test matrix, quality gates, signing, packaging, and release checks.
- [`context/ui-guidelines.md`](context/ui-guidelines.md) — responsive landscape controller rules and component inventory.
- [`context/library-docs.md`](context/library-docs.md) — dependency policy, current platform baselines, and source links.
- [`context/agent-workflow.md`](context/agent-workflow.md) — Context7, skills, MCPs, and AI-assisted workflow.
- [`context/progress-tracker.md`](context/progress-tracker.md) — living status and evidence log.

## Source-of-truth precedence

When documents disagree, use this order:

1. The user's latest explicit decision.
2. An approved spec under `docs/specs/`.
3. `context/protocol-spec.md` for wire-format questions.
4. Other maintained files under `context/`.
5. Current implementation and tests.
6. Historical notes or external examples.

Do not silently resolve a material conflict. Record the decision in the relevant context file before changing code.

## Current headline decisions

- Android: Kotlin, Jetpack Compose, coroutines, Android Studio, Android 8.0+ (`minSdk 26`).
- Windows: stable Rust, Tokio, MSVC x64, Windows 10 and Windows 11.
- Transport: one bidirectional UDP socket on port `8765`, fixed 40-byte v1 datagrams, 60 Hz full-state snapshots.
- Virtual controller: Xbox 360/XInput-compatible device through an isolated ViGEm backend for v1, subject to an early viability gate because ViGEm is retired.
- First milestone: Android `A` button -> UDP -> Rust -> visible Xbox `A` input in Windows.
- First reliable product: one paired phone only. Discovery and multiple simultaneous phones follow after reliability.
- Controller: two sticks with L3/R3, D-pad, A/B/X/Y, LB/RB, analog LT/RT, Back, Start; no Guide button and no vibration.
- Android experience: landscape locked, immersive, safe-inset aware, screen kept awake only while controller mode is active.
- Distribution: signed APK and a Windows installer with driver/firewall preflight; auto-start is optional and off by default.

## Product honesty

The target is broad compatibility with games that accept Xbox/XInput controllers. It is not a guarantee that every game, anti-cheat system, browser game, or elevated process will accept virtual input. Keyboard/mouse emulation is a later opt-in compatibility mode, not part of the controller reliability milestone.
