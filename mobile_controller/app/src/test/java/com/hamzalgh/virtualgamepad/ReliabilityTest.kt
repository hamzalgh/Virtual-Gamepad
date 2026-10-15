package com.hamzalgh.virtualgamepad

import com.hamzalgh.virtualgamepad.controller.ControllerState as LocalState
import com.hamzalgh.virtualgamepad.network.*
import com.hamzalgh.virtualgamepad.protocol.*
import com.hamzalgh.virtualgamepad.settings.Endpoint
import kotlinx.coroutines.*
import kotlinx.coroutines.test.*
import org.junit.Assert.*
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class ReliabilityTest {
    private val endpoint = Endpoint.parse("192.168.1.10", "8765")!!
    private class Transport : DatagramTransport {
        val sent = mutableListOf<Message>()
        val incoming = ArrayDeque<ReceivedDatagram>()
        var closed = 0
        var deny = false
        override fun send(bytes: ByteArray, destination: Endpoint) {
            sent += V1Codec.decode(bytes)
            if (deny) throw SecurityException("Injected permission revocation")
        }
        override suspend fun receive(): ReceivedDatagram? { delay(10); return incoming.removeFirstOrNull() }
        override fun close() { closed++ }
    }

    @Test fun cadenceMeasuresIntervalsAndSkipsWithoutChangingSixtyHzSampling() = runTest {
        val transport = Transport()
        var offset = 0L
        val sender = StateSender(transport, endpoint, 7uL, { testScheduler.currentTime + offset })
        val job = launch { sender.run(60, { true }, { LocalState.NEUTRAL }) }
        advanceTimeBy(1000)
        val first = sender.cadence.snapshot()
        assertEquals(0L, first.skipped)
        assertEquals(16L, first.minMs)
        assertEquals(17L, first.maxMs)
        assertEquals(1000.0 / 60, first.meanMs, 0.05)
        offset = 500
        advanceTimeBy(600)
        val delayed = sender.cadence.snapshot()
        assertTrue(delayed.skipped >= 29)
        val count = transport.sent.size
        job.cancelAndJoin()
        advanceTimeBy(1000)
        assertEquals(count, transport.sent.size)
        val beforeCleanup = sender.cadence.snapshot()
        sender.finish(1)
        assertEquals(beforeCleanup, sender.cadence.snapshot())
    }

    @Test fun diagnosticAgeWrapsAndMissingAcceptedFlagNeverInventsASequence() {
        val reducer = ConnectionReducer(endpoint)
        reducer.expectNonce(1uL)
        reducer.receive(HelloAck(1uL, 2uL, 7uL, 60, 250, 6, 1, 0), endpoint, 0)
        val status = Status(7uL, UInt.MAX_VALUE, UInt.MAX_VALUE - 9u, UInt.MAX_VALUE, 2u, 3, 60, 7, 250)
        assertTrue(reducer.receive(status, endpoint, 5))
        assertEquals(15L, reducer.state.diagnostics.echoAgeMs)
        assertEquals(UInt.MAX_VALUE, reducer.state.diagnostics.acceptedSequence)
        assertEquals(ConnectionPhase.Ready, reducer.state.phase)
        reducer.receive(status.copy(flags = 6), endpoint, 10)
        assertNull(reducer.state.diagnostics.acceptedSequence)
        assertNull(reducer.state.diagnostics.echoAgeMs)
        assertEquals(ConnectionPhase.Degraded, reducer.state.phase)
        reducer.receive(status.copy(clientMs = 100u), endpoint, 20)
        assertNull(reducer.state.diagnostics.echoAgeMs)
        reducer.receive(status.copy(observedRate = 44), endpoint, 30)
        assertEquals(ConnectionPhase.Degraded, reducer.state.phase)
        reducer.receive(status.copy(observedRate = 45), endpoint, 40)
        assertEquals(ConnectionPhase.Ready, reducer.state.phase)
        reducer.checkSilence(1039)
        assertEquals(ConnectionPhase.Ready, reducer.state.phase)
        reducer.checkSilence(1040)
        assertEquals(ConnectionPhase.Degraded, reducer.state.phase)
    }

    @Test fun twentyLifecycleCyclesNeutralizeExactlyOnceAndRequireFreshHello() = runTest {
        repeat(20) { cycle ->
            val transport = Transport()
            val sessionId = (cycle + 10).toULong()
            val nonce = (cycle + 1).toULong()
            transport.incoming += ReceivedDatagram(V1Codec.encode(HelloAck(nonce, 2uL, sessionId, 60, 250, 6, 1, 0)), endpoint)
            val owner = HelloSession({ transport }, { testScheduler.currentTime }, { nonce })
            val job = launch { owner.run(endpoint, 1uL, 34) {} }
            advanceTimeBy(30)
            owner.updateController(LocalState(buttons = 1))
            advanceTimeBy(40)
            assertTrue(transport.sent.filterIsInstance<ControllerState>().any { it.buttons == 1u })
            owner.close(1)
            owner.close(1) // pause followed by stop
            owner.updateController(LocalState(buttons = 1)) // queued touch after shutdown cannot revive state
            job.cancelAndJoin()
            assertEquals(1, transport.closed)
            assertEquals(3, transport.sent.count { it is Disconnect })
            assertEquals(0u, transport.sent.filterIsInstance<ControllerState>().last().buttons)
            assertEquals(nonce, (transport.sent.first() as Hello).clientNonce)
            val count = transport.sent.size
            owner.run(endpoint, 1uL, 34) { fail("A closed owner must not resume") }
            assertEquals(count, transport.sent.size)
        }
    }

    @Test fun permissionFailureCancelsChildrenAndStillClosesTransport() = runTest {
        supervisorScope {
            val transport = Transport()
            transport.incoming += ReceivedDatagram(V1Codec.encode(HelloAck(1uL, 2uL, 7uL, 60, 250, 6, 1, 0)), endpoint)
            val owner = HelloSession({ transport }, { testScheduler.currentTime }, { 1uL })
            val job = async { owner.run(endpoint, 1uL, 34) {} }
            advanceTimeBy(60)
            transport.deny = true
            try { job.await(); fail("Expected revoked permission") } catch (_: SecurityException) {}
            assertEquals(1, transport.closed)
            assertEquals(3, transport.sent.count { it is Disconnect })
            assertEquals(0u, transport.sent.filterIsInstance<ControllerState>().last().buttons)
        }
    }

    @Test fun statusFloodPublishesDiagnosticsAtMostTwicePerSecond() = runTest {
        val transport = Transport()
        transport.incoming += ReceivedDatagram(V1Codec.encode(HelloAck(1uL, 2uL, 7uL, 60, 250, 6, 1, 0)), endpoint)
        repeat(200) { sequence ->
            transport.incoming += ReceivedDatagram(V1Codec.encode(Status(7uL, sequence.toUInt(), 0u,
                sequence.toUInt(), 0u, 0, if (sequence % 2 == 0) 60 else 30, 7, 250)), endpoint)
        }
        val publications = mutableListOf<Pair<Long, ConnectionState>>()
        val owner = HelloSession({ transport }, { testScheduler.currentTime }, { 1uL })
        val job = launch { owner.run(endpoint, 1uL, 34) { publications += testScheduler.currentTime to it } }
        advanceTimeBy(1900)
        owner.close(0)
        job.cancelAndJoin()
        val updates = publications.filter { it.second.session != 0uL }
        assertTrue(updates.size <= 4)
        assertTrue(updates.zipWithNext().all { (a, b) -> b.first - a.first >= 500 })
    }
}
