# Spec 0007 — Full Single Controller

Status: **In Progress**
Milestone: M2  
Depends on: 0006  
Primary changes: Android input/UI, Rust mappings, cross-language tests

## Outcome

Complete every planned Xbox-style control for one phone while preserving coordinated multi-touch, protocol v1, timeout safety, and correct Windows mapping.

## Controls

- Left/right analog sticks.
- L3/R3.
- A/B/X/Y.
- D-pad Up/Down/Left/Right with valid diagonals.
- LB/RB.
- Analog LT/RT.
- Back/Start.
- No Guide button.
- No vibration/haptics.

## Android input design

- Add all capture regions to the single root router with deterministic non-overlap.
- Reuse the proven pure stick transform for the right stick.
- Digital buttons set/clear their bit in one immutable controller state.
- D-pad hit testing uses explicit sectors/cardinals and can emit two perpendicular directions for diagonals, never opposites.
- LT/RT use large analog rails because ordinary touchscreens have no dependable pressure. Initial design maps absolute touch position in the lane to `0..1`; release is exactly zero.
- LB/RB remain independent digital regions with no trigger overlap.
- L3/R3 use dedicated labelled digital capture regions immediately inward/above their respective sticks. They do not depend on pressure, double-tap, or the moving stick pointer.
- Back/Start and exit are separated enough that gameplay actions cannot exit controller mode.
- Pointer ownership and global cancellation rules remain unchanged.
- Digital ownership follows the project rule: leaving the assigned capture region releases that control, re-entering may press the same control, and crossing another control never transfers the pointer.
- Developer overlay displays every region/value/bit.

If physical testing rejects the trigger-rail orientation or dedicated L3/R3 placement, change the UI placement in this spec while keeping protocol fields unchanged and recording the decision.

## Functional placement decision (2026-09-09)

The user supplied `context/design/Virtual Gamepad.jpg` and requested swapping its D-pad and left stick positions. Use the image's arrangement with the left stick outside, D-pad lower/inward, right stick lower/inward, face diamond outside (Y above, A below, X left, B right), and shoulder controls at the top corners. Use Xbox LT/LB/RT/RB and Back/Start labels. The image guides placement only; keep the existing theme and defer final styling and settings to Spec 0008.

LT/RT remain analog position controls as required above. The functional adaptation uses horizontal lanes at the upper corners, labelled 0 at the outside and 1 at the inward end, with clamped absolute position and exact zero on release. LB/RB sit separately below. Dedicated L3/R3 sit inward/above their sticks. The utility row remains outside gameplay captures with an 8 dp separation. No settings or Guide control is inferred from the reference's top bar.

One shared geometry model fits the safe gameplay rectangle. Use an 8 dp internal gutter inside the unchanged safe insets and a 640 by 264 dp minimum, 112 dp stick bases with 8 dp capture expansion, 56 dp face targets in a diamond with 48 dp center offsets, a 152 dp D-pad sector circle, 144 by 48 dp trigger lanes, 112 by 48 dp shoulders, and 48 dp stick-click targets. Larger windows may spread centers while preserving capture sizes and an 8 dp gap; windows that fail containment or separation accept no input. D-pad ownership is one pointer for the whole region, with eight equal angular sectors, a neutral center of 20% radius, outside release, and same-region reentry. This allows direction transitions and perpendicular diagonals without opposing directions or pointer transfer.

Both sticks retain the accepted transform and shared knob travel. Dead zone stays 0.15 and sensitivity 1.0; Spec 0008's older 0.08 draft is stale and has no authority here. The 40-byte protocol and shared fixtures remain unchanged. Full controls are still admitted only through the explicit debug development trust path; release admission remains closed until the later pairing work.

## Rust mapping

- Map every protocol field to the corresponding Xbox 360 report field.
- Centralize mapping in a pure function tested against named examples.
- Reject invalid protocol state before mapping.
- Preserve exact neutral report and timeout behavior.
- Do not implement keyboard/mouse fallback inside the controller mapper.

## Automated tests

- Each button bit independently and meaningful combinations.
- No Guide bit/reserved bit can be produced by Android or accepted by Rust.
- D-pad cardinals, diagonals, transitions, and opposition prevention.
- Left/right axes endpoints, center, diagonal, and independent movement.
- Trigger `0`, midpoint rounding, and `255`, independently and together.
- L3/R3 independently and while axes move.
- Multi-pointer assignment for at least: both sticks; stick+A+B; both triggers+stick; D-pad+face; LB+RT; Start while other controls held.
- Pointer release/cancel clears only owned control; global cancel clears every field.
- Full mixed-state golden fixture encodes/decodes/maps identically.
- Timeout/backend fault/shutdown makes a byte/report-exact neutral state.

## Manual controller-tester matrix

On a physical phone and Windows controller tester:

1. Verify each control name/value individually.
2. Verify both sticks simultaneously and full circle/center.
3. Verify L3/R3 without unintended axis/button changes.
4. Verify D-pad cardinals/diagonals and fast transitions.
5. Verify analog trigger sweep is smooth enough for racing and releases to zero.
6. Verify simultaneous combinations representative of football/racing gameplay.
7. Repeat lifecycle, Wi-Fi loss, timeout, and agent shutdown while complex input is held.

Then test PES 2017 or another available native XInput game and one racing title if available. Record mappings/behavior; do not tune protocol around one game's remapping screen unless it exposes a genuine Xbox mapping bug.

## Reliability checkpoint

### User-approved recovery policy (2026-09-09)

The user explicitly authorized replacing disconnect-on-input-timeout because ordinary Wi-Fi stalls made the controller unusable. This decision supersedes the Spec 0004/0005 short-timeout detach policy and earlier explicit-reconnect requirements for brief delivery gaps only.

- Keep the configured input neutral deadline (default 250 ms, bounds 100–1000 ms). At that deadline submit exact neutral once, keep the same controller attached and retain source/session/sequence admission for a fixed three second idle window measured from the last valid admitted snapshot.
- A strictly newer validated full snapshot within the idle window resumes input automatically. Never re-submit the retained pre-timeout snapshot. Duplicate, reordered, malformed, wrong-source and wrong-session packets do not refresh either deadline. Sequence freshness is not an authenticated measurement of packet age; the protocol's client clock remains diagnostic.
- At three seconds without valid input, close/detach and refuse the old session even if the output worker is delayed. A fresh explicit Connect/hello is required after this terminal expiry. Initial unarmed handshake expiry remains five seconds.
- Timeout status with backend ready and attached means Degraded, not a terminal connection error. Keep Android's 60 Hz sender and touch surface active. A terminal timeout (not ready/attached), backend fault, three second PC-status silence, lifecycle exit, explicit Disconnect or shutdown still closes the connection. Backend faults require agent restart.
- No wire field, message size, fixture, trust boundary, tuning or cadence changes. This is recovery from transport gaps, not a claim to eliminate their latency. Verify neutral timing and automatic recovery separately, including hard expiry, failed neutral submission, source/session/sequence rejection, and repeated gaps without device churn or backlog.

This checkpoint also owns the physical acceptance work deferred from Spec 0005 by the user on 2026-09-08. Complete the outstanding held input timeout/lifecycle/network/backend/shutdown matrix, including measured default 250 ms release behavior, physical repetitions and resource/cadence/log evidence. Investigate the known delivery stalls and resolve them or obtain an explicit acceptance decision before calling the reliability gate passed. A short successful run with the 1000 ms development workaround does not satisfy these requirements. The exception permits Spec 0006 development; it does not waive this checkpoint or later release gates.

Run:

- Full automated checks.
- 30-minute active soak.
- At least 20 background/foreground cycles.
- At least 20 network interruption/reconnect cycles at this stage; final target is 50.
- Burst/reorder/loss simulation with mixed states.

No later feature starts if a control can remain held or stale state can replay.

## Exit criteria

- Every planned field passes unit, cross-language, Windows tester, and physical multi-touch checks.
- At least one representative native game accepts the controller.
- Complex held state always neutralizes on every proven failure path.
- No touch region overlap/pointer stealing is observed in tested geometry.
- No Guide/haptics exists.
- Remaining visual/layout issues are non-safety items assigned to 0008.

## Out of scope

- Layout customization, discovery, real pairing, tray/installer, multiple controllers, keyboard/mouse, rumble.

Physical fit correction: the API 34 screenshot showed the initial 24 dp gutter and 280 dp floor refusing the usable window. The measured safe inset/utility budget with an 8 dp gutter leaves about 775 by 279 dp. The 264 dp floor is the smallest height that preserves the 48 dp shoulder, 8 dp gap and 56 dp face targets in this functional arrangement. Capture containment and gap checks still run before enabling input.
