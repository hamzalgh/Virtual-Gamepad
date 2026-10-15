# Spec 0006 — Left Stick and Coordinated Multi-Touch

Status: **Accepted**
Milestone: M2 input foundation  
Depends on: 0005  
Primary changes: Android controller/input/UI, Rust mapping/tests

## Dependency exception (2026-09-08)

The user approved starting this spec using the implemented and automatically tested Spec 0005 foundation while its outstanding physical reliability acceptance and delivery-stall investigation move to the Spec 0007 checkpoint. Do not block this slice on those deferred campaigns. Preserve automated safety coverage and perform this spec's focused physical stick+A, release/cancel and neutral checks. Record the actual timeout used; the explicit 1000 ms development workaround does not verify the default 250 ms deadline.

## Outcome

Add a fixed left analog stick and a coordinated root pointer router so the user can move the stick while independently pressing/releasing A, with correct dead zone, full range, Y direction, cancellation, and neutral behavior.

## Input architecture

- `GamepadSurface` owns low-level multi-pointer input for all gameplay controls.
- Pointer IDs are stable ownership keys; pointer indexes are never stored across events.
- On down, deterministic hit testing assigns a pointer to one control.
- A pointer retains that assignment until up/cancel and cannot steal another active control.
- Pointer movement updates only its assigned control.
- A global cancel/lifecycle loss clears all ownership and returns local state to neutral.
- Visual components receive state/geometry; they do not run competing gesture detectors.
- Add a developer-only overlay showing safe bounds, capture regions, pointer IDs/owners, normalized axes, encoded axes, and active buttons.

## Left-stick transform

- Fixed center and circular travel radius derived from responsive geometry.
- Clamp displacement to the knob center's travel radius, not the larger visual base radius.
- Normalize to a unit circle.
- Apply configurable radial dead zone; default `0.15`, validated inclusive range `0.00..0.30`. The user's recorded settings decision, reaffirmed on 2026-09-08, supersedes the older `0.08` draft.
- Rescale magnitude outside dead zone so output still reaches full scale.
- Apply sensitivity `s` as `rescaledMagnitude^(1/s)`, default linear `1.0`, validated inclusive range `0.5..2.0`. Higher sensitivity gives greater intermediate output; every setting preserves zero and full scale without early clipping. Invalid/nonfinite saved tuning falls back to its respective default. Read persisted tuning when loading settings; editing tuning UI belongs to Spec 0008.
- Invert Android screen-space Y so protocol positive Y means up.
- Encode negative full scale to `-32768`, positive to `32767`.
- On release/cancel, output exactly `(0,0)` in the next local state.

Suggested radial dead-zone mapping for magnitude `m` and threshold `d`:

```text
if m <= d: output = 0
else: output_direction × clamp((m - d) / (1 - d), 0, 1)
```

The sensitivity curve is applied after this rescale and must preserve zero/full-scale.

## Geometry and presentation

Use the existing theme, safe drawing/gesture insets, internal gutter and separate utility row. Solve gameplay geometry from the remaining rectangle in pixels and display density. Centers are at 24% and 76% of its width, halfway down. Stick gate radius is the minimum of 38% of height, 18% of width and 90 dp; its capture radius adds 8 dp. A visual radius is the smaller of 42 dp and 55% of the stick radius; its capture radius adds 8 dp. Require a stick radius of at least 56 dp, an A radius of at least 28 dp, captures contained in the safe rectangle, and an 8 dp gap. Otherwise show an unsupported window message and accept no input.

The knob radius is 32% of the visual base radius. Its center can travel the remaining 68%; this same travel radius normalizes input and positions the untuned knob, so it follows the finger one to one until its edge contacts the base boundary. At contact the output reaches full magnitude. The dead zone overlay uses the travel radius too. With a 90 dp base, full input takes 61.2 dp of movement and the default dead zone covers 9.18 dp. Circular diagonal output remains approximately 0.707 per axis, rather than reaching the corners of a square tester.

A retains ownership when dragged out, releases there and presses again on reentry. Only a fresh down captures an unoccupied control; ignored fingers cannot acquire it later by moving. The debug build has a Touch overlay toggle in the utility row. Its passive overlay shows safe bounds, capture circles, owners, normalized/encoded axes and A. Opening diagnostics, toggling the overlay, resizing, losing focus or cancelling input clears ownership; held fingers must lift and press again. No gameplay control has its own gesture detector.

## Rust mapping

- Map left X/Y exactly to the Xbox report representation required by the backend.
- Confirm whether the backend expects the same positive-up convention; adapt only in the backend mapping and document it.
- XInput and the pinned ViGEm Xbox report use signed 16 bit left axes, with positive Y up. Copy protocol left X/Y directly in the isolated adapter; do not invert Y again. Admission and output permit only A and left axes in this slice, rejecting nonneutral right axes, triggers or other buttons.
- Preserve A mapping and neutral values for all unimplemented controls.
- Stale/timeout behavior applies to axes and button together.

## Automated tests

### Pure geometry/state

- Center, each cardinal extreme, diagonal clamp, just-inside/outside radius.
- Dead-zone at zero, exact threshold, immediately outside, midpoint, and full scale.
- Symmetry/monotonicity across directions and sensitivity settings.
- Screen-up encodes positive Y; screen-down negative Y.
- Float-to-integer asymmetry reaches both exact endpoints.
- No NaN/infinite/out-of-range output for finite coordinates/bounds.

### Pointer router

- Stick pointer plus A pointer in both press orders.
- Two downs at nearly the same time.
- Second pointer entering an occupied stick cannot take ownership.
- Pointer index reordering does not change ownership.
- Movement outside capture/visual region remains clamped and owned.
- Up/cancel releases only the owning control; global cancel releases all.
- Safe-bound/region fixtures contain no overlap.

### End-to-end

- Protocol and Xbox mapping for all axis endpoints/representative diagonals.
- Timeout/lifecycle reset centers the axis and releases A together.

## Manual physical test

1. Enable the developer overlay and verify actual touch IDs/regions on the physical phone.
2. Move through a slow full circle and snap/release from every cardinal/diagonal.
3. Hold left stick while tapping/holding/releasing A with another finger; reverse the order.
4. Add/remove extra fingers in empty/occupied regions and verify no ownership jump.
5. Drag far outside the stick, then release; verify clamp then exact center.
6. Background/lock/disable Wi-Fi while stick is held; Windows must center on timeout/lifecycle.
7. Inspect `joy.cpl`/tester for correct direction, full range, circle behavior, and center.

## Exit criteria

- Pointer router tests cover index reordering and cancellation.
- Simultaneous physical stick+A test passes without stealing/stuck state.
- Axis reaches both endpoints, has correct Y direction, and returns exactly to center.
- Dead-zone/sensitivity logic is pure, bounded, and documented.
- Existing automated reliability checks still pass; this slice's physical control/neutral checks pass, while the outstanding Spec 0005 campaigns remain assigned to Spec 0007 under the approved exception.

## Out of scope

- Right stick, stick clicks, remaining buttons, D-pad, triggers, final layout polish, customization.

## Implementation and evidence (2026-09-09)

Implemented Android normalized state, pure transform/geometry/router, persisted tuning consumption, root Compose routing, passive debug overlay, full snapshots and axis encoding. Rust admission/output permit A and left axes only; the isolated adapter copies both signed axes. Protocol v1 and every shared fixture are unchanged. No dependencies were added or updated; BuildConfig is enabled solely to gate the debug overlay.

Automated checks pass: 46 Android JVM tests, debug lint (zero errors, nine existing warnings), debug/release assembly and device test compilation/assembly; 57 Rust debug tests, format, Clippy, all target check, release build and four release trust tests. The signed zero regression first failed with `leftY=-0.0` during horizontal movement; canonical zero normalization fixes it and the regression passes. Existing 60 Hz, latest value, source/session/sequence, default timeout and explicit reconnect regressions pass.

Self review covered ownership in both press orders, rejected extra pointers, index reordering, consumed changes, reset keys, geometry containment, tuning boundaries, integer signs/endpoints, unused neutral fields, whole state cleanup, release refusal and adapter isolation. Compose device cases cover simultaneous pointers, cancel, focus epoch reset and diagnostics. Their separate test APK was rejected by the phone with `INSTALL_FAILED_USER_RESTRICTED`, so they are compiled but unexecuted. No current Android Studio sync is claimed.

The debug APK installed on the API 34 phone. Current PC Wi-Fi IPv4 is `[redacted PC private address]`; phone wlan0 is `[redacted phone private address]/24`. Initial connection probes failed before handshake with zero datagrams and incomplete peer resolution. Reconnecting the PC to its saved Wi-Fi restored peer resolution; manual Connect reached Ready. This does not resolve the deferred delivery stalls. A 45 second Windows XInput capture observed simultaneous stick+A, independent controls and exact neutral after held input, with every unimplemented field zero. The user tested the stick in Stardew Valley and joy.cpl, and confirmed that extra fingers in occupied/empty areas did not steal ownership, including after the stick owner lifted while A was held.

The user reported excessive stick travel. A focused regression reproduced full knob contact yielding only about 62% output because input used the base radius while rendering used 68% of that radius. Input and rendering now share the knob center's travel radius. The regression passes for all four exact encoded endpoints; another checks finger tracking, clamp and release across densities. The final Android command passed in 1m 24s and the corrected debug APK was installed/opened on the USB phone. The user subsequently confirmed all requested focused physical tests passed; see final acceptance below. These physical runs use the explicit 1000 ms profile, which can retain input for about one second plus scheduling/backend delay after packets stop. No default 250 ms physical pass, driver installation, firewall change, API 37 or Windows 11 test is inferred. Spec 0007 has not started.

API references: [Compose pointer ownership, consumption and cancellation](https://developer.android.com/develop/ui/compose/touch-input/pointer-input/understand-gestures) and [Microsoft XINPUT_GAMEPAD signed axes](https://learn.microsoft.com/en-us/windows/win32/api/xinput/ns-xinput-xinput_gamepad). Context7 query transport failed in this task; official documentation and the pinned local API/build were used. No dependency or redistribution boundary changed.

## Final acceptance (2026-09-09)

The user explicitly confirmed that all remaining tests passed as expected on the updated API 34 phone and Windows 10 controller: all cardinal signed endpoints and correct direction, exact center on release, stick+A in both press orders with independent release, outside clamp and release, Diagnostics cancellation without held input restoration, and background, lock and Wi-Fi loss neutralization/removal followed by explicit reconnect. Earlier extra-finger ownership checks passed. The user also confirmed working movement in Stardew Valley (version not supplied) and inspected joy.cpl. Approximate diagonal readings of +/-22000 per axis were reported; the specified circular range intentionally does not fill square tester corners. Slight left/right ergonomic preference was reported and accepted for the current unfinished layout; no direction mapping defect was reported.

Together with the recorded automated checks and Windows readback, this satisfies Spec 0006 for the tested development path. Physical loss tests used the explicit 1000 ms timeout, not the default 250 ms; no precise measured cleanup latency or reliability soak is inferred. Spec 0005's deferred campaigns and unresolved delivery stalls remain at the Spec 0007 reliability checkpoint. Spec 0007 is unstarted. API 37, Windows 11, connected instrumentation and a new Android Studio sync remain unverified. Final acceptance changes documentation only; no build was rerun or commit/push performed.
