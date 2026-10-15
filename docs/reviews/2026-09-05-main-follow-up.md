# Review, main, 2026-09-05

**Reviewed by**: gpt-6-astra (author on gpt-5.6-sol)
**Scope**: 89 files, uncommitted; full current working tree except Cargo.lock, with no HEAD or merge base
**Verdict**: Approve with nits

## Summary

The foundation remains small, with independent Android and Rust builds and no premature networking or driver integration. The previous major inset finding and the overflow and system bar findings are resolved in the implementation. The supplied physical checks support readable light system bar icons and reachable enlarged text, but the connected Android tests did not execute. Three minor issues remain in evidence tracking, one UI test assumption, and the shared verification instructions.

## Minor

### 🟡 Record the latest manual results and failed test installation, `context/progress-tracker.md:17`

**Problem**: The current status still says the corrected build needs its device rerun. The evidence entry at lines 178 and 179 also says no corrected APK launch or connected test attempt occurred and the user has not run the Rust binary. The supplied follow up evidence now includes readable light system bar icons with Android in light mode, reachable enlarged text, and the expected `cargo run` output. The tracker contains no record of Xiaomi rejecting the test installation with `INSTALL_GRANT_RUNTIME_PERMISSIONS`.

**Why it matters**: You use this tracker to decide whether Spec 0002 can begin. Its current account loses successful manual evidence and does not preserve the reason instrumentation results are missing. The reported Gradle `BUILD SUCCESSFUL` cannot establish test success when installation prevented test execution.

**Suggested fix**: Append the supplied results as user reported evidence and reconcile current status and next actions. Record the connected test attempt as installation blocked, with no instrumentation tests executed. Keep any unreported orientation or navigation configuration explicitly unverified rather than treating the general light mode and enlarged text checks as the full device matrix.

### 🟡 Make the smoke test tolerate a scrollable viewport, `mobile_controller/app/src/androidTest/java/com/hamzalgh/virtualgamepad/FoundationScreenTest.kt:29`

**Problem**: `foundationStatusIsVisible` still requires the footer to be visible immediately, while its content uses the device's current window and font configuration. The corrected screen deliberately allows that footer to start below the viewport in a short landscape window or at enlarged text sizes. Unlike the new constrained window test, this assertion does not scroll first or establish a known fitting viewport.

**Why it matters**: A valid layout can fail this smoke test solely because of the device configuration. This is inferred from the assertion and layout constraints, not an observed instrumentation failure; the supplied installation failure prevented execution.

**Suggested fix**: Recommend asserting footer reachability with `performScrollTo()` before visibility, or explicitly defining a fitting size and font scale if initial footer visibility is the behavior you want to test. Keep the constrained window regression case as a separate check.

### 🟡 Remove the nonexistent formatter command from the shared gate, `context/testing-and-release.md:104`

**Problem**: The shared Android verification block still lists `gradlew.bat ktlintCheck`, although the root README and Android package instructions explicitly state that Spec 0001 configures no separate Kotlin formatter task. The Gradle build files contain no formatter plugin or task declaration.

**Why it matters**: Following the maintained test guide sends the next session to a task that does not exist and conflicts with the documented foundation decision.

**Suggested fix**: Align this command block with the actual three Android checks and note that a formatter can be added by a future spec that selects and pins it. No new dependency is needed to resolve this documentation issue.

## Strengths

- The inset correction wraps the scrolling foreground in `safeDrawingPadding()` and preserves the full background. The inset padding is outside the scroll viewport, so scrolling cannot move text into the system bar region. This follows the official [Compose inset guidance](https://developer.android.com/develop/ui/compose/system/insets-ui).
- The modifier order gives the scrollable column a minimum height derived from the safe viewport without imposing a maximum content height. `Arrangement.SpaceBetween` preserves spacing when content fits, and the new constrained window test exercises footer access when it does not.
- Both system bars explicitly use `SystemBarStyle.dark(Color.TRANSPARENT)`, consistent with the application's fixed dark surface and the supplied light system mode observation. The [AndroidX API documentation](https://developer.android.com/reference/androidx/activity/SystemBarStyle) confirms this style expects a dark background for light system icons and is available from Activity 1.8.0, matching the pinned dependency.
- Rust remains dependency free, with a focused health output assertion and deferred module boundaries. The Android manifest adds no network permission, and the fixture directory makes no false claim of protocol implementation.

## Test coverage

Test signal is configured. The JVM harness smoke test, application ID instrumentation test, foundation labels, new 320 by 240 dp scroll case, and Rust health output test are appropriate to this narrow skeleton. The Compose tests mount the screen directly rather than `MainActivity`, so they do not establish actual window inset delivery or system bar icon appearance. They also do not set enlarged font scale. The supplied physical observations provide partial manual coverage of those platform behaviors, without proving every supported API, navigation mode, or cutout orientation.

The existing record reports passing Android `testDebugUnitTest`, `lintDebug`, `assembleDebug`, and `compileDebugAndroidTestKotlin`, and Rust `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo check --all-targets`, and `cargo test --all-targets`. The supplied user evidence also reports successful Gradle JVM 25 sync, physical launch and rotation, readable system icons in light OS mode, reachable enlarged text, and expected `cargo run` output. This review did not rerun builds or tests and did not launch the app.

The connected Android attempt is installation blocked, not a passing or failing execution of `FoundationScreenTest`. Xiaomi rejected installation with `INSTALL_GRANT_RUNTIME_PERMISSIONS`, despite Gradle ending with `BUILD SUCCESSFUL`. No instrumentation assertion result is available. No driver, `joy.cpl`, installer, game, network, or Windows 11 validation is claimed.

Scope was enumerated with `git ls-files --others --exclude-standard`, yielding 90 files before this report, of which Cargo.lock was excluded as assigned. All source, configuration, scripts, and documentation in scope were read; launcher bitmaps and the wrapper jar received file metadata inspection only, not image or bytecode review. No binary provenance verification is claimed. Only this findings file was written. The previous review remains unchanged.
