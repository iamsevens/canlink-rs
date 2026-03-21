// Build script for canlink-tscan-sys
// Ensure LibTSCAN import library and runtime DLL come from the same bundle.

#[path = "src/bundle.rs"]
mod bundle;

use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

const BUNDLE_ENV: &str = "CANLINK_TSCAN_BUNDLE_DIR";
const TSMASTER_HOME_ENV: &str = "TSMASTER_HOME";
const ALLOW_MISSING_BUNDLE_ENV: &str = "CANLINK_TSCAN_ALLOW_MISSING_BUNDLE";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed={BUNDLE_ENV}");
    println!("cargo:rerun-if-env-changed={TSMASTER_HOME_ENV}");
    println!("cargo:rerun-if-env-changed={ALLOW_MISSING_BUNDLE_ENV}");

    // Only build on Windows
    #[cfg(windows)]
    {
        let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        let workspace_root = manifest_dir.parent().unwrap().to_path_buf();
        let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());

        let selected_bundle = bundle::select_bundle_dir(&bundle_candidates(&workspace_root))
            .or_else(|| fallback_with_allow_missing_bundle(&manifest_dir));

        let Some(selected_bundle) = selected_bundle else {
            let help = format!(
                "No usable LibTSCAN bundle found (need both libTSCAN.lib and libTSCAN.dll). \
Set {BUNDLE_ENV} to the bundle directory."
            );
            panic!("{help}");
        };

        if selected_bundle.as_os_str().is_empty() {
            println!(
                "cargo:warning=Proceeding without explicit LibTSCAN bundle (dry-run mode). \
Set {BUNDLE_ENV} for full linking."
            );
        } else {
            println!(
                "cargo:warning=Using LibTSCAN bundle: {}",
                selected_bundle.display()
            );

            // Link import library from the selected bundle.
            println!(
                "cargo:rustc-link-search=native={}",
                selected_bundle.display()
            );

            // Tell cargo to link to libTSCAN.dll
            println!("cargo:rustc-link-lib=dylib=libTSCAN");

            // Copy runtime DLL bundle next to output binaries to prevent mismatched PATH DLL loading.
            let output_root = workspace_root.join("target").join(profile);
            copy_runtime_bundle(&selected_bundle, &output_root);
            copy_runtime_bundle(&selected_bundle, &output_root.join("deps"));
            copy_runtime_bundle(&selected_bundle, &output_root.join("examples"));
            println!("cargo:rerun-if-changed={}", selected_bundle.display());
        }
    }

    #[cfg(not(windows))]
    {
        println!(
            "cargo:warning=canlink-tscan-sys: non-Windows target detected; skipping LibTSCAN bundle checks"
        );
    }
}

#[cfg(windows)]
fn bundle_candidates(workspace_root: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(dir) = env::var(BUNDLE_ENV) {
        if !dir.trim().is_empty() {
            candidates.push(PathBuf::from(dir));
        }
    }

    if let Ok(home) = env::var(TSMASTER_HOME_ENV) {
        if !home.trim().is_empty() {
            let home = PathBuf::from(home);
            candidates.push(home.join("bin").join("x64"));
            candidates.push(home.join("bin"));
        }
    }

    if let Ok(program_files) = env::var("ProgramFiles") {
        let tsmaster = PathBuf::from(program_files).join("TSMaster");
        candidates.push(tsmaster.join("bin").join("x64"));
        candidates.push(tsmaster.join("bin"));
    }

    candidates.push(workspace_root.join("libs"));
    candidates.push(
        workspace_root
            .join("docs")
            .join("vendor")
            .join("tsmaster")
            .join("examples")
            .join("LibTSCAN")
            .join("lib_extracted")
            .join("lib")
            .join("lib")
            .join("windows")
            .join("x64"),
    );
    candidates
}

#[cfg(windows)]
fn fallback_with_allow_missing_bundle(manifest_dir: &Path) -> Option<PathBuf> {
    if allow_missing_bundle() {
        println!(
            "cargo:warning=No LibTSCAN bundle found. {}=1 is set, skip linking/runtime bundle copy for this build.",
            ALLOW_MISSING_BUNDLE_ENV
        );
        return Some(PathBuf::new());
    }

    if is_packaged_verification_build(manifest_dir) {
        println!(
            "cargo:warning=No LibTSCAN bundle found. Detected cargo package/publish verification build, skip linking/runtime bundle copy."
        );
        return Some(PathBuf::new());
    }

    None
}

#[cfg(windows)]
fn allow_missing_bundle() -> bool {
    matches!(
        env::var(ALLOW_MISSING_BUNDLE_ENV)
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes"
    )
}

#[cfg(windows)]
fn is_packaged_verification_build(manifest_dir: &Path) -> bool {
    let normalized = manifest_dir.to_string_lossy().replace('\\', "/");
    normalized.contains("/target/package/")
}

#[cfg(windows)]
fn copy_runtime_bundle(from_dir: &Path, to_dir: &Path) {
    if from_dir.as_os_str().is_empty() {
        return;
    }

    if let Err(err) = fs::create_dir_all(to_dir) {
        println!(
            "cargo:warning=Failed to create DLL target directory '{}': {}",
            to_dir.display(),
            err
        );
        return;
    }

    let entries = match fs::read_dir(from_dir) {
        Ok(v) => v,
        Err(err) => {
            println!(
                "cargo:warning=Failed to read runtime bundle '{}': {}",
                from_dir.display(),
                err
            );
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let is_dll = path
            .extension()
            .map(|v| v.to_string_lossy().eq_ignore_ascii_case("dll"))
            .unwrap_or(false);
        if !is_dll {
            continue;
        }

        let Some(name) = path.file_name() else {
            continue;
        };
        let target = to_dir.join(name);
        if let Err(err) = fs::copy(&path, &target) {
            println!(
                "cargo:warning=Failed to copy '{}' to '{}': {}",
                path.display(),
                target.display(),
                err
            );
        } else {
            println!("cargo:warning=Copied runtime DLL to: {}", target.display());
        }
    }
}
