package com.hamzalgh.virtualgamepad

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.unit.dp
import com.hamzalgh.virtualgamepad.controller.*
import androidx.compose.ui.platform.LocalDensity
import com.hamzalgh.virtualgamepad.ui.gamepad.GamepadSurface
import com.hamzalgh.virtualgamepad.ui.theme.VirtualGamepadTheme
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test

class GamepadSurfaceTest {
    @get:Rule val compose = createComposeRule()
    private val local = mutableStateOf(ControllerState.NEUTRAL)
    private var density = 1f
    private fun TouchInjectionScope.geometry() = GamepadGeometry.fit(width.toFloat(), height.toFloat(), density)!!
    private val epoch = mutableStateOf(0)
    private fun content(diagnostics: String = "") {
        compose.setContent {
            density = LocalDensity.current.density
            Box(Modifier.size(800.dp, 400.dp)) {
                VirtualGamepadTheme {
                    GamepadSurface(local.value, "Ready", { local.value = it }, {}, epoch.value, diagnostics)
                }
            }
        }
    }

    @Test fun downMoveOutReenterAndUpGiveImmediateFeedback() {
        content()
        val surface = compose.onNodeWithTag("GamepadSurface")
        surface.performTouchInput { down(Offset(geometry().a.x, geometry().a.y)) }
        compose.runOnIdle { assertTrue(local.value.a) }
        surface.performTouchInput { moveTo(Offset(1f, 1f)) }
        compose.runOnIdle { assertFalse(local.value.a) }
        surface.performTouchInput { moveTo(Offset(geometry().a.x, geometry().a.y)) }
        compose.runOnIdle { assertTrue(local.value.a) }
        surface.performTouchInput { up() }
        compose.runOnIdle { assertEquals(ControllerState.NEUTRAL, local.value) }
        compose.onNodeWithContentDescription("Exit controller").assertIsDisplayed()
    }

    @Test fun simultaneousPointersInBothOrdersAndIndependentRelease() {
        content()
        val surface = compose.onNodeWithTag("GamepadSurface")
        for (stickFirst in listOf(true, false)) {
            surface.performTouchInput {
                if (stickFirst) {
                    down(31, Offset(geometry().stick.x, geometry().stick.y))
                    down(7, Offset(geometry().a.x, geometry().a.y))
                } else {
                    down(7, Offset(geometry().a.x, geometry().a.y))
                    down(31, Offset(geometry().stick.x, geometry().stick.y))
                }
                moveTo(31, Offset(geometry().stick.x, 0f))
            }
            compose.runOnIdle { assertEquals(ControllerState(1, 0f, 1f), local.value) }
            surface.performTouchInput { up(7) }
            compose.runOnIdle { assertEquals(ControllerState(0, 0f, 1f), local.value) }
            surface.performTouchInput { up(31) }
            compose.runOnIdle { assertEquals(ControllerState.NEUTRAL, local.value) }
        }
    }

    @Test fun touchCancelAndFocusEpochResetBothControls() {
        content()
        val surface = compose.onNodeWithTag("GamepadSurface")
        repeat(2) { pass ->
            surface.performTouchInput {
                down(31, Offset(geometry().stick.x, geometry().stick.y))
                moveTo(31, Offset(geometry().stick.x, 0f))
                down(7, Offset(geometry().a.x, geometry().a.y))
            }
            compose.runOnIdle { assertTrue(local.value.a); assertEquals(1f, local.value.leftY, 0f) }
            if (pass == 0) surface.performTouchInput { cancel() }
            else {
                compose.runOnIdle { epoch.value++ }
                compose.waitForIdle()
                surface.performTouchInput { moveTo(31, Offset(geometry().stick.x, 1f)) }
            }
            compose.runOnIdle { assertEquals(ControllerState.NEUTRAL, local.value) }
            if (pass == 1) surface.performTouchInput { cancel() }
        }
    }

    @Test fun diagnosticsCancelHeldInputAndLeaveExitAvailable() {
        content("Sent 60 · skipped 0")
        val surface = compose.onNodeWithTag("GamepadSurface")
        surface.performTouchInput {
            down(31, Offset(geometry().stick.x, geometry().stick.y))
            moveTo(31, Offset(geometry().stick.x, 0f))
            down(7, Offset(geometry().a.x, geometry().a.y))
        }
        compose.runOnIdle { assertTrue(local.value.a); assertEquals(1f, local.value.leftY, 0f) }
        compose.onNodeWithContentDescription("Controller settings").performClick()
        compose.runOnIdle { assertEquals(ControllerState.NEUTRAL, local.value) }
        compose.onNodeWithText("Diagnostics").performScrollTo().performClick()
        compose.onNodeWithText("Session diagnostics").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("Close settings").performClick()
        compose.onNodeWithContentDescription("Exit controller").assertIsDisplayed()
        surface.performTouchInput { cancel() }
    }

    @Test fun fullSurfaceExposesControlsAndMixedTouchCancelZerosEverything() {
        content()
        for (label in listOf("A", "B", "X", "Y", "LB", "RB", "Back", "Start", "L3", "R3")) {
            compose.onNodeWithContentDescription("$label button").assertIsDisplayed()
        }
        compose.onNodeWithContentDescription("LT analog trigger").assertIsDisplayed()
        compose.onNodeWithContentDescription("RT analog trigger").assertIsDisplayed()
        val surface = compose.onNodeWithTag("GamepadSurface")
        surface.performTouchInput {
            val g = geometry()
            for ((index, control) in listOf(Control.LeftStick, Control.RightStick, Control.LT, Control.RT, Control.L3, Control.Start).withIndex()) {
                val r = g.regions.getValue(control)
                down(index, Offset((r.left + r.right) / 2, (r.top + r.bottom) / 2))
            }
            moveTo(0, Offset(g.stick.x + g.stickTravel.radius, g.stick.y))
            moveTo(1, Offset(g.rightStick.x, g.rightStick.y - g.rightStickTravel.radius))
        }
        compose.runOnIdle {
            assertEquals(1f, local.value.leftX, 0.00001f)
            assertEquals(1f, local.value.rightY, 0.00001f)
            assertEquals(0.5f, local.value.leftTrigger, 0.00001f)
            assertEquals(0.5f, local.value.rightTrigger, 0.00001f)
            assertTrue(local.value.pressed(Button.L3)); assertTrue(local.value.pressed(Button.Start))
        }
        surface.performTouchInput { cancel() }
        compose.runOnIdle { assertEquals(ControllerState.NEUTRAL, local.value) }
    }
}
