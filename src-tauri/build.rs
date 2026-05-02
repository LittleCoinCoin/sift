use std::path::PathBuf;
use std::process::Command;

fn main() {
    tauri_build::build();
    download_pdfium();
}

fn download_pdfium() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());

    // Use CARGO_CFG_TARGET_* to detect the TARGET platform, not the host.
    // #[cfg(target_arch)] in build scripts reflects the HOST, which breaks
    // cross-compilation (e.g. building x86_64 on an Apple Silicon machine).
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();

    let (url, lib_name) = match (target_os.as_str(), target_arch.as_str()) {
        ("macos", "aarch64") => (
            "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F7009/pdfium-mac-arm64.tgz",
            "libpdfium.dylib",
        ),
        ("macos", "x86_64") => (
            "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F7009/pdfium-mac-x64.tgz",
            "libpdfium.dylib",
        ),
        ("linux", _) => (
            "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F7009/pdfium-linux-x64.tgz",
            "libpdfium.so",
        ),
        ("windows", _) => (
            "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F7009/pdfium-win-x64.tgz",
            "pdfium.dll",
        ),
        (os, arch) => panic!("Unsupported target platform: {}-{}", os, arch),
    };

    let dest = manifest_dir.join(lib_name);
    // Tell cargo: re-run only if build.rs changes OR if the library disappears.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", dest.display());
    if dest.exists() {
        return;
    }

    println!("cargo:warning=Downloading pdfium binary for {}-{}...", target_os, target_arch);
    let tgz = manifest_dir.join("pdfium_download.tgz");

    let status = Command::new("curl")
        .args(["-fsSL", "-o", tgz.to_str().unwrap(), url])
        .status()
        .expect("curl not found — install curl and retry");
    assert!(status.success(), "Failed to download pdfium from {}", url);

    let status = Command::new("tar")
        .args(["-xzf", tgz.to_str().unwrap(), "-C", manifest_dir.to_str().unwrap()])
        .status()
        .expect("tar not found");
    assert!(status.success(), "Failed to extract pdfium archive");
    std::fs::remove_file(&tgz).ok();

    // Archive unpacks to lib/libpdfium.{dylib|so|dll} — move to manifest root
    let extracted = manifest_dir.join("lib").join(lib_name);
    if extracted.exists() {
        std::fs::rename(&extracted, &dest).unwrap();
        std::fs::remove_dir_all(manifest_dir.join("lib")).ok();
    }

    assert!(dest.exists(), "pdfium library missing after extraction: {:?}", dest);

    // Strip macOS quarantine attribute so dlopen isn't blocked at runtime.
    if target_os == "macos" {
        Command::new("xattr")
            .args(["-cr", dest.to_str().unwrap()])
            .status()
            .ok();
    }

    println!("cargo:warning=pdfium downloaded to {:?}", dest);
}
