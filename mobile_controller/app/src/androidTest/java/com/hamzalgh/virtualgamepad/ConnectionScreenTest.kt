package com.hamzalgh.virtualgamepad

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.unit.dp
import com.hamzalgh.virtualgamepad.app.SetupState
import com.hamzalgh.virtualgamepad.network.ConnectionPhase
import com.hamzalgh.virtualgamepad.network.ConnectionState
import com.hamzalgh.virtualgamepad.ui.setup.ConnectionScreen
import com.hamzalgh.virtualgamepad.ui.theme.VirtualGamepadTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test

class ConnectionScreenTest {
    @get:Rule val composeRule = createComposeRule()

    @Test fun setupRemainsReachableInAConstrainedWindow() {
        composeRule.setContent {
            Box(Modifier.size(320.dp, 240.dp)) {
                VirtualGamepadTheme {
                    ConnectionScreen(SetupState(loaded = true, permissionGranted = true), {}, {}, {}, {}, {}, {}, {})
                }
            }
        }
        composeRule.onNodeWithText("Full controller. Connect enters controller mode when the PC is ready.")
            .performScrollTo().assertIsDisplayed()
        composeRule.onNodeWithText("Connect").performScrollTo().assertIsDisplayed()
    }

    @Test fun permissionAndValidationRemainRecoverable() {
        var requests = 0
        composeRule.setContent {
            VirtualGamepadTheme {
                ConnectionScreen(SetupState(loaded = true, endpointError = true), {}, {}, {}, {}, { requests++ }, {}, {})
            }
        }
        composeRule.onNodeWithText("Connect").performScrollTo().assertIsNotEnabled()
        composeRule.onNodeWithText("Enter a private/local IPv4 address and port 1 to 65535.").performScrollTo().assertIsDisplayed()
        composeRule.onNodeWithText("Allow local network").performScrollTo().performClick()
        assertEquals(1, requests)
    }

    @Test fun acknowledgedBackendPrerequisiteAndEndpointCallbacksAreVisible() {
        var address = ""
        composeRule.setContent {
            VirtualGamepadTheme {
                ConnectionScreen(SetupState(loaded = true, permissionGranted = true,
                    connection = ConnectionState(ConnectionPhase.BackendRequired, manualTrust = true)),
                    { address = it }, {}, {}, {}, {}, {}, {})
            }
        }
        composeRule.onNodeWithText("Driver issue").performScrollTo().assertIsDisplayed()
        composeRule.onNodeWithText("PC IPv4 address").performScrollTo().performTextInput("192.168.1.10")
        assertEquals("192.168.1.10", address)
    }
}
