// Top-level build file where you can add configuration options common to all sub-projects/modules.
// AGP compiles Kotlin itself; this pins the compiler version it uses.
buildscript {
  dependencies {
    classpath(libs.kotlin.gradle.plugin)
  }
}

plugins {
  alias(libs.plugins.android.application) apply false
}
