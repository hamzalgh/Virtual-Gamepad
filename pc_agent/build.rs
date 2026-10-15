use std::{env, fs, path::PathBuf, process::Command};
fn main() {
    println!("cargo:rerun-if-changed=packaging/application.manifest");
    println!("cargo:rerun-if-env-changed=VG_RC_EXE");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let version = env::var("CARGO_PKG_VERSION").unwrap();
    let numeric = format!("{},0", version.replace('.', ","));
    let manifest = fs::canonicalize("packaging/application.manifest")
        .unwrap()
        .display()
        .to_string()
        .replace('\\', "\\\\");
    let rc = format!(
        r#"1 24 "{manifest}"
1 VERSIONINFO
FILEVERSION {numeric}
PRODUCTVERSION {numeric}
FILEFLAGSMASK 0x3fL
FILEFLAGS 0
FILEOS 0x40004L
FILETYPE 1
BEGIN
 BLOCK "StringFileInfo"
 BEGIN
  BLOCK "040904B0"
  BEGIN
   VALUE "CompanyName", "Virtual Gamepad contributors\0"
   VALUE "FileDescription", "Virtual Gamepad desktop agent\0"
   VALUE "FileVersion", "{version}\0"
   VALUE "ProductName", "Virtual Gamepad\0"
   VALUE "ProductVersion", "{version}\0"
   VALUE "OriginalFilename", "virtual-gamepad-agent.exe\0"
   VALUE "LegalCopyright", "Copyright 2026 Virtual Gamepad contributors\0"
  END
 END
 BLOCK "VarFileInfo"
 BEGIN
  VALUE "Translation", 0x0409, 1200
 END
END
"#
    );
    let source = out.join("application.rc");
    let resource = out.join("application.res");
    fs::write(&source, rc).unwrap();
    let compiler = env::var_os("VG_RC_EXE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let root = PathBuf::from(
                env::var_os("ProgramFiles(x86)").expect("Windows SDK required; set VG_RC_EXE"),
            )
            .join("Windows Kits/10/bin");
            let mut candidates: Vec<_> = fs::read_dir(root)
                .expect("Windows SDK resource compiler required")
                .filter_map(Result::ok)
                .map(|e| e.path().join("x64/rc.exe"))
                .filter(|p| p.is_file())
                .collect();
            candidates.sort();
            candidates
                .pop()
                .expect("Install Windows SDK build tools separately or set VG_RC_EXE")
        });
    assert!(
        Command::new(compiler)
            .arg("/nologo")
            .arg("/fo")
            .arg(&resource)
            .arg(source)
            .status()
            .expect("Run resource compiler")
            .success(),
        "Resource compiler failed"
    );
    println!(
        "cargo:rustc-link-arg-bin=virtual-gamepad-agent={}",
        resource.display()
    );
}
