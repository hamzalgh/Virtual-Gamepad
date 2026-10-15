package com.hamzalgh.virtualgamepad.network

import com.hamzalgh.virtualgamepad.controller.ControllerState
import com.hamzalgh.virtualgamepad.protocol.ControllerState as StatePacket
import com.hamzalgh.virtualgamepad.protocol.Disconnect
import com.hamzalgh.virtualgamepad.protocol.V1Codec
import com.hamzalgh.virtualgamepad.settings.Endpoint
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.delay
import kotlinx.coroutines.ensureActive
import kotlin.math.ceil

/** The I/O owner serializes cadence and cleanup; input itself is a latest snapshot. */
class StateSender(
    private val transport: DatagramTransport,
    private val endpoint: Endpoint,
    private val session: ULong,
    private val nowMs: () -> Long,
    initialSequence: UInt = 0u,
) {
    private var sequence = initialSequence
    private var finished = false
    val cadence = CadenceMeter()

    private fun send(state: ControllerState) {
        val packet = StatePacket(session, sequence++, nowMs().toUInt(), state.buttons.toUInt(),
            V1Codec.encodeAxis(state.leftX), V1Codec.encodeAxis(state.leftY),
            V1Codec.encodeAxis(state.rightX), V1Codec.encodeAxis(state.rightY),
            V1Codec.encodeTrigger(state.leftTrigger), V1Codec.encodeTrigger(state.rightTrigger))
        transport.send(V1Codec.encode(packet), endpoint)
    }

    suspend fun run(rate: Int, active: () -> Boolean, latest: () -> ControllerState) {
        require(rate == 60) { "This build requires a negotiated 60 Hz session." }
        var next = nowMs().toDouble()
        val period = 1000.0 / rate
        while (active() && !finished) {
            currentCoroutineContext().ensureActive()
            send(latest())
            cadence.record(nowMs())
            next += period
            val now = nowMs().toDouble()
            if (next <= now) {
                cadence.skip(((now - next) / period).toLong() + 1)
                next = now + period // Skip missed frames, never replay a burst.
            }
            delay(ceil(next - now).toLong().coerceAtLeast(1))
        }
    }

    /** Called only after cadence has joined, on I/O in a NonCancellable cleanup. */
    suspend fun finish(reason: Int) {
        if (finished) return
        finished = true
        try { send(ControllerState.NEUTRAL) } catch (_: java.io.IOException) {} catch (_: SecurityException) {}
        repeat(3) { attempt ->
            try { transport.send(V1Codec.encode(Disconnect(session, reason)), endpoint) }
            catch (_: java.io.IOException) {} catch (_: SecurityException) {}
            if (attempt < 2) delay(5)
        }
    }
}
