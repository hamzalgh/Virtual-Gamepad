package com.hamzalgh.virtualgamepad.ui.gamepad

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.*
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.graphics.drawscope.clipPath
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.*
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.sp
import com.hamzalgh.virtualgamepad.controller.*
import com.hamzalgh.virtualgamepad.protocol.V1Codec
import kotlin.math.roundToInt
import com.hamzalgh.virtualgamepad.ui.theme.*

/** Passive visuals: only GamepadSurface may interpret gameplay pointers. */
@Composable
internal fun BoxScope.AnalogStick(gate: Circle, knob: Circle, label: String, displacement: StickVector) {
    val colors = MaterialTheme.colorScheme
    val stroke = with(LocalDensity.current) { FoundationDimensions.controlStroke.toPx() }
    Canvas(Modifier.fillMaxSize()) {
        val center = Offset(gate.x, gate.y)
        drawCircle(Brush.radialGradient(0f to ControllerInner, 0.70f to ControllerInner,
            0.96f to ControllerGlow, 1f to ControllerRim, center = center, radius = gate.radius), gate.radius, center)
        drawCircle(ControllerRim, gate.radius - stroke, center, style = Stroke(stroke * 2))
        drawCircle(Brush.radialGradient(listOf(ControllerInner, ControllerInner, ControllerGlow),
            center = Offset(knob.x, knob.y), radius = knob.radius), knob.radius, Offset(knob.x, knob.y))
        drawCircle(if (displacement == StickVector()) ControllerRim else colors.primary,
            knob.radius, Offset(knob.x, knob.y), style = Stroke(stroke * 2))
    }
    ControlLabel(knob, "", "${if (label == "L") "Left" else "Right"} analog stick",
        "X ${displacement.x}, Y ${displacement.y}", false, button = false)
}

@Composable
internal fun BoxScope.DigitalButton(region: CaptureRegion, label: String, pressed: Boolean) {
    val colors = MaterialTheme.colorScheme
    val stroke = with(LocalDensity.current) { (if (pressed) FoundationDimensions.pressedStroke else FoundationDimensions.controlStroke).toPx() }
    val shoulder = label == "LB" || label == "RB"
    Canvas(Modifier.fillMaxSize()) {
        if (shoulder && region is RailRect) {
            val shape = shoulderPath(region, mirror = label == "RB", trigger = false)
            if (pressed) drawPath(shape, ControllerRim) else drawPath(shape,
                Brush.radialGradient(listOf(ControllerInner, ControllerInner, ControllerGlow),
                    center = Offset((region.left + region.right) / 2, (region.top + region.bottom) / 2),
                    radius = maxOf(region.right - region.left, region.bottom - region.top) / 2))
            drawPath(shape, ControllerGlow, style = Stroke(stroke * 4))
            drawPath(shape, ControllerRim, style = Stroke(stroke))
        } else {
            if (!pressed && region is Circle) {
                drawCircle(Brush.radialGradient(0f to ControllerInner, 0.6f to ControllerInner,
                    0.92f to ControllerGlow, 1f to ControllerRim,
                    center = Offset(region.x, region.y), radius = region.radius), region.radius, Offset(region.x, region.y))
            } else region(region, if (pressed) ControllerRim else ControllerInner)
            region(region, ControllerGlow, Stroke(stroke * 4))
            region(region, ControllerRim, Stroke(stroke))
        }
    }
    ControlLabel(region, when (label) { "LB" -> "L1"; "RB" -> "R1"; else -> label }, "$label button",
        if (pressed) "Pressed" else "Released", pressed, foreground = if (pressed) ControllerBlack else ControllerRim)
}

@Composable
internal fun BoxScope.AnalogTriggerRail(rail: RailRect, label: String, value: Float, inwardRight: Boolean) {
    val colors = MaterialTheme.colorScheme
    val stroke = with(LocalDensity.current) { FoundationDimensions.controlStroke.toPx() }
    Canvas(Modifier.fillMaxSize()) {
        val shape = shoulderPath(rail, mirror = !inwardRight, trigger = true)
        drawPath(shape, Brush.radialGradient(listOf(ControllerInner, ControllerInner, ControllerGlow),
            center = Offset((rail.left + rail.right) / 2, (rail.top + rail.bottom) / 2),
            radius = maxOf(rail.right - rail.left, rail.bottom - rail.top) / 2))
        val width = (rail.right - rail.left) * value
        clipPath(shape) {
            drawRect(ControllerRim.copy(alpha = 0.35f),
                Offset(if (inwardRight) rail.left else rail.right - width, rail.top), Size(width, rail.bottom - rail.top))
        }
        drawPath(shape, ControllerGlow, style = Stroke(stroke * 4))
        drawPath(shape, if (value > 0f) colors.primary else ControllerRim,
            style = Stroke(if (value > 0f) stroke * 2 else stroke))
    }
    ControlLabel(rail, if (inwardRight) "L2" else "R2", "$label analog trigger",
        "${V1Codec.encodeTrigger(value)} of 255", false, button = false)
}

@Composable
internal fun BoxScope.DPad(gate: Circle, state: ControllerState) {
    val colors = MaterialTheme.colorScheme
    val stroke = with(LocalDensity.current) { FoundationDimensions.controlStroke.toPx() }
    Canvas(Modifier.fillMaxSize()) {
        val center = Offset(gate.x, gate.y)
        for ((index, button) in listOf(Button.Up, Button.Right, Button.Down, Button.Left).withIndex()) {
            rotate(index * 90f, center) {
                val arm = Path().apply {
                    val x = gate.x; val y = gate.y; val r = gate.radius
                    moveTo(x, y - r * 0.20f)
                    cubicTo(x - r * 0.10f, y - r * 0.20f, x - r * 0.31f, y - r * 0.42f, x - r * 0.31f, y - r * 0.55f)
                    lineTo(x - r * 0.31f, y - r * 0.82f)
                    cubicTo(x - r * 0.31f, y - r * 1.02f, x + r * 0.31f, y - r * 1.02f, x + r * 0.31f, y - r * 0.82f)
                    lineTo(x + r * 0.31f, y - r * 0.55f)
                    cubicTo(x + r * 0.31f, y - r * 0.42f, x + r * 0.10f, y - r * 0.20f, x, y - r * 0.20f)
                    close()
                }
                drawPath(arm, if (state.pressed(button)) ControllerRim else ControllerInner)
                drawPath(arm, ControllerGlow, style = Stroke(stroke * 4))
                drawPath(arm, ControllerRim,
                    style = Stroke(if (state.pressed(button)) stroke * 2 else stroke))
            }
        }
    }
    for ((button, vector) in listOf(Button.Up to (0f to -1f), Button.Down to (0f to 1f),
        Button.Left to (-1f to 0f), Button.Right to (1f to 0f))) {
        val label = Circle(gate.x + vector.first * gate.radius * 0.63f,
            gate.y + vector.second * gate.radius * 0.63f, gate.radius * 0.28f)
        ControlLabel(label, "", "D-pad ${button.name}",
            if (state.pressed(button)) "Pressed" else "Released", state.pressed(button), button = false)
    }
}

@Composable
private fun BoxScope.ControlLabel(
    region: CaptureRegion, label: String, description: String, detail: String, pressed: Boolean, button: Boolean = true,
    foreground: Color = Color.Unspecified,
) {
    val density = LocalDensity.current
    Box(Modifier.align(Alignment.TopStart)
        .offset { IntOffset(region.left.roundToInt(), region.top.roundToInt()) }
        .size(with(density) { (region.right - region.left).toDp() }, with(density) { (region.bottom - region.top).toDp() })
        .semantics {
            contentDescription = description
            stateDescription = detail
            if (button) role = Role.Button
        }, contentAlignment = Alignment.Center) {
        val style = if (label.length <= 2) MaterialTheme.typography.titleLarge else MaterialTheme.typography.labelLarge
        val fontSize = if (label.length in 1..2) ((region.bottom - region.top) / density.density * 0.48f).coerceIn(22f, 40f).sp else style.fontSize
        // Gameplay labels fit fixed captures; setup/settings keep the full system text scale.
        Text(label, style = style.copy(fontSize = fontSize * (density.fontScale.coerceAtMost(1.3f) / density.fontScale)),
            maxLines = 1, softWrap = false,
            color = if (foreground != Color.Unspecified) foreground else if (pressed) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.onSurface)
    }
}

/** Curved diagonal shoulder petal, kept inside its conservative rectangular capture. */
private fun shoulderPath(r: RailRect, mirror: Boolean, trigger: Boolean): Path {
    fun x(v: Float) = r.left + (r.right - r.left) * (if (mirror) 1f - v else v)
    fun y(v: Float) = r.top + (r.bottom - r.top) * (if (trigger) 1f - v else v)
    return Path().apply {
        moveTo(x(0.50f), y(0.06f))
        cubicTo(x(0.58f), y(0.04f), x(0.79f), y(0.27f), x(0.91f), y(0.43f))
        cubicTo(x(0.99f), y(0.57f), x(0.91f), y(0.85f), x(0.70f), y(0.93f))
        cubicTo(x(0.43f), y(0.99f), x(0.20f), y(0.81f), x(0.08f), y(0.62f))
        cubicTo(x(0.03f), y(0.53f), x(0.18f), y(0.37f), x(0.43f), y(0.11f))
        close()
    }
}

private fun DrawScope.region(region: CaptureRegion, color: Color, stroke: Stroke? = null) {
    val style = stroke ?: androidx.compose.ui.graphics.drawscope.Fill
    when (region) {
        is Circle -> drawCircle(color, region.radius, Offset(region.x, region.y), style = style)
        is RailRect -> drawRoundRect(color, Offset(region.left, region.top), Size(region.right - region.left, region.bottom - region.top),
            cornerRadius = CornerRadius((region.bottom - region.top) / 2), style = style)
    }
}

@Composable
internal fun BoxScope.TouchDebugOverlay(
    geometry: GamepadGeometry, owners: Map<Long, Control>, state: ControllerState, tuning: StickTuning,
) {
    val color = MaterialTheme.colorScheme.tertiary
    Canvas(Modifier.fillMaxSize()) {
        drawRect(color, style = Stroke(2f))
        geometry.regions.values.forEach { region(it, color, Stroke(2f)) }
        for (gate in listOf(geometry.stickTravel, geometry.rightStickTravel)) {
            drawCircle(color, gate.radius * tuning.deadZone, Offset(gate.x, gate.y), style = Stroke(2f))
        }
    }
    val axes = listOf(state.leftX, state.leftY, state.rightX, state.rightY)
    Text("Owners: $owners\nLX LY RX RY: ${axes.joinToString { "%.2f".format(java.util.Locale.ROOT, it) }}\n" +
        "Encoded: ${axes.map { V1Codec.encodeAxis(it) }} | LT ${V1Codec.encodeTrigger(state.leftTrigger)} RT ${V1Codec.encodeTrigger(state.rightTrigger)}\n" +
        "Bits 0x${state.buttons.toString(16)}: ${Button.entries.filter { state.pressed(it) }}",
        modifier = Modifier.align(Alignment.Center), style = MaterialTheme.typography.labelSmall, color = color)
}
