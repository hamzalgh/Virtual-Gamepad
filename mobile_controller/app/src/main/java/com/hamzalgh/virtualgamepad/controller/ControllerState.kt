package com.hamzalgh.virtualgamepad.controller

/** Protocol button identities. No Guide or reserved bit can be selected. */
enum class Button(val mask: Int) {
    A(1), B(2), X(4), Y(8), LB(16), RB(32), Back(64), Start(128),
    L3(256), R3(512), Up(1024), Down(2048), Left(4096), Right(8192),
}

/** Immutable full local snapshot; construction rejects unsafe bit combinations. */
data class ControllerState(
    val buttons: Int = 0,
    val leftX: Float = 0f,
    val leftY: Float = 0f,
    val rightX: Float = 0f,
    val rightY: Float = 0f,
    val leftTrigger: Float = 0f,
    val rightTrigger: Float = 0f,
) {
    init {
        require(buttons and 0x3fff.inv() == 0)
        require(buttons and 0xc00 != 0xc00 && buttons and 0x3000 != 0x3000)
        require(leftX.isFinite() && leftX in -1f..1f && leftY.isFinite() && leftY in -1f..1f)
        require(rightX.isFinite() && rightX in -1f..1f && rightY.isFinite() && rightY in -1f..1f)
        require(leftTrigger.isFinite() && leftTrigger in 0f..1f)
        require(rightTrigger.isFinite() && rightTrigger in 0f..1f)
    }
    fun pressed(button: Button) = buttons and button.mask != 0
    val a: Boolean get() = pressed(Button.A)
    fun withButton(button: Button, pressed: Boolean) = copy(
        buttons = if (pressed) buttons or button.mask else buttons and button.mask.inv(),
    )
    companion object { val NEUTRAL = ControllerState() }
}
