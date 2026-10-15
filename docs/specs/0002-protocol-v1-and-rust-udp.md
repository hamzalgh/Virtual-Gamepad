# Spec 0002 — Protocol v1 and Rust UDP

Status: **Accepted** (automated checks, Windows manual evidence, and approving follow-up review recorded)
Milestone: M0  
Depends on: 0001  
Primary changes: `context/protocol-spec.md`, `protocol/fixtures/v1/`, `pc_agent/`

## Outcome

Implement the normative 40-byte protocol codec and a robust Rust UDP receive/status foundation that can be tested without Android or a virtual controller.

## Scope

- Every v1 message type and exact field offset.
- CRC-32/ISO-HDLC.
- Strict errors/validation.
- Wrap-safe sequence helper.
- UDP bind/receive/reply loop on configurable local address/port, default `0.0.0.0:8765`.
- Latest-state single-slot semantics using Tokio `watch` or a proved equivalent.
- Local test sender/integration tests.
- Structured, rate-aware diagnostics.

Pairing policy and virtual input are not activated yet, although their packet types must round-trip.

## Design requirements

- `protocol` is pure and does no I/O.
- Decode accepts a byte slice and returns a validated typed message or a typed rejection category.
- Encode always produces `[u8; 40]` and zeroes all reserved data.
- A datagram receive buffer is larger than 40 bytes; exact reported length is checked before decode.
- Unknown versions/types and nonzero reserved fields are rejected.
- Controller-state validation rejects reserved button/state bits and opposing D-pad directions.
- No parser path panics for arbitrary bytes.
- Sequence freshness implements the exact half-range rule in the protocol spec.
- Network code never logs all packet bytes or floods a log per malformed packet.
- The latest-state abstraction makes it impossible to accumulate an unbounded state backlog.

## Tasks

1. Refresh Tokio/CRC API documentation with Context7/upstream sources and record pinned versions.
2. Implement constants, typed IDs/enums, message payloads, encode/decode, CRC, ranges, and errors.
3. Create canonical binary fixtures plus a readable manifest for every valid message shape and key invalid cases.
4. Implement configuration for bind address/port with safe validation.
5. Bind a Tokio `UdpSocket`, receive into a sufficiently large buffer, parse, and route messages to a test/session shell.
6. For a valid hello in a test-only path, send a deterministic/test-configured hello acknowledgement or expose a handler interface; do not invent production pairing behavior.
7. Publish valid controller state into a latest-value holder only after a test admission hook accepts it.
8. Track accepted, invalid, stale, and oversized counters.
9. Add cancellation/clean shutdown so the socket task terminates in tests and on Ctrl+C.

## Automated tests

- Byte-exact encode/decode for each type.
- Neutral, all-buttons, axis extremes, mixed values, and timestamp/sequence extremes.
- CRC known check plus single-byte corruption at header/payload positions.
- Lengths 0, 39, 40, 41, and larger UDP payloads.
- Bad magic/version/type/reserved/button/D-pad data.
- Normal, duplicate, reordered, half-range, and wrapping sequence comparisons.
- Arbitrary byte/property/fuzz-style decode does not panic.
- Loopback hello/reply through real UDP sockets.
- Burst state input with delayed consumer applies only the newest accepted state.
- Shutdown closes promptly without task leakage.

## Manual verification

1. Run the agent on Windows with debug logging.
2. Confirm it binds `0.0.0.0:8765` and prints the logical local addresses without claiming a UDP connection.
3. Use the local test sender to deliver a valid fixture and one malformed fixture.
4. Confirm counters/rejection category change and the process remains healthy.
5. Stop with Ctrl+C and confirm clean shutdown.

## Exit criteria

- Protocol tests and shared fixtures are comprehensive and pass.
- Rust standard checks pass.
- Real loopback UDP smoke test passes.
- Oversized/malformed input cannot reach latest state.
- Continuous state storage is bounded/latest-only.
- Protocol documentation and implementation match byte for byte.

## Out of scope

- Android codec/networking, real sessions/pairing, Windows firewall installer, ViGEm, game input, discovery, and tray UI.
