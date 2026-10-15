package com.hamzalgh.virtualgamepad.controller

import kotlin.math.atan2
import kotlin.math.hypot
import kotlin.math.roundToInt

enum class Control(val button: Button? = null) {
    LeftStick, RightStick, Dpad, LT, RT,
    A(Button.A), B(Button.B), X(Button.X), Y(Button.Y), LB(Button.LB), RB(Button.RB),
    Back(Button.Back), Start(Button.Start), L3(Button.L3), R3(Button.R3),
}

/** One bounded owner per control. Ignored fingers never acquire on movement. */
class GamepadRouter(val geometry: GamepadGeometry, val tuning: StickTuning = StickTuning()) {
    private val owners = mutableMapOf<Long, Control>()
    val ownership: Map<Long, Control> get() = owners.toMap()
    var state = ControllerState.NEUTRAL
        private set
    var displacement = StickVector()
        private set
    var rightDisplacement = StickVector()
        private set

    fun owns(id: Long) = id in owners

    fun down(id: Long, x: Float, y: Float) {
        if (owns(id) || !x.isFinite() || !y.isFinite()) return
        val control = geometry.regions.entries.firstOrNull { it.value.contains(x, y) }?.key ?: return
        if (control in owners.values) return
        owners[id] = control
        move(id, x, y)
    }

    fun move(id: Long, x: Float, y: Float) {
        val control = owners[id] ?: return
        if (!x.isFinite() || !y.isFinite()) { up(id); return }
        when (control) {
            Control.LeftStick -> {
                displacement = stickDisplacement(x, y, geometry.stickTravel)
                val axis = transformStick(displacement, tuning)
                state = state.copy(leftX = axis.x, leftY = axis.y)
            }
            Control.RightStick -> {
                rightDisplacement = stickDisplacement(x, y, geometry.rightStickTravel)
                val axis = transformStick(rightDisplacement, tuning)
                state = state.copy(rightX = axis.x, rightY = axis.y)
            }
            Control.LT -> state = state.copy(leftTrigger = geometry.lt.value(x, inwardRight = true))
            Control.RT -> state = state.copy(rightTrigger = geometry.rt.value(x, inwardRight = false))
            Control.Dpad -> state = state.copy(buttons = (state.buttons and 0x3c00.inv()) or dpadBits(x, y, geometry.dpad))
            else -> state = state.withButton(requireNotNull(control.button), geometry.regions.getValue(control).contains(x, y))
        }
    }

    fun up(id: Long) {
        when (val control = owners.remove(id)) {
            Control.LeftStick -> { displacement = StickVector(); state = state.copy(leftX = 0f, leftY = 0f) }
            Control.RightStick -> { rightDisplacement = StickVector(); state = state.copy(rightX = 0f, rightY = 0f) }
            Control.LT -> state = state.copy(leftTrigger = 0f)
            Control.RT -> state = state.copy(rightTrigger = 0f)
            Control.Dpad -> state = state.copy(buttons = state.buttons and 0x3c00.inv())
            null -> Unit
            else -> state = state.withButton(requireNotNull(control.button), false)
        }
    }

    fun cancel() {
        owners.clear()
        displacement = StickVector()
        rightDisplacement = StickVector()
        state = ControllerState.NEUTRAL
    }
}

/** Eight 45 degree sectors, clockwise from right in screen coordinates. */
fun dpadBits(x: Float, y: Float, gate: Circle): Int {
    if (!x.isFinite() || !y.isFinite() || !gate.contains(x, y)) return 0
    val dx = x.toDouble() - gate.x
    val dy = y.toDouble() - gate.y
    if (hypot(dx, dy) <= gate.radius * 0.2) return 0
    return when ((atan2(dy, dx) / (Math.PI / 4)).roundToInt().mod(8)) {
        0 -> Button.Right.mask
        1 -> Button.Right.mask or Button.Down.mask
        2 -> Button.Down.mask
        3 -> Button.Down.mask or Button.Left.mask
        4 -> Button.Left.mask
        5 -> Button.Left.mask or Button.Up.mask
        6 -> Button.Up.mask
        else -> Button.Up.mask or Button.Right.mask
    }
}
