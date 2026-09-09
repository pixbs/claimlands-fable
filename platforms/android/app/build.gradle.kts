plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "com.pixbs.claimlands"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.pixbs.claimlands"
        minSdk = 28
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"
        ndk {
            // cargo xtask android builds arm64-v8a into src/main/jniLibs.
            abiFilters += listOf("arm64-v8a")
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }
}

dependencies {
    // Must stay in step with the GameActivity version bundled by the `android-activity` crate.
    implementation("androidx.games:games-activity:3.0.5")
    // GameActivity's supertypes; the Kotlin compiler needs them on the classpath explicitly.
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("androidx.core:core-ktx:1.13.1")
}
