//! The project template of a krabik-ui-dsl app.
//!
//! The framework is not published anywhere yet, so a project refers to a
//! local checkout of it by path: the Rust crate, the Kotlin and Swift hosts
//! and the build scripts all stay in the checkout. The project only holds the
//! app's crate and two thin platform shells.

use anyhow::{Context, Result, anyhow};

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::naming::ProjectName;
use crate::run_command;

/// The krabik-ui-dsl directory of the checkout this CLI was built from.
const DEFAULT_DSL_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../krabik-ui-dsl");

/// The app of the framework whose Android shell is used as the template.
const TEMPLATE_APP: &str = "example-todo-minimal";

/// What a krabik-ui-dsl directory has to contain to create a project from it.
const REQUIRED: [&str; 5] = [
    "rust/Cargo.toml",
    "shared/android/rust.gradle.kts",
    "shared/ios/KrabikIOS.h",
    "shared/build-rust-ios.sh",
    "example-todo-minimal/android/gradlew",
];

/// The parts of the template's Android project that are the same in every
/// app: the Gradle wrapper and settings, the manifest and the resources.
///
/// The list is explicit, so that build directories and the template's own
/// sources and tests are never copied.
const ANDROID_BOILERPLATE: [&str; 8] = [
    "gradlew",
    "gradlew.bat",
    "gradle",
    "gradle.properties",
    "build.gradle.kts",
    ".gitignore",
    "app/src/main/AndroidManifest.xml",
    "app/src/main/res",
];

/// Characters that cannot be written into every file that names the
/// framework's directory: TOML, Kotlin, YAML, a shell script and a C header.
const UNSUPPORTED_PATH_CHARS: [char; 6] = ['"', '\\', '$', '`', '\n', '#'];

/// Finds the krabik-ui-dsl directory a project is created from.
///
/// `path` is the value of `--dsl-path`. Without it the checkout this CLI was
/// built from is used. The result is absolute, because it is written into
/// the project.
pub fn resolve_dsl_path(path: Option<&Path>) -> Result<String> {
    let given = path.is_some();
    let path = path.map_or_else(|| PathBuf::from(DEFAULT_DSL_PATH), Path::to_path_buf);

    let hint = if given {
        "check --dsl-path"
    } else {
        "pass the krabik-ui-dsl directory of a krabik checkout with --dsl-path"
    };

    let dsl = fs::canonicalize(&path)
        .map_err(|_| anyhow!("krabik-ui-dsl not found at {}; {hint}", path.display()))?;

    if let Some(missing) = REQUIRED.iter().find(|file| !dsl.join(file).exists()) {
        return Err(anyhow!(
            "{} is not a krabik-ui-dsl directory ({missing} is missing); {hint}",
            dsl.display()
        ));
    }

    let dsl = dsl
        .into_os_string()
        .into_string()
        .map_err(|path| anyhow!("the path {} is not valid UTF-8", path.to_string_lossy()))?;

    if let Some(bad) = dsl.chars().find(|c| UNSUPPORTED_PATH_CHARS.contains(c)) {
        return Err(anyhow!(
            "the path {dsl} contains {bad:?}, which a project cannot refer to; \
             move the krabik checkout"
        ));
    }

    Ok(dsl)
}

/// Creates the Rust, Android and iOS parts of a krabik-ui-dsl app in `root`.
///
/// `dsl` is the directory returned by [`resolve_dsl_path`].
pub fn create_dsl_project(
    name: &ProjectName,
    package_prefix: &str,
    root: &Path,
    dsl: &str,
) -> Result<()> {
    create_rust(name, package_prefix, root, dsl)?;
    create_android(name, package_prefix, root, dsl)?;
    create_ios(name, package_prefix, root, dsl)?;
    Ok(())
}

/// Writes a file, creating the directories above it.
fn write(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(path, content).with_context(|| format!("failed to write {}", path.display()))
}

/// Copies a file or a directory with everything in it.
fn copy_recursively(from: &Path, to: &Path) -> Result<()> {
    if from.is_dir() {
        fs::create_dir_all(to).with_context(|| format!("failed to create {}", to.display()))?;
        let entries =
            fs::read_dir(from).with_context(|| format!("failed to read {}", from.display()))?;
        for entry in entries {
            let entry = entry.with_context(|| format!("failed to read {}", from.display()))?;
            copy_recursively(&entry.path(), &to.join(entry.file_name()))?;
        }
        return Ok(());
    }

    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    // this keeps the permissions, `gradlew` has to stay executable
    fs::copy(from, to)
        .with_context(|| format!("failed to copy {} to {}", from.display(), to.display()))?;
    Ok(())
}

/// Creates the app's crate.
fn create_rust(name: &ProjectName, package_prefix: &str, root: &Path, dsl: &str) -> Result<()> {
    let rust_dir = root.join("rust");

    write(
        &rust_dir.join("Cargo.toml"),
        &cargo_toml(name.package_name(), dsl),
    )?;
    write(&rust_dir.join("src/lib.rs"), &lib_rs(name, package_prefix))?;
    write(&rust_dir.join(".gitignore"), "/target\n")?;

    Ok(())
}

/// The manifest of the app's crate.
fn cargo_toml(package_name: &str, dsl: &str) -> String {
    format!(
        r#"[package]
name = "{package_name}"
version = "0.1.0"
edition = "2024"

[lib]
# The staticlib is linked into the iOS app, the cdylib is loaded by the
# Android app.
crate-type = ["staticlib", "cdylib"]

[dependencies]
# The framework is used from a local krabik checkout. `hot-reload` lets
# `krabik watch` put new code into the running app during development.
krabik-ui-dsl = {{ path = "{dsl}/rust", features = ["hot-reload"] }}
jni = "0.22"

# The crate is a project of its own. The empty table keeps cargo from
# adopting a workspace in a directory above.
[workspace]
"#
    )
}

/// The app: a counter, and the two functions the hosts call to install it.
fn lib_rs(name: &ProjectName, package_prefix: &str) -> String {
    let install = install_function(name);
    let jni_install = format!(
        "{}_AppBridge_install",
        name.jni_symbol_prefix(package_prefix)
    );

    format!(
        r#"//! The app. `krabik run` builds and starts it, `krabik watch` puts every
//! change into the running app.

use jni::EnvUnowned;
use jni::objects::JObject;
use krabik_ui_dsl::app::{{App, Component, set_app_factory}};
use krabik_ui_dsl::command::Command;
use krabik_ui_dsl::ui::{{Button, Column, Element, Text}};

/// Everything the user can do in the app.
#[derive(Clone, Debug, PartialEq)]
pub enum Msg {{
    Increment,
    Reset,
}}

/// The state of the app.
pub struct Counter {{
    count: i64,
}}

impl Counter {{
    pub fn new() -> Self {{
        Self {{ count: 0 }}
    }}

    /// Registers the app with the framework. The hosts call this at startup.
    pub fn install() {{
        set_app_factory(Self::new);
    }}
}}

impl Default for Counter {{
    fn default() -> Self {{
        Self::new()
    }}
}}

impl Component for Counter {{
    type Msg = Msg;

    fn update(&mut self, msg: Msg) -> Command<Msg> {{
        match msg {{
            Msg::Increment => self.count += 1,
            Msg::Reset => self.count = 0,
        }}
        Command::none()
    }}

    fn view(&self) -> Element<Msg> {{
        Column::new()
            .padding(24)
            .spacing(12)
            .child(Text::new(format!("Count: {{}}", self.count)).size(24).bold())
            .child(Button::new("+1").on_click(Msg::Increment))
            .child(Button::new("Reset").on_click(Msg::Reset))
            .into()
    }}
}}

impl App for Counter {{}}

// Lets `krabik watch` hand a rebuilt library to the running app. The state
// is kept as long as the layout of `Counter` and `Msg` stays the same.
krabik_ui_dsl::hot_reload!(Counter);

/// Called by the iOS host before it creates the root view.
#[unsafe(no_mangle)]
pub extern "C" fn {install}() {{
    Counter::install();
}}

/// Called by the Android host before it creates the root view.
#[unsafe(no_mangle)]
pub extern "system" fn {jni_install}<'caller>(
    _env: EnvUnowned<'caller>,
    _this: JObject<'caller>,
) {{
    Counter::install();
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn counts_and_resets() {{
        let mut app = Counter::new();
        let _ = app.update(Msg::Increment);
        let _ = app.update(Msg::Increment);
        assert_eq!(app.count, 2);

        let _ = app.update(Msg::Reset);
        assert_eq!(app.count, 0);
    }}
}}
"#
    )
}

/// The name of the C function that installs the app, e.g. `my_app_install`.
fn install_function(name: &ProjectName) -> String {
    format!("{}_install", name.identifier())
}

/// Creates the Android shell.
fn create_android(name: &ProjectName, package_prefix: &str, root: &Path, dsl: &str) -> Result<()> {
    let android_dir = root.join("android");
    let template = Path::new(dsl).join(TEMPLATE_APP).join("android");

    for part in ANDROID_BOILERPLATE {
        copy_recursively(&template.join(part), &android_dir.join(part))?;
    }

    let package = name.java_package(package_prefix);
    let sources = android_dir
        .join("app/src/main/java")
        .join(name.java_package_path(package_prefix));

    write(
        &android_dir.join("settings.gradle.kts"),
        &settings_gradle(name.dir_name(), dsl),
    )?;
    write(
        &android_dir.join("app/build.gradle.kts"),
        &app_build_gradle(&package),
    )?;
    write(
        &android_dir.join("app/src/main/res/values/strings.xml"),
        &strings_xml(name.dir_name()),
    )?;
    write(
        &sources.join("MainActivity.kt"),
        &main_activity(&package, name.identifier()),
    )?;
    write(&sources.join("AppBridge.kt"), &app_bridge(&package))?;

    Ok(())
}

/// The Gradle settings: the app, and the framework's library as a module.
fn settings_gradle(project: &str, dsl: &str) -> String {
    format!(
        r#"pluginManagement {{
  repositories {{
    google {{
      content {{
        includeGroupByRegex("androidx.*")
        includeGroupByRegex("com\\.android.*")
        includeGroupByRegex("com\\.google.*")
      }}
    }}
    mavenCentral()
    gradlePluginPortal()
  }}
}}

dependencyResolutionManagement {{
  repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
  repositories {{
    google {{
      content {{
        includeGroupByRegex("androidx.*")
        includeGroupByRegex("com\\.android.*")
        includeGroupByRegex("com\\.google.*")
      }}
    }}
    mavenCentral()
  }}
}}

plugins {{
  id("org.gradle.toolchains.foojay-resolver-convention") version "1.0.0"
}}

rootProject.name = "{project}"

include(":app")

// The framework's Android library, in a local krabik checkout.
include(":krabik-ui")
project(":krabik-ui").projectDir = file("{dsl}/shared/android")
"#
    )
}

/// The Gradle build of the app module.
fn app_build_gradle(package: &str) -> String {
    format!(
        r#"plugins {{
  alias(libs.plugins.android.application)
}}

android {{
  namespace = "{package}"
  compileSdk = 37

  defaultConfig {{
    applicationId = "{package}"
    minSdk = 24
    targetSdk = 37
    versionCode = 1
    versionName = "1.0"
  }}

  buildTypes {{
    release {{
      isMinifyEnabled = false
    }}
  }}
  compileOptions {{
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
  }}
}}

kotlin {{
  jvmToolchain(17)
}}

dependencies {{
  // The framework's Android half.
  implementation(project(":krabik-ui"))
}}

// Builds ../rust for Android and packages the library with the app.
extra["krabikMinSdk"] = android.defaultConfig.minSdk
apply(from = project(":krabik-ui").projectDir.resolve("rust.gradle.kts"))
"#
    )
}

/// The string resources: the name of the app.
fn strings_xml(app_name: &str) -> String {
    format!("<resources>\n    <string name=\"app_name\">{app_name}</string>\n</resources>\n")
}

/// The activity: it loads the app's library and installs the app.
fn main_activity(package: &str, lib_name: &str) -> String {
    format!(
        r#"package {package}

import dev.krabik.ui.KrabikActivity

/** Loads the app's library, which includes the framework, and installs the app. */
class MainActivity : KrabikActivity() {{
    override fun loadNativeLibrary() {{
        System.loadLibrary("{lib_name}")
    }}

    override fun installApp() {{
        AppBridge.install()
    }}
}}
"#
    )
}

/// The Kotlin side of the JNI function that installs the app.
fn app_bridge(package: &str) -> String {
    format!(
        r#"package {package}

/** Installs the app. [MainActivity] calls this once the library is loaded. */
object AppBridge {{
    /** Registers the app with the framework; implemented in `rust/src/lib.rs`. */
    external fun install()
}}
"#
    )
}

/// Creates the iOS shell and generates its Xcode project.
fn create_ios(name: &ProjectName, package_prefix: &str, root: &Path, dsl: &str) -> Result<()> {
    let ios_dir = root.join("ios");
    let app = name.swift_type_name();
    let install = install_function(name);

    write(
        &ios_dir.join("project.yml"),
        &project_yml(
            &app,
            name.identifier(),
            package_prefix,
            &name.bundle_id(package_prefix),
            dsl,
        ),
    )?;
    write(&ios_dir.join("Signing.xcconfig"), SIGNING_XCCONFIG)?;
    write(
        &ios_dir.join("Sources/Bridging.h"),
        &bridging_header(&install, dsl),
    )?;
    write(
        &ios_dir.join("Sources").join(format!("{app}App.swift")),
        &app_swift(&app, &install),
    )?;
    write(&ios_dir.join(".gitignore"), IOS_GITIGNORE)?;

    let mut cmd = Command::new("xcodegen");
    cmd.arg("generate").current_dir(&ios_dir);
    run_command(&mut cmd, "xcodegen generate")
}

/// The Xcode project is regenerated by `xcodegen`, the rest is local.
const IOS_GITIGNORE: &str = "build/\nDerivedData/\n*.xcodeproj\nLocal.xcconfig\n.DS_Store\n";

const SIGNING_XCCONFIG: &str = r#"// Signing settings for this app. Create `Local.xcconfig` next to this file
// with `DEVELOPMENT_TEAM = <your team id>` to build for a device; it is
// ignored by git.
#include? "Local.xcconfig"
"#;

/// The XcodeGen specification of the app.
///
/// The first `name:` and the first `PRODUCT_BUNDLE_IDENTIFIER:` are read by
/// `run` and `watch`.
fn project_yml(
    app: &str,
    lib_name: &str,
    package_prefix: &str,
    bundle_id: &str,
    dsl: &str,
) -> String {
    format!(
        r#"# The Xcode project is generated from this file: `krabik run --platform ios`
# does that, or run `xcodegen generate` here.
name: {app}
options:
  bundleIdPrefix: {package_prefix}
  deploymentTarget:
    iOS: "16.0"
  createIntermediateGroups: true

# `Local.xcconfig` is optional: put DEVELOPMENT_TEAM = <your team id> there to
# run on a device. The simulator needs no signing identity.
configFiles:
  Debug: Signing.xcconfig
  Release: Signing.xcconfig
settings:
  base:
    SWIFT_VERSION: "5.9"
    GENERATE_INFOPLIST_FILE: YES
    CODE_SIGN_STYLE: Automatic
    INFOPLIST_KEY_UILaunchScreen_Generation: YES
    INFOPLIST_KEY_UIApplicationSceneManifest_Generation: YES
    # The Rust build phase runs cargo, which reads and writes outside the
    # script's declared inputs.
    ENABLE_USER_SCRIPT_SANDBOXING: NO

targets:
  {app}:
    type: application
    platform: iOS
    sources:
      - path: Sources
      # The framework's Swift host, in a local krabik checkout.
      - path: "{dsl}/shared/ios"
    preBuildScripts:
      # Builds ../rust so the app never links a stale library.
      - name: Build Rust library
        basedOnDependencyAnalysis: false
        script: |
          "{dsl}/shared/build-rust-ios.sh"
    settings:
      base:
        PRODUCT_BUNDLE_IDENTIFIER: {bundle_id}
        PRODUCT_NAME: {app}
        RUST_LIB: {lib_name}
        SWIFT_OBJC_BRIDGING_HEADER: Sources/Bridging.h
        LIBRARY_SEARCH_PATHS:
          - $(SRCROOT)/../rust/target/xcode/$(PLATFORM_NAME)-$(CONFIGURATION)
        OTHER_LDFLAGS:
          - -l$(RUST_LIB)
"#
    )
}

/// The header that shows the framework's C interface and the app's install
/// function to Swift.
fn bridging_header(install: &str, dsl: &str) -> String {
    format!(
        r#"// Exposes the framework's C interface and this app's install function to Swift.

#include "{dsl}/shared/ios/KrabikIOS.h"

/// Registers the app with the framework. Call once at startup, before any
/// root view is created. Implemented in `rust/src/lib.rs`.
void {install}(void);
"#
    )
}

/// The SwiftUI entry point. `ContentView` is part of the framework's host.
fn app_swift(app: &str, install: &str) -> String {
    format!(
        r#"import SwiftUI

@main
struct {app}App: App {{
    init() {{
        // Registers the app with the framework before any view is created.
        {install}()
    }}

    var body: some Scene {{
        WindowGroup {{
            ContentView()
        }}
    }}
}}
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rust;

    const DSL: &str = "/work/krabik/packages/krabik-ui-dsl";

    #[test]
    fn test_cargo_toml_depends_on_the_framework_with_hot_reload() {
        let toml = cargo_toml("my-app", DSL);
        assert!(toml.contains("name = \"my-app\""), "{toml}");
        assert!(toml.contains("crate-type = [\"staticlib\", \"cdylib\"]"));

        // `run` and `watch` recognise the project by this dependency
        let dependency = rust::dsl_dependency(&toml).expect("the dependency is declared");
        assert!(dependency.contains("/work/krabik/packages/krabik-ui-dsl/rust"));
        assert!(crate::hot::is_hot_reload_project(&toml));
    }

    #[test]
    fn test_lib_rs_exports_the_install_functions() {
        let name = ProjectName::parse("my-app").unwrap();
        let lib = lib_rs(&name, "com.example");

        assert!(
            lib.contains("pub extern \"C\" fn my_app_install()"),
            "{lib}"
        );
        assert!(
            lib.contains(
                "pub extern \"system\" fn Java_com_example_my_1app_AppBridge_install<'caller>("
            ),
            "{lib}"
        );
        assert!(lib.contains("krabik_ui_dsl::hot_reload!(Counter);"));
        // the braces of the template are not left doubled
        assert!(!lib.contains("{{"), "{lib}");
    }

    #[test]
    fn test_android_files_agree_on_the_package() {
        let gradle = app_build_gradle("io.acme.my_app");
        assert!(gradle.contains("namespace = \"io.acme.my_app\""));
        assert!(gradle.contains("applicationId = \"io.acme.my_app\""));

        let activity = main_activity("io.acme.my_app", "my_app");
        assert!(activity.starts_with("package io.acme.my_app\n"));
        assert!(activity.contains("System.loadLibrary(\"my_app\")"));

        assert!(app_bridge("io.acme.my_app").starts_with("package io.acme.my_app\n"));
    }

    #[test]
    fn test_settings_gradle_includes_the_framework() {
        let settings = settings_gradle("my-app", DSL);
        assert!(settings.contains("rootProject.name = \"my-app\""));
        assert!(settings.contains(
            "project(\":krabik-ui\").projectDir = \
             file(\"/work/krabik/packages/krabik-ui-dsl/shared/android\")"
        ));
        // the backslashes of the regular expressions are kept
        assert!(settings.contains(r#"includeGroupByRegex("com\\.android.*")"#));
    }

    #[test]
    fn test_project_yml() {
        let yml = project_yml("MyApp", "my_app", "com.example", "com.example.my-app", DSL);
        assert!(yml.contains("\nname: MyApp\n"));
        assert!(yml.contains("PRODUCT_BUNDLE_IDENTIFIER: com.example.my-app"));
        assert!(yml.contains("RUST_LIB: my_app"));
        assert!(yml.contains("- path: \"/work/krabik/packages/krabik-ui-dsl/shared/ios\""));
        assert!(yml.contains("\"/work/krabik/packages/krabik-ui-dsl/shared/build-rust-ios.sh\""));
    }

    #[test]
    fn test_ios_sources_use_the_install_function() {
        let header = bridging_header("my_app_install", DSL);
        assert!(header.contains("void my_app_install(void);"));
        assert!(
            header.contains(
                "#include \"/work/krabik/packages/krabik-ui-dsl/shared/ios/KrabikIOS.h\""
            )
        );

        let swift = app_swift("MyApp", "my_app_install");
        assert!(swift.contains("struct MyAppApp: App {"));
        assert!(swift.contains("my_app_install()"));
    }

    #[test]
    fn test_resolve_dsl_path_defaults_to_the_checkout() {
        let dsl = resolve_dsl_path(None).unwrap();
        assert!(Path::new(&dsl).is_absolute());
        assert!(dsl.ends_with("krabik-ui-dsl"), "{dsl}");
    }

    #[test]
    fn test_resolve_dsl_path_rejects_other_directories() {
        let dir = std::env::temp_dir();
        let err = resolve_dsl_path(Some(&dir)).unwrap_err().to_string();
        assert!(err.contains("is not a krabik-ui-dsl directory"), "{err}");
        assert!(err.contains("--dsl-path"), "{err}");

        let err = resolve_dsl_path(Some(Path::new("/does/not/exist")))
            .unwrap_err()
            .to_string();
        assert!(err.contains("not found"), "{err}");
    }
}
