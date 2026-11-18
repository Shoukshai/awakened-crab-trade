use std::{env, fs, path::PathBuf, process::Command};
fn main() {
    println!("cargo:rerun-if-env-changed=APT_PORTABLE_PAYLOAD");
    println!("cargo:rerun-if-env-changed=APT_PORTABLE_ID");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let payload = match env::var_os("APT_PORTABLE_PAYLOAD") {
        Some(path) => PathBuf::from(path).canonicalize().unwrap(),
        None => {
            let path = out.join("no-payload");
            fs::write(&path, []).unwrap();
            path
        }
    };
    println!("cargo:rerun-if-changed={}", payload.display());
    println!(
        "cargo:rustc-env=APT_PORTABLE_PAYLOAD_PATH={}",
        payload.display()
    );
    println!(
        "cargo:rustc-env=APT_PORTABLE_ID={}",
        env::var("APT_PORTABLE_ID").unwrap_or_else(|_| "test".into())
    );
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
        let resources = root.join("../packaging/windows").canonicalize().unwrap();
        println!("cargo:rerun-if-changed={}", resources.display());
        println!(
            "cargo:rerun-if-changed={}",
            root.join("../assets/branding/app.ico").display()
        );
        let object = out.join("portable-res.o");
        let compiler = env::var_os("APT_WINDOWS_WINDRES").expect("Set APT_WINDOWS_WINDRES");
        let mut command = Command::new(compiler);
        if env::var_os("CARGO_FEATURE_CLOUD_OCR").is_some() {
            command.arg("-DACT_OCR");
        }
        assert!(command
            .current_dir(resources)
            .args(["-i", "app.rc", "-O", "coff", "-o"])
            .arg(&object)
            .status()
            .unwrap()
            .success());
        println!("cargo:rustc-link-arg={}", object.display());
    }
}
