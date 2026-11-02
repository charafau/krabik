//! Validation of the project name and derivation of the identifiers that the
//! generated Android, iOS and Rust code depends on.
//!
//! A project name has to work as a Cargo package name, as a Rust crate
//! identifier, as a Java/Kotlin package segment and as part of an iOS bundle
//! identifier. Deriving each of those spellings independently is how
//! `my-app` used to produce the invalid Kotlin package `com.example.my-app`.
//! Every identifier is therefore derived here, once, from a single validated
//! name.

use anyhow::{Result, anyhow};

/// Maximum number of characters allowed in a project name.
const MAX_NAME_LEN: usize = 64;

/// Rust keywords that cannot be used as a crate name.
const RUST_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "gen", "if", "impl", "in", "let", "loop", "macro", "match", "mod",
    "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait",
    "true", "try", "type", "typeof", "unsafe", "unsized", "use", "where", "while", "yield",
];

/// Java keywords and reserved literals that cannot be used as a package segment.
const JAVA_KEYWORDS: &[&str] = &[
    "abstract",
    "assert",
    "boolean",
    "break",
    "byte",
    "case",
    "catch",
    "char",
    "class",
    "const",
    "continue",
    "default",
    "do",
    "double",
    "else",
    "enum",
    "extends",
    "false",
    "final",
    "finally",
    "float",
    "for",
    "goto",
    "if",
    "implements",
    "import",
    "instanceof",
    "int",
    "interface",
    "long",
    "native",
    "new",
    "null",
    "package",
    "private",
    "protected",
    "public",
    "return",
    "short",
    "static",
    "strictfp",
    "super",
    "switch",
    "synchronized",
    "this",
    "throw",
    "throws",
    "transient",
    "true",
    "try",
    "void",
    "volatile",
    "while",
];

/// A validated project name together with the identifiers derived from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectName {
    /// The name as given by the user. Used for the project directory.
    dir_name: String,
    /// The Cargo package name. Hyphens are allowed, e.g. `my-app`.
    package_name: String,
    /// A valid identifier in Rust, Java and Kotlin, e.g. `my_app`.
    identifier: String,
}

impl ProjectName {
    /// Validates `name` and derives every identifier from it.
    ///
    /// The name must start with a lowercase ASCII letter and may only contain
    /// lowercase ASCII letters, digits, `-` and `_`. Hyphens are allowed
    /// because Cargo package names use them, but they are translated to
    /// underscores for every identifier that requires one.
    pub fn parse(name: &str) -> Result<Self> {
        if name.is_empty() {
            return Err(anyhow!("project name must not be empty"));
        }

        if name.chars().count() > MAX_NAME_LEN {
            return Err(anyhow!(
                "invalid project name {name:?}: must be at most {MAX_NAME_LEN} characters"
            ));
        }

        let first = name.chars().next().expect("name is not empty");
        if !first.is_ascii_lowercase() {
            let hint = if first.is_ascii_uppercase() {
                format!("; try {}", name.to_lowercase())
            } else {
                String::new()
            };
            return Err(anyhow!(
                "invalid project name {name:?}: must start with a lowercase ASCII letter{hint}"
            ));
        }

        if let Some(bad) = name
            .chars()
            .find(|c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_' || *c == '-'))
        {
            return Err(anyhow!(
                "invalid project name {name:?}: {bad:?} is not allowed; \
                 use only lowercase letters, digits, '-' and '_'"
            ));
        }

        let identifier = name.replace('-', "_");

        if RUST_KEYWORDS.contains(&name) || RUST_KEYWORDS.contains(&identifier.as_str()) {
            return Err(anyhow!(
                "invalid project name {name:?}: it is a Rust keyword and cannot be used as a crate name"
            ));
        }

        if JAVA_KEYWORDS.contains(&identifier.as_str()) {
            return Err(anyhow!(
                "invalid project name {name:?}: it is a Java keyword and cannot be used as a package name"
            ));
        }

        Ok(Self {
            dir_name: name.to_string(),
            package_name: name.to_string(),
            identifier,
        })
    }

    /// The directory the project is created in, e.g. `my-app`.
    pub fn dir_name(&self) -> &str {
        &self.dir_name
    }

    /// The Cargo package name, e.g. `my-app`.
    pub fn package_name(&self) -> &str {
        &self.package_name
    }

    /// The Rust/Java/Kotlin identifier, e.g. `my_app`.
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    /// The full Java/Kotlin package, e.g. `com.example.my_app`.
    pub fn java_package(&self, package_prefix: &str) -> String {
        format!("{package_prefix}.{}", self.identifier)
    }

    /// The full Java/Kotlin package as a source directory path, e.g. `com/example/my_app`.
    pub fn java_package_path(&self, package_prefix: &str) -> String {
        self.java_package(package_prefix).replace('.', "/")
    }

    /// The iOS bundle identifier, e.g. `com.example.my-app`.
    pub fn bundle_id(&self, package_prefix: &str) -> String {
        format!("{package_prefix}.{}", self.dir_name.replace('_', "-"))
    }

    /// The Swift type name, e.g. `MyApp`.
    pub fn swift_type_name(&self) -> String {
        let mut app = String::new();
        for part in self.dir_name.split(|c: char| !c.is_alphanumeric()) {
            let mut chars = part.chars();
            if let Some(first) = chars.next() {
                app.extend(first.to_uppercase());
                app.push_str(chars.as_str());
            }
        }

        if app.is_empty() {
            app.push_str("App");
        } else if app.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            app.insert_str(0, "App");
        }

        app
    }
}

/// Validates that a package prefix consists of dot-separated valid identifiers (e.g. `com.example`).
pub fn validate_package_prefix(prefix: &str) -> Result<()> {
    let valid = !prefix.is_empty()
        && prefix.split('.').all(|segment| {
            let mut chars = segment.chars();
            let Some(first) = chars.next() else {
                return false;
            };
            (first.is_ascii_alphabetic() || first == '_')
                && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        });

    if !valid {
        return Err(anyhow!(
            "invalid --package value {prefix:?}: expected dot-separated identifiers like com.example"
        ));
    }

    if let Some(bad) = prefix
        .split('.')
        .find(|segment| JAVA_KEYWORDS.contains(segment))
    {
        return Err(anyhow!(
            "invalid --package value {prefix:?}: {bad:?} is a Java keyword and cannot be used as a package name"
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accepts_valid_names() {
        for name in [
            "demo", "my-app", "my_app", "a", "app2", "my-app-2", "a1_b2-c3",
        ] {
            assert!(ProjectName::parse(name).is_ok(), "{name}");
        }
    }

    #[test]
    fn test_rejects_invalid_names() {
        for name in [
            "", "MyApp", "1app", "my app", "my.app", "my/app", "../evil", "my-app!", "über",
            "app\n",
        ] {
            assert!(ProjectName::parse(name).is_err(), "{name}");
        }
    }

    #[test]
    fn test_rejects_too_long_name() {
        let name = "a".repeat(MAX_NAME_LEN + 1);
        assert!(ProjectName::parse(&name).is_err());
        assert!(ProjectName::parse(&"a".repeat(MAX_NAME_LEN)).is_ok());
    }

    #[test]
    fn test_rejects_rust_keywords() {
        for name in ["fn", "crate", "self", "type", "match", "async", "gen"] {
            assert!(ProjectName::parse(name).is_err(), "{name}");
        }
    }

    #[test]
    fn test_rejects_java_keywords() {
        for name in ["class", "new", "int", "true", "null", "package", "switch"] {
            assert!(ProjectName::parse(name).is_err(), "{name}");
        }
    }

    #[test]
    fn test_uppercase_name_suggests_lowercase() {
        let err = ProjectName::parse("MyApp").unwrap_err().to_string();
        assert!(err.contains("myapp"), "{err}");
    }

    #[test]
    fn test_identifier_translates_hyphens() {
        let name = ProjectName::parse("my-app").unwrap();
        assert_eq!(name.identifier(), "my_app");
        assert_eq!(name.package_name(), "my-app");
        assert_eq!(name.dir_name(), "my-app");
    }

    #[test]
    fn test_identifier_keeps_underscores() {
        let name = ProjectName::parse("my_app").unwrap();
        assert_eq!(name.identifier(), "my_app");
    }

    #[test]
    fn test_java_package() {
        let name = ProjectName::parse("my-app").unwrap();
        assert_eq!(name.java_package("com.example"), "com.example.my_app");
        assert_eq!(name.java_package_path("com.example"), "com/example/my_app");
    }

    #[test]
    fn test_java_package_custom_prefix() {
        let name = ProjectName::parse("demo").unwrap();
        assert_eq!(name.java_package("io.acme"), "io.acme.demo");
        assert_eq!(name.java_package_path("io.acme"), "io/acme/demo");
    }

    #[test]
    fn test_bundle_id() {
        let name = ProjectName::parse("my-app").unwrap();
        assert_eq!(name.bundle_id("com.example"), "com.example.my-app");

        let name = ProjectName::parse("my_app").unwrap();
        assert_eq!(name.bundle_id("com.example"), "com.example.my-app");

        let name = ProjectName::parse("demo").unwrap();
        assert_eq!(name.bundle_id("io.acme"), "io.acme.demo");
    }

    #[test]
    fn test_swift_type_name() {
        assert_eq!(
            ProjectName::parse("demo").unwrap().swift_type_name(),
            "Demo"
        );
        assert_eq!(
            ProjectName::parse("my-app").unwrap().swift_type_name(),
            "MyApp"
        );
        assert_eq!(
            ProjectName::parse("my_app").unwrap().swift_type_name(),
            "MyApp"
        );
    }

    #[test]
    fn test_validate_package_prefix_accepts() {
        for prefix in ["com.example", "io.acme", "com.foo_bar", "a", "a.b.c"] {
            assert!(validate_package_prefix(prefix).is_ok(), "{prefix}");
        }
    }

    #[test]
    fn test_validate_package_prefix_rejects() {
        for prefix in [
            "",
            "com..example",
            ".com",
            "com.",
            "1com.example",
            "com.exa-mple",
        ] {
            assert!(validate_package_prefix(prefix).is_err(), "{prefix}");
        }
    }

    #[test]
    fn test_validate_package_prefix_rejects_java_keywords() {
        assert!(validate_package_prefix("com.class").is_err());
        assert!(validate_package_prefix("new.example").is_err());
    }
}
