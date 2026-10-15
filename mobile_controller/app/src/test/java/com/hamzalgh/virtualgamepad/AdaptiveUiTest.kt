package com.hamzalgh.virtualgamepad

import com.hamzalgh.virtualgamepad.app.*
import com.hamzalgh.virtualgamepad.controller.*
import com.hamzalgh.virtualgamepad.network.*
import com.hamzalgh.virtualgamepad.ui.gamepad.*
import org.junit.Assert.*
import org.junit.Test

class AdaptiveUiTest {
    @Test fun navbarAndHiddenStickClicksHaveSeparateOwnership() {
        val visible = requireNotNull(GamepadGeometry.fit(800f, 384f, 1f))
        val hidden = requireNotNull(GamepadGeometry.fit(800f, 384f, 1f, false))
        assertEquals(48f, hidden.navbar.bottom)
        assertTrue(hidden.a.radius > 28f)
        assertTrue(hidden.a.bottom < 384f - 48f)
        assertEquals(visible.regions - setOf(Control.L3, Control.R3), hidden.regions)
        val router = GamepadRouter(hidden)
        for (region in listOf(hidden.gear, hidden.exit, visible.regions.getValue(Control.L3), visible.regions.getValue(Control.R3))) {
            router.down(1, (region.left + region.right) / 2, (region.top + region.bottom) / 2)
            assertTrue(router.ownership.isEmpty())
            assertEquals(ControllerState.NEUTRAL, router.state)
        }
        for (control in listOf(Control.Back, Control.Start)) {
            val r = hidden.regions.getValue(control)
            router.down(control.ordinal.toLong(), (r.left + r.right) / 2, 24f)
        }
        assertEquals(Button.Back.mask or Button.Start.mask, router.state.buttons)
        router.cancel()
        assertEquals(ControllerState.NEUTRAL, router.state)
        val held = ControllerState(buttons = Button.L3.mask or Button.R3.mask or Button.A.mask,
            leftX = 0.75f, rightTrigger = 1f)
        assertEquals(held.copy(buttons = Button.A.mask), held.withVisibleStickClicks(false))
        assertSame(held, held.withVisibleStickClicks(true))
    }

    @Test fun safeInsetMatrixFitsBothCutoutSidesAndDensities() {
        // Window dp, left/right cutout or navigation edge, top/bottom gesture insets.
        val fixtures = listOf(
            listOf(720f, 360f, 24f, 24f, 0f, 24f),
            listOf(872f, 393f, 34f, 24f, 0f, 16f),
            listOf(960f, 440f, 40f, 24f, 0f, 24f),
            listOf(1280f, 720f, 32f, 16f, 24f, 24f),
            listOf(1920f, 1080f, 48f, 24f, 32f, 32f),
        )
        for (fixture in fixtures) {
            val (w, h, left, right, top) = fixture
            val bottom = fixture[5]
            for (reverse in listOf(false, true)) for (density in listOf(1f, 1.5f, 2.75f, 3.5f)) {
                val insetLeft = if (reverse) right else left
                val insetRight = if (reverse) left else right
                val width = (w - insetLeft - insetRight - 16f) * density
                val height = (h - top - bottom - 16f) * density
                val g = requireNotNull(GamepadGeometry.fit(width, height, density)) { "$w x $h" }
                assertEquals(Control.entries.toSet(), g.regions.keys)
                assertTrue(g.stick.x < g.dpad.x)
                assertTrue(g.rightStick.x < g.a.x)
                val regions = g.regions.values.toList() + listOf(g.gear, g.exit)
                for (r in regions) {
                    assertTrue(r.inside(width, height))
                    assertTrue(r.right - r.left >= 48f * density - 0.001f)
                    assertTrue(r.bottom - r.top >= 48f * density - 0.001f)
                }
                for (i in regions.indices) for (j in 0 until i) {
                    assertTrue(regionGap(regions[i], regions[j]) >= 8f * density - 0.001)
                }
                assertTrue(g.stick.radius * 2 / density in 112f..180f)
                assertTrue(g.a.radius * 2 / density in 56f..84f)
                assertTrue((g.lt.bottom - g.lt.top) / density <= 90.001f)
                // Renderer and router use this same object, including its scaled bounds.
                val router = GamepadRouter(g)
                assertSame(g, router.geometry)
                for ((control, r) in g.regions) {
                    router.down(0, (r.left + r.right) / 2, (r.top + r.bottom) / 2)
                    assertEquals(control, router.ownership[0])
                    router.cancel()
                    assertEquals(ControllerState.NEUTRAL, router.state)
                }
            }
        }
    }

    @Test fun profilesRespectBoundsAndPortraitIsAlwaysUnsupported() {
        assertEquals(GeometryProfile.Compact, GamepadGeometry.fit(800f, 339f, 1f)!!.profile)
        assertEquals(GeometryProfile.Regular, GamepadGeometry.fit(800f, 340f, 1f)!!.profile)
        val regular = GamepadGeometry.fit(1100f, 500f, 1f)!!
        assertTrue(regular.stick.radius > 56f)
        for ((w, h) in listOf(1000f to 1200f, 400f to 800f, 640f to 263f, 639f to 400f)) {
            assertNull(GamepadGeometry.fit(w, h, 1f))
        }
        for (invalid in listOf(Float.NaN, Float.POSITIVE_INFINITY, -1f, 0f)) {
            assertNull(GamepadGeometry.fit(800f, 400f, invalid))
        }
    }

    @Test fun tuningApplicationIsOneNeutralStateAndPreservesSessionAndEndpoint() {
        val connection = ConnectionState(ConnectionPhase.Degraded, session = 42uL, stateRate = 60)
        val before = SetupState(address = "192.168.1.10", port = "9000", loaded = true,
            connection = connection, controllerActive = true,
            controller = ControllerState(buttons = Button.A.mask, leftX = 1f, rightTrigger = 1f), inputEpoch = 10)
        val after = before.withTuning(StickTuning(0.3f, 2f))
        assertEquals(StickTuning(0.3f, 2f), after.tuning)
        assertEquals(ControllerState.NEUTRAL, after.controller)
        assertEquals(11, after.inputEpoch)
        assertSame(connection, after.connection)
        assertTrue(after.controllerActive)
        assertEquals(before.address, after.address)
        assertEquals(before.port, after.port)
    }

    @Test fun everyConnectionPhaseAndWindowExitRestoresSystemPolicy() {
        for (phase in ConnectionPhase.entries) for (session in listOf(0uL, 42uL)) {
            val connection = ConnectionState(phase, session)
            val admitted = session != 0uL && phase in listOf(ConnectionPhase.Ready, ConnectionPhase.Degraded)
            assertEquals(admitted, connection.allowsController())
            for (available in listOf(false, true)) for (panel in listOf(false, true)) for (foreground in listOf(false, true)) {
                assertEquals(admitted && available && !panel && foreground,
                    controllerWindowActive(connection.allowsController(), available, panel, foreground))
            }
        }
        val calls = mutableListOf<Boolean>()
        val window = ControllerWindowPolicy { calls.add(it) }
        window.update(false); window.update(true); window.update(true)
        window.restore(); window.restore(); window.update(true); window.restore()
        assertEquals(listOf(false, true, false, true, false), calls)
        assertEquals("Driver issue", ConnectionPhase.BackendRequired.label)
        assertEquals("Timed out", ConnectionPhase.TimedOut.label)
        assertEquals("Reaching PC", ConnectionPhase.Reaching.label)
    }
}
