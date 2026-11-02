plugins {
    alias(libs.plugins.android.application)
}

android {
    namespace = "com.example.krabik_ui_example"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.example.krabik_ui_example"
        minSdk = 24
        targetSdk = 36
        versionCode = 1
        versionName = "1.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildFeatures {
        buildConfig = false
    }

    packaging {
        resources {
            excludes += "/META-INF/{AL2.0,LGPL2.1}"
        }
    }

    sourceSets {
        getByName("main") {
            // The Rust build task writes libkrabik_ui_example.so for each ABI here.
            val rustJniLibsDir = layout.buildDirectory.get().asFile.resolve("rustJniLibs")
            jniLibs.directories.add(rustJniLibsDir.absolutePath)
        }
    }
}

kotlin {
    jvmToolchain(17)
}

dependencies {
    implementation(libs.androidx.appcompat)
    implementation(libs.androidx.recyclerview)
    implementation(libs.androidx.viewpager2)
    implementation(libs.material)

    androidTestImplementation(libs.androidx.test.ext.junit)
    androidTestImplementation(libs.androidx.test.core)
    androidTestImplementation(libs.androidx.test.runner)
    androidTestImplementation(libs.androidx.test.espresso.core)
}

val repoRoot = rootProject.projectDir.parentFile
val rustDir = repoRoot.resolve("rust")
val libraryRoot = repoRoot.parentFile
val rustJniLibs = layout.buildDirectory.dir("rustJniLibs")

val buildRust by tasks.registering(Exec::class) {
    group = "rust"
    description = "Builds the Rust JNI library for Android and stages it in jniLibs."

    workingDir = repoRoot
    commandLine(
        "bash",
        rustDir.resolve("build-android.sh").absolutePath,
        rustJniLibs.get().asFile.absolutePath,
    )

    inputs.dir(rustDir.resolve("src"))
    inputs.file(rustDir.resolve("Cargo.toml"))
    inputs.file(rustDir.resolve("Cargo.lock"))
    inputs.file(rustDir.resolve("build-android.sh"))
    // The example consumes the library at the repository root. Watch it too so the staged
    // shared library is rebuilt when only the library changes.
    inputs.dir(libraryRoot.resolve("src"))
    inputs.file(libraryRoot.resolve("Cargo.toml"))
    outputs.dir(rustJniLibs)
}

tasks.named("preBuild") {
    dependsOn(buildRust)
}
