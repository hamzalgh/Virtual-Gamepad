package com.hamzalgh.virtualgamepad.protocol

import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.util.zip.CRC32
import kotlin.math.roundToInt

enum class DecodeError { Length, Magic, Version, MessageType, Crc, Reserved, Range, DpadConflict }
class ProtocolException(val reason: DecodeError) : IllegalArgumentException(reason.name)

sealed interface Message
data class Hello(
    val clientNonce: ULong,
    val clientInstance: ULong,
    val stateRate: Int,
    val capabilities: Int,
    val appBuild: UInt,
    val apiLevel: Int,
) : Message

data class HelloAck(
    val clientNonce: ULong,
    val serverNonce: ULong,
    val session: ULong,
    val stateRate: Int,
    val timeoutMs: Int,
    val flags: Int,
    val capacity: Int,
    val slot: Int,
) : Message

data class PairRequest(
    val clientNonce: ULong,
    val serverNonce: ULong,
    val clientInstance: ULong,
    val code: UInt,
    val slot: Int,
) : Message

data class PairAccept(
    val session: ULong,
    val clientNonce: ULong,
    val serverInstance: ULong,
    val stateRate: Int,
    val timeoutMs: Int,
    val slot: Int,
    val capabilities: Int,
) : Message

data class PairReject(
    val clientNonce: ULong,
    val reason: Int,
    val retryAfter: Int,
) : Message

data class ControllerState(
    val session: ULong,
    val sequence: UInt,
    val clientMs: UInt,
    val buttons: UInt,
    val leftX: Short,
    val leftY: Short,
    val rightX: Short,
    val rightY: Short,
    val leftTrigger: Int,
    val rightTrigger: Int,
) : Message

data class Status(
    val session: ULong,
    val lastSequence: UInt,
    val clientMs: UInt,
    val accepted: UInt,
    val rejected: UInt,
    val lost: Int,
    val observedRate: Int,
    val flags: Int,
    val timeoutMs: Int,
) : Message

data class Disconnect(
    val session: ULong,
    val reason: Int,
) : Message

/** Pure codec. Reserved fields are never represented in domain values. */
object V1Codec {
    const val PACKET_LEN = 40
    fun encodeTrigger(value: Float): Int {
        require(value.isFinite())
        return (value.coerceIn(0f, 1f) * 255f).roundToInt()
    }
    fun encodeAxis(value: Float): Short {
        require(value.isFinite())
        val clamped = value.coerceIn(-1f, 1f)
        return (clamped * if (clamped >= 0f) 32767 else 32768).roundToInt().toShort()
    }
    private fun check(ok: Boolean, reason: DecodeError) {
        if (!ok) throw ProtocolException(reason)
    }
    fun decode(bytes: ByteArray): Message {
        check(bytes.size == PACKET_LEN, DecodeError.Length)
        check(bytes[0] == 0x56.toByte() && bytes[1] == 0x47.toByte(), DecodeError.Magic)
        check(bytes[2] == 1.toByte(), DecodeError.Version)
        val type = bytes[3].toInt() and 255
        check(type in setOf(1, 2, 3, 4, 5, 16, 17, 18), DecodeError.MessageType)
        val b = ByteBuffer.wrap(bytes).order(ByteOrder.LITTLE_ENDIAN)
        check(CRC32().apply { update(bytes, 0, 36) }.value.toUInt() == b.getInt(36).toUInt(), DecodeError.Crc)
        val reservedStart = when (type) { 1 -> 30; 3 -> 33; 4, 16 -> 34; 5 -> 14; 18 -> 13; else -> 36 }
        check((reservedStart until 36).all { bytes[it] == 0.toByte() }, DecodeError.Reserved)
        val message = when (type) {
            1 -> Hello(b.getLong(4).toULong(), b.getLong(12).toULong(), (b.getShort(20).toInt() and 65535), (b.getShort(22).toInt() and 65535), b.getInt(24).toUInt(), (b.getShort(28).toInt() and 65535))
            2 -> HelloAck(b.getLong(4).toULong(), b.getLong(12).toULong(), b.getLong(20).toULong(), (b.getShort(28).toInt() and 65535), (b.getShort(30).toInt() and 65535), (b.getShort(32).toInt() and 65535), (b.get(34).toInt() and 255), (b.get(35).toInt() and 255))
            3 -> PairRequest(b.getLong(4).toULong(), b.getLong(12).toULong(), b.getLong(20).toULong(), b.getInt(28).toUInt(), (b.get(32).toInt() and 255))
            4 -> PairAccept(b.getLong(4).toULong(), b.getLong(12).toULong(), b.getLong(20).toULong(), (b.getShort(28).toInt() and 65535), (b.getShort(30).toInt() and 65535), (b.get(32).toInt() and 255), (b.get(33).toInt() and 255))
            5 -> PairReject(b.getLong(4).toULong(), (b.get(12).toInt() and 255), (b.get(13).toInt() and 255))
            16 -> ControllerState(b.getLong(4).toULong(), b.getInt(12).toUInt(), b.getInt(16).toUInt(), b.getInt(20).toUInt(), b.getShort(24), b.getShort(26), b.getShort(28), b.getShort(30), (b.get(32).toInt() and 255), (b.get(33).toInt() and 255))
            17 -> Status(b.getLong(4).toULong(), b.getInt(12).toUInt(), b.getInt(16).toUInt(), b.getInt(20).toUInt(), b.getInt(24).toUInt(), (b.getShort(28).toInt() and 65535), (b.getShort(30).toInt() and 65535), (b.getShort(32).toInt() and 65535), (b.getShort(34).toInt() and 65535))
            18 -> Disconnect(b.getLong(4).toULong(), (b.get(12).toInt() and 255))
            else -> throw ProtocolException(DecodeError.MessageType)
        }
        validate(message)
        return message
    }

    fun encode(message: Message): ByteArray {
        validate(message)
        val bytes = ByteArray(PACKET_LEN)
        val b = ByteBuffer.wrap(bytes).order(ByteOrder.LITTLE_ENDIAN)
        bytes[0] = 0x56; bytes[1] = 0x47; bytes[2] = 1
        when (message) {
            is Hello -> {
                bytes[3] = 1
                b.putLong(4, message.clientNonce.toLong())
                b.putLong(12, message.clientInstance.toLong())
                b.putShort(20, message.stateRate.toShort())
                b.putShort(22, message.capabilities.toShort())
                b.putInt(24, message.appBuild.toInt())
                b.putShort(28, message.apiLevel.toShort())
            }
            is HelloAck -> {
                bytes[3] = 2
                b.putLong(4, message.clientNonce.toLong())
                b.putLong(12, message.serverNonce.toLong())
                b.putLong(20, message.session.toLong())
                b.putShort(28, message.stateRate.toShort())
                b.putShort(30, message.timeoutMs.toShort())
                b.putShort(32, message.flags.toShort())
                b.put(34, message.capacity.toByte())
                b.put(35, message.slot.toByte())
            }
            is PairRequest -> {
                bytes[3] = 3
                b.putLong(4, message.clientNonce.toLong())
                b.putLong(12, message.serverNonce.toLong())
                b.putLong(20, message.clientInstance.toLong())
                b.putInt(28, message.code.toInt())
                b.put(32, message.slot.toByte())
            }
            is PairAccept -> {
                bytes[3] = 4
                b.putLong(4, message.session.toLong())
                b.putLong(12, message.clientNonce.toLong())
                b.putLong(20, message.serverInstance.toLong())
                b.putShort(28, message.stateRate.toShort())
                b.putShort(30, message.timeoutMs.toShort())
                b.put(32, message.slot.toByte())
                b.put(33, message.capabilities.toByte())
            }
            is PairReject -> {
                bytes[3] = 5
                b.putLong(4, message.clientNonce.toLong())
                b.put(12, message.reason.toByte())
                b.put(13, message.retryAfter.toByte())
            }
            is ControllerState -> {
                bytes[3] = 16
                b.putLong(4, message.session.toLong())
                b.putInt(12, message.sequence.toInt())
                b.putInt(16, message.clientMs.toInt())
                b.putInt(20, message.buttons.toInt())
                b.putShort(24, message.leftX)
                b.putShort(26, message.leftY)
                b.putShort(28, message.rightX)
                b.putShort(30, message.rightY)
                b.put(32, message.leftTrigger.toByte())
                b.put(33, message.rightTrigger.toByte())
            }
            is Status -> {
                bytes[3] = 17
                b.putLong(4, message.session.toLong())
                b.putInt(12, message.lastSequence.toInt())
                b.putInt(16, message.clientMs.toInt())
                b.putInt(20, message.accepted.toInt())
                b.putInt(24, message.rejected.toInt())
                b.putShort(28, message.lost.toShort())
                b.putShort(30, message.observedRate.toShort())
                b.putShort(32, message.flags.toShort())
                b.putShort(34, message.timeoutMs.toShort())
            }
            is Disconnect -> {
                bytes[3] = 18
                b.putLong(4, message.session.toLong())
                b.put(12, message.reason.toByte())
            }
        }
        b.putInt(36, CRC32().apply { update(bytes, 0, 36) }.value.toInt())
        return bytes
    }

    private fun validate(m: Message) {
        fun range(ok: Boolean) = check(ok, DecodeError.Range)
        fun bits(value: Int, mask: Int) = check(value and mask.inv() == 0, DecodeError.Reserved)
        when (m) {
            is Hello -> {
                range(m.stateRate in 0..65535)
                range(m.capabilities in 0..65535)
                range(m.apiLevel in 0..65535)
                range(m.clientNonce != 0uL)
                range(m.clientInstance != 0uL)
                bits(m.capabilities, 7)
            }
            is HelloAck -> {
                range(m.stateRate in 0..65535)
                range(m.timeoutMs in 0..65535)
                range(m.flags in 0..65535)
                range(m.capacity in 0..255)
                range(m.slot in 0..255)
                range(m.clientNonce != 0uL)
                range(m.serverNonce != 0uL)
                bits(m.flags, 15)
                range(m.capacity in 1..4 && if (m.session == 0uL) m.slot == 255 else m.slot < m.capacity)
                range(m.timeoutMs in 100..1000)
            }
            is PairRequest -> {
                range(m.slot in 0..255)
                range(m.clientNonce != 0uL)
                range(m.serverNonce != 0uL)
                range(m.clientInstance != 0uL)
                range(m.code <= 999999u && m.slot == 255)
            }
            is PairAccept -> {
                range(m.stateRate in 0..65535)
                range(m.timeoutMs in 0..65535)
                range(m.slot in 0..255)
                range(m.capabilities in 0..255)
                range(m.session != 0uL)
                range(m.clientNonce != 0uL)
                range(m.serverInstance != 0uL)
                bits(m.capabilities, 7)
                range(m.slot == 0 && m.timeoutMs in 100..1000)
            }
            is PairReject -> {
                range(m.reason in 0..255)
                range(m.retryAfter in 0..255)
                range(m.reason in 0..6)
            }
            is ControllerState -> {
                range(m.leftTrigger in 0..255)
                range(m.rightTrigger in 0..255)
                range(m.session != 0uL)
                bits(m.buttons.toInt(), 0x3fff)
                check(m.buttons and 0xc00u != 0xc00u && m.buttons and 0x3000u != 0x3000u, DecodeError.DpadConflict)
            }
            is Status -> {
                range(m.lost in 0..65535)
                range(m.observedRate in 0..65535)
                range(m.flags in 0..65535)
                range(m.timeoutMs in 0..65535)
                bits(m.flags, 63)
                range(m.timeoutMs in 100..1000)
            }
            is Disconnect -> {
                range(m.reason in 0..255)
                range(m.session != 0uL)
                range(m.reason in 0..4)
            }
        }
    }
}
