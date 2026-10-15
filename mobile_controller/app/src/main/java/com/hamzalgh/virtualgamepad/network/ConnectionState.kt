package com.hamzalgh.virtualgamepad.network

import com.hamzalgh.virtualgamepad.protocol.HelloAck
import com.hamzalgh.virtualgamepad.protocol.Message
import com.hamzalgh.virtualgamepad.protocol.Status
import com.hamzalgh.virtualgamepad.protocol.PairAccept
import com.hamzalgh.virtualgamepad.protocol.PairReject
import com.hamzalgh.virtualgamepad.protocol.PairRequest
import com.hamzalgh.virtualgamepad.settings.Endpoint

enum class ConnectionPhase {
    Offline, PermissionNeeded, Reaching, BackendRequired, PairingNeeded, Ready, Degraded, TimedOut, Error,
}

data class ConnectionState(
    val phase: ConnectionPhase = ConnectionPhase.Offline,
    val session: ULong = 0uL,
    val manualTrust: Boolean = false,
    val detail: String = "Enter your PC address to begin.",
    val statusReceived: Boolean = false,
    val stateRate: Int = 0,
    val timeoutMs: Int = 250,
    val diagnostics: SessionDiagnostics = SessionDiagnostics(),
    val cadence: CadenceStats = CadenceStats(),
    val serverInstance: ULong = 0uL,
    val pairingPending: Boolean = false,
    val retryAfterSeconds: Int = 0,
)

data class SessionDiagnostics(
    val acceptedSequence: UInt? = null,
    val accepted: UInt = 0u,
    val rejected: UInt = 0u,
    val lost: Int = 0,
    val observedRate: Int = 0,
    val echoAgeMs: Long? = null,
    val flags: Int = 0,
)

fun ConnectionState.diagnosticText(): String {
    val d = diagnostics
    val age = d.echoAgeMs?.let { "${it} ms" } ?: "unknown"
    val mean = java.lang.String.format(java.util.Locale.ROOT, "%.1f", cadence.meanMs)
    return "$phase · ${d.observedRate}/$stateRate Hz · seq ${d.acceptedSequence ?: "none"} · echo age $age\n" +
        "Sent ${cadence.sent} · skipped ${cadence.skipped} · interval ${cadence.minMs}/$mean/${cadence.maxMs} ms (min/mean/max)\n" +
        "Accepted ${d.accepted} · rejected ${d.rejected} · lost ${d.lost} · timeout $timeoutMs ms\n" +
        "Backend ${if (d.flags and 2 != 0) "ready" else "unconfirmed"} · ${if (d.flags and 4 != 0) "attached" else "not attached"} · PC timeout ${if (d.flags and 8 != 0) "yes" else "no"}"
}

fun permissionState(previous: ConnectionState, granted: Boolean): ConnectionState = when {
    !granted -> ConnectionState(ConnectionPhase.PermissionNeeded,
        detail = "Allow local network access to reach your PC on the same Wi-Fi.")
    previous.phase == ConnectionPhase.PermissionNeeded -> ConnectionState()
    else -> previous
}

/** Admission is pure and never treats a successful send as proof of reachability. */
class ConnectionReducer(private val endpoint: Endpoint) {
    var state = ConnectionState(phase = ConnectionPhase.Reaching, detail = "Waiting for a valid PC acknowledgement.")
        private set
    private var nonce = 0uL
    private var acknowledged = false
    private var lastStatusAt = 0L
    private var pairingNonce = 0uL
    private var serverNonce = 0uL
    private var request: PairRequest? = null
    private var requestUntil = 0L
    private var retryAt = 0L
    private var pairingFeedback: String? = null

    fun pair(instance: ULong, code: String, nowMs: Long): PairRequest? {
        if (state.phase != ConnectionPhase.PairingNeeded || pairingNonce == 0uL || serverNonce == 0uL || request != null || nowMs < retryAt) return null
        if (code.length != 6 || code.any { it !in '0'..'9' }) return null
        return PairRequest(pairingNonce, serverNonce, instance, code.toUInt(), 255).also {
            pairingFeedback = null
            request = it; requestUntil = nowMs + 5000
            state = state.copy(pairingPending = true, detail = "Checking the code and saving pairing…")
        }
    }

    fun pendingPair(nowMs: Long): PairRequest? {
        if (request != null && nowMs >= requestUntil) {
            request = null; pairingNonce = 0uL; serverNonce = 0uL
            state = state.copy(pairingPending = false, detail = "Pairing reply timed out. Type pair on the PC for a fresh code, then retry Connect.")
        }
        val seconds = ((retryAt - nowMs + 999) / 1000).coerceAtLeast(0).toInt()
        if (state.retryAfterSeconds != seconds) state = state.copy(retryAfterSeconds = seconds)
        return request
    }

    fun expectNonce(value: ULong) { nonce = value }

    fun receive(message: Message, source: Endpoint, nowMs: Long = 0): Boolean {
        if (source != endpoint) return false
        when (message) {
            is HelloAck -> {
                if (nonce == 0uL || message.clientNonce != nonce) return false
                nonce = 0uL // Duplicate acknowledgements cannot keep a dead session alive.
                acknowledged = true
                lastStatusAt = nowMs
                val backend = message.flags and 2 != 0
                if (backend && message.session != 0uL && message.stateRate != 60) {
                    state = ConnectionState(ConnectionPhase.Error, detail = "This build needs a 60 Hz PC session. Update the PC configuration and retry.")
                    return true
                }
                val phase = when {
                    message.session == 0uL && message.flags and 1 != 0 -> ConnectionPhase.PairingNeeded
                    !backend -> ConnectionPhase.BackendRequired
                    message.session != 0uL -> ConnectionPhase.Ready
                    else -> ConnectionPhase.PairingNeeded
                }
                pairingNonce = if (phase == ConnectionPhase.PairingNeeded) message.clientNonce else 0uL
                serverNonce = if (phase == ConnectionPhase.PairingNeeded) message.serverNonce else 0uL
                state = ConnectionState(
                    phase, if (backend) message.session else 0uL, message.flags and 4 != 0,
                    when (phase) {
                        ConnectionPhase.BackendRequired -> "PC reachable. Waiting for a controller backend. If this persists, check the PC terminal and ViGEmBus device status, then restart the agent."
                        ConnectionPhase.Ready -> "PC acknowledged a session and reports its backend ready."
                        else -> pairingFeedback ?: "On the PC, type pair to display a fresh code. Enter its six digits below. Every connection needs a new code."
                    },
                    stateRate = message.stateRate, timeoutMs = message.timeoutMs,
                    retryAfterSeconds = ((retryAt - nowMs + 999) / 1000).coerceAtLeast(0).toInt(),
                )
                return true
            }
            is PairAccept -> {
                val pending = request ?: return false
                if (nowMs >= requestUntil || message.clientNonce != pending.clientNonce) return false
                request = null; pairingNonce = 0uL; serverNonce = 0uL; nonce = 0uL
                if (message.stateRate != 60 || message.slot != 0) {
                    state = ConnectionState(ConnectionPhase.Error, detail = "The PC returned an unsupported controller session.")
                    return true
                }
                acknowledged = true; lastStatusAt = nowMs
                state = ConnectionState(ConnectionPhase.Ready, session = message.session, stateRate = message.stateRate,
                    timeoutMs = message.timeoutMs, serverInstance = message.serverInstance, detail = "Paired with this PC. Your controller is ready.")
                return true
            }
            is PairReject -> {
                val pending = request ?: return false
                if (nowMs >= requestUntil || message.clientNonce != pending.clientNonce) return false
                request = null; pairingNonce = 0uL; serverNonce = 0uL
                retryAt = nowMs + message.retryAfter * 1000L
                state = state.copy(pairingPending = false, retryAfterSeconds = message.retryAfter, detail = when (message.reason) {
                    1 -> "The pairing window is closed. Type pair on the PC for a new code."
                    2 -> "The code or handshake expired or did not match. Check the PC code and try again."
                    3 -> "A controller session already exists. Type pair on the PC to end it and request a fresh code."
                    4 -> "This pairing was revoked. Open a new pairing window on the PC."
                    5 -> "Too many attempts. Wait ${message.retryAfter} seconds before trying again."
                    6 -> "The PC controller backend is unavailable. Check its terminal and driver status."
                    else -> "The PC could not save pairing. Check its terminal, then retry Connect."
                })
                pairingFeedback = state.detail
                return false // A rejection never refreshes connection/session deadlines.
            }
            is Status -> {
                if (!acknowledged || message.session != state.session) return false
                // A pre-session status has no nonce, so it cannot extend reachability.
                if (state.session == 0uL) {
                    state = state.copy(statusReceived = true)
                    return false
                }
                val phase = when {
                    message.flags and 32 != 0 -> ConnectionPhase.BackendRequired
                    message.flags and 14 == 14 -> ConnectionPhase.Degraded
                    message.flags and 8 != 0 -> ConnectionPhase.TimedOut
                    message.flags and 2 == 0 -> ConnectionPhase.BackendRequired
                    message.flags and 1 == 0 || message.observedRate < 45 -> ConnectionPhase.Degraded
                    else -> ConnectionPhase.Ready
                }
                lastStatusAt = nowMs
                val age = (nowMs.toUInt() - message.clientMs).toLong()
                state = state.copy(phase = phase, statusReceived = true, timeoutMs = message.timeoutMs,
                    diagnostics = SessionDiagnostics(
                        if (message.flags and 1 != 0) message.lastSequence else null,
                        message.accepted, message.rejected, message.lost, message.observedRate,
                        if (message.flags and 1 != 0 && age < 0x80000000L) age else null, message.flags,
                    ), detail = when (phase) {
                    ConnectionPhase.BackendRequired -> "PC reports its controller backend unavailable. Check the PC terminal and ViGEmBus device status, then restart the agent."
                    ConnectionPhase.TimedOut -> "PC reports neutral input after a timeout."
                    ConnectionPhase.Degraded -> if (message.flags and 8 != 0)
                        "Input cleared after a delivery gap. Waiting for fresh input; the controller remains connected."
                        else "Waiting for healthy accepted input or the PC reports degraded health."
                    else -> "PC reports a healthy controller session."
                })
                return true
            }
            else -> return false
        }
    }

    fun checkSilence(nowMs: Long) {
        if (state.session != 0uL && state.phase == ConnectionPhase.Ready && nowMs - lastStatusAt >= 1000) {
            state = state.copy(phase = ConnectionPhase.Degraded, detail = "No PC status for one second. Input stops after three seconds without status.")
        }
    }

    fun cancel() {
        pairingFeedback = null
        request = null; pairingNonce = 0uL; serverNonce = 0uL; retryAt = 0L
        nonce = 0uL
        acknowledged = false
        state = ConnectionState(detail = "Session closed. Connect to negotiate again.")
    }

    fun timeout() {
        val previous = state
        cancel()
        state = previous.copy(phase = ConnectionPhase.TimedOut, session = 0uL, detail = "No recent PC acknowledgement or session status. Diagnostics are from the last report. Check the endpoint, same Wi-Fi, and Private firewall, then retry.")
    }
}
