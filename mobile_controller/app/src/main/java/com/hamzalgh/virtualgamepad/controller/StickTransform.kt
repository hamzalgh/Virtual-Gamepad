package com.hamzalgh.virtualgamepad.controller

import kotlin.math.hypot
import kotlin.math.pow

data class StickTuning(val deadZone: Float = 0.15f, val sensitivity: Float = 1f) {
    init {
        require(validDeadZone(deadZone))
        require(validSensitivity(sensitivity))
    }

    companion object {
        fun validDeadZone(value: Float) = value.isFinite() && value in 0f..0.30f
        fun validSensitivity(value: Float) = value.isFinite() && value in 0.5f..2f
    }
}

data class StickVector(val x: Float = 0f, val y: Float = 0f)

/** Double intermediates keep finite Float coordinates safe even at their extremes. */
fun stickDisplacement(x: Float, y: Float, gate: Circle): StickVector {
    if (!x.isFinite() || !y.isFinite()) return StickVector()
    val dx = x.toDouble() - gate.x
    val dy = y.toDouble() - gate.y
    val divisor = maxOf(gate.radius.toDouble(), hypot(dx, dy))
    return StickVector((dx / divisor).toFloat(), (-dy / divisor).toFloat())
}

/** Radial dead zone, rescale, then magnitude^(1/sensitivity), preserving direction. */
fun transformStick(raw: StickVector, tuning: StickTuning): StickVector {
    val magnitude = hypot(raw.x.toDouble(), raw.y.toDouble())
    if (!magnitude.isFinite() || magnitude <= tuning.deadZone) return StickVector()
    val rescaled = ((magnitude.coerceAtMost(1.0) - tuning.deadZone) / (1.0 - tuning.deadZone))
        .coerceIn(0.0, 1.0).pow(1.0 / tuning.sensitivity)
    // Canonical zero also keeps immutable state equality and overlay output stable.
    fun component(value: Double): Float = value.toFloat().coerceIn(-1f, 1f).let { if (it == 0f) 0f else it }
    return StickVector(
        component(raw.x / magnitude * rescaled),
        component(raw.y / magnitude * rescaled),
    )
}
