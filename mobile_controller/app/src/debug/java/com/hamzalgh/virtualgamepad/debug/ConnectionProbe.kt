package com.hamzalgh.virtualgamepad.debug

import android.app.Activity
import android.app.Instrumentation
import android.content.Intent
import android.os.Bundle
import android.os.SystemClock
import androidx.lifecycle.ViewModelProvider
import com.hamzalgh.virtualgamepad.app.ConnectionViewModel
import com.hamzalgh.virtualgamepad.app.MainActivity
import com.hamzalgh.virtualgamepad.network.ConnectionPhase
import com.hamzalgh.virtualgamepad.network.diagnosticText
import com.hamzalgh.virtualgamepad.settings.Endpoint
import com.hamzalgh.virtualgamepad.controller.ControllerState

/** Debug APK only. Explicit adb invocation; neutral by default, optional scripted mixed input. */
class ConnectionProbe : Instrumentation() {
    private lateinit var arguments: Bundle
    override fun onCreate(arguments: Bundle?) {
        this.arguments = arguments ?: Bundle()
        start()
    }

    override fun onStart() {
        var activity: MainActivity? = null
        var model: ConnectionViewModel? = null
        val result = Bundle()
        var code = Activity.RESULT_CANCELED
        try {
            val endpoint = requireNotNull(Endpoint.parse(arguments.getString("pcAddress") ?: "", "8765"))
            val duration = (arguments.getString("durationSeconds") ?: "60").toLong().coerceIn(10, 1800) * 1000
            val pattern = arguments.getString("inputPattern") ?: "neutral"
            require(pattern in setOf("neutral", "mixed", "exercise")) { "Unknown inputPattern" }
            val mixed = ControllerState(buttons = 3, leftX = 1f, rightY = -1f,
                leftTrigger = 0.5f, rightTrigger = 1f)
            activity = startActivitySync(Intent(targetContext, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)) as MainActivity
            runOnMainSync { model = ViewModelProvider(activity!!)[ConnectionViewModel::class.java] }
            val owner = requireNotNull(model)
            val loadDeadline = SystemClock.elapsedRealtime() + 5000
            while (!owner.state.value.loaded && SystemClock.elapsedRealtime() < loadDeadline) SystemClock.sleep(50)
            check(owner.state.value.loaded) { "Settings did not load" }
            runOnMainSync { owner.editAddress(endpoint.address); owner.editPort(endpoint.port.toString()); owner.connect() }
            val connectDeadline = SystemClock.elapsedRealtime() + 8000
            while (!owner.state.value.controllerActive && SystemClock.elapsedRealtime() < connectDeadline) {
                if (owner.state.value.connection.phase in setOf(ConnectionPhase.Error, ConnectionPhase.TimedOut)) break
                SystemClock.sleep(50)
            }
            check(owner.state.value.controllerActive) { "Connection failed: ${owner.state.value.connection.detail}" }
            val started = SystemClock.elapsedRealtime()
            var nextReport = started
            while (SystemClock.elapsedRealtime() - started < duration) {
                val state = owner.state.value
                check(state.controllerActive) { "Disconnected after ${SystemClock.elapsedRealtime() - started} ms: ${state.connection.diagnosticText()}" }
                // Scripted state exercises the real sender/LAN/backend, not touchscreen ownership.
                if (pattern == "mixed") runOnMainSync { owner.setController(mixed) }
                if (pattern == "exercise") {
                    val value = when (((SystemClock.elapsedRealtime() - started) / 1000) % 3) {
                        0L -> mixed.copy(buttons = 0x27ff)
                        1L -> mixed.copy(buttons = 0x1bff, leftX = -1f, rightY = 1f,
                            leftTrigger = 1f, rightTrigger = 0.5f)
                        else -> ControllerState.NEUTRAL
                    }
                    runOnMainSync { owner.setController(value) }
                }
                if (SystemClock.elapsedRealtime() >= nextReport) {
                    sendStatus(0, Bundle().apply { putString("stream", "elapsed=${SystemClock.elapsedRealtime() - started} ${state.connection.diagnosticText()}\n") })
                    nextReport += 5000
                }
                SystemClock.sleep(50)
            }
            result.putString("stream", "PASS ${duration} ms at negotiated timeout ${owner.state.value.connection.timeoutMs} ms\n")
            code = Activity.RESULT_OK
        } catch (error: Exception) {
            result.putString("stream", "FAIL: ${error.message}\n")
        } finally {
            runOnMainSync { model?.disconnect(); activity?.finish() }
            finish(code, result)
        }
    }
}
