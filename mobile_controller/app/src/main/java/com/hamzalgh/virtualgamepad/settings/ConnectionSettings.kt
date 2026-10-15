package com.hamzalgh.virtualgamepad.settings

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.floatPreferencesKey
import androidx.datastore.preferences.core.intPreferencesKey
import androidx.datastore.preferences.core.longPreferencesKey
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import java.security.SecureRandom
import com.hamzalgh.virtualgamepad.controller.StickTuning

val Context.connectionDataStore by preferencesDataStore(name = "connection")

fun nonzeroRandom(random: () -> Long = SecureRandom()::nextLong): ULong {
    var value: ULong
    do { value = random().toULong() } while (value == 0uL)
    return value
}

data class ConnectionSettings(
    val address: String = "",
    val port: Int = 8765,
    val deadZone: Float = 0.15f,
    val sensitivity: Float = 1.0f,
    val clientInstance: ULong,
    val showStickClicks: Boolean = true,
    val serverInstance: ULong = 0uL,
    val serverName: String = "",
)

/** No session or nonce keys exist. The signed Long preserves all 64 identity bits. */
class SettingsRepository(private val store: DataStore<Preferences>) {
    suspend fun load(): ConnectionSettings {
        val saved = store.edit { p ->
            if (p[Keys.instance] == null || p[Keys.instance] == 0L) {
                p[Keys.instance] = nonzeroRandom().toLong()
            }
            if (p[Keys.port] == null) p[Keys.port] = 8765
            if (p[Keys.address] == null) p[Keys.address] = ""
            if (p[Keys.deadZone] == null) p[Keys.deadZone] = 0.15f
            if (p[Keys.sensitivity] == null) p[Keys.sensitivity] = 1.0f
        }
        return fromPreferences(saved)
    }

    suspend fun saveEndpoint(endpoint: Endpoint) {
        store.edit { p -> p[Keys.address] = endpoint.address; p[Keys.port] = endpoint.port }
    }

    suspend fun saveTuning(tuning: StickTuning) {
        store.edit { p ->
            p[Keys.deadZone] = tuning.deadZone
            p[Keys.sensitivity] = tuning.sensitivity
        }
    }

    suspend fun saveStickClicks(show: Boolean) {
        store.edit { it[Keys.showStickClicks] = show }
    }

    suspend fun savePairing(server: ULong, name: String) {
        require(server != 0uL)
        store.edit { p ->
            p[Keys.server] = server.toLong()
            p[Keys.serverName] = name.filter { !it.isISOControl() }.take(64)
        }
    }

    companion object {
        fun fromPreferences(p: Preferences): ConnectionSettings = ConnectionSettings(
            address = p[Keys.address].orEmpty(),
            port = p[Keys.port]?.takeIf { it in 1..65535 } ?: 8765,
            deadZone = p[Keys.deadZone]?.takeIf(StickTuning::validDeadZone) ?: 0.15f,
            sensitivity = p[Keys.sensitivity]?.takeIf(StickTuning::validSensitivity) ?: 1.0f,
            clientInstance = requireNotNull(p[Keys.instance]).toULong().also { require(it != 0uL) },
            showStickClicks = p[Keys.showStickClicks] ?: true,
            serverInstance = (p[Keys.server] ?: 0L).toULong(),
            serverName = p[Keys.serverName].orEmpty().filter { !it.isISOControl() }.take(64),
        )
    }

    internal object Keys {
        val address = stringPreferencesKey("pc_ipv4")
        val port = intPreferencesKey("pc_port")
        val deadZone = floatPreferencesKey("dead_zone")
        val sensitivity = floatPreferencesKey("sensitivity")
        val instance = longPreferencesKey("client_instance")
        val showStickClicks = booleanPreferencesKey("show_stick_clicks")
        val server = longPreferencesKey("server_instance")
        val serverName = stringPreferencesKey("server_name")
    }
}
