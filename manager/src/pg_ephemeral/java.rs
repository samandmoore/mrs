use super::{
    Platform, StagingItem, detect_target_platform, setup_staging_directory, verify_and_collect_file,
};
use cmd_proc::EnvVariableName;
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const ENV_EXPECTED_PG_EPHEMERAL_VERSION: EnvVariableName =
    EnvVariableName::from_static_or_panic("EXPECTED_PG_EPHEMERAL_VERSION");

fn java_version() -> String {
    let version = pg_ephemeral::version();
    let mut result = format!("{}.{}.{}", version.major, version.minor, version.patch);
    if !version.pre.is_empty() {
        result.push('-');
        result.push_str(&version.pre.to_string());
    }
    result
}

#[derive(Debug, clap::Parser)]
pub(crate) enum Command {
    /// Build Java package (main JAR + current platform classifier JAR)
    Build {
        /// Skip compilation (use pre-built binaries)
        #[clap(long)]
        no_compile: bool,
    },
    /// Merge multi-platform Java JARs from per-platform artifacts
    Merge,
    /// Test Java package against the current platform binary
    Test,
    /// Publish Java package to Maven Central from GitHub artifacts
    Publish {
        /// Actually push to Maven Central (default is dry-run)
        #[clap(long)]
        push: bool,
    },
    /// Sync generated files (build.gradle.kts version) with Rust source of truth
    Sync {
        /// Fail if git is dirty after syncing (for CI verification)
        #[clap(long)]
        reject_dirty: bool,
    },
}

impl Command {
    pub(crate) async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            Self::Build { no_compile } => {
                build(*no_compile).await;
                Ok(())
            }
            Self::Merge => {
                merge().await;
                Ok(())
            }
            Self::Test => {
                test().await;
                Ok(())
            }
            Self::Publish { push } => {
                publish(*push).await;
                Ok(())
            }
            Self::Sync { reject_dirty } => sync(*reject_dirty).await,
        }
    }
}

pub(crate) struct PlatformArtifactPaths {
    pub(crate) platform_jar: PathBuf,
    pub(crate) platform_jar_sha256: PathBuf,
    pub(crate) main_jar: PathBuf,
    pub(crate) main_jar_sha256: PathBuf,
}

pub(crate) fn platform_artifact_paths(
    workspace_root: &Path,
    platform: Platform,
) -> PlatformArtifactPaths {
    let version = java_version();
    let artifact_base = workspace_root
        .join("artifacts")
        .join(format!("pg-ephemeral-{}", platform.rust_target()));

    let java_base = artifact_base.join("dist").join("java");

    let platform_jar_name = platform_jar_name(&version, platform);
    let main_jar_name = main_jar_name(&version);

    PlatformArtifactPaths {
        platform_jar: java_base.join(&platform_jar_name),
        platform_jar_sha256: java_base.join(format!("{platform_jar_name}.sha256")),
        main_jar: java_base.join(&main_jar_name),
        main_jar_sha256: java_base.join(format!("{main_jar_name}.sha256")),
    }
}

fn platform_jar_name(version: &str, platform: Platform) -> String {
    format!("pg-ephemeral-{version}-{}.jar", platform.java_classifier())
}

fn main_jar_name(version: &str) -> String {
    format!("pg-ephemeral-{version}.jar")
}

fn integration_source(workspace_root: &Path) -> PathBuf {
    workspace_root
        .join("pg-ephemeral")
        .join("integrations")
        .join("java")
}

/// Directory consumed by `build.gradle.kts` (`build/native-staging`); binaries
/// are staged as `native/<classifier>/pg-ephemeral`.
fn native_staging_dir(workspace_root: &Path) -> PathBuf {
    integration_source(workspace_root)
        .join("build")
        .join("native-staging")
}

fn gradlew(workspace_root: &Path) -> PathBuf {
    integration_source(workspace_root).join("gradlew")
}

fn write_sha256(dir: &Path, filename: &str) {
    let file_path = dir.join(filename);
    let bytes = std::fs::read(&file_path)
        .unwrap_or_else(|error| panic!("Failed to read {filename}: {error}"));

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash = hex::encode(hasher.finalize());
    let hash_string = format!("{hash}  {filename}\n");
    let sha256_path = dir.join(format!("{filename}.sha256"));

    std::fs::write(&sha256_path, hash_string)
        .unwrap_or_else(|error| panic!("Failed to write SHA256 file: {error}"));

    log::info!("SHA256 hash written to: {}", sha256_path.display());
}

/// Stage `pg-ephemeral` into `<native_staging>/native/<classifier>/pg-ephemeral`.
fn stage_binary(workspace_root: &Path, binary_source: &Path, platform: Platform) {
    let staging = native_staging_dir(workspace_root);
    setup_staging_directory(
        &staging,
        vec![StagingItem::CopyFile {
            source: binary_source.to_path_buf(),
            destination: format!("native/{}/pg-ephemeral", platform.java_classifier()),
        }],
    );
}

/// Extract the `pg-ephemeral` entry from a `.tar.gz` into the staging dir for
/// the given platform.
fn stage_binary_from_tarball(workspace_root: &Path, tarball: &Path, platform: Platform) {
    let staging = native_staging_dir(workspace_root)
        .join("native")
        .join(platform.java_classifier());
    std::fs::create_dir_all(&staging)
        .unwrap_or_else(|error| panic!("Failed to create staging dir: {error}"));

    let file = std::fs::File::open(tarball)
        .unwrap_or_else(|error| panic!("Failed to open {}: {error}", tarball.display()));
    let mut archive = tar::Archive::new(GzDecoder::new(file));

    let destination = staging.join("pg-ephemeral");
    for entry in archive
        .entries()
        .unwrap_or_else(|error| panic!("Failed to read tarball entries: {error}"))
    {
        let mut entry = entry.unwrap_or_else(|error| panic!("Failed to read tar entry: {error}"));
        let path = entry
            .path()
            .unwrap_or_else(|error| panic!("Failed to read tar entry path: {error}"))
            .into_owned();
        if path.file_name().and_then(|name| name.to_str()) == Some("pg-ephemeral")
            && !path.to_string_lossy().contains(".dSYM")
        {
            entry
                .unpack(&destination)
                .unwrap_or_else(|error| panic!("Failed to unpack binary: {error}"));
            log::info!("Staged binary: {}", destination.display());
            return;
        }
    }
    panic!("pg-ephemeral binary not found in {}", tarball.display());
}

async fn run_gradle(workspace_root: &Path, arguments: &[&str]) {
    let gradlew = gradlew(workspace_root);
    cmd_proc::Command::new(gradlew.to_str().unwrap())
        .arguments(arguments)
        .working_directory(integration_source(workspace_root))
        .status()
        .await
        .unwrap_or_else(|error| panic!("Gradle command {arguments:?} failed: {error}"));
}

async fn build(no_compile: bool) {
    let platform = detect_target_platform();
    let rust_target = platform.rust_target();
    let classifier = platform.java_classifier();
    let version = java_version();

    log::info!("Building pg-ephemeral Java package for target: {rust_target}");
    log::info!("Java classifier: {classifier}");
    log::info!("Version: {version}");

    if no_compile {
        log::info!("Skipping compilation (--no-compile flag set)");
    } else {
        cmd_proc::Command::new("cargo")
            .arguments([
                "build",
                "--release",
                "--package",
                "pg-ephemeral",
                "--target",
                rust_target,
            ])
            .status()
            .await
            .unwrap_or_else(|error| panic!("Failed to build pg-ephemeral binary: {error}"));
    }

    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();

    let binary_source = workspace_root
        .join("target")
        .join(rust_target)
        .join("release")
        .join("pg-ephemeral");

    stage_binary(&workspace_root, &binary_source, platform);

    // `assemble` builds the platform-independent main JAR plus every classifier
    // JAR whose binary is staged (the rest are skipped via `onlyIf`).
    run_gradle(&workspace_root, &["assemble", "--no-daemon"]).await;

    let libs = integration_source(&workspace_root)
        .join("build")
        .join("libs");

    let dist_java = workspace_root.join("dist").join("java");
    std::fs::create_dir_all(&dist_java)
        .unwrap_or_else(|error| panic!("Failed to create dist/java directory: {error}"));

    let main_jar = main_jar_name(&version);
    let platform_jar = platform_jar_name(&version, platform);

    setup_staging_directory(
        &dist_java,
        vec![
            StagingItem::CopyFile {
                source: libs.join(&main_jar),
                destination: main_jar.clone(),
            },
            StagingItem::CopyFile {
                source: libs.join(&platform_jar),
                destination: platform_jar.clone(),
            },
        ],
    );

    write_sha256(&dist_java, &main_jar);
    write_sha256(&dist_java, &platform_jar);

    log::info!("Java build complete");
}

async fn merge() {
    log::info!("Merging multi-platform Java JARs");

    let workspace_root = std::env::current_dir()
        .unwrap_or_else(|error| panic!("Failed to get current directory: {error}"));

    let dist_java = workspace_root.join("dist").join("java");
    let version = java_version();

    let mut staging_items = Vec::new();

    for platform in Platform::ALL {
        let paths = platform_artifact_paths(&workspace_root, *platform);

        for source in [paths.platform_jar, paths.platform_jar_sha256] {
            let source = verify_and_collect_file(source);
            staging_items.push(StagingItem::CopyFile {
                destination: source.file_name().unwrap().to_str().unwrap().to_string(),
                source,
            });
        }
    }

    // Main JAR is platform-independent; take it from the first platform.
    let first_paths = platform_artifact_paths(&workspace_root, Platform::ALL[0]);
    let main_source = verify_and_collect_file(first_paths.main_jar);
    staging_items.push(StagingItem::CopyFile {
        destination: main_jar_name(&version),
        source: main_source,
    });
    let main_sha_source = verify_and_collect_file(first_paths.main_jar_sha256);
    staging_items.push(StagingItem::CopyFile {
        destination: format!("{}.sha256", main_jar_name(&version)),
        source: main_sha_source,
    });

    let collected = Platform::ALL.len();
    setup_staging_directory(&dist_java, staging_items);

    log::info!("Collected {collected} platform JARs + main JAR");
    log::info!("Java JARs ready at: {}", dist_java.display());
}

async fn test() {
    log::info!("Running Java integration tests");

    let workspace_root = std::env::current_dir()
        .unwrap_or_else(|error| panic!("Failed to get current directory: {error}"));

    let platform = detect_target_platform();
    let rust_target = platform.rust_target();
    let version = java_version();
    let jar_name = platform_jar_name(&version, platform);

    // Tests run against the real classifier JAR. Prefer the merged/built JAR
    // (dist/java); otherwise build it locally from the freshly compiled binary.
    let dist_jar = workspace_root.join("dist").join("java").join(&jar_name);
    let libs_jar = integration_source(&workspace_root)
        .join("build")
        .join("libs")
        .join(&jar_name);

    let classifier_jar = if dist_jar.exists() {
        dist_jar
    } else if libs_jar.exists() {
        libs_jar
    } else {
        log::info!("Classifier JAR not found; building it from the compiled binary");
        let binary_source = workspace_root
            .join("target")
            .join(rust_target)
            .join("release")
            .join("pg-ephemeral");
        stage_binary(&workspace_root, &binary_source, platform);
        run_gradle(&workspace_root, &["assemble", "--no-daemon"]).await;
        libs_jar
    };

    log::info!(
        "Testing against classifier JAR: {}",
        classifier_jar.display()
    );

    let gradlew = gradlew(&workspace_root);
    cmd_proc::Command::new(gradlew.to_str().unwrap())
        .arguments([
            "test",
            "--no-daemon",
            &format!("-PpgEphemeralClassifierJar={}", classifier_jar.display()),
        ])
        .working_directory(integration_source(&workspace_root))
        .env(&ENV_EXPECTED_PG_EPHEMERAL_VERSION, &version)
        .status()
        .await
        .unwrap_or_else(|error| panic!("Java tests failed: {error}"));

    log::info!("Java integration tests complete");
}

async fn publish(push: bool) {
    if push {
        log::info!("Publishing Java package to Maven Central");
    } else {
        log::info!("Running in DRY-RUN mode (use --push to actually publish)");
    }

    let workspace_root = std::env::current_dir()
        .unwrap_or_else(|error| panic!("Failed to get current directory: {error}"));

    let edge = crate::edge::resolve().await;
    let release_tag = &edge.tag;

    // Reuse the per-platform binary tarballs attached to the edge release
    // (produced by the Ruby build) to assemble all classifier JARs in one
    // signed Gradle publish.
    let download_dir = workspace_root.join("dist").join("java-binaries");
    if download_dir.exists() {
        std::fs::remove_dir_all(&download_dir)
            .unwrap_or_else(|error| panic!("Failed to remove download dir: {error}"));
    }
    std::fs::create_dir_all(&download_dir)
        .unwrap_or_else(|error| panic!("Failed to create download dir: {error}"));

    let tarball_names: Vec<String> = Platform::ALL
        .iter()
        .map(|platform| format!("pg-ephemeral-{}.tar.gz", platform.rust_target()))
        .collect();

    log::info!("Downloading binary tarballs from edge release {release_tag}");
    let mut arguments = vec![
        "release",
        "download",
        release_tag,
        "--repo",
        "mbj/mrs",
        "--dir",
        download_dir.to_str().unwrap(),
    ];
    for name in &tarball_names {
        arguments.push("--pattern");
        arguments.push(name);
    }
    cmd_proc::Command::new("gh")
        .arguments(arguments)
        .status()
        .await
        .unwrap_or_else(|error| {
            panic!("Failed to download binary tarballs from release {release_tag}: {error}")
        });

    for platform in Platform::ALL {
        let tarball = download_dir.join(format!("pg-ephemeral-{}.tar.gz", platform.rust_target()));
        stage_binary_from_tarball(&workspace_root, &tarball, *platform);
    }

    if push {
        log::info!("Publishing all classifier JARs to Maven Central");
        run_gradle(
            &workspace_root,
            &["publishAndReleaseToMavenCentral", "--no-daemon"],
        )
        .await;
        log::info!("Successfully published Java package to Maven Central");
    } else {
        log::info!(
            "[DRY-RUN] Would execute: gradlew publishAndReleaseToMavenCentral (all classifier JARs staged)"
        );
        log::info!("Run with --push to actually publish");
    }

    log::info!("Done");
}

const VERSION_PREFIX: &str = "version = \"";

pub(super) async fn sync(reject_dirty: bool) -> Result<(), Box<dyn std::error::Error>> {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();

    let version = java_version();
    log::info!("Syncing pg-ephemeral Java generated files (version: {version})");

    let build_gradle_path = integration_source(&workspace_root).join("build.gradle.kts");
    let existing = std::fs::read_to_string(&build_gradle_path).unwrap_or_default();
    let updated = replace_version_line(&existing, &version);

    if existing == updated {
        log::info!(
            "build.gradle.kts is up to date: {}",
            build_gradle_path.display()
        );
    } else if reject_dirty {
        let diff = similar::TextDiff::from_lines(&existing, &updated);
        return Err(format!(
            "Generated build.gradle.kts differs from {}. Run `manager pg-ephemeral java sync` to update.\n\n{}",
            build_gradle_path.display(),
            diff.unified_diff()
                .context_radius(3)
                .header("committed", "generated")
        )
        .into());
    } else {
        log::info!(
            "Writing build.gradle.kts to: {}",
            build_gradle_path.display()
        );
        std::fs::write(&build_gradle_path, &updated)
            .unwrap_or_else(|error| panic!("Failed to write build.gradle.kts: {error}"));
    }

    log::info!("Sync complete");
    Ok(())
}

/// Replace the top-level `version = "..."` line with the Rust source-of-truth
/// version. Panics if no such line exists.
fn replace_version_line(content: &str, version: &str) -> String {
    let mut replaced = false;
    let mut result: Vec<String> = content
        .lines()
        .map(|line| {
            if line.starts_with(VERSION_PREFIX) {
                replaced = true;
                format!("{VERSION_PREFIX}{version}\"")
            } else {
                line.to_string()
            }
        })
        .collect();

    if !replaced {
        panic!("No `version = \"...\"` line found in build.gradle.kts");
    }

    // Preserve trailing newline.
    if content.ends_with('\n') {
        result.push(String::new());
    }
    result.join("\n")
}
