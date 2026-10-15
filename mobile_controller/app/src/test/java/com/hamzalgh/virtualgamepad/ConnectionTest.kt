package com.hamzalgh.virtualgamepad

import com.hamzalgh.virtualgamepad.network.*
import com.hamzalgh.virtualgamepad.protocol.*
import com.hamzalgh.virtualgamepad.settings.Endpoint
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.runTest
import org.junit.Assert.*
import org.junit.Test

@OptIn(kotlinx.coroutines.ExperimentalCoroutinesApi::class)
class ConnectionTest {
    private val endpoint = Endpoint.parse("192.168.1.10", "8765")!!
    private val other = Endpoint.parse("192.168.1.10", "8766")!!
    private fun ack(nonce: ULong = 7uL, session: ULong = 0uL, flags: Int = 4) =
        HelloAck(nonce, 9uL, session, 60, 250, flags, 1, if (session == 0uL) 255 else 0)

    @Test fun permissionDenialClearsSessionAndGrantAllowsFreshRetry() {
        val denied = permissionState(ConnectionState(ConnectionPhase.Ready, session = 4uL), false)
        assertEquals(ConnectionPhase.PermissionNeeded, denied.phase)
        assertEquals(0uL, denied.session)
        assertEquals(ConnectionPhase.Offline, permissionState(denied, true).phase)
    }

    @Test fun ackSourceNonceBackendAndSessionGateReadiness() {
        val reducer = ConnectionReducer(endpoint)
        reducer.expectNonce(7uL)
        assertFalse(reducer.receive(ack(), other))
        assertFalse(reducer.receive(ack(8uL), endpoint))
        assertEquals(ConnectionPhase.Reaching, reducer.state.phase)
        assertTrue(reducer.receive(ack(), endpoint))
        assertEquals(ConnectionPhase.BackendRequired, reducer.state.phase)
        assertEquals(0uL, reducer.state.session)
        assertTrue(reducer.state.manualTrust)
        assertFalse(reducer.receive(ack(), endpoint))
        reducer.expectNonce(8uL)
        assertTrue(reducer.receive(ack(8uL, 5uL, 6), endpoint))
        assertEquals(ConnectionPhase.Ready, reducer.state.phase)
        reducer.cancel()
        assertEquals(ConnectionPhase.Offline, reducer.state.phase)
        assertEquals(0uL, reducer.state.session)
        assertFalse(reducer.receive(ack(8uL, 5uL, 6), endpoint))
    }

    @Test fun statusCannotCreateSessionOrAcceptWrongSourceOrSession() {
        val r = ConnectionReducer(endpoint)
        val status = Status(5uL, 0u, 0u, 0u, 0u, 0, 0, 2, 250)
        assertFalse(r.receive(status, endpoint))
        r.expectNonce(7uL)
        r.receive(ack(session = 5uL, flags = 6), endpoint)
        assertFalse(r.receive(status.copy(session = 6uL), endpoint))
        assertFalse(r.receive(status, other))
        assertTrue(r.receive(status.copy(flags = 34), endpoint))
        assertEquals(ConnectionPhase.BackendRequired, r.state.phase)
        r.receive(status.copy(flags = 10), endpoint)
        assertEquals(ConnectionPhase.TimedOut, r.state.phase)
        r.receive(status.copy(flags = 0), endpoint)
        assertEquals(ConnectionPhase.BackendRequired, r.state.phase)
        r.timeout()
        assertEquals(0uL, r.state.session)
        assertFalse(r.receive(status, endpoint))
    }

    @Test fun backendFlagWithoutSessionIsNotReadyAndPairingIsInactive() {
        val r = ConnectionReducer(endpoint)
        r.expectNonce(7uL)
        r.receive(ack(flags = 3), endpoint)
        assertEquals(ConnectionPhase.PairingNeeded, r.state.phase)
        assertFalse(r.receive(PairAccept(3uL, 7uL, 1uL, 60, 250, 0, 1), endpoint))
    }

    private class FakeTransport(val clock: () -> Long) : DatagramTransport {
        val sent = mutableListOf<Pair<Long, Message>>()
        val incoming = ArrayDeque<ReceivedDatagram>()
        var closed = false
        override fun send(bytes: ByteArray, destination: Endpoint) { sent += clock() to V1Codec.decode(bytes) }
        override suspend fun receive(): ReceivedDatagram? { delay(100); return incoming.removeFirstOrNull() }
        override fun close() { closed = true }
    }

    @Test fun retriesAreBoundedAndOnlySendHelloThenCloseOnTimeout() = runTest {
        val fake = FakeTransport { testScheduler.currentTime }
        var nonce = 0uL
        val session = HelloSession({ fake }, { testScheduler.currentTime }, { ++nonce })
        val states = mutableListOf<ConnectionState>()
        session.run(endpoint, 1uL, 34) { states += it }
        assertEquals(10, fake.sent.size)
        assertTrue(fake.sent.all { it.second is Hello })
        assertTrue(fake.sent.zipWithNext().all { (a, b) -> b.first - a.first >= 500 })
        assertEquals(10, fake.sent.map { (it.second as Hello).clientNonce }.distinct().size)
        assertEquals(ConnectionPhase.TimedOut, states.last().phase)
        assertTrue(fake.closed)
    }

    @Test fun malformedWrongSourceAndPresessionStatusCannotRefreshDeadline() = runTest {
        val fake = FakeTransport { testScheduler.currentTime }
        fake.incoming += ReceivedDatagram(V1Codec.encode(ack(1uL)), other)
        fake.incoming += ReceivedDatagram(ByteArray(41), endpoint)
        fake.incoming += ReceivedDatagram(V1Codec.encode(ack(99uL)), endpoint)
        repeat(60) { fake.incoming += ReceivedDatagram(V1Codec.encode(Status(0uL, 0u, 0u, 0u, 0u, 0, 0, 0, 250)), endpoint) }
        var last = ConnectionState()
        HelloSession({ fake }, { testScheduler.currentTime }, { 1uL }).run(endpoint, 1uL, 34) { last = it }
        assertEquals(5000L, testScheduler.currentTime)
        assertEquals(ConnectionPhase.TimedOut, last.phase)
    }

    @Test fun acknowledgedReachabilityExpiresWhenAgentStopsAndRetryUsesNewOwner() = runTest {
        val first = FakeTransport { testScheduler.currentTime }
        first.incoming += ReceivedDatagram(V1Codec.encode(ack(1uL)), endpoint)
        val states = mutableListOf<ConnectionState>()
        HelloSession({ first }, { testScheduler.currentTime }, { 1uL }).run(endpoint, 1uL, 34) { states += it }
        assertTrue(states.any { it.phase == ConnectionPhase.BackendRequired })
        assertEquals(ConnectionPhase.TimedOut, states.last().phase)
        assertTrue(first.closed)
        val second = FakeTransport { testScheduler.currentTime }
        val owner = HelloSession({ second }, { testScheduler.currentTime }, { 2uL })
        val job = launch { owner.run(endpoint, 1uL, 34) {} }
        delay(150)
        owner.close()
        job.cancelAndJoin()
        assertTrue(second.closed)
        assertEquals(2uL, (second.sent.single().second as Hello).clientNonce)
    }

    @Test fun closeBeforeIoStartsCannotCreateSocket() = runTest {
        val owner = HelloSession(factory = { error("Must not open") })
        owner.close()
        owner.run(endpoint, 1uL, 34) { error("Must not publish") }
    }

    @Test fun cancellationDuringInitialPublishClosesTheSocket() = runTest {
        val fake = FakeTransport { testScheduler.currentTime }
        val owner = HelloSession({ fake }, { testScheduler.currentTime })
        val job = launch { owner.run(endpoint, 1uL, 34) { delay(10000) } }
        delay(100)
        job.cancelAndJoin()
        assertTrue(fake.closed)
        assertTrue(fake.sent.isEmpty())
    }

    @Test fun presessionStatusIsVisibleButCannotKeepReachabilityAlive() = runTest {
        val fake = FakeTransport { testScheduler.currentTime }
        fake.incoming += ReceivedDatagram(V1Codec.encode(ack(1uL)), endpoint)
        repeat(60) { fake.incoming += ReceivedDatagram(V1Codec.encode(Status(0uL, 0u, 0u, 0u, 0u, 0, 0, 0, 250)), endpoint) }
        val states = mutableListOf<ConnectionState>()
        HelloSession({ fake }, { testScheduler.currentTime }, { 1uL }).run(endpoint, 1uL, 34) { states += it }
        assertTrue(states.any { it.statusReceived && it.session == 0uL })
        assertEquals(3100L, testScheduler.currentTime)
        assertEquals(ConnectionPhase.TimedOut, states.last().phase)
    }
}
