// Builds the app's Rust crate for Android and packages it.
//
// Applied by every app shell with `apply(from = ...)`. It expects the crate in
// `../rust` next to the Android project and the app's minimum API level in the
// extra property `krabikMinSdk`.

import org.gradle.api.tasks.TaskProvider
import org.gradle.process.ExecOperations
import java.util.Properties
import javax.inject.Inject

// Every build that packages native libraries first runs `cargo build` for the
// selected Android ABIs and copies the resulting `.so` files into
// `build/rustJniLibs/<debug|release>`, which is registered as a native
// library folder of that build type. Debug and release therefore never
// package each other's library. The ABI defaults to the architecture of the machine
// running Gradle, which matches local emulators; override it with
// `-PkrabikAbis=arm64-v8a,x86_64` (or the KRABIK_ABIS environment variable).

data class RustAndroidTarget(
  val abi: String,
  val target: String,
  val clangPrefix: String,
  val linkerVariable: String,
)

val rustAndroidTargets = listOf(
  RustAndroidTarget(
    abi = "arm64-v8a",
    target = "aarch64-linux-android",
    clangPrefix = "aarch64-linux-android",
    linkerVariable = "CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER",
  ),
  RustAndroidTarget(
    abi = "armeabi-v7a",
    target = "armv7-linux-androideabi",
    clangPrefix = "armv7a-linux-androideabi",
    linkerVariable = "CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER",
  ),
  RustAndroidTarget(
    abi = "x86_64",
    target = "x86_64-linux-android",
    clangPrefix = "x86_64-linux-android",
    linkerVariable = "CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER",
  ),
  RustAndroidTarget(
    abi = "x86",
    target = "i686-linux-android",
    clangPrefix = "i686-linux-android",
    linkerVariable = "CARGO_TARGET_I686_LINUX_ANDROID_LINKER",
  ),
)

/** The ABI most likely to match an emulator running on this machine. */
fun hostArchitectureAbi(): String {
  val architecture = System.getProperty("os.arch").lowercase()
  return if (architecture == "aarch64" || architecture == "arm64") "arm64-v8a" else "x86_64"
}

fun configuredAbis(): List<String> =
  (providers.gradleProperty("krabikAbis").orNull
    ?: providers.environmentVariable("KRABIK_ABIS").orNull
    ?: hostArchitectureAbi())
    .split(',')
    .map { it.trim() }
    .filter { it.isNotEmpty() }

fun cargoPackageName(manifest: File): String {
  var inPackageSection = false
  for (line in manifest.readLines()) {
    val trimmed = line.trim()
    if (trimmed.startsWith("[") && trimmed.endsWith("]")) {
      inPackageSection = trimmed == "[package]"
    } else if (inPackageSection) {
      Regex("""^name\s*=\s*"([^"]+)"""").find(trimmed)?.let { return it.groupValues[1] }
    }
  }
  error("Could not find the [package] name in ${manifest.path}")
}

fun androidSdkDirectory(): File? {
  val fromEnvironment = listOfNotNull(
    System.getenv("ANDROID_HOME"),
    System.getenv("ANDROID_SDK_ROOT"),
  )
  val fromLocalProperties = File(rootProject.projectDir, "local.properties")
    .takeIf { it.isFile }
    ?.let { file ->
      val properties = Properties()
      file.inputStream().use(properties::load)
      properties.getProperty("sdk.dir")
    }

  return (fromEnvironment + listOfNotNull(fromLocalProperties))
    .map(::File)
    .firstOrNull { it.isDirectory }
}

fun compareVersions(left: String, right: String): Int {
  val leftParts = left.split('.')
  val rightParts = right.split('.')
  for (index in 0 until maxOf(leftParts.size, rightParts.size)) {
    val a = leftParts.getOrNull(index)?.toIntOrNull() ?: 0
    val b = rightParts.getOrNull(index)?.toIntOrNull() ?: 0
    if (a != b) return a.compareTo(b)
  }
  return 0
}

fun androidNdkDirectory(): File {
  listOfNotNull(
    System.getenv("ANDROID_NDK_HOME"),
    System.getenv("ANDROID_NDK_ROOT"),
  ).map(::File).firstOrNull { it.isDirectory }?.let { return it }

  val sdk = androidSdkDirectory()
    ?: error("Android SDK not found; set ANDROID_HOME or sdk.dir in local.properties")

  File(sdk, "ndk-bundle").takeIf { it.isDirectory }?.let { return it }

  return File(sdk, "ndk")
    .listFiles()
    ?.filter { it.isDirectory }
    ?.maxWithOrNull { left, right -> compareVersions(left.name, right.name) }
    ?: error("Android NDK not found; set ANDROID_NDK_HOME or install one with the SDK manager")
}

fun ndkPrebuiltDirectory(ndk: File): File {
  val os = System.getProperty("os.name").lowercase()
  val host = when {
    "mac" in os || "darwin" in os -> "darwin"
    "win" in os -> "windows"
    else -> "linux"
  }
  val architecture = System.getProperty("os.arch").lowercase()
  val archHint = if (architecture == "aarch64" || architecture == "arm64") "arm64" else "x86_64"
  val candidates = File(ndk, "toolchains/llvm/prebuilt")
    .listFiles()
    ?.filter { it.isDirectory && it.name.startsWith(host) }
    .orEmpty()

  return candidates.firstOrNull { it.name.contains(archHint) }
    ?: candidates.firstOrNull()
    ?: error("No $host NDK toolchain found in ${File(ndk, "toolchains/llvm/prebuilt").path}")
}

fun ndkClang(prebuilt: File, prefix: String, api: Int): File {
  val name = "$prefix$api-clang"
  return listOf(File(prebuilt, "bin/$name"), File(prebuilt, "bin/$name.cmd"))
    .firstOrNull { it.isFile }
    ?: error("NDK compiler $name not found in ${File(prebuilt, "bin").path}")
}

abstract class BuildRustLibs : DefaultTask() {
  @get:Input
  abstract val abis: ListProperty<String>

  @get:Input
  abstract val triples: ListProperty<String>

  @get:Input
  abstract val linkers: ListProperty<String>

  @get:Input
  abstract val linkerVariables: ListProperty<String>

  @get:Input
  abstract val profile: Property<String>

  @get:Input
  abstract val manifest: Property<String>

  @get:Input
  abstract val libraryName: Property<String>

  @get:InputDirectory
  abstract val sources: DirectoryProperty

  /**
   * Sources and manifests of the crates the rust crate statically includes:
   * the framework and its macro crate. A change in any of them must rebuild
   * the library.
   */
  @get:InputFiles
  abstract val dependencySources: ConfigurableFileCollection

  @get:InputFile
  abstract val cargoToml: RegularFileProperty

  @get:Optional
  @get:InputFile
  abstract val buildScript: RegularFileProperty

  @get:Optional
  @get:InputFile
  abstract val cargoLock: RegularFileProperty

  @get:OutputDirectory
  abstract val outputDirectory: DirectoryProperty

  @get:Inject
  abstract val execOperations: ExecOperations

  @TaskAction
  fun build() {
    val manifestFile = File(manifest.get())
    val crateDirectory = manifestFile.parentFile
    val outputRoot = outputDirectory.get().asFile
    outputRoot.mkdirs()

    // Drop libraries of ABIs this build does not target, they may be stale.
    val wanted = abis.get().toSet()
    outputRoot.listFiles()
      ?.filter { it.isDirectory && it.name !in wanted }
      ?.forEach { it.deleteRecursively() }

    abis.get().forEachIndexed { index, abi ->
      execOperations.exec {
        workingDir = crateDirectory
        environment(linkerVariables.get()[index], linkers.get()[index])
        val arguments = mutableListOf(
          "cargo",
          "build",
          "--manifest-path",
          manifestFile.path,
          "--target",
          triples.get()[index],
        )
        if (profile.get() == "release") {
          arguments += "--release"
        }
        commandLine(*arguments.toTypedArray())
      }

      val library = File(
        crateDirectory,
        "target/${triples.get()[index]}/${profile.get()}/lib${libraryName.get()}.so",
      )
      val abiDirectory = File(outputRoot, abi).apply { mkdirs() }
      library.copyTo(File(abiDirectory, library.name), overwrite = true)
    }
  }
}

val rustCrateDirectory = File(rootProject.projectDir.parentFile, "rust")
/** The framework's Rust crate, next to this script's module. */
val frameworkCrate = File(project(":krabik-ui").projectDir.parentFile.parentFile, "rust")
val rustManifest = File(rustCrateDirectory, "Cargo.toml")
check(rustManifest.isFile) {
  "Rust crate not found at ${rustManifest.path}; this Android project expects it in ../rust"
}

val rustLibraryName = cargoPackageName(rustManifest).replace('-', '_')
val rustAbis = configuredAbis()
val rustTargets = rustAbis.map { abi ->
  rustAndroidTargets.firstOrNull { it.abi == abi }
    ?: error("Unsupported ABI '$abi'. Choose from: ${rustAndroidTargets.joinToString { it.abi }}")
}

val rustNdkPrebuilt = ndkPrebuiltDirectory(androidNdkDirectory())
// The NDK compiler is chosen for the app's minimum API level.
val rustLinkerApi = (extra["krabikMinSdk"] as? Int) ?: 24

fun registerRustBuildTask(name: String, buildProfile: String): TaskProvider<BuildRustLibs> =
  tasks.register<BuildRustLibs>(name) {
    group = "rust"
    description = "Builds the Rust library ($buildProfile) into build/rustJniLibs/$buildProfile."
    abis.set(rustAbis)
    triples.set(rustTargets.map { it.target })
    linkerVariables.set(rustTargets.map { it.linkerVariable })
    linkers.set(rustTargets.map { ndkClang(rustNdkPrebuilt, it.clangPrefix, rustLinkerApi).path })
    profile.set(buildProfile)
    manifest.set(rustManifest.path)
    libraryName.set(rustLibraryName)
    sources.set(File(rustCrateDirectory, "src"))
    // An app crate statically includes the framework and its macros, so a
    // change in either must also rebuild it.
    if (frameworkCrate.isDirectory && frameworkCrate != rustCrateDirectory) {
      dependencySources.from(
        fileTree(File(frameworkCrate, "src")),
        fileTree(File(frameworkCrate, "macros/src")),
        File(frameworkCrate, "Cargo.toml"),
        File(frameworkCrate, "macros/Cargo.toml"),
      )
    }
    cargoToml.set(rustManifest)
    File(rustCrateDirectory, "build.rs").takeIf { it.isFile }?.let { buildScript.set(it) }
    File(rustCrateDirectory, "Cargo.lock").takeIf { it.isFile }?.let { cargoLock.set(it) }
    outputDirectory.set(layout.buildDirectory.dir("rustJniLibs/$buildProfile"))
  }

val rustBuildTasks = mapOf(
  "debug" to registerRustBuildTask("buildRustLibsDebug", "debug"),
  "release" to registerRustBuildTask("buildRustLibsRelease", "release"),
)

// Each build type packages the folder its own task writes. The Android
// extension is reached by name because this script is applied, not a plugin,
// and has no typed accessor for it.
extensions.getByName("android").withGroovyBuilder {
  val sourceSets = getProperty("sourceSets") as NamedDomainObjectContainer<*>
  rustBuildTasks.keys.forEach { buildType ->
    val sourceSet = sourceSets.getByName(buildType) ?: return@forEach
    val jniLibs = sourceSet.withGroovyBuilder { getProperty("jniLibs") } ?: return@forEach
    // A plain directory: the source-set API refuses providers. The task
    // dependency comes from the hook on the merge tasks below.
    val directory = layout.buildDirectory.dir("rustJniLibs/$buildType").get().asFile
    jniLibs.withGroovyBuilder { "srcDir"(directory) }
  }
}

// The folder merge runs before the native library merge, so hook both.
tasks.matching {
  it.name.startsWith("merge") &&
    (it.name.endsWith("JniLibFolders") || it.name.endsWith("NativeLibs"))
}.configureEach {
  val buildProfile = if (name.contains("Release")) "release" else "debug"
  rustBuildTasks[buildProfile]?.let { rustTask -> dependsOn(rustTask) }
}
