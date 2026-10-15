# Android UI and Controller Guidelines

## Design intent

The product should feel like a serious utility controller: dark, calm, legible, immediate, and physically predictable. Visual style must never reduce touch reliability or usable controller area.

Use a minimal/Swiss-inspired information hierarchy for setup and diagnostics, and a restrained console-like surface for gameplay. Avoid decorative pixel-art, glass effects, long animation and visual noise. The approved Spec 0008 reference uses black surfaces, pale violet rims and soft radial edge highlights on gameplay controls.

## Experience principles

1. **Touch certainty:** a user can feel where controls are from consistent placement and large separated capture regions.
2. **Immediate local response:** pressed/analog feedback follows the finger locally and never waits for the PC.
3. **Safe failure:** connection and timeout states are visible without covering core controls; controller input neutralizes on exit.
4. **Phone independence:** geometry derives from usable safe bounds, not brand/model checks or one screenshot size.
5. **Low motion:** input feedback changes color/fill/stroke, not layout position or distracting animation.
6. **No haptics:** the experience cannot rely on vibration; visual feedback and consistent geometry carry the interaction.

## Screens

### Setup and connection

The setup screen contains:

- Product name and a one-sentence private-LAN explanation.
- Discovered PC list when discovery is available.
- Manual PC IPv4 address and UDP port fields, always retained as fallback.
- Local-network permission rationale/action when required.
- Connect/retry action.
- Pairing-code field when requested by the PC.
- Driver/backend status returned by the PC.
- Expandable tuning/settings area for dead zone and sensitivity.
- Clear troubleshooting link/copy for same-Wi-Fi, Private firewall, and router client isolation.

Do not call UDP “connected” merely because a send call succeeded. Status is `Ready` only after a valid acknowledgement/status indicates backend readiness and a session exists.

### Controller

Controller mode is:

- Locked landscape and not sensor-rotated when the phone moves.
- Edge-to-edge immersive with system bars hidden.
- Laid out inside current safe drawing/gesture bounds.
- Kept awake only while this screen is active.
- Fixed in control placement for v1; customization is deferred.

The integrated top navbar contains only gear, Back, Start and an X exit icon. Put status, development trust text and diagnostics behind the gear. Gear and Exit have separate validated touch regions; Back and Start remain held controls under the root router.

## Layout system

### Usable rectangle

Start with the window's safe content rectangle derived from current insets, including cutouts, rounded corners, and gesture areas. Add a small internal safety gutter. Never position controls against raw screen edges or assume hidden bars have no reserved/unsafe area.

Conceptually split the usable width into:

- Left control zone: approximately 42%.
- Center utility gap: approximately 16%.
- Right control zone: approximately 42%.

These proportions are guidance, not independent boxes. A single layout solver/custom `Layout` must ensure capture regions do not overlap.

### Default control placement

- Top-left: LT analog rail at the outer edge; LB as a wide shoulder region below/inside it.
- Top-right: RT analog rail; RB below/inside it.
- Outer left zone: left stick, above the D-pad center.
- Lower inward left zone: D-pad. The updated reference already includes the user's requested swap.
- Upper right zone: A/B/X/Y diamond.
- Lower right zone: right stick.
- Top center: gear, Back, Start and X exit, with generous separation.
- L3/R3: dedicated labelled digital buttons immediately inward/above their respective sticks. A persisted Show L3/R3 switch may hide their visuals and capture regions together; default is visible. Do not hide stick-click behind pressure or an ambiguous double-tap; ordinary phones cannot sense a physical stick press reliably.

Both stick centers remain fixed. Floating-stick behavior is out of v1 because it reduces positional predictability and complicates shared capture zones.

### Analog trigger interaction

Phones do not provide reliable pressure input, so LT/RT are visible analog rails:

- The capture is a large top-corner lane, rendered as a curved petal labeled `L2` or `R2`; Settings explains the unchanged LT/RT mapping.
- Absolute touch position within the rail maps from released to full; drag updates continuously.
- Fill level and numeric/stepped debug feedback make the value obvious during development.
- Release returns immediately to zero.
- LB/RB remain separate digital capture regions and cannot overlap the rail.

The exact rail orientation and reach must be validated on the user's phone in Spec 0007. If physical testing shows vertical rails are uncomfortable, the spec may choose horizontal inward rails without changing the protocol.

### Responsive sizes

Calculate from the shorter safe dimension and clamp to tested density-independent bounds. Initial design ranges:

- Stick visual diameter: `112–180 dp`.
- Stick capture diameter: visual diameter plus `12–24 dp`, while remaining non-overlapping.
- Face/D-pad visible target: `56–84 dp` depending on fit.
- Shoulder height: compressed `48 dp`, expanded `64–80 dp`; expanded triggers are `72–90 dp` high, per Spec 0008.
- Back/Start/exit targets: at least `48 dp` in both dimensions.
- Gap between unrelated capture regions: target `8 dp` or more; never zero/overlap.

Use two geometry profiles chosen from usable bounds, not device names:

- `Compact`: width below roughly `720 dp` or height below `340 dp`.
- `Regular`: larger usable landscape bounds.

Breakpoints are starting values and must be adjusted from screenshots and physical testing. If the layout cannot satisfy minimum capture/gap constraints, show a clear unsupported-window/rotate state rather than silently overlap controls.

### Capture-region rules

- Visual and touch bounds are modeled separately and can be displayed in a developer overlay.
- Every pixel belongs to at most one active control region.
- Hit-test priority is deterministic and unit-tested.
- A pointer keeps its original control ownership until up/cancel; it does not slide into an unrelated button.
- D-pad sectors deliberately support diagonals and never produce opposing directions.
- Moving outside a held digital control releases it but retains ownership; moving back inside may press that same control again. It never invents a press on a different control.
- System-gesture edges receive extra caution even in immersive mode.

## Visual tokens

Implement tokens in one Compose theme file; do not scatter literal values through controls.

### Color roles

- `background`: near-black graphite.
- `surface`: dark neutral controller plate.
- `surfaceRaised`: distinct control surface.
- `outline`: visible neutral separation.
- `textPrimary`: near-white.
- `textSecondary`: muted cool gray that still passes contrast.
- `accent`: cool cyan/blue for focus, connection, and analog fill.
- `success`: green for healthy session.
- `warning`: amber for degraded status.
- `danger`: red for disconnected/backend error.
- Gameplay face buttons use the reference monochrome pale violet rims and letter labels; pressed surfaces fill and reverse label contrast.

Color alone never communicates pressed state or connection health. Pair it with label, icon/shape, stroke/fill change, or text.

### Typography

- Use the Android system/default sans family unless a bundled font has a demonstrated readability benefit.
- Setup title: strong but compact.
- Body/settings: readable at system font scale.
- Controller labels: bold, short, and optically centered.
- Diagnostic numbers use tabular figures where supported.

Do not require condensed or novelty fonts for game controls.

### Shape and elevation

- Sticks and face buttons are circular.
- D-pad uses four rounded inward-pointing petals over the existing sector capture.
- Shoulder/trigger controls use mirrored diagonal petals over conservative rectangular captures.
- Setup surfaces use modest consistent radii.
- Prefer outlines, tonal contrast, and spacing over heavy shadow. Controller mode should remain readable in dark and bright rooms.

### Motion

- Press feedback appears in the same rendered frame where practical.
- Use fill/stroke/opacity changes; do not move a control under the finger.
- If animated, transition duration is at most about 80 ms and interruptible.
- Connection-state animation is subtle and stops when not useful.
- Respect reduced-motion settings for nonessential motion.

## Interaction details

### Joysticks

- Base and knob always show center and current displacement.
- Knob clamps to the circular gate.
- Dead-zone can be shown only in developer/settings visualization, not as gameplay clutter.
- On release/cancel, knob and state return immediately to center.
- Sensitivity changes the output curve, not the visual geometry unexpectedly.

### Buttons and D-pad

- Pressed state uses at least two cues: e.g. filled surface plus stronger outline/label change.
- Visual feedback is local even when network is degraded.
- Network status does not disable release handling.
- Labels remain visible under normal thumb coverage as much as geometry allows.

### Status and errors

Status vocabulary:

- `Offline`: controller mode not active.
- `Reaching PC`: hello pending.
- `Permission needed`: Android local-network access missing.
- `Pairing needed`: agent reachable but no paired identity/session.
- `Ready`: valid session and virtual backend ready.
- `Degraded`: recent status reports loss/age/backend warning while input remains available.
- `Timed out`: PC reports neutral due to missing valid state.
- `Driver issue`: PC is reachable but cannot provide a safe virtual controller.

Gameplay status is accessible inside Settings. Blocking errors return to a focused recovery screen; they do not leave an apparently active controller surface.

## Accessibility

- Setup/settings follow normal Android semantics, focus order, labels, error association, keyboard behavior, and scalable text.
- Fixed gameplay labels cap visual font scaling at 1.3 so they remain within capture bounds. Setup, settings and recovery retain the full system text scale; no full gameplay screen-reader claim follows from passive control semantics.
- All actionable targets meet at least Android's recommended touch size; gameplay targets are intentionally larger.
- Face-button colors have letter labels. Status colors have text/icon equivalents.
- Contrast is checked for normal, pressed, disabled, success, warning, and danger states.
- Gameplay multi-touch may conflict with screen-reader gesture interception. Detect/document this limitation honestly and keep setup/recovery accessible; do not claim full accessible gameplay without device testing.
- No instruction relies on vibration, sound, or color alone.

## Orientation, insets, and system UI

- Use fixed `landscape` orientation for the controller activity/flow.
- Apply modern edge-to-edge and `WindowInsetsControllerCompat` immersive handling.
- Re-hide transient system bars when appropriate without trapping the user; the explicit exit action remains available.
- Consume `safeDrawing`/`safeGestures` or their current supported equivalents.
- Test display cutouts in both landscape directions because left/right unsafe regions can swap.
- Restore ordinary system UI and clear keep-screen-on when controller mode exits.

## Component registry

The implementation should converge on these reusable components rather than one monolithic composable:

| Component | Responsibility | First spec |
|---|---|---|
| `EndpointForm` | IP/port validation and persistence | 0003 |
| `PermissionPanel` | Local-network rationale/request/retry | 0003 |
| `ConnectionStatus` | Logical session/backend state | 0003 |
| `GamepadSurface` | Safe content bounds and root pointer router | 0004/0006 |
| `DigitalButton` | Visual button with externally routed held state | 0004 |
| `AnalogStick` | Fixed gate/knob visualization | 0006 |
| `DPad` | Four-direction/diagonal visualization | 0007 |
| `AnalogTriggerRail` | Touch-position analog value | 0007 |
| `ShoulderButton` | LB/RB surface | 0007 |
| `ReferenceNavbar` | Gear/Back/Start/X geometry and utility actions | 0008 |
| Controller settings | Session status, visibility and diagnostics | 0008 |
| `TuningPanel` | Dead zone/sensitivity settings | 0008 |
| `PairingPanel` | Discovered host and six-digit code | 0009 |
| `TouchDebugOverlay` | Developer-only regions/pointers/values | 0006 |

Visual components render state; the root input router owns pointer interpretation.

## Review checklist

- Every control exists, including L3/R3 and analog LT/RT; Guide and haptics do not.
- No touch capture regions overlap in compact or regular fixtures.
- Safe insets are applied in both landscape directions.
- Controller stays fixed when the phone moves.
- System bars are hidden in active controller mode and restored afterward.
- Screen awake flag exists only while active.
- Simultaneous stick/button/trigger input is physically tested.
- Local feedback is immediate and never depends on UDP acknowledgement.
- Connection wording reflects acknowledgements/status, not UDP send success.
- Setup is usable at large font scale.
- Dark/light-room contrast and color-independent states pass review.

## References

- [Compose pointer input and gesture handling](https://developer.android.com/develop/ui/compose/touch-input/pointer-input/understand-gestures)
- [Window insets in Compose](https://developer.android.com/develop/ui/compose/system/insets)
- [Edge-to-edge Compose setup](https://developer.android.com/develop/ui/compose/system/setup-e2e)
- [Keep the screen on](https://developer.android.com/develop/background-work/background-tasks/awake/screen-on)
