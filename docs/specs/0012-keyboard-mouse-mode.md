# Spec 0012 — Keyboard/Mouse Compatibility Mode

Status: **Ready**  
Milestone: M4 optional compatibility  
Depends on: reliable controller core; preferably 0010 packaging  
Primary changes: PC output/profile backend, tray/settings, tests/docs

## Outcome

Add an explicit, opt-in mapping mode for browser/legacy games that do not accept gamepads. The phone still sends controller state; the PC converts selected controls to Windows keyboard/mouse input and guarantees release on every timeout/exit/fault.

## Product rules

- Xbox controller mode remains the default and is never silently replaced.
- User selects a named keyboard/mouse profile in the PC tray/settings.
- UI clearly warns that `SendInput` is subject to Windows privilege/UIPI, game, and anti-cheat restrictions.
- Do not attempt anti-cheat bypasses, kernel input injection, or elevation tricks.
- No raw Android touch coordinates are sent; protocol controller state remains the input abstraction.
- Switching profiles neutralizes the old output before enabling the new one.

## Output abstraction

Generalize application output without weakening `VirtualGamepadBackend`, for example:

```text
InputOutputBackend
  apply(ControllerState)
  neutral()
  health()
  shutdown()
```

Xbox and keyboard/mouse implementations remain separate adapters with common lifecycle safety. A recording fake verifies emitted transitions.

## Profile model

- Versioned, validated local TOML or equivalent.
- Built-in starter profiles plus user-copy/edit workflow; no downloaded marketplace.
- Map digital controller controls to allowlisted keyboard scan codes/virtual keys and mouse buttons.
- Map axes/triggers to configurable digital thresholds or bounded relative mouse motion where explicitly selected.
- Define press/release hysteresis for analog-to-digital mapping to prevent chatter.
- Disallow mappings that invoke system-dangerous sequences by default; document any override.
- Reject unknown schema versions, duplicate/conflicting mappings where unsafe, out-of-range speed/thresholds, and unsupported keys.
- Keep active emitted key/button set as authoritative state so neutral can release everything actually held.

## Windows integration

- Use the current official Win32 `SendInput` API through a narrow `windows`/`windows-sys` adapter.
- Prefer scan-code semantics where appropriate and test keyboard-layout consequences.
- Handle partial/failed sends; failure triggers best-effort release and backend fault state.
- Relative mouse motion is rate-limited and computed from latest state/time, not a queued history.
- Runtime stays at ordinary integrity; document that it cannot inject into a higher-integrity target due to UIPI.

## Safety

Every path releases all emitted keyboard keys and mouse buttons:

- Controller state release.
- 250 ms valid-state timeout.
- Session disconnect/replacement.
- Android lifecycle stop/Wi-Fi loss.
- Profile switch/disable.
- Backend error.
- Agent shutdown/tray exit.

Mouse motion stops immediately when state is neutral; there is no stored movement queue.

## Automated tests

- Profile parse/validation/migration and safe defaults.
- Each digital mapping press/release and multi-key combination.
- Analog threshold hysteresis and trigger mapping.
- Bounded mouse velocity/acceleration and latest-state semantics.
- Profile switch releases old output before new output.
- Timeout/disconnect/backend failure/shutdown releases exact active set once/idempotently.
- Partial `SendInput` failure enters fault and attempts safe release.
- Xbox mode regression is byte/report identical and default.

## Manual test

1. Use a local test window/page that visibly reports keys/mouse and does not require elevation.
2. Verify every selected mapping, simultaneous input, hold/release, and profile switch.
3. Hold inputs then background phone, disable Wi-Fi, switch profile, stop agent; no key/mouse button remains held and motion stops.
4. Test one keyboard-only browser game and one selected legacy game if available.
5. Test an elevated target only to confirm/document the expected limitation; do not weaken system security.
6. Confirm controller/Gamepad API mode still works unchanged.

## Exit criteria

- Mode is opt-in, profile-driven, validated, and disabled by default.
- All-key/button release matrix passes automated and manual tests.
- Browser/legacy sample works where Windows policy permits.
- UIPI, anti-cheat, layout, and compatibility limits are visible in documentation/UI.
- Xbox controller reliability has no regression.

## Out of scope

- Anti-cheat bypass, kernel driver input injection, macros/scripts/automation, remote desktop control, raw touch streaming, per-game automatic detection.
