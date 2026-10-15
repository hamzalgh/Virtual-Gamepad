package com.hamzalgh.virtualgamepad.ui.setup

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.ui.semantics.*
import com.hamzalgh.virtualgamepad.controller.StickTuning
import com.hamzalgh.virtualgamepad.controller.StickVector
import com.hamzalgh.virtualgamepad.controller.transformStick
import com.hamzalgh.virtualgamepad.ui.theme.FoundationDimensions
import java.util.Locale

/** A draft changes only the preview. Apply saves a validated immutable pair. */
@Composable
fun TuningPanel(
    tuning: StickTuning,
    saving: Boolean,
    error: String?,
    onApply: (StickTuning) -> Unit,
) {
    var deadZone by rememberSaveable(tuning.deadZone) { mutableFloatStateOf(tuning.deadZone) }
    var sensitivity by rememberSaveable(tuning.sensitivity) { mutableFloatStateOf(tuning.sensitivity) }
    val draft = StickTuning(deadZone, sensitivity)
    Column(verticalArrangement = Arrangement.spacedBy(FoundationDimensions.inlineGap)) {
        Text("Stick tuning", style = MaterialTheme.typography.titleLarge,
            modifier = Modifier.semantics { heading() })
        Text("Both sticks use the same settings. Apply clears held input; lift your fingers before playing again.")
        Text("Dead zone: ${decimal(deadZone)}", style = MaterialTheme.typography.titleMedium)
        Slider(value = deadZone, onValueChange = { deadZone = it }, valueRange = 0f..0.30f,
            steps = 29, enabled = !saving,
            modifier = Modifier.semantics { contentDescription = "Dead zone" })
        Text("Ignores small movements near the center. Default 0.15; range 0.00 to 0.30.",
            style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text("Sensitivity: ${decimal(sensitivity)}${if (sensitivity == 1f) " (Linear)" else ""}",
            style = MaterialTheme.typography.titleMedium)
        Slider(value = sensitivity, onValueChange = { sensitivity = it }, valueRange = 0.5f..2f,
            steps = 29, enabled = !saving,
            modifier = Modifier.semantics { contentDescription = "Sensitivity" })
        Text("Higher values give more output for the same movement. Default 1.00 (Linear); range 0.50 to 2.00. Full travel always reaches full output.",
            style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        TuningPreview(draft)
        if (error != null) Text(error, color = MaterialTheme.colorScheme.error,
            modifier = Modifier.semantics { liveRegion = LiveRegionMode.Polite })
        Button(onClick = { onApply(draft) }, enabled = !saving && (draft != tuning || error != null),
            modifier = Modifier.fillMaxWidth()) { Text(if (saving) "Saving tuning…" else "Apply tuning") }
        OutlinedButton(onClick = { deadZone = StickTuning().deadZone; sensitivity = StickTuning().sensitivity },
            enabled = !saving, modifier = Modifier.fillMaxWidth()) { Text("Reset to defaults") }
        Text(if (draft == tuning && error == null) "Current settings saved on this phone." else "Preview only. Apply to save both values.",
            style = MaterialTheme.typography.bodySmall)
    }
}

@Composable
private fun TuningPreview(tuning: StickTuning) {
    val colors = MaterialTheme.colorScheme
    val stroke = with(LocalDensity.current) { FoundationDimensions.pressedStroke.toPx() }
    val middle = transformStick(StickVector(0.5f, 0f), tuning).x
    Text("Preview: 50% stick travel → ${(middle * 100).toInt()}% output",
        style = MaterialTheme.typography.bodyMedium)
    Canvas(Modifier.fillMaxWidth().height(FoundationDimensions.previewHeight).semantics {
        contentDescription = "Response curve from centered to full stick travel"
    }) {
        drawRect(colors.surfaceVariant)
        drawLine(colors.outline, Offset(0f, size.height), Offset(size.width, 0f), stroke / 2)
        val curve = Path().apply {
            moveTo(0f, size.height)
            for (step in 1..100) {
                val input = step / 100f
                val output = transformStick(StickVector(input, 0f), tuning).x
                lineTo(input * size.width, (1f - output) * size.height)
            }
        }
        drawPath(curve, colors.primary, style = Stroke(stroke))
    }
}

private fun decimal(value: Float) = String.format(Locale.ROOT, "%.2f", value)

@Preview(name = "Tuning with large text", widthDp = 640, heightDp = 360, fontScale = 2f)
@Composable
private fun TuningPreviewScreen() {
    com.hamzalgh.virtualgamepad.ui.theme.VirtualGamepadTheme {
        Surface {
            Column(Modifier.verticalScroll(rememberScrollState()).padding(FoundationDimensions.cardPadding)) {
                TuningPanel(StickTuning(), false, null, {})
            }
        }
    }
}
