package com.hamzalgh.virtualgamepad

import org.junit.Assert.assertEquals
import org.junit.Test

class FoundationSmokeTest {
    @Test
    fun testHarnessRuns() {
        assertEquals("Virtual Gamepad", listOf("Virtual", "Gamepad").joinToString(" "))
    }
}
