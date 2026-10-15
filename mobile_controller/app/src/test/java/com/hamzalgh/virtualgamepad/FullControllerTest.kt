package com.hamzalgh.virtualgamepad

import com.hamzalgh.virtualgamepad.controller.*
import com.hamzalgh.virtualgamepad.network.*
import com.hamzalgh.virtualgamepad.protocol.Disconnect
import com.hamzalgh.virtualgamepad.protocol.V1Codec
import com.hamzalgh.virtualgamepad.protocol.ControllerState as Packet
import com.hamzalgh.virtualgamepad.settings.Endpoint
import kotlinx.coroutines.*
import kotlinx.coroutines.test.*
import org.junit.Assert.*
import org.junit.Test
import java.io.File
import kotlin.math.*

@OptIn(ExperimentalCoroutinesApi::class)
class FullControllerTest {
    private val geometry = GamepadGeometry.fit(800f, 280f, 1f)!!
    @Test fun measuredPhoneBoundsFitAfterKeepingSafeInsetsAndAnEightDpGutter() {
        // API 34 phone: 440 dpi, measured safe inset/utility row budget leaves this rectangle.
        val g = GamepadGeometry.fit(2132f, 766f, 2.75f)
        assertNotNull(g)
    }
    private fun down(router: GamepadRouter, control: Control, id: Long = control.ordinal.toLong()) {
        val r = router.geometry.regions.getValue(control)
        router.down(id, (r.left + r.right) / 2, (r.top + r.bottom) / 2)
        when (control) {
            Control.LeftStick -> router.move(id, geometry.stick.x + geometry.stickTravel.radius, geometry.stick.y)
            Control.RightStick -> router.move(id, geometry.rightStick.x, geometry.rightStick.y - geometry.rightStickTravel.radius)
            Control.Dpad -> router.move(id, geometry.dpad.x + 40f, geometry.dpad.y - 40f)
            else -> Unit
        }
    }

    @Test fun everyBitIsIndependentAndInvalidLocalStatesCannotBeConstructed() {
        for ((index, button) in Button.entries.withIndex()) {
            val state = ControllerState.NEUTRAL.withButton(button, true)
            assertEquals(1 shl index, state.buttons)
            assertEquals(listOf(button), Button.entries.filter { state.pressed(it) })
            assertEquals(ControllerState.NEUTRAL, state.withButton(button, false))
        }
        for (bits in listOf(0x4000, 0x8000, Int.MIN_VALUE, -1, 0xc00, 0x3000)) {
            assertThrows(IllegalArgumentException::class.java) { ControllerState(buttons = bits) }
        }
        for (value in listOf(Float.NaN, Float.POSITIVE_INFINITY, -0.01f, 1.01f)) {
            assertThrows(IllegalArgumentException::class.java) { ControllerState(leftTrigger = value) }
            assertThrows(IllegalArgumentException::class.java) { ControllerState(rightTrigger = value) }
        }
    }

    @Test fun referenceSwapAndAllRegionsAreContainedSeparatedAndBounded() {
        for ((w, h) in listOf(640f to 264f, 640f to 280f, 752f to 280f, 800f to 292f, 960f to 340f, 1200f to 500f)) {
            for (density in listOf(1f, 2.75f, 3f)) {
                val g = requireNotNull(GamepadGeometry.fit(w * density, h * density, density)) { "$w x $h at $density" }
                assertEquals(Control.entries.toSet(), g.regions.keys)
                assertTrue(g.stick.x < g.dpad.x)
                assertTrue(g.rightStick.x < g.a.x)
                assertTrue(g.regions.values.all { it.inside(w * density, h * density) })
                val values = g.regions.values.toList()
                for (i in values.indices) for (j in 0 until i) assertTrue(regionGap(values[i], values[j]) >= 8f * density - 0.001)
                for (r in values) { assertTrue(r.right - r.left >= 48f * density); assertTrue(r.bottom - r.top >= 48f * density) }
            }
        }
        for ((w, h) in listOf(639f to 400f, 800f to 263f, 480f to 800f, Float.MAX_VALUE to 400f)) {
            // Extreme finite sizes may fail region separation; they must never overflow into live input.
            assertNull(GamepadGeometry.fit(w, h, 1f))
        }
    }

    @Test fun dpadCardinalsDiagonalsCenterOutsideAndTransitionsCannotOppose() {
        val expected = listOf(8192, 10240, 2048, 6144, 4096, 5120, 1024, 9216)
        val router = GamepadRouter(geometry)
        down(router, Control.Dpad, 44)
        for (degrees in 0 until 360) {
            val angle = Math.toRadians(degrees.toDouble())
            router.move(44, geometry.dpad.x + (cos(angle) * 60).toFloat(), geometry.dpad.y + (sin(angle) * 60).toFloat())
            val bits = router.state.buttons
            assertEquals(expected[((degrees + 22.5) / 45).toInt() % 8], bits)
            assertTrue(bits and 0xc00 != 0xc00 && bits and 0x3000 != 0x3000)
        }
        router.move(44, geometry.dpad.x, geometry.dpad.y)
        assertEquals(0, router.state.buttons)
        router.move(44, geometry.a.x, geometry.a.y)
        assertEquals(0, router.state.buttons)
        assertEquals(Control.Dpad, router.ownership[44])
        router.move(44, geometry.dpad.x, geometry.dpad.y - 60)
        assertEquals(Button.Up.mask, router.state.buttons)
        down(router, Control.Dpad, 45)
        router.up(44)
        router.move(45, geometry.dpad.x, geometry.dpad.y + 60)
        assertEquals(ControllerState.NEUTRAL, router.state)
    }

    @Test fun independentTriggersUseAbsoluteMirroredPositionClampAndExactRelease() {
        val router = GamepadRouter(geometry)
        down(router, Control.LT, 10); down(router, Control.RT, 11)
        assertEquals(128, V1Codec.encodeTrigger(router.state.leftTrigger))
        assertEquals(128, V1Codec.encodeTrigger(router.state.rightTrigger))
        for (value in listOf(0f, 0.5f, 1f)) {
            router.move(10, geometry.lt.left + value * (geometry.lt.right - geometry.lt.left), -1000f)
            router.move(11, geometry.rt.right - value * (geometry.rt.right - geometry.rt.left), 1000f)
            assertEquals(value, router.state.leftTrigger, 0f)
            assertEquals(value, router.state.rightTrigger, 0f)
        }
        router.move(10, Float.MAX_VALUE, 0f); router.move(11, -Float.MAX_VALUE, 0f)
        assertEquals(255, V1Codec.encodeTrigger(router.state.leftTrigger))
        assertEquals(255, V1Codec.encodeTrigger(router.state.rightTrigger))
        router.up(10)
        assertEquals(0f, router.state.leftTrigger, 0f); assertEquals(1f, router.state.rightTrigger, 0f)
        router.up(11)
        assertEquals(ControllerState.NEUTRAL, router.state)
    }

    @Test fun rightStickReusesEndpointsDiagonalCenterAndUntunedKnobTravel() {
        val router = GamepadRouter(geometry)
        down(router, Control.RightStick, 5)
        for ((x, y) in listOf(-1f to 0f, 1f to 0f, 0f to -1f, 0f to 1f, 0f to 0f)) {
            router.move(5, geometry.rightStick.x + x * geometry.rightStickTravel.radius,
                geometry.rightStick.y - y * geometry.rightStickTravel.radius)
            assertEquals(V1Codec.encodeAxis(x), V1Codec.encodeAxis(router.state.rightX))
            assertEquals(V1Codec.encodeAxis(y), V1Codec.encodeAxis(router.state.rightY))
            assertEquals(0f, router.state.leftX, 0f); assertEquals(0f, router.state.leftY, 0f)
        }
        router.move(5, geometry.rightStick.x + 1000, geometry.rightStick.y - 1000)
        assertEquals(sqrt(0.5f), router.state.rightX, 0.00001f)
        assertEquals(sqrt(0.5f), router.state.rightY, 0.00001f)
        val knob = geometry.rightStickKnob(router.rightDisplacement)
        assertEquals(geometry.rightStick.radius, hypot(knob.x - geometry.rightStick.x, knob.y - geometry.rightStick.y) + knob.radius, 0.0001f)
        router.up(5)
        assertEquals(ControllerState.NEUTRAL, router.state)
    }

    @Test fun requiredMultiPointerCombinationsInBothOrdersAndIndependentRelease() {
        for (controls in listOf(
            listOf(Control.LeftStick, Control.RightStick), listOf(Control.LeftStick, Control.A, Control.B),
            listOf(Control.LT, Control.RT, Control.LeftStick), listOf(Control.Dpad, Control.X),
            listOf(Control.LB, Control.RT), listOf(Control.LeftStick, Control.RightStick, Control.Start),
            listOf(Control.LeftStick, Control.L3, Control.RightStick, Control.R3), Control.entries,
        )) for (order in listOf(controls, controls.reversed())) {
            val router = GamepadRouter(geometry)
            order.forEach { down(router, it) }
            assertEquals(controls.toSet(), router.ownership.values.toSet())
            controls.filter { it.button != null }.forEach { assertTrue(router.state.pressed(it.button!!)) }
            for (control in order) {
                router.up(control.ordinal.toLong())
                val expected = GamepadRouter(geometry)
                router.ownership.values.forEach { down(expected, it) }
                assertEquals(expected.state, router.state)
            }
            assertEquals(ControllerState.NEUTRAL, router.state)
        }
    }

    @Test fun allDigitalControlsLeaveReenterAndNeverTransfer() {
        for (control in Control.entries.filter { it.button != null }) {
            val router = GamepadRouter(geometry)
            down(router, control, 77)
            router.move(77, geometry.stick.x, geometry.stick.y)
            assertEquals(ControllerState.NEUTRAL, router.state)
            down(router, control, 88)
            assertFalse(router.owns(88))
            val r = geometry.regions.getValue(control)
            router.move(77, (r.left + r.right) / 2, (r.top + r.bottom) / 2)
            assertEquals(control.button!!.mask, router.state.buttons)
            router.up(77)
            router.move(88, (r.left + r.right) / 2, (r.top + r.bottom) / 2)
            assertEquals(ControllerState.NEUTRAL, router.state)
        }
    }

    @Test fun globalCancelAndNonfiniteEventsCannotRestoreHeldInput() {
        val router = GamepadRouter(geometry)
        Control.entries.forEach { down(router, it) }
        assertTrue(router.state != ControllerState.NEUTRAL)
        router.cancel()
        assertEquals(ControllerState.NEUTRAL, router.state)
        assertTrue(router.ownership.isEmpty())
        assertEquals(StickVector(), router.displacement); assertEquals(StickVector(), router.rightDisplacement)
        Control.entries.forEach { router.move(it.ordinal.toLong(), geometry.a.x, geometry.a.y) }
        assertEquals(ControllerState.NEUTRAL, router.state)
        down(router, Control.LT, 99)
        router.move(99, Float.NaN, 0f)
        assertEquals(ControllerState.NEUTRAL, router.state)
        assertFalse(router.owns(99))
    }

    @Test fun mixedLocalSnapshotMatchesSharedFixtureAndCleanupZerosEveryField() = runTest {
        val sent = mutableListOf<ByteArray>()
        val transport = object : DatagramTransport {
            override fun send(bytes: ByteArray, destination: Endpoint) { sent += bytes }
            override suspend fun receive(): ReceivedDatagram? = null
            override fun close() {}
        }
        val local = ControllerState(9253, -12345f / 32768f, 23456f / 32767f,
            13579f / 32767f, -24680f / 32768f, 85f / 255f, 170f / 255f)
        val sender = StateSender(transport, Endpoint.parse("127.0.0.1", "8765")!!, 3544952156018063160uL,
            { 2271560481L + testScheduler.currentTime }, 305419896u)
        val job = launch { sender.run(60, { true }, { local }) }
        runCurrent()
        assertArrayEquals(File("../../protocol/fixtures/v1/mixed.bin").readBytes(), sent.single())
        advanceTimeBy(1000)
        job.cancelAndJoin()
        assertEquals(60, sent.size)
        assertTrue(sent.map { V1Codec.decode(it) as Packet }.all {
            it.buttons == 9253u && it.rightX == 13579.toShort() && it.rightTrigger == 170
        })
        sender.finish(1)
        val neutral = V1Codec.decode(sent[sent.size - 4]) as Packet
        assertEquals(Packet(neutral.session, neutral.sequence, neutral.clientMs, 0u, 0, 0, 0, 0, 0, 0), neutral)
        assertTrue(sent.takeLast(3).all { V1Codec.decode(it) == Disconnect(neutral.session, 1) })
    }
}
