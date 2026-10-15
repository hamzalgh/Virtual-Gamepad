package com.hamzalgh.virtualgamepad.ui.setup

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.tooling.preview.Preview
import com.hamzalgh.virtualgamepad.app.SetupState
import com.hamzalgh.virtualgamepad.network.ConnectionPhase
import com.hamzalgh.virtualgamepad.network.ConnectionState
import com.hamzalgh.virtualgamepad.network.diagnosticText
import com.hamzalgh.virtualgamepad.ui.theme.FoundationDimensions
import com.hamzalgh.virtualgamepad.ui.theme.VirtualGamepadTheme
import com.hamzalgh.virtualgamepad.controller.StickTuning
import com.hamzalgh.virtualgamepad.app.label

@Composable
fun ConnectionScreen(
    state: SetupState,
    onAddress: (String) -> Unit,
    onPort: (String) -> Unit,
    onConnect: () -> Unit,
    onDisconnect: () -> Unit,
    onPermission: () -> Unit,
    onSettings: () -> Unit,
    onReload: () -> Unit,
    onTuning: (StickTuning) -> Unit = {},
    onPairingCode: (String) -> Unit = {},
    onPair: () -> Unit = {},
    onFindPcs: () -> Unit = {},
    onStopDiscovery: () -> Unit = {},
    onSelectPc: (com.hamzalgh.virtualgamepad.network.DiscoveredPc) -> Unit = {},
) {
    var showTuning by remember { mutableStateOf(false) }
    Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
        Box(Modifier.fillMaxSize(), contentAlignment = Alignment.TopCenter) {
            Column(
                modifier = Modifier.widthIn(max = FoundationDimensions.contentMaxWidth).safeDrawingPadding().imePadding()
                    .verticalScroll(rememberScrollState()).padding(FoundationDimensions.screenPadding),
                verticalArrangement = Arrangement.spacedBy(FoundationDimensions.sectionGap),
            ) {
                Text("VIRTUAL GAMEPAD", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.primary)
                Text("Connect to your PC", style = MaterialTheme.typography.headlineLarge)
                Text("Use your phone and Windows PC on the same private Wi-Fi network.", color = MaterialTheme.colorScheme.onSurfaceVariant)
                ConnectionStatus(state.connection)
                if (!state.loaded) {
                    Text("Connection settings are not loaded yet.")
                    OutlinedButton(onClick = onReload) { Text("Retry settings") }
                }
                EndpointForm(state, onAddress, onPort)
                OutlinedButton(onClick = if (state.discovery.searching) onStopDiscovery else onFindPcs,
                    enabled = state.loaded && state.permissionGranted, modifier = Modifier.fillMaxWidth()) {
                    Text(if (state.discovery.searching) "Stop searching" else "Find PCs")
                }
                Text(state.discovery.detail, modifier = Modifier.semantics { liveRegion = LiveRegionMode.Polite })
                state.discovery.pcs.forEach { pc ->
                    OutlinedButton(onClick = { onSelectPc(pc) }, modifier = Modifier.fillMaxWidth()) {
                        Text("${pc.name}\n${pc.endpoint.address}:${pc.endpoint.port}")
                    }
                }
                if (state.connection.phase == ConnectionPhase.PairingNeeded) {
                    Text("Pair this phone", style = MaterialTheme.typography.titleMedium)
                    OutlinedTextField(value = state.pairingCode, onValueChange = onPairingCode,
                        label = { Text("Six digit PC code") }, singleLine = true,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.NumberPassword),
                        enabled = !state.connection.pairingPending && state.connection.retryAfterSeconds == 0,
                        modifier = Modifier.fillMaxWidth(),
                        supportingText = { Text("Type pair in the PC terminal. The code expires after two minutes.") })
                    Button(onClick = onPair, enabled = state.pairingCode.length == 6 &&
                        !state.connection.pairingPending && state.connection.retryAfterSeconds == 0,
                        modifier = Modifier.fillMaxWidth()) {
                        Text(when { state.connection.pairingPending -> "Pairing…"
                            state.connection.retryAfterSeconds > 0 -> "Retry in ${state.connection.retryAfterSeconds}s"
                            else -> "Pair" })
                    }
                    OutlinedButton(onClick = onDisconnect) { Text("Cancel pairing") }
                }
                if (!state.permissionGranted) PermissionPanel(onPermission, onSettings)
                Button(onClick = onConnect, enabled = state.loaded && state.permissionGranted, modifier = Modifier.fillMaxWidth()) {
                    Text(if (state.connection.phase == ConnectionPhase.Offline) "Connect" else "Retry connection")
                }
                OutlinedButton(onClick = onDisconnect, modifier = Modifier.fillMaxWidth()) { Text("Disconnect") }
                OutlinedButton(onClick = { showTuning = !showTuning }, enabled = state.loaded,
                    modifier = Modifier.fillMaxWidth()) { Text(if (showTuning) "Hide stick tuning" else "Stick tuning") }
                if (showTuning) TuningPanel(state.tuning, state.tuningSaving, state.tuningError, onTuning)
                Text("Troubleshooting", style = MaterialTheme.typography.titleMedium)
                Text("Check the PC address and port, Windows Private firewall access, and router client isolation. Type pair in the PC terminal for a new phone. Pairing is for your trusted home LAN and is not encrypted.", color = MaterialTheme.colorScheme.onSurfaceVariant)
                Text("Full controller. Connect enters controller mode when the PC is ready.", style = MaterialTheme.typography.bodySmall)
                Text("Gameplay requires simultaneous direct touch; screen-reader gestures may intercept controls. Setup and recovery use standard accessible controls.", style = MaterialTheme.typography.bodySmall)
            }
        }
    }
}

@Composable
private fun EndpointForm(state: SetupState, onAddress: (String) -> Unit, onPort: (String) -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(FoundationDimensions.inlineGap)) {
        OutlinedTextField(
            value = state.address, onValueChange = onAddress, label = { Text("PC IPv4 address") },
            placeholder = { Text("192.168.1.10") }, singleLine = true, enabled = state.loaded,
            isError = state.endpointError, modifier = Modifier.fillMaxWidth(),
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
        )
        OutlinedTextField(
            value = state.port, onValueChange = onPort, label = { Text("UDP port") },
            singleLine = true, enabled = state.loaded, isError = state.endpointError,
            modifier = Modifier.fillMaxWidth(), keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
            supportingText = { Text(if (state.endpointError) "Enter a private/local IPv4 address and port 1 to 65535." else "Default port: 8765. Saved when you connect.") },
        )
    }
}

@Composable
private fun PermissionPanel(onPermission: () -> Unit, onSettings: () -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(FoundationDimensions.inlineGap)) {
        Text("Local network permission", style = MaterialTheme.typography.titleMedium)
        Text("Allow access so your phone can connect to your PC on the same Wi-Fi. If Android no longer asks, enable Nearby devices in app settings.")
        Button(onClick = onPermission) { Text("Allow local network") }
        OutlinedButton(onClick = onSettings) { Text("Open app settings") }
    }
}

@Composable
private fun ConnectionStatus(state: ConnectionState) {
    var showDiagnostics by remember { mutableStateOf(false) }
    Surface(color = MaterialTheme.colorScheme.surface, shape = MaterialTheme.shapes.large) {
        Column(modifier = Modifier.fillMaxWidth().padding(FoundationDimensions.cardPadding)
            .semantics { liveRegion = LiveRegionMode.Polite },
            verticalArrangement = Arrangement.spacedBy(FoundationDimensions.inlineGap)) {
            Text(state.phase.label, style = MaterialTheme.typography.titleMedium)
            Text(state.detail)
            if (state.statusReceived) {
                OutlinedButton(onClick = { showDiagnostics = !showDiagnostics }) { Text(if (showDiagnostics) "Hide diagnostics" else "Diagnostics") }
                if (showDiagnostics) Text(state.diagnosticText(), style = MaterialTheme.typography.labelMedium)
            }
            if (state.manualTrust) Text("Unsafe development trust is active. This is not pairing.", color = MaterialTheme.colorScheme.tertiary)
        }
    }
}

@Preview(name = "Connection setup", widthDp = 412, heightDp = 915)
@Composable
private fun ConnectionPreview() {
    VirtualGamepadTheme {
        ConnectionScreen(SetupState(loaded = true, permissionGranted = true), {}, {}, {}, {}, {}, {}, {})
    }
}
