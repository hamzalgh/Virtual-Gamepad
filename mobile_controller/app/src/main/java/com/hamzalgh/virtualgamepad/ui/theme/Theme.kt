package com.hamzalgh.virtualgamepad.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.material3.Shapes
import androidx.compose.foundation.shape.RoundedCornerShape

private val FoundationColorScheme = darkColorScheme(
    primary = CoolCyan,
    onPrimary = GraphiteBackground,
    tertiary = CalmGreen,
    background = GraphiteBackground,
    onBackground = TextPrimary,
    surface = GraphiteSurface,
    onSurface = TextPrimary,
    surfaceVariant = GraphiteSurfaceRaised,
    onSurfaceVariant = TextSecondary,
    outline = ControlOutline,
    error = StatusDanger,
    onError = GraphiteBackground,
)

@Composable
fun VirtualGamepadTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = FoundationColorScheme,
        typography = Typography,
        shapes = Shapes(small = RoundedCornerShape(FoundationDimensions.inlineGap),
            medium = RoundedCornerShape(FoundationDimensions.cardRadius),
            large = RoundedCornerShape(FoundationDimensions.cardRadius)),
        content = content,
    )
}
