use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=icon.rc");
    println!("cargo:rerun-if-changed=icon.ico");

    let windres = "C:\\msys64\\ucrt64\\bin\\windres.exe";
    let out = std::env::var("OUT_DIR").unwrap();

    let status = Command::new(windres)
        .args(&[
            "icon.rc",
            "-O",
            "coff",
            &format!("{}\\icon.res", out),
        ])
        .status()
        .expect("failed to run windres");

    assert!(status.success(), "windres failed");

    println!("cargo:rustc-link-arg={}\\icon.res", out);
}
