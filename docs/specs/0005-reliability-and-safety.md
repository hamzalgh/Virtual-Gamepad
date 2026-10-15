# Spec 0005 — Reliability and Safety

Status: **In Progress**
Milestone: M2 prerequisite  
Depends on: 0004 gate passed  
Primary changes: both applications, tests, local diagnostics

## User-approved development gate exception (2026-09-08)

Implementation and automated checks are complete for this slice. At the user's request, the remaining physical acceptance work and investigation of intermittent delivery stalls move to the reliability checkpoint in Spec 0007. Spec 0006 may begin now; this supersedes the earlier instruction to block it until all Spec 0005 physical evidence passes. Status remains In Progress because the deferred evidence has not passed.

Spec 0007 owns measured held input neutral timing, the outstanding lifecycle/network/backend/shutdown matrix, repeated background/reconnect tests, the 30 minute physical soak with resource/cadence diagnostics, and resolution of the delivery stalls or an explicit acceptance decision about remaining limitations. Its checkpoint must account for both this slice's outstanding requirements and mixed controls. These obligations are not dropped or silently deferred to release certification.

Keep the default timeout at 250 ms, its validated 100–1000 ms range, strict 60 Hz snapshots, explicit reconnect, validation and neutral cleanup. The explicit `--input-timeout-ms 1000` development workaround may be used during feature tests, with the slower loss cleanup documented. It is not proof of the default deadline or a fix for delivery stalls. Existing automated safety regressions and each new control's physical cancellation/neutral checks remain required. This exception does not authorize release, multiple phones, or removal of later reliability gates.

## Outcome

Harden the one-button path so packet loss, delay, reordering, duplicate traffic, Android lifecycle changes, Wi-Fi loss, backend failure, and ordinary process shutdown always converge to current or neutral output without backlog.

## Invariants

- Only an exact, valid, admitted, source-matching, fresh state may reach the backend.
- Only such a state refreshes the valid-input deadline.
- Continuous state storage is one latest value, never an unbounded queue.
- Default neutral timeout is 250 ms and configurable only within 100–1000 ms.
- After timeout, submit neutral once and remain neutral until a fresh valid state or new-session transition.
- Android local state becomes neutral before/while its sender is stopped.
- PC safety does not depend on delivery of Android's best-effort disconnect.
- A logging/telemetry failure cannot hold input or block the hot path.

## Execution policy

The user-approved Spec 0007 recovery policy on 2026-09-09 supersedes the short-input-timeout detach rule below: neutral remains due at 250 ms, the attached session recovers on newer valid input within three seconds, and terminal idle expiry still detaches and requires explicit reconnect. Earlier implementation evidence remains historical, not evidence for the new policy.

Retain Spec 0004 explicit reconnect: timeout neutralizes and detaches, old session input is refused, and a fresh hello is required. Backend faults require process restart. Admission checks its own monotonic deadline so a delayed output worker cannot allow traffic to revive an expired session.

The existing three second Android status silence deadline remains. One second without admitted status, no accepted state, or observed rate below 45 Hz shows degraded health without automatic reconnection. Echo age includes state age and scheduling and is not pure network RTT. Cadence measurements exclude cleanup sends and publish at most twice per second. Local totals saturate, wire accepted/rejected totals wrap, and wire loss saturates.

Transport checks output transitions every 5 ms and sends best effort status alongside the periodic 2 Hz report. The same validated timeout value controls admission, output and advertised timeout. Bounded terminal logging drops on full/error rather than delaying safety. Physical campaign evidence remains required for acceptance.

## Android work

- Implement a monotonic 60 Hz cadence negotiated from the hello acknowledgement. Avoid cumulative drift from repeated fixed delays; use a deliberate scheduler and measure actual intervals.
- Read one atomic/immutable latest `ControllerState` per tick and encode a full snapshot.
- Keep socket/send loop on I/O dispatcher and UI state updates bounded.
- Avoid per-tick recomposition or allocation where practical; optimize only with measurement.
- Maintain wrapping sequence and monotonic diagnostic timestamp.
- Receive 2 Hz/event status without blocking sends and expose accepted sequence, observed rate, age/RTT estimate, timeout/backend flags, and degraded state.
- Define session health thresholds as UI policy; never infer readiness only from local send success.
- On controller exit, lifecycle pause/stop, display lock, permission loss, network loss, socket error, or app shutdown: neutralize local state, best-effort send neutral/disconnect, cancel sender/receiver, clear keep-screen-on, and require hello before resuming.
- Ensure touch cancellation/global reset runs before or with lifecycle shutdown.

## Rust work

- Complete session/source/sequence validation for the manual-development session.
- Separate receive/decode/admission from backend application.
- Publish accepted state through a single-slot/latest-value mechanism.
- Track last valid state with Tokio monotonic time.
- Implement explicit input states: unpaired, active, timed out, backend fault, shutting down.
- Use a timer that cannot be starved by packet/error logging and that checks the valid-input deadline.
- Submit status at 2 Hz plus timeout/backend/session transitions.
- Add local counters for accepted, stale, invalid by category, wrong source/session, estimated loss, backend updates/failures, and timeout transitions.
- Rate-limit repetitive malformed/admission logs while keeping counters accurate.
- Handle Ctrl+C and ordinary task failure through coordinated cancellation, neutral, detach, and socket close.
- If backend submit fails, attempt neutral/detach and refuse further unsafe updates until an explicit recovery/re-attach policy succeeds.

## Fault-injection harness

Create a development-only local sender/proxy or deterministic integration helper that can:

- Drop selected state datagrams.
- Duplicate packets.
- Reorder a bounded window.
- Delay packets/consumer work.
- Burst many states.
- Corrupt bytes/CRC/reserved fields.
- Change source socket/session.
- Stop without disconnect.

It must bind only locally/private test scope and never become an exposed production service.

## Automated tests

- 60 Hz scheduler uses monotonic time and cancels promptly.
- Sequence gaps update loss diagnostics; duplicates/reordering never alter backend state.
- Delayed consumer sees only newest accepted state.
- Timeout occurs at the configured deadline using fake/paused time and neutral is not spammed repeatedly.
- Invalid/stale/wrong-source traffic does not delay timeout.
- Fresh state after timeout follows the defined reactivation policy and applies safely.
- Disconnect neutralizes immediately only when session/source are valid.
- Android lifecycle events neutralize/cancel exactly once and reconnect with new hello/session.
- Socket/permission/network/backend errors yield safe state and recoverable status.
- Counters wrap/saturate as documented and logs are rate-limited.
- Graceful shutdown ordering is deterministic and idempotent.

## Manual campaign

1. Hold A while backgrounding, locking, force-stopping Android, disabling Wi-Fi, stopping the router briefly, closing the socket/agent, and triggering any safe backend failure available.
2. Confirm Windows releases A within 250 ms plus one reasonable scheduler/backend update interval whenever valid state stops.
3. Run loss/reorder/duplicate/burst modes and observe no replayed old presses after a newer release.
4. Run for 30 minutes with periodic input and inspect memory, CPU, packet/status rate, timeout count, and logs.
5. Repeat at least 20 background/foreground cycles and a preliminary set of reconnect cycles.
6. Confirm the phone does not stay awake after leaving controller mode.

## Exit criteria

- Every invariant has an automated test or named manual evidence.
- No safety case relies solely on disconnect delivery.
- Latest-state boundedness is demonstrated under bursts/slow consumer.
- Timeout and lifecycle matrices pass for A.
- Soak shows no unbounded memory/log growth or stuck state.
- Status diagnostics are truthful enough to debug later controls.

## Out of scope

- More controls, visual polish, pairing/discovery, multiple sessions, installer/tray, and public-LAN security.
