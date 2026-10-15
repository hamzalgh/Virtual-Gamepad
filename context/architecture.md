# Architecture

## System boundary

The repository contains two independently built applications connected by one versioned UDP protocol.

```text
┌──────────────── Android phone ────────────────┐
│ Compose UI → touch router → ControllerState   │
│                         ↓                     │
│ settings/status ← session + v1 codec → UDP   │
└──────────────────────────┬────────────────────┘
                           │ private LAN, UDP/8765
┌──────────────────────────▼────────────────────┐
│ UDP → decoder → admission → latest state     │
│                    │              ↓           │
│ status/pairing ← session     timeout/mapping  │
│                                   ↓           │
│                     VirtualGamepadBackend     │
│                                   ↓           │
│                    virtual Xbox 360 device    │
└──────────────── Windows PC agent ─────────────┘
```

The Android app never invokes Windows APIs. The PC agent never depends on Android types. The protocol document and golden byte fixtures are the compatibility boundary.

## Repository shape

```text
virtual_gamepad/
├── AGENTS.md
├── PROJECT_CONTEXT.md
├── context/
├── docs/specs/
├── protocol/fixtures/          # added with protocol implementation
├── mobile_controller/          # independent Gradle project
│   ├── AGENTS.md
│   └── app/src/...
└── pc_agent/                   # independent Cargo project
    ├── AGENTS.md
    └── src/...
```

Root tooling may coordinate documentation or cross-language fixture checks, but it must not merge Gradle and Cargo into one build system.

## Runtime qualities

Every architectural choice is judged against four properties:

- **Safety:** a loss of valid control always becomes neutral.
- **Freshness:** the output reflects the latest accepted state, never an old queue.
- **Determinism:** byte layout, axis mapping, and state transitions are testable without hardware.
- **Replaceability:** platform-risky integrations, especially ViGEm, are isolated.

## Android application

### Suggested packages

```text
com.virtualgamepad
├── app/                 # activity, navigation, lifecycle wiring
├── controller/          # pure state, buttons, axes, geometry, touch routing
├── protocol/            # v1 encoder/decoder and golden-vector tests
├── network/             # UDP socket, send cadence, session/status handling
├── settings/            # DataStore-backed preferences and validation
└── ui/
    ├── setup/           # address, permission, pairing, diagnostics
    ├── gamepad/         # controller surface and visual feedback
    └── theme/           # project tokens
```

Names may evolve, but responsibilities must remain separated.

### State model

Spec 0003 persists tuning defaults of `0.15` radial dead zone and `1.0` sensitivity, explicitly selected by the user. Specs 0006–0007 apply the same transform to both sticks: validated dead zone `0.00..0.30`, sensitivity `0.5..2.0`, and magnitude raised to `1/sensitivity` after radial rescaling. Invalid saved values fall back to their defaults. Spec 0008 provides a draft/preview panel; Apply persists both values atomically before replacing tuning and clearing held ownership without restarting the session. Reset restores the draft defaults; saving failure keeps the prior tuning pair.

`ControllerState` is an immutable value containing:

- `leftX`, `leftY`, `rightX`, `rightY` in normalized domain form.
- `leftTrigger`, `rightTrigger` in normalized domain form.
- A typed button set/bitmask.

Neutral is a named constant. Input code produces state; the protocol codec converts it to fixed integer ranges. UI does not write directly to a socket.

The screen ViewModel exposes one immutable `StateFlow<GamepadUiState>` containing controller state, session state, status/health, and recoverable errors. Compose observes it with lifecycle-aware collection. One-shot events are used only for true transient UI actions.

### Coordinated multi-touch router

The controller surface owns one low-level pointer router instead of giving each visual control an independent gesture recognizer.

On pointer down:

1. Read the stable Android pointer ID.
2. Hit-test against non-overlapping capture regions in a deterministic priority order.
3. Assign the pointer to exactly one logical control.

While active:

- Stick pointers update a clamped radial vector around a fixed center.
- Trigger pointers update their analog lane value.
- Button/D-pad pointers remain assigned to their original control. Leaving that control's capture region releases it; re-entering may press the same control again, but the pointer never transfers to another control.
- Pointer movement does not silently transfer ownership to another control in v1.

On pointer up/cancel, lifecycle loss, or surface disposal, release every control owned by that pointer. A global cancel resets all ownership and returns the local controller model to neutral. Stick/trigger pointers continue to update with clamped geometry when dragged beyond their visual region; digital buttons use the leave/re-enter rule above.

This design makes simultaneous stick/button/trigger input and cancellation independently testable.

### Joystick transform

1. Translate the pointer from the fixed stick center.
2. Clamp its radius to the outer gate.
3. Normalize the vector to `[-1, 1]`.
4. Apply a configurable radial dead zone with rescaling outside the dead zone.
5. Apply the configured sensitivity curve and clamp again.
6. Invert screen-space Y so positive protocol Y means “up.”
7. Encode negative full scale as `-32768` and positive full scale as `32767`.

The transform is pure and covered by boundary/diagonal/property-style tests.

### Network session

The Android UDP owner:

- Binds one ephemeral local UDP port and uses it for both send and receive.
- Runs hello/pairing/status and state messages on that same socket.
- Maintains at most one active PC session.
- Publishes the latest controller state without blocking the UI.
- Sends full-state snapshots at a configurable cadence, initially 60 Hz.
- Reuses a preallocated 40-byte buffer where safe and avoids per-frame UI allocations.
- Stops on lifecycle loss and best-effort sends neutral/disconnect before closing.

The cadence is a replaceable configuration value so 90/120 Hz can be evaluated later without changing protocol v1.

### Android lifecycle and window policy

- Manifest orientation is fixed landscape, not sensor landscape.
- Edge-to-edge/immersive behavior uses current AndroidX window APIs.
- Layout consumes safe drawing and safe gesture insets even when bars are hidden.
- Keep-screen-on is enabled only after entering active controller mode and cleared on every exit path.
- Network permission is requested transparently, including Android 17 local-network access when required.
- If a large-screen/desktop environment overrides orientation, layout remains functional and may show a “rotate to landscape” blocking state rather than overlap controls.

## Windows PC agent

### Suggested modules

```text
src/
├── main.rs
├── config.rs
├── protocol/
├── network/
├── session/
├── input/
├── virtual_gamepad/
│   ├── mod.rs             # trait and neutral/report types
│   ├── fake.rs            # deterministic tests
│   └── vigem.rs           # Windows-only adapter
├── telemetry/
├── discovery/             # later phase
├── tray/                  # later phase
└── keyboard_mouse/        # later opt-in phase
```

### Receive and apply pipeline

```text
recv_from
  → exact-length/magic/version/type/reserved/CRC validation
  → message payload validation
  → pairing session and source-address admission
  → wrap-safe sequence freshness check
  → replace latest accepted state
  → map to backend report
  → submit to virtual controller
```

Parsing errors, admission failures, stale packets, and backend failures are structured outcomes with rate-limited logs. They are not panics.

### Latest-value semantics

The receiver must not push state packets into an unbounded work queue. A Tokio `watch` channel or an equivalent single-slot abstraction stores the most recent accepted state. If three packets arrive before the backend task runs, only the newest matters.

Pairing/control messages may use a small bounded command channel because they are infrequent and stateful. Controller snapshots never use an unbounded channel.

### Timeout state machine

The input owner tracks the monotonic time of the latest **valid, admitted, fresh** state.

- `Unpaired`: virtual device is absent or neutral.
- `Active`: accepted state is being applied.
- `TimedOut`: neutral report was submitted once; remain neutral until a fresh admitted state or a new session policy decision.
- `BackendFault`: attempt neutral/detach, report error, and refuse unsafe updates.
- `ShuttingDown`: neutral, detach, close sockets, then exit.

Default valid-input timeout is 250 ms, configurable from 100–1000 ms. Invalid packets do not extend it. The PC timeout is authoritative even when Android sends a disconnect message.

Spec 0007 keeps the validated input timeout for neutral output and diagnostics, and retains an attached neutral session for a three second valid-input idle window. A newer admitted snapshot resumes automatically; retained/duplicate/older state never replays. Admission independently rejects the three second expiry even if output health lags. Terminal expiry, explicit disconnect and lifecycle loss require fresh Connect/hello; backend faults require process restart. The receiver checks output transitions every 5 ms in addition to periodic status. Bounded terminal logging drops on backpressure or failure rather than waiting in the safety path. This user-approved policy supersedes Spec 0005's detach on brief input timeout.

### Virtual controller abstraction

```text
VirtualGamepadBackend
  attach() -> device identity
  submit(XboxReport)
  neutral()
  detach()
  health()
```

Domain/network code knows only this interface. Tests use a recording fake. ViGEm types, errors, and any unsafe/platform calls stay in `virtual_gamepad/vigem.rs`.

Spec 0010 adds checked startup to this boundary. Only wait-ready Windows error 483 allows one fresh target attempt after successful neutral/unplug and release of the first client/target. Each target receives one readiness wait; the driver clears its boot event, and an unchanged neutral report is not readiness proof. The output owner checks current session, sender closure and grant expiry between startup calls and before activation. These checks prevent subsequent work but cannot cancel an in-flight driver call. Neutral succeeds before readiness is published; Stop/Exit join cleanup before another service can start. Fixed local diagnostics preserve operation, native code, attempt and cleanup failure, with one nonmodal tray notification per faulted run and persistent tray/terminal details. See the September 14 decision in Spec 0010; it does not establish the failing Windows 11 PC's root cause or release compatibility.

The first driver spike must demonstrate attach, A press/release, axis movement/center, neutral timeout, detach, relaunch recovery, and visibility in Windows controller tools before broad feature work.

### Tray and configuration

The development interface remains a terminal until the controller path is reliable. The later tray shell owns:

- Start/stop/listening state.
- Local addresses and UDP port.
- Pairing code/window.
- Driver and firewall health.
- Connected controller status and diagnostics.
- Optional start-with-Windows toggle, off by default.
- Log-folder and exit actions.

Spec 0010 uses `application::Application` as the common socket, pairing store, discovery and output lifetime owner. Terminal and native tray use bounded commands, separate exit signals and redacted latest status; a separate memory-only pairing view carries expiring codes. The main thread owns the Windows message pump and menus; `desktop` supervises one application on a separate Tokio runtime, retaining storage ownership while stopped and joining cleanup before restart. A native timer updates expiry even inside menu/dialog loops. Counters are sampled at 500 ms, with no per-packet UI queue. EOF alone leaves terminal service running. Versioned desktop settings, local rotating logs, current-user startup and read-only setup inventory remain presentation concerns; protocol, cadence and backend ownership are unchanged. See Spec 0010 for implementation and outstanding packaging/manual gates.

## Protocol and fixture ownership

`context/protocol-spec.md` is normative. When implemented, `protocol/fixtures/v1/` contains canonical datagrams plus a small manifest describing field values and expected decode results. Kotlin and Rust tests both consume the same fixtures.

A protocol change requires:

1. Updated normative documentation.
2. Compatibility decision and version impact.
3. Updated Kotlin codec/tests.
4. Updated Rust codec/tests.
5. Updated shared fixtures.

## Discovery and pairing evolution

The September 11 revised Spec 0009 policy requires a fresh code for every new connection. Persisted identity metadata is informational, not reconnect authorization. Terminal `pair` revokes the active session before opening a new code window. This supersedes the original saved-phone admission policy; same-session brief gap recovery is unchanged.

Manual IPv4/port is always retained as a recovery path.

Spec 0009 is implemented under the recorded development entry exception, while formal reliability acceptance remains open:

- The PC advertises `_virtual-gamepad._udp.local.` via DNS-SD/mDNS.
- Android displays discovered agents and can still enter an address manually.
- The PC opens a two-minute pairing window and displays a six-digit code.
- A successful exchange creates a random session ID bound to the phone's source IP and UDP port.
- State packets without the current session/source are ignored.

Production admission wraps the same session engine as the release gated development harness. A bounded policy stores hello challenges and cooldowns. An isolated worker atomically persists one client and the stable server identity under LocalAppData with an exclusive lock and restrictive DACLs. PairAccept waits for storage and backend readiness; forget clears the current request/latest input before saving removal. Discovery is independent of UDP/output, uses explicit Private IPv4 interfaces and fails closed when profile inspection is unavailable. Android owns bounded NSD results and requires explicit selection/Connect; pairing metadata shares the existing DataStore owner. See Spec 0009 for adapter timing and pending platform evidence.

The pairing design prevents accidental control by another device on a trusted home LAN. It is not encrypted and does not defend against a local attacker who can capture/replay traffic. A cryptographic PAKE/encrypted transport is a separate future protocol version if the threat model expands.

## Multiple controllers

After the M2 reliability gate, session management can become a bounded map from paired device identity to controller slot. Initial target is up to four Xbox-style slots. Each slot has independent sequence state, latest value, timeout, neutralization, diagnostics, and virtual backend instance. A fault in one slot must not hold or disconnect another.

## Keyboard/mouse compatibility mode

This is a separate, explicit output backend added near the end:

- User chooses controller mode or a named keyboard/mouse profile.
- The phone still sends controller state, not raw key or mouse messages.
- PC mapping converts state to approved `SendInput` actions.
- Exiting, timeout, or profile change releases every injected key/button.
- UI warns about privilege/anti-cheat limitations.

It must not complicate or delay XInput reliability.

## Observability

Logs are local, structured, and privacy-minimal. Useful fields include message type, source, session suffix, sequence, validation outcome, state age, timeout transitions, driver state, and counters. Pairing code and full session ID should not be logged at normal levels.

Metrics shown locally may include accepted/stale/invalid packets, estimated loss, state rate, last status RTT, time since valid state, backend updates, and timeout count. Packet-by-packet logging is disabled in normal use.

## References

- [Android architecture recommendations](https://developer.android.com/topic/architecture/recommendations)
- [Android Compose pointer input](https://developer.android.com/develop/ui/compose/touch-input/pointer-input/understand-gestures)
- [Android window insets in Compose](https://developer.android.com/develop/ui/compose/system/insets)
- [Tokio `watch` channel](https://docs.rs/tokio/latest/tokio/sync/watch/)
- [Tokio `UdpSocket`](https://docs.rs/tokio/latest/tokio/net/struct.UdpSocket.html)
- [ViGEm client for Rust](https://docs.rs/vigem-client/latest/vigem_client/)
