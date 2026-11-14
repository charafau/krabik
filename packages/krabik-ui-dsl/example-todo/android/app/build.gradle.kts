plugins {
  alias(libs.plugins.android.application)
}

android {
  namespace = "com.example.krabik_ui_dsl"
  compileSdk = 37

  defaultConfig {
    // Differs from the other shells so all can be installed side by side.
    applicationId = "com.example.krabik_ui_dsl_todo"
    minSdk = 24
    targetSdk = 37
    versionCode = 1
    versionName = "1.0"
    testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
  }

  buildTypes {
    release {
      isMinifyEnabled = false
    }
  }
  // View finders shared by the instrumented tests of every app.
  sourceSets["androidTest"].kotlin.srcDir(project(":krabik-ui").projectDir.resolve("../android-test"))
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

  androidTestImplementation(libs.androidx.recyclerview)
  androidTestImplementation(libs.androidx.test.core)
  androidTestImplementation(libs.androidx.test.ext.junit)
  androidTestImplementation(libs.androidx.test.runner)
  androidTestImplementation(libs.androidx.test.espresso.core)
}

// Builds ../rust for Android and packages the library with the app.
extra["krabikMinSdk"] = android.defaultConfig.minSdk
apply(from = project(":krabik-ui").projectDir.resolve("rust.gradle.kts"))
