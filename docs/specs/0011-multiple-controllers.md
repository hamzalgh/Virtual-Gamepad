# Spec 0011 — Multiple Controllers

Status: **Ready**  
Milestone: M4  
Depends on: M2 reliability, 0009 pairing, 0010 tray, 0010a settings and layout customization
Primary changes: PC session/slot/backend manager, pairing/tray UI, multi-device tests

## Outcome

Support multiple Android phones simultaneously, each mapped to an independent virtual Xbox controller slot, without allowing one phone's traffic, timeout, fault, or disconnect to affect another.

## Capacity

- Target up to four controller slots, matching common XInput expectations.
- Initial acceptance requires two physical/credible independent clients; expand manual evidence to available hardware and keep automated coverage for four.
- Capacity is configured/bounded `1..4` and advertised in hello/discovery.
- One client instance owns at most one active slot.

## Architecture changes

- Replace single session fields with a bounded `ControllerManager` keyed by assigned slot/session.
- Each slot owns independently:
  - Paired client instance record.
  - Active source-bound session and sequence baseline.
  - Latest-state slot/channel.
  - Valid-input deadline and timeout state.
  - Virtual backend instance/report.
  - Counters/status and cancellation scope.
- Shared UDP receive path parses once, then routes only after session/source admission.
- Shared failures (socket/service shutdown) neutralize all slots deterministically; slot-local failures neutralize/detach only that slot.
- Tray displays ordered slots and explicit pair/forget/replace per slot.

## Pairing and assignment

- `requested slot = 0xFF` selects the lowest available permitted slot.
- An explicitly selected slot may be supported by tray flow if it uses the existing v1 field.
- Pairing code/window is associated with a target/available slot and cannot displace another without explicit replace confirmation.
- Persist each paired client ID with slot identity; reconnect attempts return the same slot when available/configured.
- Duplicate client identity, session collision, and simultaneous reconnect are handled deterministically.
- Phone displays assigned slot from `PAIR_ACCEPT`/`HELLO_ACK`.

## Resource/scheduling policy

- Four phones at 60 Hz remain small network traffic, but receive/apply work stays bounded.
- Do not create an unbounded task/channel per packet.
- Backend updates and timeout timers are isolated enough that a slow/faulty slot cannot starve others.
- Logs/counters include slot and redacted session identity.

## Automated tests

- Capacity 1–4, lowest-slot assignment, explicit slot, full-capacity rejection.
- Four paired identities persist/reload and reconnect to correct slots.
- Interleaved sequences/sources route to correct state/backend.
- Wrong session/source cannot cross-control a slot.
- Duplicate client/session collision rejected.
- One slot timeout/backend failure/disconnect leaves others unchanged.
- Shared shutdown neutralizes/detaches every slot exactly once.
- Burst on one slot cannot create backlog/starve another.
- Status/discovery capacity/slot fields are correct.
- Forget/replace affects only target slot.

## Manual test

1. Pair two physical phones or one phone plus a clearly labelled protocol test client to separate slots.
2. Confirm two virtual Xbox controllers appear with stable slot/order behavior.
3. Hold distinct controls on each and verify independent reports/game assignments.
4. Disable Wi-Fi/background/exit one phone; only its controller neutralizes.
5. Flood/delay one test client safely; the other remains responsive.
6. Restart agent and reconnect both; verify slot persistence and new sessions.
7. Test a local multiplayer-capable game when available and record its own controller-order behavior separately.

## Exit criteria

- At least two simultaneous clients pass real independent-input/failure tests.
- Automated four-slot routing/capacity/safety tests pass.
- No cross-slot state, timeout, sequence, source, or backend leakage.
- CPU/memory/logs remain bounded at four 60 Hz streams.
- Existing one-controller UX remains simple when capacity is one.

## Out of scope

- One phone controlling multiple slots, internet multiplayer, account-based pairing, dynamic game-specific slot reordering, more than four XInput controllers.
