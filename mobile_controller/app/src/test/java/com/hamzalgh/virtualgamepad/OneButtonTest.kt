package com.hamzalgh.virtualgamepad

import com.hamzalgh.virtualgamepad.controller.GamepadRouter
import com.hamzalgh.virtualgamepad.controller.GamepadGeometry
import com.hamzalgh.virtualgamepad.controller.ControllerState as LocalState
import com.hamzalgh.virtualgamepad.network.*
import com.hamzalgh.virtualgamepad.protocol.*
import com.hamzalgh.virtualgamepad.settings.Endpoint
import kotlinx.coroutines.*
import kotlinx.coroutines.test.*
import org.junit.Assert.*
import org.junit.Test
import java.io.IOException

@OptIn(ExperimentalCoroutinesApi::class)
class OneButtonTest {
    private val endpoint = Endpoint.parse("192.168.1.10", "8765")!!
    private class Transport(val clock: () -> Long) : DatagramTransport {
        val sent = mutableListOf<Pair<Long, Message>>()
        val incoming = ArrayDeque<ReceivedDatagram>()
        var fail = false
        var closed = false
        override fun send(bytes: ByteArray, destination: Endpoint) {
            check(!closed)
            sent += clock() to V1Codec.decode(bytes)
            if (fail) throw IOException("Injected socket failure")
        }
        override suspend fun receive(): ReceivedDatagram? { delay(100); return incoming.removeFirstOrNull() }
        override fun close() { closed = true }
    }

    @Test fun stablePointerCannotBeStolenAndCancelAlwaysReleases() {
        val router = GamepadRouter(GamepadGeometry.fit(640f, 280f, 1f)!!)
        router.down(31, router.geometry.a.x, router.geometry.a.y)
        assertTrue(router.state.a)
        router.down(7, router.geometry.a.x, router.geometry.a.y)
        router.up(7)
        assertTrue(router.state.a)
        router.move(31, 1f, 1f)
        assertFalse(router.state.a)
        assertTrue(router.owns(31))
        router.move(31, router.geometry.a.x, router.geometry.a.y)
        assertTrue(router.state.a)
        router.cancel()
        assertEquals(LocalState.NEUTRAL, router.state)
        router.move(31, router.geometry.a.x, router.geometry.a.y)
        assertFalse(router.state.a)
        router.down(8, 320f, 56f)
        router.move(8, router.geometry.a.x, router.geometry.a.y)
        assertFalse(router.state.a)
        router.down(55, router.geometry.a.x, router.geometry.a.y)
        router.up(55)
        assertEquals(LocalState.NEUTRAL, router.state)
    }

    @Test fun sixtyHzUsesFullSnapshotsWrapsSequenceAndReleasesOnNextFrame() = runTest {
        val transport = Transport { testScheduler.currentTime }
        val sender = StateSender(transport, endpoint, 77uL, { testScheduler.currentTime }, UInt.MAX_VALUE)
        var local = LocalState(buttons = 1)
        val job = launch { sender.run(60, { true }, { local }) }
        advanceTimeBy(100)
        local = LocalState.NEUTRAL
        advanceTimeBy(900)
        job.cancelAndJoin()
        val frames = transport.sent.map { it.second as ControllerState }
        assertEquals(60, frames.size)
        assertEquals(UInt.MAX_VALUE, frames[0].sequence)
        assertEquals(0u, frames[1].sequence)
        assertTrue(frames.take(6).all { it.buttons == 1u })
        assertTrue(frames.drop(6).all { it.buttons == 0u })
        assertTrue(frames.all { it.session == 77uL && it.leftX == 0.toShort() && it.leftY == 0.toShort() &&
            it.rightX == 0.toShort() && it.rightY == 0.toShort() && it.leftTrigger == 0 && it.rightTrigger == 0 })
        sender.finish(1)
        assertEquals(0u, (transport.sent[60].second as ControllerState).buttons)
        assertTrue(transport.sent.takeLast(3).all { it.second == Disconnect(77uL, 1) })
        assertEquals(listOf(1000L, 1005L, 1010L), transport.sent.takeLast(3).map { it.first })
    }

    @Test fun skippedTimeDoesNotReplayHistoricalFrames() = runTest {
        var offset = 0L
        val transport = Transport { testScheduler.currentTime + offset }
        val sender = StateSender(transport, endpoint, 77uL, { testScheduler.currentTime + offset })
        val job = launch { sender.run(60, { true }, { LocalState.NEUTRAL }) }
        advanceTimeBy(10)
        offset = 1000
        advanceTimeBy(30)
        job.cancelAndJoin()
        assertTrue(transport.sent.size <= 3)
        assertTrue(transport.sent.map { it.first }.distinct().size == transport.sent.size)
    }

    @Test fun leftAxisSnapshotsPreserveEndpointsDiagonalsAndUnusedNeutralFields() = runTest {
        val transport = Transport { testScheduler.currentTime }
        val sender = StateSender(transport, endpoint, 77uL, { testScheduler.currentTime })
        var local = LocalState.NEUTRAL
        val job = launch { sender.run(60, { true }, { local }) }
        for ((x, y) in listOf(-1f to 0f, 1f to 0f, 0f to -1f, 0f to 1f,
            0.70710677f to -0.70710677f, -0.70710677f to 0.70710677f, 0f to 0f)) {
            local = LocalState(1, x, y)
            advanceTimeBy(35)
            val frame = transport.sent.last().second as ControllerState
            assertEquals(V1Codec.encodeAxis(x), frame.leftX)
            assertEquals(V1Codec.encodeAxis(y), frame.leftY)
            assertEquals(1u, frame.buttons)
            assertEquals(0.toShort(), frame.rightX)
            assertEquals(0.toShort(), frame.rightY)
            assertEquals(0, frame.leftTrigger)
            assertEquals(0, frame.rightTrigger)
        }
        local = LocalState.NEUTRAL
        advanceTimeBy(35)
        job.cancelAndJoin()
        val release = transport.sent.last().second as ControllerState
        assertEquals(0u, release.buttons)
        assertEquals(0.toShort(), release.leftX)
        assertEquals(0.toShort(), release.leftY)
    }

    // Characterizes the explicitly accepted 60 Hz limitation, not a tap-delivery guarantee.
    @Test fun strictSixtyHzMayCoalesceAReleaseBetweenTicks() = runTest {
        val transport = Transport { testScheduler.currentTime }
        val sender = StateSender(transport, endpoint, 77uL, { testScheduler.currentTime })
        val router = GamepadRouter(GamepadGeometry.fit(640f, 280f, 1f)!!)
        router.down(1, router.geometry.a.x, router.geometry.a.y)
        val job = launch { sender.run(60, { true }, { router.state }) }
        advanceTimeBy(40)
        router.up(1)
        assertFalse(router.state.a)
        advanceTimeBy(6)
        router.down(2, router.geometry.a.x, router.geometry.a.y)
        advanceTimeBy(40)
        router.up(2)
        advanceTimeBy(30)
        job.cancelAndJoin()
        val buttons = transport.sent.map { (it.second as ControllerState).buttons }
        val presses = buttons.indices.count { buttons[it] == 1u && (it == 0 || buttons[it - 1] == 0u) }
        assertEquals("A 6 ms release between ticks is not captured by latest-state sampling", 1, presses)
        assertEquals(0u, buttons.last())
    }

    @Test fun releaseSpanningASenderTickSeparatesTwoPresses() = runTest {
        val transport = Transport { testScheduler.currentTime }
        val sender = StateSender(transport, endpoint, 77uL, { testScheduler.currentTime })
        val router = GamepadRouter(GamepadGeometry.fit(640f, 280f, 1f)!!)
        router.down(1, router.geometry.a.x, router.geometry.a.y)
        val job = launch { sender.run(60, { true }, { router.state }) }
        advanceTimeBy(40)
        router.up(1)
        advanceTimeBy(25)
        router.down(2, router.geometry.a.x, router.geometry.a.y)
        advanceTimeBy(40)
        router.up(2)
        advanceTimeBy(30)
        job.cancelAndJoin()
        val buttons = transport.sent.map { (it.second as ControllerState).buttons }
        val presses = buttons.indices.count { buttons[it] == 1u && (it == 0 || buttons[it - 1] == 0u) }
        assertEquals(2, presses)
        assertEquals(0u, buttons.last())
    }

    @Test fun cleanupIsIdempotentAndTriesDisconnectEvenAfterNeutralFailure() = runTest {
        val transport = Transport { testScheduler.currentTime }
        transport.fail = true
        val sender = StateSender(transport, endpoint, 77uL, { testScheduler.currentTime })
        sender.finish(2)
        sender.finish(2)
        assertEquals(4, transport.sent.size)
        assertEquals(0u, (transport.sent.first().second as ControllerState).buttons)
        assertTrue(transport.sent.drop(1).all { it.second == Disconnect(77uL, 2) })
    }

    @Test fun lifecycleCancellationJoinsSenderThenNeutralDisconnectAndCloses() = runTest {
        val transport = Transport { testScheduler.currentTime }
        transport.incoming += ReceivedDatagram(V1Codec.encode(HelloAck(1uL, 2uL, 77uL, 60, 250, 6, 1, 0)), endpoint)
        val owner = HelloSession({ transport }, { testScheduler.currentTime }, { 1uL })
        val job = launch { owner.run(endpoint, 1uL, 34) {} }
        advanceTimeBy(120)
        owner.updateController(LocalState(buttons = 1, leftX = -1f, leftY = 1f))
        advanceTimeBy(50)
        assertTrue(transport.sent.any { (it.second as? ControllerState)?.buttons == 1u })
        assertTrue(transport.sent.any { (it.second as? ControllerState)?.let { s ->
            s.buttons == 1u && s.leftX == Short.MIN_VALUE && s.leftY == Short.MAX_VALUE
        } == true })
        owner.close(1)
        job.cancelAndJoin()
        val count = transport.sent.size
        assertEquals(0u, (transport.sent[count - 4].second as ControllerState).buttons)
        val neutral = transport.sent[count - 4].second as ControllerState
        assertEquals(0.toShort(), neutral.leftX)
        assertEquals(0.toShort(), neutral.leftY)
        assertTrue(transport.sent.takeLast(3).all { it.second == Disconnect(77uL, 1) })
        assertTrue(transport.closed)
        advanceTimeBy(1000)
        assertEquals(count, transport.sent.size)
        owner.run(endpoint, 1uL, 34) { fail("Closed owner cannot reconnect") }
    }

    @Test fun recoverableTimeoutKeepsSameSenderAndSessionUntilHealthyStatusReturns() = runTest {
        val transport = Transport { testScheduler.currentTime }
        transport.incoming += ReceivedDatagram(V1Codec.encode(HelloAck(1uL, 2uL, 77uL, 60, 250, 6, 1, 0)), endpoint)
        val owner = HelloSession({ transport }, { testScheduler.currentTime }, { 1uL })
        var last = ConnectionState()
        val job = launch { owner.run(endpoint, 1uL, 34) { last = it } }
        advanceTimeBy(150)
        owner.updateController(LocalState(buttons = 3, rightX = 1f, leftTrigger = 1f))
        transport.incoming += ReceivedDatagram(V1Codec.encode(Status(77uL, 6u, 100u, 7u, 0u, 0, 30, 15, 250)), endpoint)
        advanceTimeBy(700)
        assertEquals(ConnectionPhase.Degraded, last.phase)
        assertFalse(transport.closed)
        assertEquals(1, transport.sent.count { it.second is Hello })
        assertEquals(0, transport.sent.count { it.second is Disconnect })
        val states = transport.sent.mapNotNull { it.second as? ControllerState }
        assertTrue(states.size >= 45)
        assertTrue(states.all { it.session == 77uL })
        assertTrue(states.zipWithNext().all { (a, b) -> b.sequence == a.sequence + 1u })
        assertEquals(3u, states.last().buttons)
        transport.incoming += ReceivedDatagram(V1Codec.encode(Status(77uL, 50u, 830u, 51u, 0u, 0, 60, 7, 250)), endpoint)
        advanceTimeBy(600)
        assertEquals(ConnectionPhase.Ready, last.phase)
        owner.close(1)
        job.cancelAndJoin()
        assertTrue(transport.closed)
    }

    @Test fun backendTimeoutStatusStopsCadenceWithoutWaitingForAnotherConnect() = runTest {
        val transport = Transport { testScheduler.currentTime }
        transport.incoming += ReceivedDatagram(V1Codec.encode(HelloAck(1uL, 2uL, 77uL, 60, 250, 6, 1, 0)), endpoint)
        transport.incoming += ReceivedDatagram(V1Codec.encode(Status(77uL, 0u, 0u, 0u, 0u, 0, 0, 8, 250)), endpoint)
        var last = ConnectionState()
        HelloSession({ transport }, { testScheduler.currentTime }, { 1uL }).run(endpoint, 1uL, 34) { last = it }
        assertEquals(ConnectionPhase.TimedOut, last.phase)
        assertTrue(transport.closed)
        assertTrue(testScheduler.currentTime < 250)
        assertEquals(3, transport.sent.count { it.second is Disconnect })
    }

    @Test fun activeStatusSilenceStopsAndCleansUpRatherThanHangingOnChildSender() = runTest {
        val transport = Transport { testScheduler.currentTime }
        transport.incoming += ReceivedDatagram(V1Codec.encode(HelloAck(1uL, 2uL, 77uL, 60, 250, 6, 1, 0)), endpoint)
        var last = ConnectionState()
        HelloSession({ transport }, { testScheduler.currentTime }, { 1uL }).run(endpoint, 1uL, 34) { last = it }
        assertEquals(ConnectionPhase.TimedOut, last.phase)
        assertTrue(transport.closed)
        assertTrue(testScheduler.currentTime in 3100..3150)
        assertTrue(transport.sent.takeLast(3).all { it.second == Disconnect(77uL, 2) })
    }

    @Test fun incompatibleRateCannotEnterReadyOrStartSender() {
        val reducer = ConnectionReducer(endpoint)
        reducer.expectNonce(1uL)
        reducer.receive(HelloAck(1uL, 2uL, 77uL, 120, 250, 6, 1, 0), endpoint)
        assertEquals(ConnectionPhase.Error, reducer.state.phase)
        assertEquals(0uL, reducer.state.session)
    }

    @Test fun senderSocketFailureStillAttemptsNeutralDisconnectAndClosesOwner() = runTest {
        supervisorScope {
            val transport = Transport { testScheduler.currentTime }
            transport.incoming += ReceivedDatagram(V1Codec.encode(HelloAck(1uL, 2uL, 77uL, 60, 250, 6, 1, 0)), endpoint)
            val owner = HelloSession({ transport }, { testScheduler.currentTime }, { 1uL })
            val job = async { owner.run(endpoint, 1uL, 34) {} }
            advanceTimeBy(120)
            owner.updateController(LocalState(buttons = 1))
            advanceTimeBy(50)
            transport.fail = true
            try { job.await(); fail("Expected socket failure") } catch (_: IOException) {}
            assertTrue(transport.closed)
            assertEquals(0u, (transport.sent[transport.sent.size - 4].second as ControllerState).buttons)
            assertTrue(transport.sent.takeLast(3).all { it.second == Disconnect(77uL, 2) })
        }
    }
}
