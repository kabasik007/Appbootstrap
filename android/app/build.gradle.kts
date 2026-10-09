plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
}

val versionNameFromTag = providers.environmentVariable("ANDROID_VERSION_NAME")
    .getOrElse("0.1.0").removePrefix("android-v")
val versionParts = versionNameFromTag.split('.').mapNotNull(String::toIntOrNull)
val monotonicVersionCode = if (versionParts.size == 3 &&
    versionParts[0] in 0..2000 && versionParts[1] in 0..999 && versionParts[2] in 0..999
) versionParts[0] * 1_000_000 + versionParts[1] * 1_000 + versionParts[2] else 1

val signingKeystore = providers.environmentVariable("ANDROID_KEYSTORE_PATH").orNull

android {
    namespace = "dev.appbootstrap.android"
    compileSdk = 36

    defaultConfig {
        applicationId = "dev.appbootstrap.android"
        minSdk = 26
        targetSdk = 36
        versionCode = monotonicVersionCode
        versionName = versionNameFromTag
    }

    if (signingKeystore != null) {
        signingConfigs {
            create("ciRelease") {
                storeFile = file(signingKeystore)
                storePassword = providers.environmentVariable("ANDROID_KEYSTORE_PASSWORD").get()
                keyAlias = providers.environmentVariable("ANDROID_KEY_ALIAS").get()
                keyPassword = providers.environmentVariable("ANDROID_KEY_PASSWORD").get()
            }
        }
    }

    buildTypes {
        getByName("release") {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            if (signingKeystore != null) {
                signingConfig = signingConfigs.getByName("ciRelease")
            }
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    buildFeatures { compose = true }
}
kotlin { jvmToolchain(17) }

dependencies {
    implementation(project(":core:model"))
    implementation(project(":core:data"))
    implementation(project(":feature:home"))
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.lifecycle.viewmodel.compose)
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.compose.material3)
}
