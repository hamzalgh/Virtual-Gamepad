# UDP Protocol v1

Status: **normative v1 baseline, Rust codec implemented in Spec 0002 and Kotlin codec in Spec 0003**. Any implementation difference must update this document, implemented codecs, and shared golden fixtures in one change.

## Goals

- Fixed-size, allocation-friendly controller snapshots.
- Explicit message types for hello, pairing, status, and disconnect on one UDP socket.
- Corruption detection, strict parsing, source/session admission, and wrap-safe freshness.
- Latest-state behavior at 60 Hz without acknowledgements for every input packet.
- Enough stable room for diagnostics and future send-rate changes without altering v1 state layout.

This protocol is optimized for a trusted private LAN. It is not encrypted and is not safe to expose to the public internet.

## Transport

- UDP over IPv4 for the first implementation.
- PC default bind address: `0.0.0.0:8765`.
- Android binds one ephemeral local port and sends/receives through the same socket.
- All v1 datagrams are exactly **40 bytes**.
- All multi-byte integers are **little-endian**.
- Receivers must use a buffer larger than 40 bytes and reject any datagram whose reported length is not exactly 40, including oversized datagrams.
- State packets are full snapshots, not deltas.

At 60 state packets/second, the application payload is 2,400 bytes/second per phone before IP/UDP overhead.

## Common envelope

| Offset | Size | Field | Rule |
|---:|---:|---|---|
| 0 | 2 | Magic | Exact bytes `0x56 0x47` (`VG`) |
| 2 | 1 | Version | `0x01` |
| 3 | 1 | Message type | One value from the type registry |
| 4 | 32 | Payload | Type-specific; unused bytes are zero |
| 36 | 4 | CRC-32 | CRC of bytes 0 through 35, encoded little-endian |

### CRC algorithm

Use CRC-32/ISO-HDLC (also called CRC-32/ADCCP):

- Width: 32.
- Polynomial: `0x04C11DB7` (reflected representation `0xEDB88320`).
- Initial value: `0xFFFFFFFF`.
- Input and output reflected.
- Final XOR: `0xFFFFFFFF`.
- Check value for ASCII `123456789`: `0xCBF43926`.

Java `java.util.zip.CRC32` and Rust `crc32fast` implement this family. CRC detects accidental corruption; it is not a message-authentication code.

## Message type registry

| Value | Name | Direction | Purpose |
|---:|---|---|---|
| `0x01` | `HELLO` | Phone → PC | Discover reachability and request/recover a session |
| `0x02` | `HELLO_ACK` | PC → Phone | Advertise readiness, pairing state, and optional session |
| `0x03` | `PAIR_REQUEST` | Phone → PC | Submit the visible pairing code |
| `0x04` | `PAIR_ACCEPT` | PC → Phone | Establish a session and persist the paired device ID |
| `0x05` | `PAIR_REJECT` | PC → Phone | Reject pairing with a bounded reason/retry delay |
| `0x10` | `CONTROLLER_STATE` | Phone → PC | Full current controller snapshot |
| `0x11` | `STATUS` | PC → Phone | Health, counters, and state acknowledgement |
| `0x12` | `DISCONNECT` | Phone → PC | Best-effort immediate neutral/session close |

Unknown types are rejected. They do not refresh any deadline.

## `HELLO` (`0x01`)

| Offset | Size | Field | Rule |
|---:|---:|---|---|
| 4 | 8 | Client nonce | Random per hello/reconnect attempt; nonzero `u64` |
| 12 | 8 | Client instance ID | Random nonzero `u64`, generated once and persisted locally |
| 20 | 2 | Requested state rate | Initial value `60` Hz |
| 22 | 2 | Client capabilities | v1 bits below; unknown bits zero |
| 24 | 4 | Android app build | Monotonic build/version code; zero allowed in development |
| 28 | 2 | Android API level | `u16` |
| 30 | 6 | Reserved | All zero |

Client capability bits:

- Bit 0: supports status messages.
- Bit 1: supports pairing.
- Bit 2: supports discovery metadata.
- Bits 3–15: zero in v1.

The phone retries hello at most every 500 ms with bounded UI feedback. A new attempt uses a new nonce.

## `HELLO_ACK` (`0x02`)

| Offset | Size | Field | Rule |
|---:|---:|---|---|
| 4 | 8 | Echoed client nonce | Must match the active hello attempt |
| 12 | 8 | Server nonce | Random nonzero `u64` for this handshake |
| 20 | 8 | Session ID | Nonzero only when this client may send state now |
| 28 | 2 | Accepted state rate | Initially `60` Hz |
| 30 | 2 | Valid-input timeout | Milliseconds, initially `250` |
| 32 | 2 | Server flags | v1 bits below |
| 34 | 1 | Controller capacity | Initially `1`; later up to `4` |
| 35 | 1 | Assigned slot | `0..3`, or `0xFF` when no session |

Server flag bits:

- Bit 0: pairing required.
- Bit 1: virtual controller backend ready.
- Bit 2: manual development trust mode active.
- Bit 3: discovery is enabled.
- Bits 4–15: zero in v1.

Under the revised Spec 0009 policy, every production client receives session `0` plus `pairing required` here, including a previously paired client after restart or source change. Only a fresh code exchange can grant a new production session through `PAIR_ACCEPT`. Stored identity is metadata only; the wire layout and fixtures are unchanged.

Manual development trust mode may issue a source-bound session to the first client before the pairing feature exists. It must be visibly marked unsafe, disabled by default in distributable builds, and removed from normal release flow.

## `PAIR_REQUEST` (`0x03`)

| Offset | Size | Field | Rule |
|---:|---:|---|---|
| 4 | 8 | Client nonce | From the active hello |
| 12 | 8 | Server nonce | From its matching hello acknowledgement |
| 20 | 8 | Client instance ID | Must match the hello |
| 28 | 4 | Pairing code | Numeric `0..999999`, displayed as six zero-padded digits |
| 32 | 1 | Requested slot | `0xFF` for any available slot in single-controller v1 |
| 33 | 3 | Reserved | All zero |

Pairing policy:

- The PC user explicitly opens a pairing window.
- The window lasts 120 seconds.
- The code is generated with an operating-system cryptographic random source.
- After five failed requests for the same source/client identity, reject for a 60-second cooldown.
- Successful pairing stores the client instance ID locally and invalidates the visible code/window.
- The first product supports one stored paired phone. Replacement/revocation requires an explicit tray/terminal action.

## `PAIR_ACCEPT` (`0x04`)

| Offset | Size | Field | Rule |
|---:|---:|---|---|
| 4 | 8 | Session ID | Fresh random nonzero `u64` |
| 12 | 8 | Echoed client nonce | Must match the request |
| 20 | 8 | Server instance ID | Random nonzero `u64`, persisted by the PC installation |
| 28 | 2 | Accepted state rate | Initially `60` Hz |
| 30 | 2 | Valid-input timeout | Milliseconds, initially `250` |
| 32 | 1 | Assigned slot | Initially `0` |
| 33 | 1 | Accepted capabilities | v1 negotiated bits |
| 34 | 2 | Reserved | All zero |

The receiver binds the session to the datagram source IP and UDP port. Starting a new session for the same paired client invalidates the old session.

## `PAIR_REJECT` (`0x05`)

| Offset | Size | Field | Rule |
|---:|---:|---|---|
| 4 | 8 | Echoed client nonce | Matches the attempted handshake when known |
| 12 | 1 | Reason | Registry below |
| 13 | 1 | Retry after | Seconds, saturated to `255` |
| 14 | 22 | Reserved | All zero |

Reason registry:

- `0`: unspecified.
- `1`: pairing window closed.
- `2`: incorrect/expired code.
- `3`: no controller slot.
- `4`: client already replaced/revoked.
- `5`: rate limited.
- `6`: backend not ready.

## `CONTROLLER_STATE` (`0x10`)

| Offset | Size | Field | Rule |
|---:|---:|---|---|
| 4 | 8 | Session ID | Current nonzero session |
| 12 | 4 | Sequence | Wrapping `u32`, incremented once per state datagram |
| 16 | 4 | Client monotonic ms | Low 32 bits of monotonic milliseconds; diagnostics only |
| 20 | 4 | Buttons | Bitmask registry below |
| 24 | 2 | Left X | Signed axis |
| 26 | 2 | Left Y | Signed axis; positive is up |
| 28 | 2 | Right X | Signed axis |
| 30 | 2 | Right Y | Signed axis; positive is up |
| 32 | 1 | Left trigger | `0..255` |
| 33 | 1 | Right trigger | `0..255` |
| 34 | 2 | State flags | All zero in v1 |

### Button bits

| Bit | Button |
|---:|---|
| 0 | A |
| 1 | B |
| 2 | X |
| 3 | Y |
| 4 | LB |
| 5 | RB |
| 6 | Back |
| 7 | Start |
| 8 | L3 |
| 9 | R3 |
| 10 | D-pad Up |
| 11 | D-pad Down |
| 12 | D-pad Left |
| 13 | D-pad Right |
| 14–31 | Reserved; zero in v1 |

There is no Guide bit.

Opposing D-pad directions in a received state (`Up+Down` or `Left+Right`) are invalid and the entire state packet is rejected. Diagonals such as `Up+Right` are valid.

### Axis and trigger encoding

- Stick domain: `-32768` full negative, `0` center, `32767` full positive.
- Positive X is right. Positive Y is up.
- Android screen Y increases downward and must be inverted before encoding.
- For normalized `v` clamped to `[-1,1]`:
  - if `v >= 0`, encode `round(v × 32767)`;
  - otherwise encode `round(v × 32768)`.
- Trigger domain: `0` released, `255` full; encode `round(clamp(v,0,1) × 255)`.

### Sequence freshness

The first valid state in a new session establishes the baseline. Thereafter sequence `a` is newer than accepted `b` exactly when:

```text
a != b AND wrapping_u32(a - b) < 2^31
```

Equal or older/ambiguous-half-range values are stale and ignored. Sequence state resets only for a new session.

## `STATUS` (`0x11`)

| Offset | Size | Field | Rule |
|---:|---:|---|---|
| 4 | 8 | Session ID | Current session, or zero if the response is pre-session |
| 12 | 4 | Last accepted sequence | Meaningful when `has accepted state` flag is set |
| 16 | 4 | Echoed client monotonic ms | From the last accepted state |
| 20 | 4 | Accepted count | Wrapping count for this session |
| 24 | 4 | Rejected count | Wrapping validation/admission count for this source/session |
| 28 | 2 | Estimated lost states | Saturating diagnostic estimate |
| 30 | 2 | Observed state rate | Packets/second, saturating |
| 32 | 2 | Status flags | v1 bits below |
| 34 | 2 | Valid-input timeout | Current milliseconds |

Status flag bits:

- Bit 0: has accepted state.
- Bit 1: backend ready.
- Bit 2: controller attached.
- Bit 3: input currently neutral due to timeout.
- Bit 4: session is paired.
- Bit 5: degraded health detected.
- Bits 6–15: zero in v1.

The PC sends status at 2 Hz while a session is active and immediately on important transitions when practical. State packets are not individually acknowledged. The phone may calculate approximate round-trip age using the echoed monotonic value; it must handle `u32` wrap and treat the result as diagnostic only.

## `DISCONNECT` (`0x12`)

| Offset | Size | Field | Rule |
|---:|---:|---|---|
| 4 | 8 | Session ID | Current session |
| 12 | 1 | Reason | Registry below |
| 13 | 23 | Reserved | All zero |

Reasons:

- `0`: user left controller mode.
- `1`: Android lifecycle pause/stop.
- `2`: network lost.
- `3`: local error.
- `4`: application shutdown.

Android sends one neutral state with the next sequence, then up to three best-effort disconnect datagrams a few milliseconds apart before closing. It must not block lifecycle completion to guarantee UDP delivery. The PC immediately neutralizes an admitted disconnect and closes the active session, but retains identity metadata. Under the revised Spec 0009 policy, resume requires hello followed by a fresh code exchange before a new session/source binding is granted; persisted identity alone is insufficient.

## Timing and state rules

- Active controller state rate: 60 Hz initial default.
- Future rates: negotiated field allows 90/120 Hz evaluation without changing packet layout.
- Valid-input timeout: 250 ms default, configurable from 100–1000 ms.
- Status rate: 2 Hz plus transitions.
- Hello retry floor: 500 ms.
- Pairing window: 120 seconds.
- Session IDs, nonces, instance IDs, and pairing codes use an OS cryptographic random source.
- A session that receives no valid state for the configured input timeout (default 250 ms) neutralizes output. Spec 0007 retains the neutral attached session until three seconds after the last valid admitted state; a newer source/session/sequence-valid snapshot resumes automatically within that window. At three seconds it detaches and refuses old-session input. Explicit reconnect after terminal loss always performs hello and may rotate the session. Invalid traffic refreshes neither deadline. Status timeout bit 3 together with ready/attached bits 1/2 reports recoverable neutral input; without ready/attached it reports terminal expiry. Wire fields and fixtures are unchanged.

Only a valid, paired/manual-development, source-matching, fresh `CONTROLLER_STATE` refreshes the input deadline. Invalid control/status traffic never does.

## Receiver validation order

For predictable rejection and minimal work:

1. Exact datagram length.
2. Magic.
3. Version.
4. Known message type.
5. CRC.
6. Reserved bytes/bits and type-specific ranges.
7. Expected direction/state transition.
8. Handshake nonce or session ID.
9. Bound source IP and port.
10. Sequence freshness for controller state.

Malformed packets must not panic, allocate without bound, or produce per-packet log spam.

## Latest-state processing

The PC records the newest accepted state in a one-slot/latest-value channel. If the virtual-controller task is delayed, intermediate historical states are overwritten. Button releases are safe because each new packet describes all buttons, axes, and triggers, and timeout provides the final neutral fallback.

## Pairing security statement

Pairing is an admission and usability feature for a trusted home LAN:

- It prevents an unpaired/accidental phone from taking the controller.
- A random persistent client ID identifies the phone; revised Spec 0009 admission requires a fresh code for every new connection. Wire layout and existing fixtures remain unchanged.
- A random per-connection session and source binding make blind packet injection difficult.

It does **not** provide confidentiality, cryptographic authenticity, or replay protection against someone who can capture local traffic. If the product later targets dorms, offices, public Wi-Fi, or internet transport, introduce a new authenticated/encrypted protocol version rather than implying v1 is secure.

## Compatibility and versioning

- A v1 receiver rejects other versions; there is no silent reinterpretation.
- Adding meaning to reserved bytes/bits requires a capability-negotiated backward-compatible rule or a new protocol version.
- Field order, size, sign, byte order, CRC coverage, and numeric ranges cannot change in only one application.
- Protocol fixtures must include every message type plus invalid length, magic, version, type, CRC, reserved data, D-pad conflict, wrong session/source, stale sequence, and wraparound cases.
