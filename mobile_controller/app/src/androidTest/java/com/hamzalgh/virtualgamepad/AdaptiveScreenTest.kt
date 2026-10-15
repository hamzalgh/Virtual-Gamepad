package com.hamzalgh.virtualgamepad

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import com.hamzalgh.virtualgamepad.controller.*
import com.hamzalgh.virtualgamepad.ui.gamepad.GamepadSurface
import com.hamzalgh.virtualgamepad.ui.setup.TuningPanel
import com.hamzalgh.virtualgamepad.ui.theme.VirtualGamepadTheme
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test

/** Compiled on the host; executing these tests requires explicit device authorization. */
class AdaptiveScreenTest {
    @get:Rule val compose = createComposeRule()

    @Test fun tuningAtDoubleFontScaleSupportsResetApplyAndRetry() {
        var applied: StickTuning? = null
        compose.setContent {
            val density = LocalDensity.current
            CompositionLocalProvider(LocalDensity provides Density(density.density, 2f)) {
                VirtualGamepadTheme {
                    Column(Modifier.size(320.dp, 240.dp).verticalScroll(rememberScrollState())) {
                        TuningPanel(StickTuning(0.3f, 2f), false, "Could not save tuning.") { applied = it }
                    }
                }
            }
        }
        compose.onNodeWithText("Reset to defaults").performScrollTo().performClick()
        compose.onNodeWithText("Could not save tuning.").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("Apply tuning").performScrollTo().performClick()
        compose.runOnIdle { assertEquals(StickTuning(), applied) }
        compose.onNodeWithContentDescription("Dead zone").performScrollTo()
            .performSemanticsAction(SemanticsActions.SetProgress) { it(0.3f) }
        compose.onNodeWithContentDescription("Sensitivity").performScrollTo()
            .performSemanticsAction(SemanticsActions.SetProgress) { it(2f) }
        compose.onNodeWithText("Apply tuning").performScrollTo().performClick()
        compose.runOnIdle { assertEquals(StickTuning(0.3f, 2f), applied) }
    }

    @Test fun statusChangesDoNotResizeSurfaceAndSettingsRestoreWindowPolicy() {
        val status = mutableStateOf("Ready")
        var running = false
        compose.setContent {
            VirtualGamepadTheme {
                Box(Modifier.size(800.dp, 400.dp)) {
                    GamepadSurface(ControllerState.NEUTRAL, status.value, {}, {},
                        onSurfaceRunning = { running = it })
                }
            }
        }
        val bounds = compose.onNodeWithTag("GamepadSurface").fetchSemanticsNode().boundsInRoot
        compose.runOnIdle { assertTrue(running); status.value = "Degraded" }
        compose.onNodeWithText("Degraded").assertDoesNotExist()
        assertEquals(bounds, compose.onNodeWithTag("GamepadSurface").fetchSemanticsNode().boundsInRoot)
        compose.onNodeWithText("VIRTUAL GAMEPAD").assertDoesNotExist()
        compose.onNodeWithText("Diagnostics").assertDoesNotExist()
        compose.onNodeWithContentDescription("Controller settings").performClick()
        compose.onNodeWithText("Degraded").assertIsDisplayed()
        compose.runOnIdle { assertFalse(running) }
        compose.onNodeWithText("Close settings").performClick()
        compose.runOnIdle { assertTrue(running) }
    }

    @Test fun portraitOrSmallWindowIsNeutralWithExitAvailable() {
        var running = true
        compose.setContent {
            VirtualGamepadTheme {
                Box(Modifier.size(320.dp, 480.dp)) {
                    GamepadSurface(ControllerState.NEUTRAL, "Ready", {}, {},
                        onSurfaceRunning = { running = it })
                }
            }
        }
        compose.onNodeWithText("More landscape space needed").assertIsDisplayed()
        compose.onNodeWithContentDescription("A button").assertDoesNotExist()
        compose.onNodeWithContentDescription("Exit controller").assertIsDisplayed()
        compose.runOnIdle { assertFalse(running) }
    }

    @Test fun gearContainsVisibilityAndTrustWhileNavbarKeepsHeldControls() {
        val visible = mutableStateOf(true)
        compose.setContent {
            VirtualGamepadTheme {
                Box(Modifier.size(800.dp, 400.dp)) {
                    GamepadSurface(ControllerState.NEUTRAL, "Ready", {}, {},
                        manualTrust = true, showStickClicks = visible.value,
                        onShowStickClicks = { visible.value = it })
                }
            }
        }
        compose.onNodeWithContentDescription("Back button").assertIsDisplayed()
        compose.onNodeWithContentDescription("Start button").assertIsDisplayed()
        compose.onNodeWithContentDescription("Exit controller").assertIsDisplayed()
        compose.onNodeWithText("Ready").assertDoesNotExist()
        compose.onNodeWithText("Unsafe development trust is active. This is not pairing.").assertDoesNotExist()
        compose.onNodeWithContentDescription("L3 button").assertIsDisplayed()
        compose.onNodeWithContentDescription("Controller settings").performClick()
        compose.onNodeWithText("Ready").assertIsDisplayed()
        compose.onNodeWithContentDescription("Show L3/R3").performScrollTo().performClick()
        compose.onNodeWithText("Close settings").performClick()
        compose.onNodeWithContentDescription("L3 button").assertDoesNotExist()
        compose.onNodeWithContentDescription("R3 button").assertDoesNotExist()
        compose.onNodeWithContentDescription("Controller settings").performClick()
        compose.onNodeWithContentDescription("Show L3/R3").performScrollTo().performClick()
        compose.onNodeWithText("Close settings").performClick()
        compose.onNodeWithContentDescription("L3 button").assertIsDisplayed()
        compose.onNodeWithContentDescription("R3 button").assertIsDisplayed()
    }
}
