package com.hamzalgh.virtualgamepad.network

import com.hamzalgh.virtualgamepad.protocol.Hello
import com.hamzalgh.virtualgamepad.protocol.ProtocolException
import com.hamzalgh.virtualgamepad.protocol.V1Codec
import com.hamzalgh.virtualgamepad.settings.Endpoint
import com.hamzalgh.virtualgamepad.settings.nonzeroRandom
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.Job
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicReference
import com.hamzalgh.virtualgamepad.controller.ControllerState
import java.io.Closeable
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.InetAddress
import java.net.InetSocketAddress
import java.net.SocketTimeoutException

data class ReceivedDatagram(val bytes: ByteArray, val source: Endpoint?)

interface DatagramTransport : Closeable {
    fun send(bytes: ByteArray, destination: Endpoint)
    suspend fun receive(): ReceivedDatagram?
}

/** Constructed on Dispatchers.IO. close() interrupts a blocked receive immediately. */
class UdpTransport : DatagramTransport {
    private val socket = DatagramSocket(InetSocketAddress(0)).apply { soTimeout = 100 }
    private val buffer = ByteArray(65535)

    override fun send(bytes: ByteArray, destination: Endpoint) {
        val address = InetAddress.getByAddress(destination.address.split('.').map { it.toInt().toByte() }.toByteArray())
        socket.send(DatagramPacket(bytes, bytes.size, address, destination.port))
    }

    override suspend fun receive(): ReceivedDatagram? = try {
        val packet = DatagramPacket(buffer, buffer.size)
        socket.receive(packet)
        ReceivedDatagram(
            buffer.copyOf(packet.length),
            Endpoint.parse(packet.address.hostAddress.orEmpty(), packet.port.toString()),
        )
    } catch (_: SocketTimeoutException) { null }

    override fun close() = socket.close()
}

/** One owner, one socket, bounded retries. Transport and clock are replaceable in JVM tests. */
class HelloSession(
    private val factory: () -> DatagramTransport = ::UdpTransport,
    private val nowMs: () -> Long = { System.nanoTime() / 1_000_000 },
    private val random: () -> ULong = { nonzeroRandom() },
) : Closeable {
    private val lock = Any()
    private val closed = AtomicBoolean(false)
    private val controller = AtomicReference(ControllerState.NEUTRAL)
    @Volatile private var closeReason = 3
    private val pairingCode = AtomicReference<String?>(null)
    fun pair(code: String) { if (!closed.get()) pairingCode.compareAndSet(null, code) }

    fun updateController(state: ControllerState) = synchronized(lock) {
        if (!closed.get()) controller.set(state)
    }

    /** No I/O or join on the lifecycle thread. The running I/O owner cleans up. */
    fun close(reason: Int) = synchronized(lock) {
        if (!closed.get()) { closeReason = reason; closed.set(true) }
        controller.set(ControllerState.NEUTRAL)
        pairingCode.set(null)
    }
    override fun close() = close(0)

    suspend fun run(endpoint: Endpoint, instance: ULong, api: Int,
        onPaired: suspend (ULong) -> Unit = {}, publish: suspend (ConnectionState) -> Unit) {
        val socket = synchronized(lock) {
            if (closed.get()) return
            factory()
        }
        val reducer = ConnectionReducer(endpoint)
        var deadline = nowMs() + 5000
        var nextHello = nowMs()
        var lastAck = false
        var sender: StateSender? = null
        var cadence: Job? = null
        var lastPublished = reducer.state
        var nextPublish = nowMs()
        var nextPair = 0L
        var persistedServer = 0uL
        try {
            coroutineScope {
              publish(reducer.state)
              while (!closed.get()) {
                currentCoroutineContext().ensureActive()
                val now = nowMs()
                val submitted = pairingCode.getAndSet(null)?.let { reducer.pair(instance, it, now) }
                val pending = submitted ?: reducer.pendingPair(now)
                if (pending != null && now >= nextPair) {
                    socket.send(V1Codec.encode(pending), endpoint)
                    nextPair = now + 500
                }
                reducer.checkSilence(now)
                val visible = reducer.state.copy(cadence = sender?.cadence?.snapshot() ?: CadenceStats())
                if (visible != lastPublished && now >= nextPublish) {
                    publish(visible)
                    lastPublished = visible
                    nextPublish = now + 500
                }
                if (now >= deadline && pending == null) {
                    close(2)
                    reducer.timeout()
                    publish(reducer.state.copy(cadence = sender?.cadence?.snapshot() ?: CadenceStats()))
                    return@coroutineScope
                }
                if (reducer.state.session == 0uL && pending == null && now >= nextHello) {
                    val nonce = random()
                    reducer.expectNonce(nonce)
                    socket.send(V1Codec.encode(Hello(nonce, instance, 60, 7, 1u, api)), endpoint)
                    nextHello = now + if (lastAck) 2000 else 500
                }
                val datagram = socket.receive() ?: continue
                val source = datagram.source ?: continue
                val message = try { V1Codec.decode(datagram.bytes) } catch (_: ProtocolException) { continue }
                if (reducer.receive(message, source, nowMs())) {
                    lastAck = true
                    deadline = nowMs() + 3000
                    nextHello = nowMs() + 2000
                }
                if (reducer.state.serverInstance != 0uL && reducer.state.serverInstance != persistedServer) {
                    val receivedAt = nowMs()
                    onPaired(reducer.state.serverInstance)
                    persistedServer = reducer.state.serverInstance
                    if (nowMs() - receivedAt >= 3000) reducer.timeout()
                }
                val ready = reducer.state.session != 0uL && reducer.state.phase in setOf(ConnectionPhase.Ready, ConnectionPhase.Degraded)
                val started = ready && sender == null
                if (started) {
                    val sending = StateSender(socket, endpoint, reducer.state.session, nowMs)
                    sender = sending
                    cadence = launch { sending.run(reducer.state.stateRate, { !closed.get() }, controller::get) }
                }
                val terminal = reducer.state.phase == ConnectionPhase.Error || (sender != null && !ready)
                if (terminal) close(3)
                if (started || terminal) {
                    val update = reducer.state.copy(cadence = sender?.cadence?.snapshot() ?: CadenceStats())
                    publish(update)
                    lastPublished = update
                    nextPublish = nowMs() + 500
                }
                if (terminal) {
                    return@coroutineScope
                }
              }
              cadence?.cancelAndJoin()
            }
        } catch (error: java.io.IOException) {
            close(2)
            throw error
        } finally {
            controller.set(ControllerState.NEUTRAL)
            closed.set(true)
            withContext(NonCancellable) {
                try {
                    cadence?.cancelAndJoin()
                    sender?.finish(closeReason)
                } finally { socket.close(); reducer.cancel() }
            }
        }
    }
}
