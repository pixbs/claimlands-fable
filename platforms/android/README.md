# platforms/android

Gradle project that wraps `crates/cl-app` as a `GameActivity` app. `cargo xtask android` runs
`cargo ndk -t arm64-v8a` into `app/src/main/jniLibs` and then `gradle assembleDebug` when Gradle is
installed; CI (`mobile.yml`) does both on every push to `main` and nightly.

Requirements locally: Android SDK with platform 35 and an NDK (`sdkmanager "ndk;27.2.12479018"`),
JDK 17, `cargo install cargo-ndk`, `rustup target add aarch64-linux-android`.

`androidx.games:games-activity` in `app/build.gradle.kts` must match the GameActivity version
bundled by the `android-activity` crate; a mismatch shows as a crash at start-up, not at build time.
