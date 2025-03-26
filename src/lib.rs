use std::{env, fs, io};
use std::fs::OpenOptions;
use std::io::{Error, ErrorKind, Write};
use std::path::PathBuf;
use std::process::Command;
use toml::Value;


/// Holds configuration settings for a Rust project build.
///
/// This struct encapsulates settings like build target, output paths, enabled features, and
/// release/debug modes.
///
/// # Fields
///
/// * `compilation_target` - Optional string specifying a custom compilation target.
/// * `features` - Optional list of features to enable during the build.
/// * `output_path` - Optional path to store compiled artifacts.
/// * `release` - Whether to compile in release mode (`true`) or debug mode (`false`).
/// * `is_lib` - If `true`, builds the project as a library (`--lib`), otherwise builds as a binary (`--bin`).
/// * `no_default_features` - If `true`, disables default features (`--no-default-features`).
/// * `project_path` - The root directory of the Rust project.
/// * `cargo_toml_path` - Path to the project's `Cargo.toml`.
/// * `target` - Optional specific binary/library to build.
#[derive(Default, Debug, Clone)]
pub struct ProjectSettings {
    compilation_target: Option<String>,
    features: Option<Vec<String>>,
    output_path: Option<PathBuf>,
    release: bool,
    is_lib: bool,
    no_default_features: bool,
    project_path: PathBuf,
    cargo_toml_path: PathBuf,
    target: Option<String>
}

impl ProjectSettings {

    /// Creates a new `ProjectSettings` instance for managing build configurations.
    ///
    /// # Arguments
    ///
    /// * `project_path` - The root directory of the Rust project.
    /// * `output_path` - Optional path where the build output should be stored.
    /// * `target` - Optional target triple (e.g., "x86_64-unknown-linux-gnu").
    /// * `is_lib` - If `true`, builds the project as a library (`--lib`). If `false`, builds as a binary (`--bin`).
    ///
    /// # Returns
    ///
    /// A `ProjectSettings` instance with default values.
    ///
    /// # Example
    /// ```rust
    /// use cargo_wrap::ProjectSettings;
    /// let settings = ProjectSettings::new("/path/to/project", None, None, false);
    /// ```
    pub fn new(project_path: impl Into<PathBuf>, output_path: Option<impl Into<PathBuf>>, target: Option<String>,
               is_lib: bool) -> Self {
        let project_path = project_path.into();
        let cargo_toml = project_path.clone().join("Cargo.toml");
        Self {
            project_path,
            release: false,
            output_path: output_path.map(Into::into),
            cargo_toml_path: cargo_toml,
            is_lib,
            target,
            ..Default::default()
        }
    }

    /// Retrieves a list of available features from `Cargo.toml`.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<String>)` - A list of feature names if parsing succeeds.
    /// * `Err(io::Error)` - If `Cargo.toml` is missing or cannot be parsed.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - `Cargo.toml` does not exist.
    /// - The file cannot be read due to I/O issues.
    /// - The `features` section in `Cargo.toml` is invalid.
    ///
    /// # Example
    /// ```rust
    /// use cargo_wrap::ProjectSettings;
    /// let settings = ProjectSettings::new("/path/to/project", None, None, false);
    /// match settings.get_features() {
    ///     Ok(features) => println!("Available features: {:?}", features),
    ///     Err(e) => eprintln!("Error retrieving features: {}", e),
    /// }
    /// ```
    pub fn get_features(&self) -> io::Result<Vec<String>> {
        let cargo_content = fs::read_to_string(&self.cargo_toml_path)?;
        let parsed_toml: Value = cargo_content.parse().map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
        if let Some(features) = parsed_toml.get("features").and_then(|f| f.as_table()) {
            Ok(features.keys().cloned().collect())
        } else {
            Ok(vec![])
        }
    }

    /// Marks the project to be built as `release`
    pub fn set_release(&mut self) {
        self.release = true;
    }

    /// Manually enable a feature that's available in the project
    pub fn add_feature(&mut self, feature: String) {
        self.features.get_or_insert_with(Vec::new).push(feature)
    }
}

/// The main struct responsible for building a Rust project.
///
/// `Builder` acts as a wrapper around the `cargo build` command, allowing
/// users to configure build settings such as verbosity, threading, and output paths.
///
/// # Fields
///
/// * `cargo_path` - Path to the `cargo` binary.
/// * `project_settings` - The `ProjectSettings` instance containing build configurations.
/// * `thread_count` - Optional number of jobs (`--jobs N`) to use during the build. Default value is 0.
/// * `output_path` - Optional log file to store output.
/// * `verbose_build` - If `true`, enables verbose output (`--verbose`).
/// * `additional_flags` - Optional flags to pass to the `rustc` binary (via the `RUSTFLAGS` environment variable)
#[derive(Default, Debug)]
pub struct Builder {
    cargo_path: PathBuf,
    project_settings: ProjectSettings,
    thread_count: usize,
    output_path: Option<PathBuf>,
    verbose_build: bool,
    additional_flags: Vec<String>
}

impl Builder {

    /// Private function to get the `cargo` binary path from the environment
    fn get_cargo_path() -> io::Result<PathBuf> {
        env::var_os("CARGO")
            .map(PathBuf::from)
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "CARGO environment variable not found"))
    }

    /// Creates a new `Builder` instance for managing and executing cargo builds.
    ///
    /// This function initializes the builder with the given project settings,
    /// allowing for configuration of build parameters such as job count and log output.
    ///
    /// # Arguments
    ///
    /// * `project_settings` - A `ProjectSettings` instance containing the configuration
    ///   for the Rust project to be built.
    /// * `thread_count` - Optional number of parallel jobs (`--jobs N`) to use for building.
    ///   If `0`, the default job count will be used.
    /// * `output_path` - Optional path to a log file where build output will be stored.
    ///
    /// # Returns
    ///
    /// * `Ok(Builder)` - A new `Builder` instance ready to execute a build.
    /// * `Err(io::Error)` - If the `cargo` binary is not found in the environment.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The `CARGO` environment variable is not set, meaning `cargo` cannot be found.
    /// - The provided `output_path` is invalid or cannot be written to.
    ///
    /// # Example
    /// ```rust
    /// use cargo_wrap::{Builder, ProjectSettings};
    /// use std::io;
    ///
    /// fn main() -> io::Result<()> {
    ///     let settings = ProjectSettings::new("/path/to/project", None, None, false);
    ///     let builder = Builder::new(settings, 4, Some("build.log"))?;
    ///     Ok(())
    /// }
    pub fn new(project_settings: ProjectSettings, thread_count: usize, output_path:
    Option<impl Into<PathBuf>>) ->
               io::Result<Builder> {
        let cargo_path = Builder::get_cargo_path()?;
        Ok(Self {
            cargo_path,
            project_settings,
            thread_count,
            output_path: output_path.map(Into::into),
            ..Default::default()
        })
    }

    /// Tells the builder to use the `--verbose` flag when building
    pub fn set_verbose(&mut self) {
        self.verbose_build = true;
    }

    /// Adds a flag to the list of additional flags that will be passed to `rustc`
    pub fn add_rustc_flag(&mut self, flag: String) {
        self.additional_flags.push(flag);
    }

    /// Executes the build process using `cargo build`.
    ///
    /// This function spawns a `cargo build` process with the specified settings,
    /// such as release/debug mode, enabled features, and output directories.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the build succeeds.
    /// * `Err(io::Error)` - If the build process fails.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The `cargo` binary is missing from the system.
    /// - The build process fails (e.g., compilation errors).
    /// - The log file cannot be written to (if logging is enabled).
    ///
    /// # Example
    /// ```rust
    /// use cargo_wrap::{Builder, ProjectSettings};
    /// use std::io;
    ///
    /// fn main() -> io::Result<()> {
    ///     let settings = ProjectSettings::new("/path/to/project", None, None, false);
    ///     let builder = Builder::new(settings, 4, Some("build.log"))?;
    ///     builder.build()?;
    ///     Ok(())
    /// }
    /// ```
    pub fn build(&self) -> io::Result<()> {
        let mut command = Command::new(self.cargo_path.clone());
        command.arg("build");
        if self.verbose_build {
            command.arg("--verbose");
        }
        if self.project_settings.release {
            command.arg("--release");
        }
        if self.thread_count > 0 {
            command.arg("--jobs").arg(self.thread_count.to_string());

        }
        if let Some(output_path) = &self.project_settings.output_path {
            command.env("CARGO_TARGET_DIR", output_path);
        }
        if !self.additional_flags.is_empty() {
            command.env("RUSTFLAGS", self.additional_flags.join(" "));
        }
        if let Some(ref target) = self.project_settings.compilation_target {
            command.arg("--target").arg(target);
        }
        if let Some(features) = &self.project_settings.features {
            command.arg("--features");
            features.iter().for_each(|f| { command.arg(f); });
        }
        if self.project_settings.no_default_features {
            command.arg("--no-default-features");
        }
        if let Some(target) = &self.project_settings.target {
            command.arg(if self.project_settings.is_lib { "--lib" } else { "--bin" }).arg(target);
        }

        let output = command.current_dir(&self.project_settings.project_path).output()?;
        if let Some(output_log) = &self.output_path {
            let mut output_file = OpenOptions::new().create(true).append(true).open(output_log)?;
            output_file.write_all(&output.stdout)?;
            output_file.write_all(&output.stderr)?;
        }
        if output.status.success() {
            Ok(())
        } else {
            Err(Error::new(ErrorKind::Other, format!("Failed to compile project: {}", output.status)))
        }
    }
}

