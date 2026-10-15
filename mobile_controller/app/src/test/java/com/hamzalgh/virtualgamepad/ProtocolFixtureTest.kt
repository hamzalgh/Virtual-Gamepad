package com.hamzalgh.virtualgamepad

import com.hamzalgh.virtualgamepad.protocol.*
import java.io.File
import org.junit.Assert.*
import org.junit.Test

class ProtocolFixtureTest {
    private val directory = File("../../protocol/fixtures/v1")

    @Test fun sharedManifestFieldsAndBytesMatchInBothDirections() {
        var valid = 0
        var invalid = 0
        directory.resolve("manifest.txt").readLines().filter { it.isNotBlank() && !it.startsWith("#") }.forEach { line ->
            val (file, result, type, fields) = line.split('|')
            val bytes = directory.resolve(file).readBytes()
            if (result == "Valid") {
                val values = fields.split(' ').associate { it.substringBefore('=') to it.substringAfter('=') }
                fun v(key: String) = values.getValue(key)
                val expected: Message = when (type) {
                    "Hello" -> Hello(v("client_nonce").toULong(), v("client_instance").toULong(), v("state_rate").toInt(), v("capabilities").toInt(), v("app_build").toUInt(), v("api_level").toInt())
                    "HelloAck" -> HelloAck(v("client_nonce").toULong(), v("server_nonce").toULong(), v("session").toULong(), v("state_rate").toInt(), v("timeout_ms").toInt(), v("flags").toInt(), v("capacity").toInt(), v("slot").toInt())
                    "PairRequest" -> PairRequest(v("client_nonce").toULong(), v("server_nonce").toULong(), v("client_instance").toULong(), v("code").toUInt(), v("slot").toInt())
                    "PairAccept" -> PairAccept(v("session").toULong(), v("client_nonce").toULong(), v("server_instance").toULong(), v("state_rate").toInt(), v("timeout_ms").toInt(), v("slot").toInt(), v("capabilities").toInt())
                    "PairReject" -> PairReject(v("client_nonce").toULong(), v("reason").toInt(), v("retry_after").toInt())
                    "ControllerState" -> ControllerState(v("session").toULong(), v("sequence").toUInt(), v("client_ms").toUInt(), v("buttons").toUInt(), v("left_x").toShort(), v("left_y").toShort(), v("right_x").toShort(), v("right_y").toShort(), v("left_trigger").toInt(), v("right_trigger").toInt())
                    "Status" -> Status(v("session").toULong(), v("last_sequence").toUInt(), v("client_ms").toUInt(), v("accepted").toUInt(), v("rejected").toUInt(), v("lost").toInt(), v("observed_rate").toInt(), v("flags").toInt(), v("timeout_ms").toInt())
                    "Disconnect" -> Disconnect(v("session").toULong(), v("reason").toInt())
                    else -> error(type)
                }
                assertEquals(file, expected, V1Codec.decode(bytes))
                assertArrayEquals(file, bytes, V1Codec.encode(expected))
                valid++
            } else {
                val error = assertThrows(file, ProtocolException::class.java) { V1Codec.decode(bytes) }
                assertEquals(file, result, error.reason.name)
                invalid++
            }
        }
        assertEquals(30, valid)
        assertEquals(28, invalid)
    }

    @Test fun callerRangesAndIdentifiersAreNeverTruncated() {
        val hello = Hello(1uL, 2uL, 60, 1, 1u, 34)
        listOf(hello.copy(clientNonce = 0uL), hello.copy(clientInstance = 0uL),
            hello.copy(stateRate = -1), hello.copy(apiLevel = 65536),
            hello.copy(capabilities = 8)).forEach {
            assertThrows(ProtocolException::class.java) { V1Codec.encode(it) }
        }
        val random = java.util.Random(3)
        repeat(10000) {
            val bytes = ByteArray(random.nextInt(100))
            random.nextBytes(bytes)
            try { V1Codec.decode(bytes) } catch (_: ProtocolException) { }
        }
    }
}

