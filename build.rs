// The bundled MinGW toolchain lacks some Windows import libraries (e.g. shlwapi);
// libs/ holds import libraries generated with dlltool from the system DLL exports.
// Also embeds the application icon, version information and manifest (asInvoker, Windows 10+)
// into NeramitPlanner.exe.
fn main() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("libs");
    println!("cargo:rustc-link-search=native={}", dir.display());
    println!("cargo:rerun-if-changed=libs");
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=assets/app.manifest");
    println!("cargo:rerun-if-changed=Cargo.toml");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // maker credit = Cargo.toml `authors`
        let designer = std::env::var("CARGO_PKG_AUTHORS").unwrap_or_default();
        let copyright = format!("© 2026 Neramit Planner Made by {designer}");
        let mut res = winresource::WindowsResource::new();
        res.set("ProductName", "Neramit Planner");
        res.set("FileDescription", "Neramit Planner - Project Scheduling");
        res.set("LegalCopyright", &copyright);
        res.set("CompanyName", &designer);
        embed(res).expect("Could not embed the Windows application icon");
    }
}

/// windres hands its include folder and input to the preprocessor through a shell without
/// quotes, so a project folder with a space ("Neramit Planner") breaks it: the resource is
/// compiled in a temporary folder and the object linked into the program from OUT_DIR.
fn embed(mut res: winresource::WindowsResource) -> std::io::Result<()> {
    use std::path::PathBuf;
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let work = std::env::temp_dir().join(format!("neramit-planner-res-{}", std::process::id()));
    std::fs::create_dir_all(&work)?;
    std::fs::copy(manifest.join("assets").join("icon.ico"), work.join("icon.ico"))?;
    std::fs::copy(manifest.join("assets").join("app.manifest"), work.join("app.manifest"))?;
    res.set_icon("icon.ico");
    res.set_manifest_file("app.manifest");
    res.write_resource_file(work.join("resource.rc"))?;
    let windres = std::env::var("WINDRES").unwrap_or_else(|_| "windres".into());
    let status = std::process::Command::new(windres)
        .current_dir(&work)
        .args(["--target", "pe-x86-64", "resource.rc", "resource.o"])
        .status()?;
    if !status.success() {
        return Err(std::io::Error::other("Could not compile resource file"));
    }
    let obj = out.join("neramit_project_res.o");
    std::fs::copy(work.join("resource.o"), &obj)?;
    let _ = std::fs::remove_dir_all(&work);
    println!("cargo:rustc-link-arg-bin=NeramitPlanner={}", obj.display());
    Ok(())
}
