package com.hamzalgh.virtualgamepad

import androidx.datastore.preferences.core.edit
import com.hamzalgh.virtualgamepad.settings.*
import kotlinx.coroutines.test.runTest
import org.junit.Assert.*
import org.junit.Test

class SettingsTest {
    @Test fun pairingMetadataPersistsAtomicallyAndFailurePreservesPriorIdentity() = runTest {
        val store = MemoryPreferences()
        val repository = SettingsRepository(store)
        val initial = repository.load()
        val writes = store.writes
        repository.savePairing(ULong.MAX_VALUE, "My PC\n")
        assertEquals(writes + 1, store.writes)
        val paired = SettingsRepository(store).load()
        assertEquals(ULong.MAX_VALUE, paired.serverInstance)
        assertEquals("My PC", paired.serverName)
        assertEquals(initial.clientInstance, paired.clientInstance)
        store.failNext = true
        try { repository.savePairing(1uL, "Other PC"); fail("Expected storage failure") }
        catch (_: java.io.IOException) { assertEquals(paired, repository.load()) }
    }
    @Test fun stickClickVisibilityPersistsAndFailedSaveCanBeRetried() = runTest {
        val store = MemoryPreferences()
        val repository = SettingsRepository(store)
        val initial = repository.load()
        assertTrue(initial.showStickClicks)
        repository.saveStickClicks(false)
        assertFalse(SettingsRepository(store).load().showStickClicks)
        store.failNext = true
        try {
            repository.saveStickClicks(true)
            fail("Expected storage failure")
        } catch (_: java.io.IOException) {
            assertFalse(repository.load().showStickClicks)
        }
        repository.saveStickClicks(true)
        assertEquals(initial, repository.load())
    }

    @Test fun tuningSavesAsOnePairAndResetPreservesIdentityAndEndpoint() = runTest {
        val store = MemoryPreferences()
        val repository = SettingsRepository(store)
        val initial = repository.load()
        repository.saveEndpoint(Endpoint.parse("192.168.1.2", "9000")!!)
        val beforeSave = store.writes
        repository.saveTuning(com.hamzalgh.virtualgamepad.controller.StickTuning(0.3f, 2f))
        assertEquals(beforeSave + 1, store.writes)
        val saved = SettingsRepository(store).load()
        assertEquals(0.3f, saved.deadZone)
        assertEquals(2f, saved.sensitivity)
        repository.saveTuning(com.hamzalgh.virtualgamepad.controller.StickTuning())
        val reset = SettingsRepository(store).load()
        assertEquals(0.15f, reset.deadZone)
        assertEquals(1f, reset.sensitivity)
        assertEquals(initial.clientInstance, reset.clientInstance)
        assertEquals("192.168.1.2", reset.address)
        assertEquals(9000, reset.port)
    }

    @Test fun failedTuningTransactionRetainsThePreviousPair() = runTest {
        val store = MemoryPreferences()
        val repository = SettingsRepository(store)
        repository.load()
        store.failNext = true
        try {
            repository.saveTuning(com.hamzalgh.virtualgamepad.controller.StickTuning(0.3f, 2f))
            fail("Expected a storage failure")
        } catch (_: java.io.IOException) {
            val saved = repository.load()
            assertEquals(0.15f, saved.deadZone)
            assertEquals(1f, saved.sensitivity)
        }
    }

    @Test fun validatesPrivateLocalEndpointsAndPortsWithoutDns() {
        listOf("10.0.0.1", "172.16.0.1", "172.31.255.254", "192.168.1.1", "169.254.1.2", "127.0.0.1").forEach {
            assertNotNull(Endpoint.parse(it, "1")); assertNotNull(Endpoint.parse(it, "65535"))
        }
        listOf("", "localhost", "::1", "8.8.8.8", "0.0.0.0", "224.0.0.1", "255.255.255.255", "172.32.0.1",
            "192.168.1", "192.168.1.256", "192.168.01.2", "192.168.+1.2").forEach { assertNull(it, Endpoint.parse(it, "8765")) }
        listOf("0", "65536", "-1", "+1", "1.0", "", "abc").forEach { assertNull(Endpoint.parse("10.0.0.1", it)) }
    }

    @Test fun randomIdsSkipZeroAndPreserveUnsignedBits() {
        val values = ArrayDeque(listOf(0L, 0L, -1L))
        assertEquals(ULong.MAX_VALUE, nonzeroRandom { values.removeFirst() })
        assertNotEquals(0uL, nonzeroRandom())
    }

    @Test fun persistenceMappingRetainsEndpointDefaultsAndUnsignedIdentity() = runTest {
        val store = MemoryPreferences()
        val repository = SettingsRepository(store)
        val initial = repository.load()
        assertEquals(0.15f, initial.deadZone)
        assertEquals(1.0f, initial.sensitivity)
        assertEquals(8765, initial.port)
        repository.saveEndpoint(Endpoint.parse("192.168.1.2", "9000")!!)
        val saved = SettingsRepository(store).load()
        assertEquals(initial.clientInstance, saved.clientInstance)
        assertEquals("192.168.1.2", saved.address)
        assertEquals(9000, saved.port)
        store.edit { p ->
            p[SettingsRepository.Keys.instance] = -1L
            p[SettingsRepository.Keys.deadZone] = Float.NaN
            p[SettingsRepository.Keys.port] = 0
        }
        val repaired = repository.load()
        assertEquals(ULong.MAX_VALUE, repaired.clientInstance)
        assertEquals(0.15f, repaired.deadZone)
        assertEquals(8765, repaired.port)
    }

    // Windows JVM File.renameTo cannot replace the existing DataStore file.
    @Test fun savedTuningUsesValidatedRangesAndDefaults() = runTest {
        val store = MemoryPreferences()
        val repository = SettingsRepository(store)
        repository.load()
        for ((d, s) in listOf(0f to 0.5f, 0.30f to 2f)) {
            store.edit { p -> p[SettingsRepository.Keys.deadZone] = d; p[SettingsRepository.Keys.sensitivity] = s }
            assertEquals(d, repository.load().deadZone)
            assertEquals(s, repository.load().sensitivity)
        }
        for ((d, s) in listOf(-1f to 0f, 0.31f to 2.1f, Float.NaN to Float.POSITIVE_INFINITY)) {
            store.edit { p -> p[SettingsRepository.Keys.deadZone] = d; p[SettingsRepository.Keys.sensitivity] = s }
            assertEquals(0.15f, repository.load().deadZone)
            assertEquals(1f, repository.load().sensitivity)
        }
    }

    // Actual disk recreation coverage lives in SettingsPersistenceTest on Android.
    private class MemoryPreferences : androidx.datastore.core.DataStore<androidx.datastore.preferences.core.Preferences> {
        var writes = 0
        var failNext = false
        private val current = kotlinx.coroutines.flow.MutableStateFlow(androidx.datastore.preferences.core.emptyPreferences())
        override val data: kotlinx.coroutines.flow.Flow<androidx.datastore.preferences.core.Preferences> = current
        override suspend fun updateData(transform: suspend (androidx.datastore.preferences.core.Preferences) -> androidx.datastore.preferences.core.Preferences): androidx.datastore.preferences.core.Preferences {
            val next = transform(current.value)
            if (failNext) { failNext = false; throw java.io.IOException("Test storage failure") }
            writes++
            return next.also { current.value = it }
        }
    }
}
