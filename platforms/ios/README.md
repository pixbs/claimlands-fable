# platforms/ios

`project.yml` is an xcodegen spec; the generated `ClaimLands.xcodeproj` is not committed. The
Xcode target's pre-build step (`build-rust.sh`) compiles `crates/cl-app` as a static library for the
device or simulator target and links it; `Sources/main.m` calls `claimlands_main` from
`crates/cl-app/src/platform.rs`.

```bash
brew install xcodegen
cargo xtask ios            # cargo build for device + simulator, then xcodegen + xcodebuild on macOS
```

CI (`mobile.yml`) builds unsigned on `macos-latest` after every push to `main` and nightly. Signing,
TestFlight and store assets are M7 work.
