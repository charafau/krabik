// The framework's Android half: `RustRootView`, the host controller and the
// native widgets Rust drives. Each app shell includes this directory as the
// `:krabik-ui` module.

plugins {
  alias(libs.plugins.android.library)
}

android {
  namespace = "dev.krabik.ui"
  compileSdk = 37

  defaultConfig {
    minSdk = 24
  }

  compileOptions {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
  }
}

kotlin {
  jvmToolchain(17)
}

// Several app builds include this same directory; keep their outputs apart.
layout.buildDirectory.set(rootProject.layout.buildDirectory.dir("krabik-ui"))

dependencies {
  // Part of the public API: apps subclass `KrabikActivity`.
  api(libs.androidx.core.ktx)
  api(libs.androidx.fragment)
  api(libs.androidx.activity.ktx)
  api(libs.material)

  implementation(libs.androidx.recyclerview)
  implementation(libs.coil)
  implementation(libs.coil.network.okhttp)
}
