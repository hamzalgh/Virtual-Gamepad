package com.hamzalgh.virtualgamepad

import com.hamzalgh.virtualgamepad.controller.*
import com.hamzalgh.virtualgamepad.protocol.V1Codec
import kotlin.math.*
import org.junit.Assert.*
import org.junit.Test

class LeftStickTest {
    private val gate = Circle(0f, 0f, 100f)
    private val geometry = GamepadGeometry.fit(640f, 280f, 1f)!!
    private fun axis(x: Float, y: Float, tuning: StickTuning = StickTuning()) =
        transformStick(stickDisplacement(x, y, gate), tuning)
    private fun stick(router: GamepadRouter, id: Long = 31) =
        router.down(id, geometry.stick.x + geometry.stick.radius, geometry.stick.y)
    private fun button(router: GamepadRouter, id: Long = 7) =
        router.down(id, geometry.a.x, geometry.a.y)

    @Test fun visibleKnobContactReachesEveryCardinalEndpoint() {
        val router = GamepadRouter(geometry)
        val travel = geometry.stick.radius * (1f - 0.32f)
        router.down(31, geometry.stick.x, geometry.stick.y)
        for ((dx, dy) in listOf(1f to 0f, -1f to 0f, 0f to 1f, 0f to -1f)) {
            router.move(31, geometry.stick.x + dx * travel, geometry.stick.y - dy * travel)
            assertEquals(V1Codec.encodeAxis(dx), V1Codec.encodeAxis(router.state.leftX))
            assertEquals(V1Codec.encodeAxis(dy), V1Codec.encodeAxis(router.state.leftY))
        }
    }

    @Test fun knobFollowsFingerBeforeContactAndStaysInsideBaseBeyondContact() {
        for (density in listOf(1f, 2.75f, 3f)) {
            val g = GamepadGeometry.fit(640f * density, 280f * density, density)!!
            val router = GamepadRouter(g)
            router.down(31, g.stick.x, g.stick.y)
            for (dx in listOf(-0.3f, 0f, 0.3f)) for (dy in listOf(-0.3f, 0f, 0.3f)) {
                val x = g.stick.x + dx * g.stick.radius
                val y = g.stick.y + dy * g.stick.radius
                router.move(31, x, y)
                val knob = g.stickKnob(router.displacement)
                assertEquals(x, knob.x, 0.0001f)
                assertEquals(y, knob.y, 0.0001f)
            }
            router.move(31, g.stick.x + g.stick.radius * 10f, g.stick.y)
            val knob = g.stickKnob(router.displacement)
            assertEquals(g.stick.x + g.stick.radius, knob.x + knob.radius, 0.0001f)
            assertEquals(1f, router.state.leftX, 0f)
            router.up(31)
            assertEquals(ControllerState.NEUTRAL, router.state)
            assertEquals(g.stick.x, g.stickKnob(router.displacement).x, 0f)
        }
    }

    @Test fun centerCardinalsYDirectionAndExactEncodingEndpoints() {
        assertEquals(StickVector(), axis(0f, 0f))
        for ((point, expected) in listOf(
            (100f to 0f) to (32767 to 0), (-100f to 0f) to (-32768 to 0),
            (0f to -100f) to (0 to 32767), (0f to 100f) to (0 to -32768),
        )) {
            val result = axis(point.first, point.second)
            assertEquals(expected, V1Codec.encodeAxis(result.x).toInt() to V1Codec.encodeAxis(result.y).toInt())
        }
    }

    @Test fun diagonalAndRadiusClampUseCircularMagnitude() {
        val diagonal = axis(100f, -100f)
        assertEquals(sqrt(0.5f), diagonal.x, 0.00001f)
        assertEquals(diagonal.x, diagonal.y, 0f)
        assertTrue(axis(99.9f, 0f).x < 1f)
        assertEquals(1f, axis(100.1f, 0f).x, 0f)
        assertEquals(diagonal, axis(10000f, -10000f))
        assertEquals(1f, hypot(diagonal.x, diagonal.y), 0.00001f)
    }

    @Test fun deadZoneZeroThresholdOutsideMidpointAndFullScale() {
        assertEquals(0.1f, axis(10f, 0f, StickTuning(0f)).x, 0.00001f)
        assertEquals(StickVector(), axis(15f, 0f))
        assertEquals(StickVector(), axis(14.999f, 0f))
        assertTrue(axis(15.01f, 0f).x > 0f)
        assertEquals(0.5f, axis(57.5f, 0f).x, 0.00001f)
        assertEquals(1f, axis(100f, 0f).x, 0f)
    }

    @Test fun curveIsMonotonicSymmetricBoundedAndPreservesEndpoints() {
        for (deadZone in listOf(0f, 0.15f, 0.30f)) for (sensitivity in listOf(0.5f, 1f, 2f)) {
            val tuning = StickTuning(deadZone, sensitivity)
            for (angle in 0 until 360 step 15) {
                var previous = 0f
                val radians = Math.toRadians(angle.toDouble())
                for (radius in 0..120) {
                    val x = (cos(radians) * radius).toFloat()
                    val y = (sin(radians) * radius).toFloat()
                    val value = axis(x, y, tuning)
                    val opposite = axis(-x, -y, tuning)
                    val magnitude = hypot(value.x, value.y)
                    assertTrue(magnitude + 0.000001f >= previous)
                    assertTrue(magnitude <= 1.000001f)
                    assertEquals(-value.x, opposite.x, 0f)
                    assertEquals(-value.y, opposite.y, 0f)
                    if (radius == 0) assertEquals(StickVector(), value)
                    if (radius >= 100) assertEquals(1f, magnitude, 0.000001f)
                    previous = magnitude
                }
            }
        }
        assertTrue(axis(57.5f, 0f, StickTuning(sensitivity = 2f)).x > 0.5f)
        assertTrue(axis(57.5f, 0f, StickTuning(sensitivity = 0.5f)).x < 0.5f)
    }

    @Test fun finiteExtremeCoordinatesAndBoundsNeverProduceNanOrInfinity() {
        val tiny = Circle(Float.MAX_VALUE, -Float.MAX_VALUE, Float.MIN_VALUE)
        for (x in listOf(-Float.MAX_VALUE, 0f, Float.MAX_VALUE)) for (y in listOf(-Float.MAX_VALUE, 0f, Float.MAX_VALUE)) {
            val value = transformStick(stickDisplacement(x, y, tiny), StickTuning())
            assertTrue(value.x.isFinite() && value.x in -1f..1f)
            assertTrue(value.y.isFinite() && value.y in -1f..1f)
        }
        assertEquals(StickVector(), stickDisplacement(Float.NaN, 1f, gate))
        assertEquals(StickVector(), stickDisplacement(1f, Float.POSITIVE_INFINITY, gate))
        assertNull(GamepadGeometry.fit(Float.NaN, 200f, 1f))
    }

    @Test fun tuningRejectsInvalidAndNonfiniteValues() {
        for (d in listOf(-0.01f, 0.301f, Float.NaN, Float.POSITIVE_INFINITY)) {
            assertThrows(IllegalArgumentException::class.java) { StickTuning(deadZone = d) }
        }
        for (s in listOf(0f, 0.499f, 2.001f, Float.NaN, Float.POSITIVE_INFINITY)) {
            assertThrows(IllegalArgumentException::class.java) { StickTuning(sensitivity = s) }
        }
    }

    @Test fun simultaneousStickAndAInBothOrdersReleaseOnlyTheirOwner() {
        for (stickFirst in listOf(true, false)) {
            val router = GamepadRouter(geometry)
            if (stickFirst) { stick(router); button(router) } else { button(router); stick(router) }
            assertEquals(ControllerState(1, 1f, 0f), router.state)
            assertEquals(mapOf(31L to Control.LeftStick, 7L to Control.A), router.ownership)
            router.up(if (stickFirst) 31 else 7)
            assertEquals(if (stickFirst) ControllerState(buttons = 1) else ControllerState(leftX = 1f), router.state)
            router.cancel()
            assertEquals(ControllerState.NEUTRAL, router.state)
            assertTrue(router.ownership.isEmpty())
        }
    }

    @Test fun occupiedAndEmptyDownsNeverStealOrAcquireOnMove() {
        val router = GamepadRouter(geometry)
        stick(router)
        stick(router, 90)
        router.down(99, 320f, 56f)
        router.up(31)
        for (id in listOf(90L, 99L)) router.move(id, geometry.stick.x + geometry.stick.radius, geometry.stick.y)
        assertEquals(ControllerState.NEUTRAL, router.state)
        assertTrue(router.ownership.isEmpty())
        stick(router, 100)
        router.move(100, geometry.a.x, geometry.a.y)
        assertFalse(router.state.a)
        assertTrue(router.state.leftX > 0f)
        assertEquals(Control.LeftStick, router.ownership[100])
    }

    @Test fun reorderedPointerUpdatesAndDigitalReentryRetainStableIds() {
        val router = GamepadRouter(geometry)
        stick(router); button(router)
        for (order in listOf(listOf(7L, 31L), listOf(31L, 7L))) {
            for (id in order) when (id) {
                7L -> router.move(id, 1f, 1f)
                31L -> router.move(id, geometry.stick.x, geometry.stick.y - 10000f)
            }
            assertEquals(ControllerState(0, 0f, 1f), router.state)
        }
        router.move(7, geometry.a.x, geometry.a.y)
        assertTrue(router.state.a)
        router.up(999)
        assertEquals(2, router.ownership.size)
        router.up(31)
        assertEquals(ControllerState(buttons = 1), router.state)
        router.cancel()
        router.move(7, geometry.a.x, geometry.a.y)
        assertEquals(ControllerState.NEUTRAL, router.state)
        assertEquals(StickVector(), router.displacement)
    }

    @Test fun compactWideAndDensityFixturesStayWithinSafeBoundsWithNoOverlap() {
        for ((width, height) in listOf(640f to 280f, 752f to 280f, 800f to 300f, 960f to 340f)) {
            for (density in listOf(1f, 2.75f, 3f)) {
                val g = GamepadGeometry.fit(width * density, height * density, density)!!
                assertTrue(g.stickCapture.inside(width * density, height * density))
                assertTrue(g.aCapture.inside(width * density, height * density))
                assertTrue(g.aCapture.x - g.stickCapture.x - g.stickCapture.radius - g.aCapture.radius >= 8f * density)
            }
        }
        assertNull(GamepadGeometry.fit(200f, 100f, 1f))
        assertNull(GamepadGeometry.fit(0f, 0f, 1f))
    }
}
