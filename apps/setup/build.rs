//! Packs the folder `RYBOCRAFT_PAYLOAD` names into the installer (nothing when it is not set).
use std::io::Write as _;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-env-changed=RYBOCRAFT_PAYLOAD");
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").ok_or("OUT_DIR")?);
    let mut zip = zip::ZipWriter::new(std::fs::File::create(out.join("payload.zip"))?);
    if let Some(dir) = std::env::var_os("RYBOCRAFT_PAYLOAD") {
        for e in std::fs::read_dir(dir)? {
            let e = e?;
            println!("cargo:rerun-if-changed={}", e.path().display());
            zip.start_file(e.file_name().to_string_lossy(), zip::write::SimpleFileOptions::default())?;
            zip.write_all(&std::fs::read(e.path())?)?;
        }
    }
    zip.finish()?;
    if std::env::var("CARGO_CFG_TARGET_OS")? == "windows" {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/icons/app/windows/exports/rybocraft.ico");
        let path = |p: &std::path::Path| p.display().to_string().replace('\\', "/");
        std::fs::copy(root, out.join("setup.ico"))?;
        std::fs::write(
            out.join("setup.manifest"),
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
<trustInfo xmlns="urn:schemas-microsoft-com:asm.v3"><security><requestedPrivileges>
<requestedExecutionLevel level="requireAdministrator" uiAccess="false"/>
</requestedPrivileges></security></trustInfo>
<dependency><dependentAssembly><assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0" processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/></dependentAssembly></dependency>
<application xmlns="urn:schemas-microsoft-com:asm.v3"><windowsSettings><dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness></windowsSettings></application>
</assembly>"#,
        )?;
        let v = std::env::var("CARGO_PKG_VERSION")?;
        let n: Vec<&str> = v.split(['-', '+']).next().unwrap_or("0.0.0").split('.').collect();
        let rc = out.join("setup.rc");
        std::fs::write(
            &rc,
            format!(
                "1 ICON \"{}\"\n1 24 \"{}\"\n1 VERSIONINFO\nFILEVERSION {},0\nPRODUCTVERSION {},0\nFILEOS 0x40004\nFILETYPE 0x1\n{{\nBLOCK \"StringFileInfo\"\n{{\nBLOCK \"040904B0\"\n{{\nVALUE \"CompanyName\", \"R.G. Studios\"\nVALUE \"FileDescription\", \"RyboCraft Setup\"\nVALUE \"FileVersion\", \"{v}\"\nVALUE \"ProductName\", \"RyboCraft\"\nVALUE \"ProductVersion\", \"{v}\"\n}}\n}}\nBLOCK \"VarFileInfo\"\n{{\nVALUE \"Translation\", 0x0409, 1200\n}}\n}}\n",
                path(&out.join("setup.ico")),
                path(&out.join("setup.manifest")),
                n.join(","),
                n.join(",")
            ),
        )?;
        embed_resource::compile(&rc, embed_resource::NONE).manifest_required()?;
    }
    Ok(())
}
