use std::path::PathBuf;
use std::process::Command;

fn main() {
    tauri_build::build();
    download_pdfium();
}

fn download_pdfium() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    let (url, lib_name) = (
        "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F7009/pdfium-mac-arm64.tgz",
        "libpdfium.dylib",
    );
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    let (url, lib_name) = (
        "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F7009/pdfium-mac-x64.tgz",
        "libpdfium.dylib",
    );
    #[cfg(target_os = "linux")]
    let (url, lib_name) = (
        "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F7009/pdfium-linux-x64.tgz",
        "libpdfium.so",
    );
    #[cfg(target_os = "windows")]
    let (url, lib_name) = (
        "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F7009/pdfium-win-x64.tgz",
        "pdfium.dll",
    );

    let dest = manifest_dir.join(lib_name);
    // Tell cargo: re-run only if build.rs changes OR if the library disappears.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", dest.display());
    if dest.exists() {
        return;
    }

    println!("cargo:warning=Downloading pdfium binary...");
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

    // Archive unpacks to lib/libpdfium.dylib — move to manifest root
    let extracted = manifest_dir.join("lib").join(lib_name);
    if extracted.exists() {
        std::fs::rename(&extracted, &dest).unwrap();
        std::fs::remove_dir_all(manifest_dir.join("lib")).ok();
    }

    assert!(dest.exists(), "pdfium library missing after extraction: {:?}", dest);

    // Strip macOS quarantine attribute so the dylib mtime stays stable and
    // so the OS does not block dlopen at runtime.
    #[cfg(target_os = "macos")]
    {
        Command::new("xattr")
            .args(["-cr", dest.to_str().unwrap()])
            .status()
            .ok();
    }

    println!("cargo:warning=pdfium downloaded to {:?}", dest);
}
