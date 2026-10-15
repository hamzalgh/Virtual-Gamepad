package com.hamzalgh.virtualgamepad

import com.hamzalgh.virtualgamepad.network.*
import com.hamzalgh.virtualgamepad.app.allowsController
import com.hamzalgh.virtualgamepad.protocol.*
import com.hamzalgh.virtualgamepad.settings.Endpoint
import org.junit.Assert.*
import org.junit.Test
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.runTest

@OptIn(kotlinx.coroutines.ExperimentalCoroutinesApi::class)
class PairingDiscoveryTest {
    private val endpoint = Endpoint.parse("192.168.1.10", "8765")!!
    private fun challenge(r: ConnectionReducer, nonce: ULong = 7uL, now: Long = 0) {
        r.expectNonce(nonce)
        assertTrue(r.receive(HelloAck(nonce, 9uL, 0uL, 60, 250, 1, 1, 255), endpoint, now))
    }
    @Test fun pairingRequiresExplicitCodeAndMatchingSourceNonceAndDeadline() {
        val r = ConnectionReducer(endpoint)
        assertNull(r.pair(1uL, "123456", 0))
        challenge(r)
        assertNull(r.pair(1uL, "12345", 0))
        assertNull(r.pair(1uL, "abcdef", 0))
        val request = r.pair(1uL, "001234", 0)!!
        assertEquals(1234u, request.code)
        assertEquals(request, r.pendingPair(4999))
        assertFalse(r.state.allowsController())
        val accept = PairAccept(3uL, 7uL, 99uL, 60, 250, 0, 3)
        assertFalse(r.receive(accept, Endpoint.parse(endpoint.address, "8766")!!, 100))
        assertFalse(r.receive(accept.copy(clientNonce = 8uL), endpoint, 100))
        assertTrue(r.receive(accept, endpoint, 100))
        assertEquals(99uL, r.state.serverInstance)
        assertFalse(r.state.manualTrust)
        assertTrue(r.state.allowsController())
        assertFalse(r.receive(accept, endpoint, 101))
        r.cancel()
        assertFalse(r.receive(accept, endpoint, 102))
    }
    @Test fun expiredOrUnsupportedAcceptanceNeverStartsInput() {
        for (rate in listOf(30, 60)) {
            val r = ConnectionReducer(endpoint); challenge(r); r.pair(1uL, "123456", 0)
            r.receive(PairAccept(3uL, 7uL, 99uL, rate, 250, 0, 3), endpoint, if (rate == 60) 5000 else 1)
            r.pendingPair(5000)
            assertFalse(r.state.allowsController())
            assertFalse(r.state.pairingPending)
        }
    }
    @Test fun rejectionCooldownSurvivesNewChallengeAndEndsAtDeadline() {
        val r = ConnectionReducer(endpoint); challenge(r); r.pair(1uL, "123456", 0)
        assertFalse(r.receive(PairReject(7uL, 5, 60), endpoint, 100))
        challenge(r, 8uL, 200)
        assertNull(r.pair(1uL, "123456", 60099))
        r.pendingPair(60099)
        assertEquals(1, r.state.retryAfterSeconds)
        assertNotNull(r.pair(1uL, "123456", 60100))
    }
    @Test fun discoveryRejectsIncompatibleAndPublicAddressesAndSanitizesNames() {
        val txt = mapOf("v" to "1", "capacity" to "1", "pairing" to "required")
        assertNull(discoveredPc("pc", "PC", "8.8.8.8", 8765, txt))
        assertNull(discoveredPc("pc", "PC", "192.168.1.10", 0, txt))
        assertNull(discoveredPc("pc", "PC", "192.168.1.10", 8765, txt + ("v" to "2")))
        assertNull(discoveredPc("pc", "PC", "192.168.1.10", 8765, emptyMap()))
        val pc = discoveredPc("pc", "PC\n" + "x".repeat(100), "192.168.1.10", 8765, txt)!!
        assertEquals(64, pc.name.length)
        assertFalse(pc.name.contains('\n'))
        assertEquals(endpoint, pc.endpoint)
    }
    @Test fun discoveryIsBoundedDeduplicatedAndDisappearanceInvalidatesPendingCallbacks() {
        val c = DiscoveryCatalog<String>()
        repeat(32) { assertTrue(c.add("$it", "old")) }
        assertFalse(c.add("0", "duplicate")); assertFalse(c.add("overflow", "x"))
        val old = c.next()!!
        c.lost(old.key)
        assertTrue(c.add(old.key, "replacement"))
        c.resolve(old, DiscoveredPc(old.key, "Stale PC", endpoint))
        assertTrue(c.pcs.isEmpty())
        var lookup = c.next()
        while (lookup != null) {
            c.resolve(lookup, DiscoveredPc(lookup.key, lookup.value, endpoint)); lookup = c.next()
        }
        assertEquals(32, c.pcs.size)
        c.lost("1"); assertEquals(31, c.pcs.size)
        c.clear(); assertEquals(0, c.size); assertTrue(c.pcs.isEmpty()); assertNull(c.next())
        c.resolve(old, DiscoveredPc(old.key, "Cancelled", endpoint)); assertTrue(c.pcs.isEmpty())
    }
    @Test fun sessionWaitsForPairingPersistenceBeforeStartingSnapshotsAndClosesOnFailure() = runTest {
        for (diskFailure in listOf(false, true)) {
            var saved = false
            var closed = false
            val sent = mutableListOf<Message>()
            val replies = ArrayDeque<Message>()
            val transport = object : DatagramTransport {
                override fun send(bytes: ByteArray, destination: Endpoint) {
                    val message = V1Codec.decode(bytes)
                    if (message is com.hamzalgh.virtualgamepad.protocol.ControllerState) assertTrue(saved)
                    sent += message
                    when (message) {
                        is Hello -> replies.add(HelloAck(message.clientNonce, 9uL, 0uL, 60, 250, 1, 1, 255))
                        is PairRequest -> replies.add(PairAccept(3uL, message.clientNonce, 99uL, 60, 250, 0, 3))
                        else -> Unit
                    }
                }
                override suspend fun receive(): ReceivedDatagram? {
                    delay(10)
                    return replies.removeFirstOrNull()?.let { ReceivedDatagram(V1Codec.encode(it), endpoint) }
                }
                override fun close() { closed = true }
            }
            val owner = HelloSession({ transport }, { testScheduler.currentTime }, { 7uL })
            try {
                owner.run(endpoint, 1uL, 34, onPaired = {
                    assertEquals(99uL, it); delay(400)
                    assertFalse(sent.any { packet -> packet is com.hamzalgh.virtualgamepad.protocol.ControllerState })
                    if (diskFailure) throw java.io.IOException("disk full")
                    saved = true
                }) {
                    if (it.phase == ConnectionPhase.PairingNeeded) owner.pair("123456")
                    if (it.phase == ConnectionPhase.Ready) launch { delay(100); owner.close() }
                }
                assertFalse(diskFailure)
            } catch (_: java.io.IOException) { assertTrue(diskFailure) }
            assertTrue(closed)
            assertEquals(!diskFailure, sent.any { it is com.hamzalgh.virtualgamepad.protocol.ControllerState })
        }
    }
}
