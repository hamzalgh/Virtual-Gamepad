# Spec 0010a — Settings and Layout Customization

Status: **Proposed**
Registered: **2026-09-11**
Depends on: 0010 desktop tray and packaging
Required before: 0011 multiple controllers
Primary changes: Android settings, layout editing, profile persistence and tests

## Approved scope and order

The user explicitly requests a separate settings slice after Spec 0010 and before Spec 0011. This includes settings simplification together with user drag/rearrange controls and saved layout profiles. These are scheduled requirements, not merely optional feedback. The suffix preserves existing spec numbers.

This document registers the scope and order. Implementation has not started. Detailed interaction and storage decisions must be completed before marking this spec Ready. It does not expand Spec 0008 or waive any existing reliability or release gate.

## Outcome

Make Settings useful and understandable for everyday play. Let the user arrange controls to suit their hands and save layouts they can select again.

## Required capabilities

1. Simplify Settings, with everyday controls and layout options easy to find. Place technical tuning and diagnostics in clearly separated secondary sections with plain explanations.
2. Provide an explicit edit mode where the user can drag and rearrange gameplay controls.
3. Provide saved layout profiles and a clear way to select a profile and return to the default reference layout.
4. Preserve the Show L3/R3 preference and the approved default appearance. Dead zone defaults to 0.15 and sensitivity to 1.0.
5. Keep editing and profile changes safe: neutralize held input, clear pointer ownership, and require fresh gameplay touches afterward. Editing gestures must not send game input.
6. Validate layouts against safe screen bounds, minimum touch targets and control separation. Invalid or incompatible saved layouts must have a safe recovery path.
7. Persist user choices locally without changing the protocol, network cadence, admission, timeout policy or isolated backend.

## Design work before implementation

Specify edit mode entry/exit, save/cancel/reset behavior, movable control groups, navbar constraints, profile creation/naming/selection/deletion, persistence format and migration, and adaptation to different safe window bounds. Decide whether resizing is included; it is not implied by the drag/rearrange request. Define exact acceptance tests for persistence, invalid layouts, cancellation, simultaneous touch after editing, and restoring defaults.

## Verification plan

Run geometry, profile validation, persistence and state cancellation tests, plus Android build/lint checks. Compile UI tests and request user physical review of editing, profile switching and resumed gameplay. Do not operate the phone or launch a device campaign without specific authorization.

## Sequencing boundary

Complete this settings and layout slice before starting Spec 0011. Formal Spec 0005/0007 evidence and other unverified platform claims remain recorded independently. No multiple controller implementation is included here.
