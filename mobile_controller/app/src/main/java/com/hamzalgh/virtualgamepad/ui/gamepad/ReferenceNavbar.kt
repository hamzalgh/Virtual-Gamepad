package com.hamzalgh.virtualgamepad.ui.gamepad

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.*
import androidx.compose.material3.IconButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.IntOffset
import com.hamzalgh.virtualgamepad.controller.GamepadGeometry
import com.hamzalgh.virtualgamepad.controller.RailRect
import com.hamzalgh.virtualgamepad.ui.theme.*
import kotlin.math.roundToInt

/** Utilities have separate validated bounds; Back/Start stay in the gameplay router. */
@Composable
internal fun BoxScope.ReferenceNavbar(g: GamepadGeometry, onSettings: () -> Unit, onExit: () -> Unit) {
    Canvas(Modifier.fillMaxSize()) {
        val r = g.navbar
        drawRoundRect(ControllerPlate, Offset(r.left, r.top), Size(r.right - r.left, r.bottom - r.top),
            CornerRadius(FoundationDimensions.inlineGap.toPx()))
    }
    NavbarIcon(g.gear, "Controller settings", true, onSettings)
    NavbarIcon(g.exit, "Exit controller", false, onExit)
}

@Composable
private fun BoxScope.NavbarIcon(bounds: RailRect, description: String, gear: Boolean, onClick: () -> Unit) {
    val density = LocalDensity.current
    IconButton(onClick = onClick,
        modifier = Modifier.align(Alignment.TopStart)
            .offset { IntOffset(bounds.left.roundToInt(), bounds.top.roundToInt()) }
            .size(with(density) { (bounds.right - bounds.left).toDp() }, with(density) { (bounds.bottom - bounds.top).toDp() })
            .semantics { contentDescription = description }) {
        Canvas(Modifier.size(FoundationDimensions.iconSize)) {
            val stroke = FoundationDimensions.controlStroke.toPx()
            val c = center
            if (gear) {
                drawCircle(ControllerRim, size.minDimension * 0.31f, c, style = Stroke(stroke))
                drawCircle(ControllerRim, size.minDimension * 0.10f, c, style = Stroke(stroke))
                repeat(12) { tooth ->
                    rotate(tooth * 30f, c) {
                        drawLine(ControllerRim, c + Offset(0f, -size.height * 0.31f),
                            c + Offset(0f, -size.height * 0.43f), stroke * 1.5f)
                    }
                }
            } else {
                val d = size.minDimension * 0.33f
                drawLine(ControllerRim, c - Offset(d, d), c + Offset(d, d), stroke)
                drawLine(ControllerRim, c + Offset(d, -d), c + Offset(-d, d), stroke)
            }
        }
    }
}
