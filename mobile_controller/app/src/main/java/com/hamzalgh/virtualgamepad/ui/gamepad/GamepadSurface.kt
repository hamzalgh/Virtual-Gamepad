package com.hamzalgh.virtualgamepad.ui.gamepad

import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.semantics.*
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import androidx.compose.ui.tooling.preview.Preview
import com.hamzalgh.virtualgamepad.BuildConfig
import com.hamzalgh.virtualgamepad.controller.*
import com.hamzalgh.virtualgamepad.ui.theme.FoundationDimensions
import com.hamzalgh.virtualgamepad.ui.theme.CalmGreen
import com.hamzalgh.virtualgamepad.ui.theme.ControllerBlack
import com.hamzalgh.virtualgamepad.ui.theme.StatusWarning
import com.hamzalgh.virtualgamepad.ui.setup.TuningPanel

/** One gameplay input surface. Controls and the overlay only render state. */
@Composable
fun GamepadSurface(
    state: ControllerState,
    status: String,
    onState: (ControllerState) -> Unit,
    onExit: () -> Unit,
    resetKey: Int = 0,
    diagnostics: String = "",
    tuning: StickTuning = StickTuning(),
    tuningSaving: Boolean = false,
    tuningError: String? = null,
    onApplyTuning: (StickTuning) -> Unit = {},
    onSurfaceRunning: (Boolean) -> Unit = {},
    manualTrust: Boolean = false,
    showStickClicks: Boolean = true,
    appearanceSaving: Boolean = false,
    appearanceError: String? = null,
    onShowStickClicks: (Boolean) -> Unit = {},
) {
    var surfaceSize by remember { mutableStateOf(IntSize.Zero) }
    var showDiagnostics by remember { mutableStateOf(false) }
    var showSettings by remember { mutableStateOf(false) }
    var showTouches by remember { mutableStateOf(false) }
    val density = LocalDensity.current
    val geometry = remember(surfaceSize, density.density, showStickClicks) {
        GamepadGeometry.fit(surfaceSize.width.toFloat(), surfaceSize.height.toFloat(), density.density, showStickClicks)
    }
    val router = remember(geometry, tuning) { geometry?.let { GamepadRouter(it, tuning) } }
    var displacement by remember { mutableStateOf(StickVector()) }
    var rightDisplacement by remember { mutableStateOf(StickVector()) }
    var owners by remember { mutableStateOf(emptyMap<Long, Control>()) }
    val update by rememberUpdatedState(onState)
    val running by rememberUpdatedState(onSurfaceRunning)
    fun publish() {
        displacement = router?.displacement ?: StickVector()
        rightDisplacement = router?.rightDisplacement ?: StickVector()
        owners = if (showTouches) router?.ownership.orEmpty() else emptyMap()
        update(router?.state ?: ControllerState.NEUTRAL)
    }
    fun cancel() { router?.cancel(); publish() }
    DisposableEffect(router) {
        onDispose { router?.cancel(); update(ControllerState.NEUTRAL) }
    }
    val surfaceRunning = geometry != null && !showSettings
    DisposableEffect(surfaceRunning) {
        running(surfaceRunning)
        onDispose { running(false) }
    }
    Surface(Modifier.fillMaxSize(), color = ControllerBlack) {
        Column(Modifier.windowInsetsPadding(WindowInsets.safeDrawing.union(WindowInsets.safeGestures))
            .padding(FoundationDimensions.inlineGap)) {
            Box(Modifier.fillMaxSize()
                .testTag("GamepadSurface")
                .onSizeChanged { surfaceSize = it }
                .pointerInput(router, resetKey, showSettings, showDiagnostics, showTouches) {
                    // Reset keys invalidate all old ownership before accepting another down.
                    cancel()
                    if (router != null && !showSettings) awaitEachGesture {
                        try {
                            do {
                                val event = awaitPointerEvent()
                                // Release first so same-event replacements do not depend on pointer indexes.
                                event.changes.filter { !it.pressed }.forEach { change ->
                                    if (router.owns(change.id.value)) { router.up(change.id.value); change.consume() }
                                }
                                for (change in event.changes) {
                                    if (!change.pressed) continue
                                    val id = change.id.value
                                    val owned = router.owns(id)
                                    when {
                                        owned && change.isConsumed -> router.up(id)
                                        !change.previousPressed && !change.isConsumed -> router.down(id, change.position.x, change.position.y)
                                        owned -> router.move(id, change.position.x, change.position.y)
                                    }
                                    if (owned || router.owns(id)) change.consume()
                                }
                                publish()
                            } while (event.changes.any { it.pressed })
                        } finally { cancel() }
                    }
                }, contentAlignment = Alignment.Center) {
                if (geometry == null) {
                    Column(Modifier.padding(FoundationDimensions.cardPadding), horizontalAlignment = Alignment.CenterHorizontally) {
                        Text("More landscape space needed", style = MaterialTheme.typography.titleLarge)
                        Text("Rotate to landscape or enlarge the window. Input is neutral until every control fits safely.")
                        TextButton(onClick = { cancel(); onExit() },
                            modifier = Modifier.semantics { contentDescription = "Exit controller" }) { Text("Exit controller") }
                    }
                } else {
                    ReferenceNavbar(geometry, { cancel(); showSettings = true }, { cancel(); onExit() })
                    AnalogStick(geometry.stick, geometry.stickKnob(displacement), "L", displacement)
                    AnalogStick(geometry.rightStick, geometry.rightStickKnob(rightDisplacement), "R", rightDisplacement)
                    DPad(geometry.dpad, state)
                    AnalogTriggerRail(geometry.lt, "LT", state.leftTrigger, true)
                    AnalogTriggerRail(geometry.rt, "RT", state.rightTrigger, false)
                    for ((control, region) in geometry.regions) {
                        control.button?.let { DigitalButton(region, control.name, state.pressed(it)) }
                    }
                    if (BuildConfig.DEBUG && showTouches) {
                        TouchDebugOverlay(geometry, owners, state, tuning)
                    }
                }
            }
        }
    }
    if (showSettings) Dialog(onDismissRequest = { showSettings = false },
        properties = DialogProperties(usePlatformDefaultWidth = false)) {
        Surface(Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.safeDrawing.union(WindowInsets.safeGestures)),
            color = MaterialTheme.colorScheme.background) {
            Column(Modifier.fillMaxSize().padding(FoundationDimensions.cardPadding)) {
                Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                    Text("Controller settings", Modifier.weight(1f), style = MaterialTheme.typography.titleLarge)
                    TextButton(onClick = { showSettings = false }) { Text("Close settings") }
                }
                Column(Modifier.weight(1f).widthIn(max = FoundationDimensions.contentMaxWidth)
                    .fillMaxWidth().align(Alignment.CenterHorizontally).verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(FoundationDimensions.sectionGap)) {
                    Text(status, style = MaterialTheme.typography.titleMedium,
                        modifier = Modifier.semantics { liveRegion = LiveRegionMode.Polite })
                    if (manualTrust) Text("Unsafe development trust is active. This is not pairing.", color = StatusWarning)
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text("Show L3/R3", Modifier.weight(1f))
                        Switch(checked = showStickClicks, onCheckedChange = onShowStickClicks, enabled = !appearanceSaving,
                            modifier = Modifier.semantics { contentDescription = "Show L3/R3" })
                    }
                    if (appearanceError != null) Text(appearanceError, color = MaterialTheme.colorScheme.error)
                    Text("L1 = LB, L2 = LT, R1 = RB, R2 = RT. Slide L2/R2 horizontally inward for more trigger input; release returns to zero.",
                        style = MaterialTheme.typography.bodySmall)
                    TuningPanel(tuning, tuningSaving, tuningError, onApplyTuning)
                    Text("Gameplay uses simultaneous direct touch. Screen-reader gestures may intercept these controls; full accessible gameplay is not verified.",
                        style = MaterialTheme.typography.bodySmall)
                    if (diagnostics.isNotEmpty()) OutlinedButton(onClick = { cancel(); showDiagnostics = !showDiagnostics }) {
                        Text(if (showDiagnostics) "Hide diagnostics" else "Diagnostics")
                    }
                    if (showDiagnostics) {
                        Text("Session diagnostics", style = MaterialTheme.typography.titleMedium)
                        Text(diagnostics, style = MaterialTheme.typography.bodySmall)
                        Text("Echo age includes PC state age and scheduling, not pure network RTT. Cadence excludes cleanup packets.")
                    }
                    if (BuildConfig.DEBUG) Row(verticalAlignment = Alignment.CenterVertically) {
                        Text("Touch overlay", Modifier.weight(1f))
                        Switch(checked = showTouches, onCheckedChange = { cancel(); showTouches = it },
                            modifier = Modifier.semantics { contentDescription = "Touch overlay" })
                    }
                    OutlinedButton(onClick = { cancel(); onExit() }, modifier = Modifier.fillMaxWidth()) { Text("Exit controller") }
                }
            }
        }
    }
}

@Preview(name = "Compact controller", widthDp = 720, heightDp = 360)
@Preview(name = "Wide controller", widthDp = 960, heightDp = 440)
@Preview(name = "Large window", widthDp = 1280, heightDp = 720)
@Composable
private fun ControllerPreview() {
    com.hamzalgh.virtualgamepad.ui.theme.VirtualGamepadTheme {
        GamepadSurface(ControllerState.NEUTRAL, "Ready", {}, {})
    }
}
