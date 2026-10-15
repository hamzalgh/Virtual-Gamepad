# Spec 0008 — Adaptive Controller UI and Settings

Status: **Accepted**
Milestone: M2 completion / M3 usability  
Depends on: 0007  
Primary changes: Android UI/theme/layout/settings/tests

## Outcome

Turn the functionally complete controller into a polished, fixed, responsive landscape experience across supported Android phones without changing pointer semantics or protocol behavior.

## Development path acceptance (2026-09-11)

The user accepts this slice as done and explicitly requests Spec 0009 implementation after the revised appearance, five focused physical checks and final automated verification recorded in the progress tracker. Acceptance applies to the tested development path. Broader device/accessibility evidence, the full revised build manual matrix, connected instrumentation, current Studio sync, API 37 and Windows 11 remain unverified. Formal Spec 0005/0007 reliability acceptance and M2 remain open. Settings simplification is scheduled in Spec 0010a. The user's new authorization supersedes this document's earlier restriction against starting Spec 0009, without authorizing automatic device interaction or campaigns.

## Authorized entry and reconciled design (2026-09-10)

The user authorizes Spec 0008 based on successful play in Stardew Valley, PES 2017 and Need for Speed 2005, approximately 15–30 minutes combined. This permits UI/settings development while the formal Spec 0005/0007 campaigns remain unverified. Do not restart those campaigns automatically or claim M2 acceptance. Physical testing belongs to the user; this slice runs automated host checks only unless a specific device interaction is authorized.

Use the updated `context/design/Virtual Gamepad.jpg`, which now includes the requested swap: left stick outside and D-pad inward, as in Spec 0007. Do not swap them again. Use the reference control treatment and integrated gear/Back/Start/X navbar specified below. Remove all gameplay headings, status text and counters. Put diagnostics and the debug-only touch switch inside Settings. Settings opens a separate scrollable panel and cancels all held input; closing requires fresh touches. The sender continues neutral full snapshots while the panel is open. Apply persists both tuning values in one transaction, then replaces the immutable tuning pair and cancels ownership again without reconnecting. Failed persistence keeps the previous pair and shows a retryable error. Reset edits the draft back to defaults; Apply saves it. Fixed gameplay labels cap visual font scaling at 1.3 to stay inside their capture regions; setup, settings and recovery retain system text scaling.

The compressed layout retains the verified Spec 0007 main-control dimensions; larger bounds use the reference proportions specified below. Portrait or insufficient windows show a recovery message and Exit, with neutral input and ordinary system UI. Safe drawing and gesture insets are consumed once before the internal gutter. Fixed landscape remains; opposite cutout sides are geometry fixtures, not sensor rotation.

The 40-byte protocol, shared fixtures, strict 60 Hz, bounded latest state, release trust, isolated backend and Spec 0007 timeout/recovery policy are unchanged. No Spec 0009 work is authorized.

## Design source

### User correction: reference navbar and controls (2026-09-10)

The user corrected the earlier separate status strip and conservative styling. Use one integrated top navbar containing only a vector gear, Back and Start controls, and an X exit icon, matching the reference's arrangement. All status, development trust text, diagnostics and options belong behind the gear; no gameplay titles, DEV marker or counters. Back/Start remain real held gamepad controls owned by the root router. Gear/Exit occupy separate, validated geometry and cancel input on activation.

Remove the extra header height. Use the entire safe rectangle for the reference arrangement, with larger, higher main controls when space permits. Retain a compressed fit for the previously supported 640 by 264 dp bound; at 320 dp height and above use larger reference proportions. Main controls grow up to 1.25 times the 640 by 320 design: stick bases 128–160 dp, face buttons 64–80 dp, shoulder petals 72–90 dp wide and 64–80 dp high, trigger petals 88–110 dp wide and 72–90 dp high. Keep at least 8 dp separation and 48 dp utility targets. All hit regions, including gear/Exit, must remain contained and disjoint. Use monochrome pale violet rims, soft edge highlights, rounded inward-pointing D-pad petals, blank ringed stick knobs, round face buttons and diagonal shoulder petals as in the image. Display L1/L2/R1/R2 with the unchanged LB/LT/RB/RT mappings explained in Settings; triggers retain mirrored absolute horizontal analog position and release to zero.

Add a persisted Show L3/R3 switch, enabled by default to preserve existing availability. Hiding removes both visuals and capture regions, clears ownership and leaves those button bits neutral. Saving failure keeps the previous preference with retryable feedback. Changing visibility never reconnects or changes the other controls' positions. Reset tuning remains scoped to tuning.

Follow `context/ui-guidelines.md`. Use the UI/UX review skill for design-system/layout/accessibility analysis, then verify Compose APIs against official Android docs and behavior on devices.

## Scope

- Central Compose tokens for colors, typography, shapes, dimensions, motion, and status roles.
- Safe-bound responsive geometry with compact/regular profiles.
- Immersive landscape and active-only keep-screen-on.
- Fixed full controller layout and immediate local feedback.
- Status inside Settings and recoverable blocking states.
- Persisted dead zone and sensitivity with bounds/reset.
- Setup/settings accessibility and controller usability review.
- Developer overlay retained behind a debug-only switch.

No user drag/rearrange/layout profiles.

## Layout work

- Compute one usable rectangle from current safe drawing/gesture insets plus internal gutter.
- Place all visual and capture regions through one geometry model consumed by renderer, router, and tests.
- Implement compact/regular profiles based on usable dp bounds, not phone model.
- Clamp control sizes to the documented ranges and preserve minimum target/gap rules.
- Assert regions are in bounds and non-overlapping for fixture matrix and runtime debug builds.
- Test both landscape directions/cutout sides.
- If a window is too small to satisfy constraints, show an explicit unsupported/rotate state instead of overlapping input.

## Window/lifecycle work

- Manifest/activity controller flow uses fixed landscape (not sensor landscape).
- Use current AndroidX edge-to-edge and system-bar controller APIs; avoid deprecated flags.
- Hide system bars in controller mode while retaining an explicit exit/pause path.
- Restore system UI on exit/error.
- Set keep-screen-on only while an active controller session/surface is running; clear on pause/stop/disconnect/error.
- Test large-screen/windowing environments that may ignore orientation requests and degrade safely.

## Settings

- Dead zone: default `0.15`, inclusive range `0.00..0.30`, reaffirmed by the user; the old `0.08` draft is superseded.
- Sensitivity: default linear `1.0`, inclusive range `0.5..2.0`; magnitude raised to `1/sensitivity` after dead-zone rescaling, preserving zero and full scale.
- PC address and port remain persisted.
- Show L3/R3 persists independently of tuning; hidden buttons have no capture regions or active bits.
- Provide reset-to-default and inline explanation/preview.
- Apply changes atomically to controller calculation without restarting network where safe.
- Do not add layout customization, accounts, analytics, or sensitive data.

## Status UX

- Implement exact vocabulary from UI guidelines: Offline, Reaching PC, Permission needed, Pairing needed (prepared), Ready, Degraded, Timed out, Driver issue.
- Status appears inside Settings; blocking failures return to the recovery screen.
- Driver/permission/session failures move to a focused recovery path rather than leaving a fake-ready surface.
- Local pressed feedback remains immediate during degraded network.

## Automated tests

- Geometry fixtures for compact/regular, multiple aspect ratios, left/right cutouts/insets, gesture/nav insets, and large window.
- Every capture region in bounds, minimum target met where feasible, and no overlap.
- Renderer/router share identical geometry IDs/bounds.
- Settings persistence, validation, atomic application, and reset.
- System/UI state reducer for entering/exiting controller mode and every error path.
- Compose tests for setup errors, status labels, settings at large font scale, and control presence.
- Keep-screen-on/system-bar controller called/cleared through testable adapter where possible.

## Manual device review

1. Test primary physical phone in both landscape directions.
2. Inspect cutout/rounded corner/gesture edges and attempt system gestures near controls.
3. Use all controls in common multi-finger combinations for at least 20 minutes.
4. Change dead zone/sensitivity and verify immediate predictable effect and persistence after restart.
5. Leave controller mode by every path; confirm system bars and screen timeout policy restore.
6. Test bright/dark room legibility and color-independent labels/states.
7. Use compact/wide emulator or second device and capture screenshots/evidence.
8. Test setup at large font scale and basic TalkBack navigation; document gameplay screen-reader limitations honestly.

## Exit criteria

- Full controller remains functionally identical to 0007.
- Tested sizes/insets have no obstruction/overlap and all controls are reachable.
- Landscape/immersive/keep-awake requirements work and restore correctly.
- Settings persist and remain within safe bounds.
- Status is truthful and recovery is clear.
- Formal M2 reliability acceptance remains unverified under the authorized entry exception. UI implementation does not close that gate.

## Out of scope

- Layout editor/profiles, themes marketplace, discovery/pairing implementation, tray, installer, multiple phones, keyboard/mouse.
