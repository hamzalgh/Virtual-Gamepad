package com.hamzalgh.virtualgamepad.app

import com.hamzalgh.virtualgamepad.network.ConnectionPhase
import com.hamzalgh.virtualgamepad.network.ConnectionState
import com.hamzalgh.virtualgamepad.controller.ControllerState
import com.hamzalgh.virtualgamepad.controller.Button

fun ControllerState.withVisibleStickClicks(show: Boolean) = if (show) this else
    copy(buttons = buttons and (Button.L3.mask or Button.R3.mask).inv())

val ConnectionPhase.label: String get() = when (this) {
    ConnectionPhase.Offline -> "Offline"
    ConnectionPhase.PermissionNeeded -> "Permission needed"
    ConnectionPhase.Reaching -> "Reaching PC"
    ConnectionPhase.BackendRequired -> "Driver issue"
    ConnectionPhase.PairingNeeded -> "Pairing needed"
    ConnectionPhase.Ready -> "Ready"
    ConnectionPhase.Degraded -> "Degraded"
    ConnectionPhase.TimedOut -> "Timed out"
    ConnectionPhase.Error -> "Connection error"
}

fun ConnectionState.allowsController() = session != 0uL &&
    (phase == ConnectionPhase.Ready || phase == ConnectionPhase.Degraded)

/** Keep window policy testable without creating an Activity or a driver session. */
fun controllerWindowActive(sessionActive: Boolean, surfaceAvailable: Boolean, panelOpen: Boolean, foreground: Boolean) =
    sessionActive && surfaceAvailable && !panelOpen && foreground

fun interface ControllerWindowAdapter { fun apply(active: Boolean) }

class ControllerWindowPolicy(private val adapter: ControllerWindowAdapter) {
    private var applied: Boolean? = null
    fun update(active: Boolean) {
        if (applied != active) { adapter.apply(active); applied = active }
    }
    fun restore() = update(false)
}
