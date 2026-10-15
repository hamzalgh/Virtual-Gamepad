# Spec 0003 — Android Connection and Hello

Status: **Accepted**
Milestone: M0/M1 prerequisite  
Depends on: 0002  
Primary changes: `mobile_controller/`, small session/ack changes in `pc_agent/`

## Outcome

A physical Android phone can enter the PC's private IPv4 address and port, persist them, obtain required network permission, exchange valid protocol hello/ack/status datagrams, and show truthful backend/session health.

## Scope

- Kotlin protocol v1 codec against the shared fixtures.
- Manual IPv4/port setup and persistence.
- Android internet/local-network permission flow.
- Lifecycle-owned single UDP socket with bidirectional hello/ack/status.
- Rust manual-development trust mode for one first client, explicitly unsafe and non-release.
- Clear setup/retry/error state.

No gameplay control is exposed yet.

## Android design

- Use immutable UI/session state exposed from a ViewModel via `StateFlow` and collected lifecycle-aware.
- Persist PC IPv4 string, port, dead-zone default, sensitivity default, and client instance ID in DataStore. Do not persist session IDs/nonces.
- Validate ordinary private/local IPv4 input and port `1..65535`; default port `8765`. Hostnames/IPv6 are deferred unless deliberately added to the protocol plan.
- Generate nonzero client instance ID once using `SecureRandom` and persist it.
- Create/bind one UDP socket on an I/O dispatcher; use the same socket for send and receive.
- Encode/decode only through the pure protocol module.
- Match hello acknowledgements by client nonce and ignore unrelated/wrong-source responses.
- Retry hello no faster than 500 ms with cancellation and an overall recoverable UI state.
- The UI says `Ready` only after valid ack with backend-ready flag and nonzero provisional session. A successful `send()` alone is not readiness.
- Cancel socket work and clear session on ViewModel/session close; do not launch networking directly from a composable body.

## Android permissions

- Include `INTERNET`.
- Include network-state permission only when used for truthful diagnostics/discovery preparation.
- Implement the current Android 17/API 37 local-network permission declaration/runtime request as required by the pinned target SDK.
- Show a concise rationale tied to finding/connecting to the PC on the same Wi-Fi.
- Permission denial remains recoverable and never looks like a driver failure.

## Rust manual-development trust

- Add an explicit command/config flag such as `--manual-trust-first-client`; default behavior and release policy are documented as unsafe/non-release.
- When enabled, every valid hello receives a truthful acknowledgement. If no safe backend is ready yet, the acknowledgement keeps session `0`, slot `0xFF`, and backend-ready false; the phone shows PC reachability plus a driver/backend prerequisite state. Once a safe backend is ready in 0004, the first client can receive a source-bound provisional session.
- Only one active source/client instance is accepted.
- A new hello for that same instance/source rotation invalidates the old session.
- Do not persist “paired” status in this mode and set the documented manual-trust flag.
- Pairing packet codecs remain inactive until 0009.

## Automated tests

### Kotlin

- Encode/decode every shared valid fixture byte-identically.
- Reject every marked invalid fixture.
- Endpoint/port validation, settings persistence mapping, and client ID creation.
- Session reducer: permission, reaching, ack mismatch, ready, timeout, retry, cancel, wrong source.
- Socket/network code tested with a fake transport where Android local tests cannot bind reliably.

### Rust

- Manual-trust disabled rejects/advises correctly.
- First-client claim, second-client rejection, same-instance reconnect, source binding, and old-session invalidation.
- Hello ack fields/flags/timeouts match protocol.

## Manual physical test

1. Put phone and Windows PC on the same 2.4 GHz home Wi-Fi.
2. Confirm router client/AP isolation is off or detect/report if devices cannot reach each other.
3. Start the Rust agent with manual-development trust enabled.
4. Open Android app, grant local-network permission if requested, enter the PC LAN IPv4 and `8765`, and connect.
5. Confirm the phone reports a valid acknowledgement/backend state and the PC logs one admitted client without logging secrets.
6. Enter wrong IP/port, deny/revoke permission where possible, stop/restart the agent, and verify precise recoverable UI.
7. Reopen the Android app and confirm endpoint settings persist while the session is renegotiated.

For an emulator, document its host-network address/routing separately; emulator success never replaces the physical-phone LAN test.

## Exit criteria

- Kotlin and Rust consume the same fixtures.
- Physical phone exchanges hello/ack over the actual home LAN.
- UI status is acknowledgement/session based.
- Permission denial, wrong endpoint, retry, lifecycle cancel, and agent restart are recoverable.
- No controller packet or virtual input exists yet.

## Out of scope

- Real pairing/discovery, controller surface, fixed-rate state sender, ViGEm, tray, multiple phones, and internet transport.
