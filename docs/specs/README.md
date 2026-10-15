# Implementation Specs

These specs turn the project roadmap into ordered, reviewable vertical slices. They are planning/acceptance documents; a spec marked **Ready** is not implemented.

## Working rule

1. Select exactly one spec whose dependencies are complete.
2. Read root/package instructions and governing context.
3. Update documentation first for any protocol/architecture decision.
4. Implement the smallest coherent slice.
5. Run automated checks and the named manual/hardware gate.
6. Record evidence in the spec and `context/progress-tracker.md`.

A later spec must not be used to bypass a failed earlier safety/driver gate.

Explicit user exception, 2026-09-08: Spec 0006 may start on the implemented Spec 0005 foundation; its remaining physical acceptance and delivery-stall investigation are assigned to the Spec 0007 reliability checkpoint. The exception is defined in Specs 0005–0007 and does not mark the deferred evidence passed or waive automated safety and focused new-control checks.

## Ordered index

1. [`0001-foundation-and-tooling.md`](0001-foundation-and-tooling.md)
2. [`0002-protocol-v1-and-rust-udp.md`](0002-protocol-v1-and-rust-udp.md)
3. [`0003-android-connection-and-hello.md`](0003-android-connection-and-hello.md)
4. [`0004-one-button-vertical-slice.md`](0004-one-button-vertical-slice.md)
5. [`0005-reliability-and-safety.md`](0005-reliability-and-safety.md)
6. [`0006-left-stick-and-multitouch.md`](0006-left-stick-and-multitouch.md)
7. [`0007-full-single-controller.md`](0007-full-single-controller.md)
8. [`0008-adaptive-controller-ui-and-settings.md`](0008-adaptive-controller-ui-and-settings.md)
9. [`0009-discovery-and-pairing.md`](0009-discovery-and-pairing.md)
10. [`0010-desktop-tray-and-packaging.md`](0010-desktop-tray-and-packaging.md)
11. [`0010a-settings-and-layout-customization.md`](0010a-settings-and-layout-customization.md)
12. [`0011-multiple-controllers.md`](0011-multiple-controllers.md)
13. [`0012-keyboard-mouse-mode.md`](0012-keyboard-mouse-mode.md)
14. [`0013-release-certification.md`](0013-release-certification.md)

## Status meanings

- **Proposed:** scope is registered; detailed design and acceptance criteria remain to be completed before implementation.
- **Ready:** scope and exit criteria are approved for implementation, subject to refreshed dependency/API research.
- **In progress:** active implementation; incomplete evidence is visible.
- **Blocked:** a named hard gate needs user/environment/architecture resolution.
- **Complete:** all required automated and manual evidence passed.
- **Deferred:** explicitly excluded from the current release; not silently unfinished.

## Cross-cutting sources

- Product: [`../../context/project-overview.md`](../../context/project-overview.md)
- Architecture: [`../../context/architecture.md`](../../context/architecture.md)
- Protocol: [`../../context/protocol-spec.md`](../../context/protocol-spec.md)
- Build order: [`../../context/build-plan.md`](../../context/build-plan.md)
- Tests/releases: [`../../context/testing-and-release.md`](../../context/testing-and-release.md)
- Progress: [`../../context/progress-tracker.md`](../../context/progress-tracker.md)
