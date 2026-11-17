fn main() {
    println!("cargo:rerun-if-changed=packaging/windows/app.rc");
    println!("cargo:rerun-if-changed=packaging/windows/app.manifest");
    println!("cargo:rerun-if-changed=../main/build/icons/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let root = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("app-res.o");
    let tool = std::env::var_os("APT_WINDOWS_WINDRES").unwrap_or_else(|| "llvm-windres".into());
    let status = std::process::Command::new(tool)
        .current_dir(root.join("packaging/windows"))
        .args(["-i", "app.rc", "-O", "coff", "-o"])
        .arg(&output)
        .status()
        .expect("Windows resource compiler missing");
    assert!(status.success(), "Windows resources failed to compile");
    println!(
        "cargo:rustc-link-arg-bin=awakened-poe-trade-rs={}",
        output.display()
    );
}
