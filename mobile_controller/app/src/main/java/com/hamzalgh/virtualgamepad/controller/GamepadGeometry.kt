package com.hamzalgh.virtualgamepad.controller

import kotlin.math.hypot

sealed interface CaptureRegion {
    val left: Float
    val top: Float
    val right: Float
    val bottom: Float
    fun contains(x: Float, y: Float): Boolean
    fun inside(width: Float, height: Float) = left >= 0f && top >= 0f && right <= width && bottom <= height
}

data class Circle(val x: Float, val y: Float, val radius: Float) : CaptureRegion {
    init { require(x.isFinite() && y.isFinite() && radius.isFinite() && radius > 0f) }
    override val left get() = x - radius
    override val top get() = y - radius
    override val right get() = x + radius
    override val bottom get() = y + radius
    override fun contains(x: Float, y: Float) = hypot(x.toDouble() - this.x, y.toDouble() - this.y) <= radius
}

data class RailRect(override val left: Float, override val top: Float, override val right: Float, override val bottom: Float) : CaptureRegion {
    init { require(listOf(left, top, right, bottom).all { it.isFinite() } && right > left && bottom > top) }
    override fun contains(x: Float, y: Float) = x in left..right && y in top..bottom
    fun value(x: Float, inwardRight: Boolean): Float {
        if (!x.isFinite()) return 0f
        val value = ((x.toDouble() - left) / (right.toDouble() - left)).coerceIn(0.0, 1.0).toFloat()
        return if (inwardRight) value else 1f - value
    }
}

/** Exact closest-boundary distance, also used to reject unsupported layouts. */
fun regionGap(a: CaptureRegion, b: CaptureRegion): Double = when {
    a is Circle && b is Circle -> hypot(a.x.toDouble() - b.x, a.y.toDouble() - b.y) - a.radius - b.radius
    a is Circle -> hypot(a.x.toDouble() - a.x.coerceIn(b.left, b.right), a.y.toDouble() - a.y.coerceIn(b.top, b.bottom)) - a.radius
    b is Circle -> regionGap(b, a)
    else -> hypot(maxOf(0f, a.left - b.right, b.left - a.right).toDouble(), maxOf(0f, a.top - b.bottom, b.top - a.bottom).toDouble())
}

class GamepadGeometry private constructor(
    val profile: GeometryProfile,
    val stick: Circle,
    val rightStick: Circle,
    val dpad: Circle,
    val lt: RailRect,
    val rt: RailRect,
    val navbar: RailRect,
    val gear: RailRect,
    val exit: RailRect,
    val regions: Map<Control, CaptureRegion>,
) {
    val stickCapture get() = regions.getValue(Control.LeftStick) as Circle
    val a get() = regions.getValue(Control.A) as Circle
    val aCapture get() = a
    val stickTravel get() = travel(stick)
    val rightStickTravel get() = travel(rightStick)
    fun stickKnob(displacement: StickVector) = knob(stick, displacement)
    fun rightStickKnob(displacement: StickVector) = knob(rightStick, displacement)

    companion object {
        private fun travel(gate: Circle) = gate.copy(radius = gate.radius * (1f - 0.32f))
        private fun knob(gate: Circle, value: StickVector) = Circle(
            gate.x + value.x * travel(gate).radius, gate.y - value.y * travel(gate).radius, gate.radius * 0.32f,
        )

        fun fit(width: Float, height: Float, density: Float, showStickClicks: Boolean = true): GamepadGeometry? {
            if (!width.isFinite() || !height.isFinite() || !density.isFinite() || density <= 0f) return null
            val usableW = width / density
            val usableH = height / density
            if (usableW < usableH) return null
            val profile = if (usableW < 720f || usableH < 340f) GeometryProfile.Compact else GeometryProfile.Regular
            val expanded = usableW >= 640f && usableH >= 320f
            val scale = if (expanded) minOf(usableW / 640f, usableH / 320f, 1.25f) else 1f
            val unit = density * scale
            val w = width / unit
            val h = height / unit
            if (!w.isFinite() || !h.isFinite() || w < 640f || h < 264f ||
                w - 144f == w || h - 184f == h) return null
            // Fractional dp scaling can put an edge one Float ULP outside the window.
            // Snap only subpixel rounding; genuine out-of-bounds geometry is still rejected.
            fun snapCenter(value: Float, radius: Float, limit: Float): Float = when {
                value - radius < 0f && value - radius >= -0.001f * density -> radius
                value + radius > limit && value + radius - limit <= 0.001f * density -> Math.nextDown(limit - radius)
                else -> value
            }
            fun circle(x: Float, y: Float, r: Float): Circle {
                val radius = r * unit
                return Circle(snapCenter(x * unit, radius, width), snapCenter(y * unit, radius, height), radius)
            }
            fun rect(x: Float, y: Float, rw: Float, rh: Float) = RailRect(
                x * unit, y * unit,
                ((x + rw) * unit).let { if (it > width && it - width <= 0.001f * density) width else it },
                ((y + rh) * unit).let { if (it > height && it - height <= 0.001f * density) height else it },
            )
            val inner = w * 0.36f
            val leftCapture = if (expanded) circle(72f, h - 96f, 72f) else circle(64f, h - 80f, 64f)
            val rightCapture = if (expanded) circle(w * 0.66f, h - 72f, 72f) else circle(w - inner, h - 64f, 64f)
            val stick = leftCapture.copy(radius = (if (expanded) 64f else 56f) * unit)
            val rightStick = rightCapture.copy(radius = stick.radius)
            val dpad = if (expanded) circle(inner, h - 72f, 72f) else circle(inner, h - 76f, 76f)
            val lt = if (expanded) rect(0f, 0f, 88f, 72f) else rect(0f, 0f, 144f, 48f)
            val rt = if (expanded) rect(w - 88f, 0f, 88f, 72f) else rect(w - 144f, 0f, 144f, 48f)
            fun nav(dx: Float, navWidth: Float) = RailRect(width / 2 + dx * density, 0f,
                width / 2 + (dx + navWidth) * density, 48f * density)
            val navbar = nav(-152f, 304f)
            val gear = nav(-144f, 48f)
            val exit = nav(96f, 48f)
            val regions = linkedMapOf<Control, CaptureRegion>(
                Control.LeftStick to leftCapture,
                Control.RightStick to rightCapture,
                Control.Dpad to dpad, Control.LT to lt, Control.RT to rt,
                Control.LB to if (expanded) rect(124f, 64f, 72f, 64f) else rect(32f, 56f, 112f, 48f),
                Control.RB to if (expanded) rect(w - 196f, 64f, 72f, 64f) else rect(w - 144f, 56f, 112f, 48f),
                Control.A to if (expanded) circle(w - 84f, h - 100f, 32f) else circle(w - 76f, h - 28f, 28f),
                Control.B to if (expanded) circle(w - 32f, h - 152f, 32f) else circle(w - 28f, h - 76f, 28f),
                Control.X to if (expanded) circle(w - 136f, h - 152f, 32f) else circle(w - 124f, h - 76f, 28f),
                Control.Y to if (expanded) circle(w - 84f, h - 204f, 32f) else circle(w - 76f, h - 124f, 28f),
                Control.L3 to if (expanded) circle(240f, 96f, 24f) else circle(184f, h - 184f, 24f),
                Control.R3 to if (expanded) circle(w - 240f, 96f, 24f) else circle(w - inner - 24f, h - 164f, 24f),
                Control.Back to nav(-72f, 64f),
                Control.Start to nav(8f, 64f),
            )
            if (!showStickClicks) { regions.remove(Control.L3); regions.remove(Control.R3) }
            val captures = regions.values.toList() + listOf(gear, exit)
            if (captures.any { !it.inside(width, height) }) return null
            for (i in captures.indices) for (j in 0 until i) {
                if (regionGap(captures[i], captures[j]) < 8f * density - 0.001) return null
            }
            return GamepadGeometry(profile, stick, rightStick, dpad, lt, rt, navbar, gear, exit, regions.toMap())
        }
    }
}

enum class GeometryProfile { Compact, Regular }
