package com.pixbs.claimlands

import com.google.androidgamesdk.GameActivity

/** Hosts the Rust application; `android_main` in `crates/cl-app/src/platform.rs` takes over. */
class MainActivity : GameActivity() {
    companion object {
        init {
            System.loadLibrary("cl_app")
        }
    }
}
