package com.hamzalgh.virtualgamepad.app

import android.graphics.Color
import android.content.Intent
import android.os.Bundle
import android.provider.Settings
import androidx.activity.ComponentActivity
import androidx.activity.SystemBarStyle
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.core.net.toUri
import com.hamzalgh.virtualgamepad.ui.setup.ConnectionScreen
import com.hamzalgh.virtualgamepad.ui.theme.VirtualGamepadTheme
import androidx.activity.compose.BackHandler
import androidx.compose.runtime.DisposableEffect
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import android.view.WindowManager
import com.hamzalgh.virtualgamepad.ui.gamepad.GamepadSurface
import com.hamzalgh.virtualgamepad.network.diagnosticText

class MainActivity : ComponentActivity() {
    private lateinit var model: ConnectionViewModel
    private var foreground = false
    private var surfaceRunning = false
    private val windowPolicy by lazy { ControllerWindowPolicy(::applyControllerWindow) }
    private val permission = registerForActivityResult(ActivityResultContracts.RequestPermission()) {
        model.permissionChanged(it)
    }
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        model = ViewModelProvider(this)[ConnectionViewModel::class.java]
        enableEdgeToEdge(
            statusBarStyle = SystemBarStyle.dark(Color.TRANSPARENT),
            navigationBarStyle = SystemBarStyle.dark(Color.TRANSPARENT),
        )
        setContent {
            VirtualGamepadTheme {
                val state = model.state.collectAsStateWithLifecycle().value
                DisposableEffect(state.controllerActive) {
                    if (!state.controllerActive) { surfaceRunning = false; windowPolicy.restore() }
                    onDispose { surfaceRunning = false; windowPolicy.restore() }
                }
                BackHandler(enabled = state.controllerActive) { model.disconnect() }
                if (state.controllerActive) {
                    GamepadSurface(state.controller,
                        state.connection.phase.label,
                        model::setController, model::disconnect, state.inputEpoch, state.connection.diagnosticText(), state.tuning,
                        state.tuningSaving, state.tuningError, model::saveTuning,
                        { active -> surfaceRunning = active; updateControllerWindow() }, state.connection.manualTrust,
                        state.showStickClicks, state.appearanceSaving, state.appearanceError, model::showStickClicks)
                } else ConnectionScreen(state, model::editAddress, model::editPort, model::connect,
                    model::disconnect, { permission.launch(LOCAL_NETWORK_PERMISSION) },
                    { startActivity(Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, "package:$packageName".toUri())) },
                    model::loadSettings, model::saveTuning, model::editPairingCode, model::pair,
                    model::findPcs, model::stopDiscovery, model::selectPc)
            }
        }
    }

    override fun onResume() { super.onResume(); foreground = true; model.resume(); updateControllerWindow() }
    override fun onPause() { foreground = false; model.pause(); windowPolicy.restore(); super.onPause() }
    override fun onStop() { foreground = false; model.pause(); windowPolicy.restore(); super.onStop() }
    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (::model.isInitialized) {
            if (!hasFocus) model.cancelTouch()
            else updateControllerWindow()
        }
    }

    private fun updateControllerWindow() {
        windowPolicy.update(controllerWindowActive(model.state.value.controllerActive, surfaceRunning, false, foreground))
    }

    private fun applyControllerWindow(active: Boolean) {
        val bars = WindowCompat.getInsetsController(window, window.decorView)
        if (active) {
            window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            bars.systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
            bars.hide(WindowInsetsCompat.Type.systemBars())
        } else {
            window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            bars.systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_DEFAULT
            bars.show(WindowInsetsCompat.Type.systemBars())
        }
    }
}
