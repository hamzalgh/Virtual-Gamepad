package com.hamzalgh.virtualgamepad.app

import android.app.Application
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.content.ContextCompat
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.hamzalgh.virtualgamepad.network.ConnectionPhase
import com.hamzalgh.virtualgamepad.network.ConnectionState
import com.hamzalgh.virtualgamepad.network.HelloSession
import com.hamzalgh.virtualgamepad.network.PcDiscovery
import com.hamzalgh.virtualgamepad.network.DiscoveryState
import com.hamzalgh.virtualgamepad.network.DiscoveredPc
import com.hamzalgh.virtualgamepad.network.permissionState
import com.hamzalgh.virtualgamepad.settings.Endpoint
import com.hamzalgh.virtualgamepad.settings.SettingsRepository
import com.hamzalgh.virtualgamepad.settings.connectionDataStore
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.IOException
import com.hamzalgh.virtualgamepad.controller.ControllerState
import com.hamzalgh.virtualgamepad.controller.StickTuning

const val LOCAL_NETWORK_PERMISSION = "android.permission.ACCESS_LOCAL_NETWORK"

data class SetupState(
    val address: String = "",
    val port: String = "8765",
    val loaded: Boolean = false,
    val permissionGranted: Boolean = false,
    val connection: ConnectionState = ConnectionState(),
    val endpointError: Boolean = false,
    val controllerActive: Boolean = false,
    val controller: ControllerState = ControllerState.NEUTRAL,
    val inputEpoch: Int = 0,
    val tuning: StickTuning = StickTuning(),
    val tuningSaving: Boolean = false,
    val tuningError: String? = null,
    val showStickClicks: Boolean = true,
    val appearanceSaving: Boolean = false,
    val appearanceError: String? = null,
    val pairingCode: String = "",
    val pcName: String = "",
    val discovery: DiscoveryState = DiscoveryState(),
)

fun SetupState.withTuning(value: StickTuning) = copy(
    tuning = value, controller = ControllerState.NEUTRAL, inputEpoch = inputEpoch + 1,
    tuningSaving = false, tuningError = null,
)

class ConnectionViewModel(application: Application) : AndroidViewModel(application) {
    private val settings = SettingsRepository(application.connectionDataStore)
    private val mutableState = MutableStateFlow(SetupState())
    val state = mutableState.asStateFlow()
    private var instance = 0uL
    private var work: Job? = null
    private var session: HelloSession? = null
    private var foreground = false
    private val discovery = PcDiscovery(application) { value ->
        mutableState.value = mutableState.value.copy(discovery = value)
    }

    fun findPcs() {
        if (foreground && state.value.loaded && hasPermission()) {
            disconnect()
            discovery.start()
        }
    }

    fun stopDiscovery() = discovery.stop()

    fun selectPc(pc: DiscoveredPc) {
        disconnect()
        mutableState.value = mutableState.value.copy(address = pc.endpoint.address,
            port = pc.endpoint.port.toString(), pcName = pc.name, endpointError = false)
    }

    init { loadSettings() }

    fun hasPermission(): Boolean = Build.VERSION.SDK_INT < 37 ||
        ContextCompat.checkSelfPermission(getApplication(), LOCAL_NETWORK_PERMISSION) == PackageManager.PERMISSION_GRANTED

    fun loadSettings() {
        if (work?.isActive == true) return
        work = viewModelScope.launch {
            try {
                val saved = settings.load()
                instance = saved.clientInstance
                mutableState.value = mutableState.value.copy(
                    address = saved.address, port = saved.port.toString(), loaded = true,
                    permissionGranted = hasPermission(), connection = ConnectionState(),
                    tuning = StickTuning(saved.deadZone, saved.sensitivity),
                    showStickClicks = saved.showStickClicks,
                    pcName = saved.serverName,
                )
            } catch (_: IOException) {
                mutableState.value = mutableState.value.copy(connection = ConnectionState(
                    ConnectionPhase.Error, detail = "Could not read or save settings. Retry settings before connecting.",
                ))
            }
        }
    }

    fun resume() {
        foreground = true
        permissionChanged(hasPermission())
    }

    fun pause() {
        foreground = false
        disconnect(1)
    }

    fun permissionChanged(granted: Boolean) {
        if (!granted) disconnect()
        val previous = mutableState.value
        mutableState.value = previous.copy(
            permissionGranted = granted,
            connection = permissionState(previous.connection, granted),
        )
    }

    fun editAddress(value: String) {
        disconnect()
        mutableState.value = mutableState.value.copy(address = value.take(64), pcName = "", endpointError = false)
    }

    fun editPort(value: String) {
        disconnect()
        mutableState.value = mutableState.value.copy(port = value.take(8), pcName = "", endpointError = false)
    }

    fun connect() {
        if (!foreground || !mutableState.value.loaded) return
        if (!hasPermission()) { permissionChanged(false); return }
        val endpoint = Endpoint.parse(state.value.address, state.value.port)
        if (endpoint == null) {
            mutableState.value = mutableState.value.copy(endpointError = true)
            return
        }
        val previousWork = work
        disconnect()
        val owner = HelloSession()
        session = owner
        work = viewModelScope.launch {
            previousWork?.join()
            try {
                try {
                    settings.saveEndpoint(endpoint)
                } catch (_: IOException) {
                    mutableState.value = mutableState.value.copy(connection = ConnectionState(
                        ConnectionPhase.Error, detail = "Could not save the PC endpoint. Retry after checking device storage.",
                    ))
                    return@launch
                }
                withContext(Dispatchers.IO) {
                    owner.run(endpoint, instance, Build.VERSION.SDK_INT,
                        onPaired = { server -> settings.savePairing(server, state.value.pcName.ifBlank { endpoint.address }) }) { connection ->
                        withContext(Dispatchers.Main) {
                            if (session === owner && foreground) {
                                val active = connection.allowsController()
                                mutableState.value = mutableState.value.copy(connection = connection, endpointError = false,
                                    controllerActive = active, controller = if (active) mutableState.value.controller else ControllerState.NEUTRAL)
                            }
                        }
                    }
                }
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (_: SecurityException) {
                permissionChanged(false)
            } catch (_: IOException) {
                if (!hasPermission()) permissionChanged(false)
                else mutableState.value = mutableState.value.copy(connection = ConnectionState(
                    ConnectionPhase.Error,
                    detail = "Could not exchange UDP packets. Check local network permission, Wi-Fi and the PC endpoint, then retry.",
                ), controllerActive = false, controller = ControllerState.NEUTRAL)
            } finally {
                owner.close()
                if (session === owner) session = null
            }
        }
    }

    fun setController(value: ControllerState) {
        val local = if (foreground && mutableState.value.controllerActive)
            value.withVisibleStickClicks(mutableState.value.showStickClicks) else ControllerState.NEUTRAL
        mutableState.value = mutableState.value.copy(controller = local)
        session?.updateController(local)
    }

    fun editPairingCode(value: String) {
        mutableState.value = mutableState.value.copy(pairingCode = value.filter { it in '0'..'9' }.take(6))
    }

    fun pair() {
        val current = state.value
        if (!foreground || current.connection.phase != ConnectionPhase.PairingNeeded || current.pairingCode.length != 6) return
        session?.pair(current.pairingCode)
        mutableState.value = current.copy(pairingCode = "")
    }

    fun cancelTouch() {
        setController(ControllerState.NEUTRAL)
        mutableState.value = mutableState.value.copy(inputEpoch = mutableState.value.inputEpoch + 1)
    }

    fun saveTuning(value: StickTuning) {
        if (!state.value.loaded || state.value.tuningSaving) return
        cancelTouch()
        mutableState.value = mutableState.value.copy(tuningSaving = true, tuningError = null)
        // Independent of the network job: endpoint/session lifecycle cannot half-save tuning.
        viewModelScope.launch {
            try {
                settings.saveTuning(value)
                setController(ControllerState.NEUTRAL)
                mutableState.value = mutableState.value.withTuning(value)
            } catch (_: IOException) {
                mutableState.value = mutableState.value.copy(tuningSaving = false,
                    tuningError = "Could not save tuning. Check device storage and apply again.")
            }
        }
    }

    fun showStickClicks(show: Boolean) {
        if (!state.value.loaded || state.value.appearanceSaving) return
        cancelTouch()
        mutableState.value = mutableState.value.copy(appearanceSaving = true, appearanceError = null)
        viewModelScope.launch {
            try {
                settings.saveStickClicks(show)
                cancelTouch()
                mutableState.value = mutableState.value.copy(showStickClicks = show, appearanceSaving = false)
            } catch (_: IOException) {
                mutableState.value = mutableState.value.copy(appearanceSaving = false,
                    appearanceError = "Could not save button visibility. Try the switch again.")
            }
        }
    }

    fun disconnect() = disconnect(0)

    private fun disconnect(reason: Int) {
        discovery.stop()
        session?.close(reason)
        session = null
        // Settings initialization is independent of foreground network ownership.
        if (mutableState.value.loaded) work?.cancel()
        mutableState.value = mutableState.value.copy(connection = ConnectionState(
            detail = "Session closed. Connect to negotiate again.",
        ), controllerActive = false, controller = ControllerState.NEUTRAL, pairingCode = "")
    }

    override fun onCleared() {
        discovery.stop()
        session?.close(4)
        super.onCleared()
    }
}
