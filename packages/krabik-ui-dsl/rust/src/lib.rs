uniffi::setup_scaffolding!();

include!(concat!(env!("OUT_DIR"), "/uniffi_bindgen_kotlin_jni.uniffi.rs"));

#[uniffi::export]
pub fn get_greeting() -> String {
    "Hello from Rust!".to_string()
}
