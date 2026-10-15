# Windows PC Agent Instructions

These rules apply under `pc_agent/` in addition to the repository instructions.

## Toolchain and targets

- Stable Rust, edition 2024, pinned with `rust-toolchain.toml`.
- Primary target: `x86_64-pc-windows-msvc` on Windows 10 x64; maintain Windows 11 compatibility.
- Tokio owns asynchronous UDP/timers. Keep `main.rs` as composition and shutdown wiring.

## Module boundaries

Use focused modules with names equivalent to:

- `config`: validated settings and defaults.
- `protocol`: fixed-size codecs and errors only.
- `network`: UDP receive/send loops.
- `session`: pairing, source binding, sequence freshness, and connection health.
- `input`: latest state, mapping, and timeout-neutral state machine.
- `virtual_gamepad`: backend trait, fake backend, and isolated Windows/ViGEm adapter.
- `telemetry`: structured local logs and counters.
- Later only: `keyboard_mouse`; discovery and native tray presentation are implemented.

- Spec 0009 implements `pairing` policy, `paired_session` admission, `pairing_store` persistence and `discovery`; Spec 0010 adds desktop/tray presentation. `windows_network` and `windows_storage` contain the native profile and file security adapters.

- Spec 0010 moves runtime ownership to `application::Application`; `main.rs` selects terminal or `--tray`. Preparation locks storage and binds UDP, so it is a runtime action. Await `run` through neutral/detach and discovery/storage cleanup; never abort the service future. Commands are bounded, exit has a separate signal, and ordinary status omits identifiers/codes. Copy watch values before await/presentation. `desktop` retains the store lock while stopped and reloads records fail closed; `windows_tray` owns the native main-thread pump. Its 200 ms timer refreshes code expiry during nested menu loops. PairingView is memory only and must never implement Debug or enter logs.
- Desktop settings.ini is versioned and strictly validated; reload only while stopped. Unknown/corrupt files remain untouched. Local logging uses the bounded worker, 1 MiB rotation and three backups. Startup changes only the exact owned current-user Run value; preflight is read only and cannot prove driver usability. Setup/runtime mutex guards supplement exact process checks; installer-triggered --tray-after-setup waits before starting any service.
- Windows executable builds require SDK rc.exe (or VG_RC_EXE) for version/asInvoker resources. Keep tray-icon pinned and its native types inside the adapter. No new ViGEm calls or protocol fields are needed for desktop presentation; an unavailable slot index must stay labelled unavailable.
- `windows_pairing_window` owns the modeless code popup and native fonts on the tray thread. Close/Escape cancels only the displayed deadline, rechecked by the admission owner to protect a newer queued Pair. Clear the popup before joining service cleanup. Stopped/Starting labels use validated settings; Running uses the actual bound socket, published atomically with the desktop phase.
- Edit settings must work without Notepad installed: on editor NotFound, use the native Open with chooser for the exact settings.ini path. Cancellation leaves the service running; permission and chooser failures stay visible. Do not install an editor or change file associations automatically.

Networking, packet parsing, session admission, and virtual-device calls must not be combined in one loop.

## Safety

- Decode untrusted bytes into a validated domain type before use.
- Store only the newest accepted controller state, preferably with latest-value semantics such as a Tokio `watch` channel.
- Copy a watched snapshot immediately and release its borrow before any await or backend work; holding a watch borrow blocks the publisher.
- Invalid, stale, unpaired, or wrong-source packets never update the input deadline.
- On input timeout, submit neutral once and retain the attached session until the three second valid-input idle deadline. Only newer admitted snapshots resume it. Terminal idle expiry, backend error, pairing loss or graceful shutdown neutralizes and detaches cleanly.
- Unit-test timeout and shutdown with a fake backend and paused/fake time.
- `--input-timeout-ms` is validated through `InputTimeout` (100–1000, default 250) and advertised in hello/status. `SESSION_IDLE_TIMEOUT` is three seconds. Admission must reject terminal idle expiry even if output health is delayed; invalid/stale traffic refreshes neither deadline.
- Terminal diagnostics use the bounded dropping writer in `telemetry`; do not add synchronous terminal/file I/O to the UDP or output path. Local counters saturate; wire counters follow the protocol.
- `Controller arrival gap` compares decoded state arrivals from the same source/session, with client timestamp/sequence deltas and maximum receiver poll gap. Reports are limited to one per second for gaps of at least 100 ms. Client values are untrusted diagnostics, not admission evidence or a measurement of network latency.
- Hard process termination cannot be made fully graceful; document driver behavior and verify recovery on the next launch.

## Windows integration

- ViGEm is a retired v1 dependency, not an architectural primitive. Hide it behind `VirtualGamepadBackend`.
- Spec 0010 startup permits one fresh target retry only for the first wait-ready Windows error 483, after successful neutral/unplug and release of the old client/target. Call wait_ready once per target; its driver boot event is cleared after completion. An unchanged neutral update cannot prove readiness. Other errors and cleanup failures never trigger replacement.
- `attach_checked` receives a short current-request predicate; check cancellation/expiry between native calls and before neutral activation. Keep all calls on the joined output owner. A predicate or Tokio timeout cannot cancel a synchronous driver call or guarantee a fixed shutdown duration. Backend diagnostics carry only operation, native code, attempt and cleanup-failure facts into local status; full errors stay in bounded logs. Tray fault notifications are nonmodal and sent once per run; status details persist if Windows suppresses them.
- Run the driver viability spike before broad UI work.
- Runtime should not require administrator privileges after driver/firewall installation.
- Installer firewall rules must be program-scoped UDP inbound on Private networks only.
- Keyboard/mouse injection is a later opt-in backend and must never silently replace controller mode.

## Dependencies and unsafe code

- Keep the initial dependency set small: Tokio, error/logging utilities, CRC, configuration only when needed, and a Windows-only virtual-gamepad client.
- Add tray, mDNS, installer helpers, or Win32 bindings only in the spec that needs them.
- Any unsafe block must be isolated, justified with a safety comment, and covered by boundary tests/review.

## Verification

Run after Rust changes:

Run Cargo commands from `pc_agent/`; the repository root has no Cargo manifest.

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo check --all-targets
cargo test --all-targets
```

Also report separately whether the agent was run on Windows, the driver was present, a controller appeared in `joy.cpl`, neutral timeout worked, and a real game was tested.

- Local UDP smoke: run `cargo run -- --debug`, then `cargo run --example send_fixtures` from a second terminal. The example sends one valid hello and one bad CRC packet to loopback; expect `decoded=1`, `invalid=1`, and `rejection=Some(Crc)` on a fresh listener.
- The default executable refuses all admission and hello replies. Debug-only `--manual-trust-first-client` grants source bound provisional sessions after backend readiness, admitting full valid controller snapshots in Spec 0007. Release configuration and the output-enabled handler refuse this development path. `DevelopmentTrust::new` remains diagnostic only for the Spec 0003 regression tests.

- Spec 0009 supersedes default denial: production debug/release use `PairedSession`, acknowledge unknown phones without a session, and admit only the persisted phone after backend readiness. Terminal `pair`, `forget` and `replace` use bounded queues; persistence completes before PairAccept, while forget clears output before disk completion. The default executable now owns persistent storage and discovery, so running it is a runtime action, not a read only test.

- The revised September 11 policy supersedes saved-phone admission: every new production session needs a fresh code. `pair` explicitly revokes the existing session before opening a window. Persisted client metadata never grants Hello admission; response retries and same-session brief gap recovery remain valid.
- The terminal prints a Running/IP/port summary once. Periodic diagnostics are optional under `--debug` and quiet while no new input is accepted; dependency debug/info chatter is filtered. Keep warnings/errors and bounded dropping output, with no synchronous terminal work in UDP/output loops.
- Terminal `status` reads service/backend facts on demand; the IP list is startup inventory, not reachability evidence. `exit` and Ctrl+C await the same cleanup. EOF closes the input reader only. Presentation threads own no socket, backend or storage and may remain blocked on console I/O without blocking input cleanup.
- Keep codes/nonces/sessions out of disk records and tracing. The pairing store holds an exclusive lock, validates its fixed record, rejects reparse points and protects Windows directory/file DACLs. Disk work never runs on the UDP owner. Failed removal remains blocked in memory; retry forget before restarting.
- `cargo test --release --test development_trust --test pairing` covers both release bypass refusal and production pairing. Discovery uses only explicitly enumerated Private IPv4 interfaces; unknown profiles fail closed. Multicast/native profile behavior still requires authorized platform verification.
- `supported_controls` gates both admission and output, rejecting reserved buttons and opposing D-pad directions. `xbox_report` translates the 14 protocol button positions into XInput bits and copies all four signed axes and both triggers; only `virtual_gamepad/vigem.rs` maps into ViGEm types. `XboxReport::NEUTRAL` zeros every field.
- The PC receiver rejects PC to phone message types before calling a control handler. Invalid handler replies are counted and withheld without stopping reception.
- Protocol tests consume `../protocol/fixtures/v1/manifest.txt` and `admission.txt`. Normal tests must not regenerate fixtures; `../protocol/fixtures/v1/generate.ps1` is the independent reference builder.
- `cargo test --release --test development_trust` checks that the development flag is rejected by release configuration. Preserve that boundary when adding backend readiness and provisional sessions.
- `cargo test --offline --test reliability` runs the development only loopback fault link with a fake backend. Its simulated results do not replace the physical campaign in `docs/handoffs/2026-09-06-spec-0005-manual-campaign.md` at the repository root.
- When LAN traffic counters stay zero, inspect the real Wi-Fi IPv4 and program firewall rules. An explicit inbound Block overrides an Allow rule; any user-authorized correction must remain program scoped, Private profile, UDP port scoped and limited to local peers.
