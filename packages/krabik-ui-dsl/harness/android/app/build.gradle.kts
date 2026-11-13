plugins {
  alias(libs.plugins.android.application)
}

android {
  namespace = "com.example.krabik_ui_dsl"
  compileSdk = 37

  defaultConfig {
    // The dev harness: runs the built-in gallery.
    applicationId = "com.example.krabik_ui_dsl"
    minSdk = 24
    targetSdk = 37
    versionCode = 1
    versionName = "1.0"
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
}

kotlin {
  jvmToolchain(17)
}

dependencies {
  // The framework's Android half (shared/android).
  implementation(project(":krabik-ui"))

  androidTestImplementation(libs.androidx.test.core)
  androidTestImplementation(libs.androidx.test.ext.junit)
  androidTestImplementation(libs.androidx.test.runner)
  androidTestImplementation(libs.androidx.test.espresso.core)
}

// Builds ../rust for Android and packages the library with the app.
extra["krabikMinSdk"] = android.defaultConfig.minSdk
apply(from = project(":krabik-ui").projectDir.resolve("rust.gradle.kts"))
