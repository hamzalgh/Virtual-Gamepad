plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
}

// Unsigned review builds remain possible. Official builds never use the debug key.
val officialRelease = providers.gradleProperty("officialRelease").orNull == "true"
fun signingValue(name: String): String = providers.environmentVariable(name).orNull
    ?.takeIf { it.isNotBlank() }
    ?: throw GradleException("Official release requires $name from the release owner's environment")

android {
    namespace = "com.hamzalgh.virtualgamepad"
    compileSdk {
        version = release(37)
    }

    defaultConfig {
        applicationId = "com.hamzalgh.virtualgamepad"
        minSdk = 26
        targetSdk = 37
        versionCode = 1
        versionName = "1.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    signingConfigs {
        if (officialRelease) {
            create("official") {
                storeFile = file(signingValue("VG_ANDROID_KEYSTORE"))
                require(storeFile!!.isFile) { "Official release keystore is unavailable" }
                require(!storeFile!!.canonicalFile.toPath().startsWith(rootDir.parentFile.canonicalFile.toPath())) {
                    "Keep the official keystore outside the repository"
                }
                storePassword = signingValue("VG_ANDROID_STORE_PASSWORD")
                keyAlias = signingValue("VG_ANDROID_KEY_ALIAS")
                keyPassword = signingValue("VG_ANDROID_KEY_PASSWORD")
            }
        }
    }
    buildTypes {
        release {
            isDebuggable = false
            if (officialRelease) signingConfig = signingConfigs.getByName("official")
            optimization {
                enable = false
            }
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }
    buildFeatures {
        compose = true
        buildConfig = true
    }
}

dependencies {
    implementation(libs.androidx.datastore.preferences)
    implementation(libs.androidx.lifecycle.runtime.compose)
    implementation(libs.androidx.lifecycle.viewmodel.ktx)
    implementation(libs.kotlinx.coroutines.android)
    testImplementation(libs.kotlinx.coroutines.test)
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.compose.material3)
    implementation(libs.androidx.compose.ui)
    implementation(libs.androidx.compose.ui.graphics)
    implementation(libs.androidx.compose.ui.tooling.preview)
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.lifecycle.runtime.ktx)
    testImplementation(libs.junit)
    androidTestImplementation(platform(libs.androidx.compose.bom))
    androidTestImplementation(libs.androidx.compose.ui.test.junit4)
    androidTestImplementation(libs.androidx.espresso.core)
    androidTestImplementation(libs.androidx.junit)
    debugImplementation(libs.androidx.compose.ui.test.manifest)
    debugImplementation(libs.androidx.compose.ui.tooling)
}

tasks.register("exportReleaseDependencyInventory") {
    description = "Export resolved release dependencies for local notice preparation; no device or signing action"
    doLast {
        val output = layout.buildDirectory.file("reports/release-dependencies.tsv").get().asFile
        output.parentFile.mkdirs()
        val artifacts = configurations.getByName("releaseRuntimeClasspath").resolvedConfiguration.resolvedArtifacts
        output.writeText(artifacts.sortedBy { it.moduleVersion.id.toString() }.joinToString("\n") {
            "${it.moduleVersion.id.group}\t${it.name}\t${it.moduleVersion.id.version}\t${it.file.absolutePath}"
        } + "\n")
    }
}
