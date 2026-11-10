fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // The Kotlin JNI scaffolding is only used by the Android renderer. iOS
    // drives UIKit through its own C ABI in `src/ios.rs`.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("ios") {
        uniffi_bindgen_kotlin_jni::generate_scaffolding();
    }
}
