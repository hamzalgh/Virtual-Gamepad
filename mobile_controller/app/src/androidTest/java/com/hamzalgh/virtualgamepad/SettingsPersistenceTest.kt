package com.hamzalgh.virtualgamepad

import androidx.datastore.preferences.core.PreferenceDataStoreFactory
import androidx.test.platform.app.InstrumentationRegistry
import com.hamzalgh.virtualgamepad.settings.Endpoint
import com.hamzalgh.virtualgamepad.settings.SettingsRepository
import java.io.File
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Test

class SettingsPersistenceTest {
    @Test fun endpointAndIdentitySurviveDatastoreRecreation() = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val file = File(context.cacheDir, "settings-test-${System.nanoTime()}.preferences_pb")
        val firstJob = SupervisorJob()
        val secondJob = SupervisorJob()
        try {
            val first = SettingsRepository(PreferenceDataStoreFactory.create(
                scope = CoroutineScope(Dispatchers.IO + firstJob), produceFile = { file }))
            val initial = first.load()
            first.saveEndpoint(Endpoint.parse("192.168.1.2", "9000")!!)
            first.saveTuning(com.hamzalgh.virtualgamepad.controller.StickTuning(0.25f, 1.5f))
            first.saveStickClicks(false)
            firstJob.cancelAndJoin()
            val second = SettingsRepository(PreferenceDataStoreFactory.create(
                scope = CoroutineScope(Dispatchers.IO + secondJob), produceFile = { file }))
            val loaded = second.load()
            assertEquals(initial.clientInstance, loaded.clientInstance)
            assertEquals("192.168.1.2", loaded.address)
            assertEquals(9000, loaded.port)
            assertEquals(0.25f, loaded.deadZone)
            assertEquals(1.5f, loaded.sensitivity)
            assertEquals(false, loaded.showStickClicks)
            second.saveStickClicks(true)
            assertEquals(true, second.load().showStickClicks)
            second.saveTuning(com.hamzalgh.virtualgamepad.controller.StickTuning())
            assertEquals(0.15f, second.load().deadZone)
            assertEquals(1.0f, second.load().sensitivity)
        } finally {
            firstJob.cancelAndJoin()
            secondJob.cancelAndJoin()
            file.delete()
        }
    }
}
