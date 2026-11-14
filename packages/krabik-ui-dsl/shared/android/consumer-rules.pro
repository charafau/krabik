# Keep rules that travel with the library into every app that uses it.
#
# Rust reaches this package through JNI by name: it looks up classes
# (`dev/krabik/ui/KrabikSwitch`), their constructors and methods
# (`setCheckedSilently`, `(Z)V`), and the JVM looks up the `external`
# functions of RustRootView by their mangled names. R8 cannot see any of
# those uses, so without these rules it would rename or remove them and the
# app would stop at launch with "the Rust session could not be created".
#
# The package is small and nearly all of it is called from Rust, so it is
# kept whole.
-keep class dev.krabik.ui.** { *; }

# `external` functions, in this package or in an app's own classes.
-keepclasseswithmembernames,includedescriptorclasses class * {
    native <methods>;
}
