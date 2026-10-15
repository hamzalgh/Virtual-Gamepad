# Spec 0009 — Discovery and Pairing

Status: **Accepted**
Milestone: M3  
Depends on: M2/0008 passed  
Primary changes: both applications, config/session storage, protocol-flow tests

## Authorized development entry (2026-09-11)

The user accepts Spec 0008 for the tested development path and explicitly authorizes this spec's implementation. This is a development exception to the M2/0008 prerequisite: broader Spec 0008 evidence and formal Spec 0005/0007 campaigns remain unverified, with M2 still open. Do not repeat passed checks or restart campaigns automatically. Revisit outstanding evidence before accepting this spec; its own verification remains required. No later feature, release acceptance, phone interaction or agent/device campaign is authorized by entry alone. Preserve the approved gameplay design, pointer ownership, existing 40 byte protocol and fixtures, strict 60 Hz bounded snapshots, validation and release trust, isolated ViGEm, default 250 ms neutral timeout, brief gap recovery and terminal detach after three seconds followed by explicit reconnect.

## Outcome

### Revised connection policy (user decision, 2026-09-11)

Every new production connection requires a fresh six digit code. This supersedes the original saved-phone reconnect design: stored client/server metadata is informational and never grants a future session. Terminal `pair` explicitly neutralizes/revokes any current session and opens a new 120 second code window. Only the successful session may use identical PairRequest response retries, and brief input gaps keep the existing safety behavior. Disconnect, terminal expiry, source changes and either application restarting require another code. Network traffic cannot open a window or replace a current session.

The terminal shows one Running summary with IPv4 endpoints, UDP port and pairing instructions. Running means listening, not controller readiness. Routine dependency debug logs are suppressed; periodic diagnostics are debug-only and quiet when no input has been accepted during that interval. Errors and relevant state changes remain visible. All runtime output remains bounded and outside input/UDP work.

The phone can discover the PC on the local LAN or use manual IP, pair with an explicit six-digit PC code, reconnect after app/agent restart with a fresh code, and remain the only phone allowed to control the single virtual controller.

## Implementation design (2026-09-11)

The user authorized completing these architecture details and then building in this task. Keep the existing terminal and Compose setup screen. Add discovery and pairing there; gameplay and Settings appearance are unchanged. Implementation uses develop, ui-ux-pro-max, test, same model security review and sync, without subagents.

### Admission and storage

- Extract the existing session admission engine behind the development wrapper and a production pairing handler. Both use the same output requests, freshness checks and two deadlines. The development wrapper remains impossible to enable for output in release.
- Use a bounded pairing state machine with at most 32 pending hello challenges, 64 failure buckets and one pending persistence transaction. Challenges bind client identity, both nonces and complete source socket, expire after ten seconds, and are consumed on successful admission. Unknown hellos cannot allocate a virtual controller or displace a paired phone. A repeated hello with the same client nonce/source retains its challenge. A fresh nonce replaces that identity's challenge.
- Pairing window lasts 120 seconds. Only terminal `pair` opens it; `forget` immediately revokes output and closes the window before persisting removal; `replace` performs forget and then opens a fresh window only after removal is durable. Commands use a bounded channel. EOF closes command input, not the agent.
- Five failed requests count against both source IPv4 (independent of UDP port) and client identity for a 60 second cooldown. Full tables refuse new attempts without evicting live cooldowns. Invalid packets never refresh session deadlines. A successful pair closes the window and clears challenges; a bounded successful response cache permits identical request retries for three seconds without rotating or extending the session. Closed/expired windows cannot be reopened by traffic.
- Persist a versioned fixed local record containing server instance ID and optional paired client ID. Random nonzero IDs use existing getrandom. A cryptographic unbiased six digit code uses rejection sampling. No code, nonce or session is persisted or logged. The record lives under the current user's LocalAppData/VirtualGamepad directory; tests use isolated temporary directories. Protect the dedicated directory with a current owner Windows DACL and reject reparse points. Write a new temporary file, flush, then atomically replace the record. A held exclusive lock prevents two agents from sharing it. Corrupt/inaccessible records fail closed, never silently reset pairing.
- A bounded storage worker owns disk operations outside UDP/output work. A pair is not admitted before its record is saved and the backend is ready. Revocation remains effective in memory on write failure and blocks reconnect until retry or restart; report the storage failure explicitly. Shutdown joins pending persistence work.
- Android adds the server ID and display name to its existing single DataStore transaction owner only after an admitted PAIR_ACCEPT. Manual IP supplies the endpoint as display fallback; discovery supplies a sanitized service name. Session/nonces/code remain memory only and clear on cancel/background/endpoint change. Reconnect is always an explicit Connect with a fresh code after terminal loss.

### Discovery adapters and setup

- Pin mdns-sd 0.21.1 behind a Rust discovery module. Use explicit allowed IPv4 interfaces, with automatic address selection disabled. Before advertising, inspect Windows network categories through an isolated Windows adapter; unknown, Public and DomainAuthenticated profiles are excluded. Interface/category changes remove stale advertisements; inspection failure withdraws discovery. No firewall mutation is part of runtime. Manual IP remains available when multicast is blocked.
- Service name comes from configured PC name (COMPUTERNAME fallback) plus the low 24 bits of the persistent server ID, with labels bounded and sanitized. TXT fields are exactly `v=1`, `agent=<package version>`, `capacity=1`, and `pairing=required`. These are hints, never admission authority. The endpoint port is the actual bound UDP port. No full identity appears in metadata.
- Android uses platform NsdManager, no new dependency or multicast lock. Discovery runs only after explicit Find PCs while foreground, loaded and permitted; stops on Connect, background, cancellation or network change. Resolve at most one service at a time, bound results/pending entries to 32, expire each resolve after five seconds, and ignore callbacks from cancelled generations or lost services. Network changes clear results and require another Find PCs. Use API 34 service info callbacks where available and the legacy resolver at the API floor.
- Extend the scrollable setup screen with Find PCs/Stop, truthful searching/empty/failure states, and selectable PC rows alongside the existing manual endpoint form. Selection fills the form; Connect remains explicit. Pairing needed shows a labelled six digit numeric field, Pair button, retry countdown/error and Cancel. Preserve full system text scaling, existing tokens, keyboard safe insets and permission actions. Discovery failure never disables manual editing. No fake Ready state or automatic phone control.

Implementation limits: Windows polls profile/address inventory every 500 ms on its discovery worker, with five second retry after advertisement creation failure. Native calls and daemon command delivery can add delay; this is not an instantaneous network category switch guarantee. Profile change disables interfaces before withdrawing stale registration. Android resolves each discovered service once, removes it on discovery loss, and requires Find PCs again after local network changes. On API 26 through 33 an outstanding platform lookup cannot be cancelled; after timeout or cancellation, no new lookup starts until its callback returns. The manual form remains available. Native multicast start/stop, network category changes and Android callback behavior require physical/platform evidence, not inference from compilation or pure tests.

### Build plan and verification

1. Implement and test bounded pairing policy, durable record handling and shared session integration, including release admission without development trust.
2. Wire terminal commands and manual IP Android pairing/persistence as the first complete connection path. Verify failure, cancellation, response retry, restart and revocation with fake backends and loopback.
3. Add Private interface advertisement and lifecycle scoped Android discovery/manual fallback, then verify metadata, bounds, disappearance and callback cancellation.
4. Run both application checks and same model correctness/security review. Compile Android device tests; request only the new focused LAN/device tests with specific authorization. Do not launch a production agent or campaign automatically.

The main tradeoff is retaining trusted LAN identity admission rather than cryptographic authentication. Persistent storage and discovery adapters are replaceable; neither participates in the input hot path. mdns-sd avoids implementing a DNS stack or requiring Bonjour. Windows native APIs are preferred for profile and file protection; platform integration must fail closed and remain isolated. Context7 returned a Go mDNS library instead of the selected Rust crate and irrelevant NSD snippets, so versioned upstream source and official Android/Microsoft documentation govern those APIs.

References: [mdns-sd](https://docs.rs/mdns-sd/0.21.1/mdns_sd/), [Android NSD](https://developer.android.com/develop/connectivity/wifi/use-nsd), [Android local network permission](https://developer.android.com/privacy-and-security/local-network-permission), [Windows Network List Manager](https://learn.microsoft.com/en-us/windows/win32/api/netlistmgr/nn-netlistmgr-inetworklistmanager).

## Discovery requirements

- PC advertises DNS-SD service type `_virtual-gamepad._udp.local.` on suitable Private-network interfaces.
- Instance name is human-readable and collision-safe, based on configured PC name plus a short stable suffix.
- TXT metadata is minimal and non-secret: protocol version, agent version, capacity, and pairing-required/ready hints as appropriate.
- Do not advertise pairing code, session ID, full device identifiers, logs, or public addresses.
- Android uses current `NsdManager`/DNS-SD APIs, lifecycle-scoped discovery, deduplication, resolve timeout, and interface/network-change handling.
- Manual IPv4/port entry always remains available and saved.
- Do not implement a custom subnet/UDP broadcast scan unless DNS-SD has a documented target-device failure and the security/network cost is separately approved.
- Use a scoped multicast lock only if current Android APIs/device evidence require it; release immediately when discovery stops.

## Local-network permission

- Recheck target SDK/API 37 behavior with official Android docs and Context7 for the selected AndroidX APIs.
- Request access in the discovery/connect flow with clear purpose.
- Permission denial does not remove manual UI; if both DNS-SD and direct UDP are blocked by platform permission, explain it precisely.
- Re-test INTERNET, network state, local-network, and NSD permission/manifest combinations at the supported API floor and API 37.

## Pairing model

- PC user opens a pairing window from terminal/tray.
- Window lasts 120 seconds and displays one cryptographically random six-digit code.
- Phone sends `PAIR_REQUEST` matching current hello client/server nonces and its persisted random client instance ID.
- PC validates window, code, nonce, source, rate limit, capacity, and backend health.
- Success persists exactly one paired client instance ID and issues a fresh source-bound session.
- Android persists client instance ID and server instance ID/display metadata, not ephemeral session/nonces/code.
- Every client, including a previously paired phone, must complete a fresh code exchange after restart or source change. Hello alone never grants production admission.
- Starting a new session invalidates the old one.
- Disconnect closes session/neutralizes but retains pairing.
- PC provides explicit forget/replace action. Replacing cannot happen merely because a second phone sends traffic.
- Pairing code/failure logs are redacted/rate-limited.

## Manual development trust removal

- Normal release/debug user flow no longer enables first-client trust by default.
- If a compile-time/developer-only bypass remains for tests, it is visibly unsafe, impossible to enable accidentally in release artifacts, and covered by a build/config test.
- Status `paired` flag and UI wording match actual persisted admission.

## Rate limits and failures

- Five failed pairing requests for a source/client identity trigger 60-second cooldown.
- Window expiry closes pairing and invalidates its code/server nonce context.
- Wrong/expired/replayed nonce or code does not create/refresh a session.
- Pairing requests from a different source do not displace the active paired phone.
- If the driver/backend is unavailable, reject with actionable reason and keep output neutral.
- Persistent pairing records are validated, versioned, and written atomically with restrictive ordinary-user permissions.

## Automated tests

### Discovery

- Service metadata encode/parse, version/capacity filtering, deduplication, disappearance, network change, cancellation, and manual fallback reducer.
- Rust advertisement start/stop follows agent/interface state and exposes no secret fields.

### Pairing/session

- Correct code, wrong code, expired window, closed window, five-attempt cooldown, retry-after.
- Nonce mismatch/replay, wrong source, wrong client ID, capacity/backend unavailable.
- Pair persistence, agent restart, phone restart, source-port change, session rotation, old-session rejection.
- Disconnect retains paired identity but invalidates session.
- Explicit forget neutralizes/invalidates and requires a new code.
- Second phone cannot take control or replace the paired phone.
- Manual trust is off in release configuration.

## Manual LAN test

1. Start PC agent and confirm it appears in Android discovery.
2. Pair with the displayed code; verify wrong and expired codes first.
3. Control the gamepad, then restart Android, restart PC agent, and change phone source socket; verify that reconnect requires a fresh code and creates a new session.
4. Attempt from a second phone/emulator/client ID; it must not control or replace.
5. Forget the phone on PC, confirm immediate neutral/session invalidation, and verify a code is required again.
6. Disable discovery/multicast if possible and connect successfully by manual IP.
7. Test firewall/permission denial and router client isolation messaging.
8. Perform Android API 37 permission-flow test before claiming support.

## Security statement shown in docs

This pairing is for admission on a trusted home LAN. It is not encrypted and cannot stop a local attacker who can capture and replay traffic. Public Wi-Fi/internet use is unsupported.

## Exit criteria

- Discovery and manual fallback both work.
- Correct short-code flow and failure/rate-limit cases pass.
- Paired reconnect works across both app restarts with rotated source-bound session.
- Only the paired phone controls; replacement is explicit.
- Release configuration cannot silently use manual trust.
- Existing M2 safety and soak tests still pass.

## Out of scope

- Multiple paired controller slots, encrypted transport, internet discovery, accounts/cloud, QR pairing, Bluetooth.
