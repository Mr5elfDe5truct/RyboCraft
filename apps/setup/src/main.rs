//! RyboCraft's installer. `/S` installs without a window, `/D=<folder>` chooses where; the copy it
//! leaves as `Uninstall.exe` removes everything again (`/uninstall`, `/uninstall /S`).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const PAYLOAD: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/payload.zip"));
const VERSION: &str = env!("CARGO_PKG_VERSION");
const UNINSTALLER: &str = "Uninstall.exe";

type Res<T = ()> = Result<T, String>;

fn default_folder() -> PathBuf {
    let base = std::env::var_os("ProgramW6432").or_else(|| std::env::var_os("ProgramFiles")).unwrap_or_else(|| r"C:\Program Files".into());
    PathBuf::from(base).join("RG Studios").join("RyboCraft")
}

/// The files this installer carries, by name.
fn payload() -> Res<zip::ZipArchive<std::io::Cursor<&'static [u8]>>> {
    zip::ZipArchive::new(std::io::Cursor::new(PAYLOAD)).map_err(|e| format!("the installer is damaged: {e}"))
}

#[derive(Clone, Copy)]
struct Options {
    desktop: bool,
    associate: bool,
    path: bool,
}

#[cfg(windows)]
mod win {
    use super::*;
    use std::os::windows::process::CommandExt as _;
    use winreg::RegKey;
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE};

    const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\RyboCraft";
    const APP_KEY: &str = r"Software\RG Studios\RyboCraft";
    const ENV_KEY: &str = r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment";
    const NO_WINDOW: u32 = 0x0800_0000;

    fn hklm() -> RegKey {
        RegKey::predef(HKEY_LOCAL_MACHINE)
    }
    fn set(key: &str, name: &str, value: &str) -> Res {
        let (k, _) = hklm().create_subkey(key).map_err(|e| format!("{key}: {e}"))?;
        k.set_value(name, &value).map_err(|e| format!("{key}: {e}"))
    }

    pub fn installed_folder() -> Option<PathBuf> {
        let dir: String = hklm().open_subkey(APP_KEY).ok()?.get_value("InstallDir").ok()?;
        Some(PathBuf::from(dir)).filter(|d| d.is_absolute())
    }

    /// Run a script with values passed in the environment, never spliced into its text.
    fn powershell(script: &str, env: &[(&str, &str)]) -> Res {
        let mut c = std::process::Command::new("powershell.exe");
        c.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script]).creation_flags(NO_WINDOW);
        c.envs(env.iter().copied());
        let out = c.output().map_err(|e| format!("PowerShell: {e}"))?;
        if out.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) }
    }

    fn start_menu() -> PathBuf {
        let base = std::env::var_os("ProgramData").unwrap_or_else(|| r"C:\ProgramData".into());
        PathBuf::from(base).join(r"Microsoft\Windows\Start Menu\Programs\RG Studios")
    }
    fn desktop() -> PathBuf {
        PathBuf::from(std::env::var_os("PUBLIC").unwrap_or_else(|| r"C:\Users\Public".into())).join("Desktop")
    }

    fn shortcut(at: &Path, target: &Path, folder: &Path) -> Res {
        powershell(
            "$ErrorActionPreference='Stop'; $s=(New-Object -ComObject WScript.Shell).CreateShortcut($env:RC_LINK); $s.TargetPath=$env:RC_TARGET; $s.WorkingDirectory=$env:RC_DIR; $s.Description='RyboCraft: parametric CAD for 3D printing'; $s.Save()",
            &[("RC_LINK", &at.to_string_lossy()), ("RC_TARGET", &target.to_string_lossy()), ("RC_DIR", &folder.to_string_lossy())],
        )
    }

    /// The system PATH with `folder` added or taken out; expandable text stays expandable.
    fn edit_path(folder: &Path, add: bool) -> Res {
        let k = hklm().open_subkey_with_flags(ENV_KEY, KEY_READ | KEY_WRITE).map_err(|e| format!("PATH: {e}"))?;
        let mut raw = k.get_raw_value("Path").map_err(|e| format!("PATH: {e}"))?;
        let wide: Vec<u16> = raw.bytes.as_chunks::<2>().0.iter().map(|b| u16::from_le_bytes(*b)).take_while(|c| *c != 0).collect();
        let text = String::from_utf16(&wide).map_err(|e| format!("PATH: {e}"))?;
        let own = folder.to_string_lossy();
        let same = |p: &str| p.trim_end_matches('\\').eq_ignore_ascii_case(own.trim_end_matches('\\'));
        if add == text.split(';').any(same) {
            return Ok(());
        }
        let new = if add {
            format!("{};{own}", text.trim_end_matches(';'))
        } else {
            text.split(';').filter(|p| !same(p)).collect::<Vec<_>>().join(";")
        };
        raw.bytes = new.encode_utf16().chain([0]).flat_map(u16::to_le_bytes).collect::<Vec<u8>>().into();
        k.set_raw_value("Path", &raw).map_err(|e| format!("PATH: {e}"))
    }

    fn running() -> bool {
        std::process::Command::new("tasklist.exe")
            .args(["/FI", "IMAGENAME eq rybocraft.exe", "/NH"])
            .creation_flags(NO_WINDOW)
            .output()
            .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).to_ascii_lowercase().contains("rybocraft.exe"))
    }

    pub fn install(folder: &Path, o: Options, say: &dyn Fn(f32, &str)) -> Res {
        if !folder.is_absolute() {
            return Err("choose a full folder path".into());
        }
        if running() {
            return Err("RyboCraft is running: close it first".into());
        }
        let mut z = payload()?;
        if z.is_empty() {
            return Err("this installer was built without RyboCraft in it".into());
        }
        std::fs::create_dir_all(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
        let n = z.len();
        let mut size = 0u64;
        let mut files = vec![UNINSTALLER.to_string()];
        for i in 0..n {
            let mut f = z.by_index(i).map_err(|e| e.to_string())?;
            let name = f.name().map_err(|e| e.to_string())?.to_string();
            say(0.05 + 0.8 * i as f32 / n as f32, &format!("Copying {name}"));
            let at = folder.join(&name);
            let mut out = std::fs::File::create(&at).map_err(|e| format!("{}: {e}", at.display()))?;
            size += std::io::copy(&mut f, &mut out).map_err(|e| format!("{}: {e}", at.display()))?;
            files.push(name);
        }
        // The program alone, without the files it carries, would be smaller; it is kept whole.
        let me = std::env::current_exe().map_err(|e| e.to_string())?;
        let uninstaller = folder.join(UNINSTALLER);
        if me != uninstaller {
            std::fs::copy(&me, &uninstaller).map_err(|e| format!("{}: {e}", uninstaller.display()))?;
        }
        say(0.88, "Creating shortcuts");
        let exe = folder.join("rybocraft.exe");
        std::fs::create_dir_all(start_menu()).map_err(|e| e.to_string())?;
        shortcut(&start_menu().join("RyboCraft.lnk"), &exe, folder)?;
        let on_desktop = desktop().join("RyboCraft.lnk");
        if o.desktop {
            shortcut(&on_desktop, &exe, folder)?;
        }
        say(0.94, "Registering");
        let (dir, exe_s) = (folder.to_string_lossy().to_string(), exe.to_string_lossy().to_string());
        set(APP_KEY, "InstallDir", &dir)?;
        set(APP_KEY, "Files", &files.join("|"))?;
        set(UNINSTALL_KEY, "DisplayName", "RyboCraft")?;
        set(UNINSTALL_KEY, "DisplayVersion", VERSION)?;
        set(UNINSTALL_KEY, "Publisher", "R.G. Studios")?;
        set(UNINSTALL_KEY, "InstallLocation", &dir)?;
        set(UNINSTALL_KEY, "DisplayIcon", &exe_s)?;
        set(UNINSTALL_KEY, "UninstallString", &format!("\"{}\" /uninstall", uninstaller.display()))?;
        set(UNINSTALL_KEY, "QuietUninstallString", &format!("\"{}\" /uninstall /S", uninstaller.display()))?;
        if let Ok((k, _)) = hklm().create_subkey(UNINSTALL_KEY) {
            let _ = k.set_value("EstimatedSize", &((size / 1024) as u32));
            let _ = k.set_value("NoModify", &1u32);
            let _ = k.set_value("NoRepair", &1u32);
        }
        set(r"Software\Microsoft\Windows\CurrentVersion\App Paths\rybocraft.exe", "", &exe_s)?;
        let open = format!("\"{exe_s}\" \"%1\"");
        for (id, what) in [("RyboCraft.Design", "RyboCraft Design"), ("RyboCraft.Model", "3D model")] {
            set(&format!(r"Software\Classes\{id}"), "", what)?;
            set(&format!(r"Software\Classes\{id}\DefaultIcon"), "", &format!("{exe_s},0"))?;
            set(&format!(r"Software\Classes\{id}\shell\open\command"), "", &open)?;
        }
        if o.associate {
            set(r"Software\Classes\.sc", "", "RyboCraft.Design")?;
        }
        set(r"Software\Classes\.sc\OpenWithProgids", "RyboCraft.Design", "")?;
        for ext in ["step", "stp", "iges", "igs", "3mf", "stl", "obj"] {
            set(&format!(r"Software\Classes\.{ext}\OpenWithProgids"), "RyboCraft.Model", "")?;
        }
        if o.path {
            edit_path(folder, true)?;
        }
        say(1.0, "Done");
        Ok(())
    }

    pub fn uninstall() -> Res {
        if running() {
            return Err("RyboCraft is running: close it first".into());
        }
        let folder = installed_folder().ok_or("RyboCraft is not installed")?;
        let files: String = hklm().open_subkey(APP_KEY).and_then(|k| k.get_value("Files")).unwrap_or_default();
        // Only the files that were installed go, and the folders only when that leaves them empty.
        for f in files.split('|').filter(|f| !f.is_empty() && !f.contains(['/', '\\', ':']) && *f != UNINSTALLER) {
            let _ = std::fs::remove_file(folder.join(f));
        }
        let _ = std::fs::remove_file(start_menu().join("RyboCraft.lnk"));
        let _ = std::fs::remove_dir(start_menu());
        let _ = std::fs::remove_file(desktop().join("RyboCraft.lnk"));
        edit_path(&folder, false)?;
        if let Ok(k) = hklm().open_subkey_with_flags(r"Software\Classes\.sc", KEY_READ | KEY_WRITE)
            && k.get_value::<String, _>("").is_ok_and(|v| v == "RyboCraft.Design")
        {
            let _ = k.delete_value("");
        }
        for (ext, id) in ["step", "stp", "iges", "igs", "3mf", "stl", "obj"].map(|e| (e, "RyboCraft.Model")).into_iter().chain([("sc", "RyboCraft.Design")]) {
            if let Ok(k) = hklm().open_subkey_with_flags(format!(r"Software\Classes\.{ext}\OpenWithProgids"), KEY_WRITE) {
                let _ = k.delete_value(id);
            }
        }
        for key in [
            r"Software\Classes\RyboCraft.Design",
            r"Software\Classes\RyboCraft.Model",
            r"Software\Microsoft\Windows\CurrentVersion\App Paths\rybocraft.exe",
            UNINSTALL_KEY,
            APP_KEY,
        ] {
            let _ = hklm().delete_subkey_all(key);
        }
        // This program is in that folder: it goes once it has closed.
        let script = "Start-Sleep 2; Remove-Item -LiteralPath $env:RC_EXE -Force -ErrorAction SilentlyContinue; foreach ($d in $env:RC_DIR, (Split-Path $env:RC_DIR)) { if ((Test-Path -LiteralPath $d) -and -not (Get-ChildItem -LiteralPath $d -Force)) { Remove-Item -LiteralPath $d } }";
        std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script])
            .env("RC_EXE", folder.join(UNINSTALLER))
            .env("RC_DIR", &folder)
            .current_dir(std::env::temp_dir())
            .creation_flags(NO_WINDOW)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Start RyboCraft as the signed-in user, not as the administrator the installer runs as.
    pub fn launch(folder: &Path) {
        let _ = std::process::Command::new("explorer.exe").arg(folder.join("rybocraft.exe")).spawn();
    }
}

#[cfg(not(windows))]
mod win {
    use super::*;
    pub fn installed_folder() -> Option<PathBuf> {
        None
    }
    pub fn install(_: &Path, _: Options, _: &dyn Fn(f32, &str)) -> Res {
        Err("Windows only".into())
    }
    pub fn uninstall() -> Res {
        Err("Windows only".into())
    }
    pub fn launch(_: &Path) {}
}

enum Page {
    Ready,
    Working,
    Done,
    Failed(String),
}

struct Wizard {
    remove: bool,
    folder: String,
    options: Options,
    launch: bool,
    page: Page,
    logo: Option<egui::TextureHandle>,
    progress: Arc<Mutex<(f32, String, Option<Res>)>>,
}

impl Wizard {
    fn go(&mut self, ctx: &egui::Context) {
        self.page = Page::Working;
        let (progress, ctx, folder, options, remove) = (self.progress.clone(), ctx.clone(), PathBuf::from(self.folder.trim()), self.options, self.remove);
        std::thread::spawn(move || {
            let say = |f: f32, t: &str| {
                if let Ok(mut p) = progress.lock() {
                    (p.0, p.1) = (f, t.to_string());
                }
                ctx.request_repaint();
            };
            let r = if remove { win::uninstall() } else { win::install(&folder, options, &say) };
            if let Ok(mut p) = progress.lock() {
                p.2 = Some(r);
            }
            ctx.request_repaint();
        });
    }
}

impl eframe::App for Wizard {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.logo.is_none()
            && let Ok(i) = image::load_from_memory(include_bytes!("../../../assets/branding/rybocraft/rybocraft-512.png"))
        {
            let i = i.resize(192, 192, image::imageops::FilterType::Lanczos3).to_rgba8();
            let size = [i.width() as usize, i.height() as usize];
            self.logo = Some(ctx.load_texture("logo", egui::ColorImage::from_rgba_unmultiplied(size, &i), Default::default()));
        }
        if let Ok(mut p) = self.progress.lock()
            && let Some(r) = p.2.take()
        {
            self.page = match r {
                Ok(()) => Page::Done,
                Err(e) => Page::Failed(e),
            };
        }
        egui::Frame::new().inner_margin(24.0).show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(t) = &self.logo {
                    ui.image((t.id(), egui::vec2(72.0, 72.0)));
                }
                ui.vertical(|ui| {
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(format!("RyboCraft {VERSION}")).size(26.0).strong());
                    ui.label("R.G. Studios · Parametric CAD for 3D printing");
                });
            });
            ui.add_space(14.0);
            ui.separator();
            ui.add_space(10.0);
            let mut go = false;
            match &self.page {
                Page::Ready if self.remove => {
                    ui.label("Remove RyboCraft from this computer? Your designs and settings stay.");
                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        go = ui.button("   Uninstall   ").clicked();
                        if ui.button("   Cancel   ").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                }
                Page::Ready => {
                    ui.label("Install to");
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.folder).desired_width(390.0));
                        if ui.button("Browse…").clicked()
                            && let Some(d) = rfd::FileDialog::new().pick_folder()
                        {
                            self.folder = d.join("RyboCraft").to_string_lossy().into_owned();
                        }
                    });
                    ui.add_space(8.0);
                    ui.checkbox(&mut self.options.desktop, "Desktop shortcut");
                    ui.checkbox(&mut self.options.associate, "Open .sc designs with RyboCraft");
                    ui.checkbox(&mut self.options.path, "Add rybocraft-cli to the system PATH");
                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        go = ui.button(egui::RichText::new("   Install   ").strong()).clicked();
                        if ui.button("   Cancel   ").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                }
                Page::Working => {
                    let (f, t) = self.progress.lock().map(|p| (p.0, p.1.clone())).unwrap_or_default();
                    ui.label(t);
                    ui.add(egui::ProgressBar::new(f).show_percentage());
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                }
                Page::Done => {
                    ui.label(if self.remove { "RyboCraft was removed." } else { "RyboCraft is installed." });
                    if !self.remove {
                        ui.checkbox(&mut self.launch, "Start RyboCraft");
                    }
                    ui.add_space(16.0);
                    if ui.button("   Finish   ").clicked() {
                        if !self.remove && self.launch {
                            win::launch(Path::new(self.folder.trim()));
                        }
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
                Page::Failed(e) => {
                    ui.colored_label(egui::Color32::from_rgb(230, 90, 60), e);
                    ui.add_space(16.0);
                    go = ui.button("   Try again   ").clicked();
                }
            }
            if go {
                self.go(&ctx);
            }
        });
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |f: &str| args.iter().any(|a| a.eq_ignore_ascii_case(f));
    let folder = args
        .iter()
        .find_map(|a| a.strip_prefix("/D=").map(PathBuf::from))
        .or_else(win::installed_folder)
        .unwrap_or_else(default_folder);
    let own_name = std::env::current_exe().ok().and_then(|p| p.file_name().map(|n| n.eq_ignore_ascii_case(UNINSTALLER))).unwrap_or(false);
    let remove = flag("/uninstall") || own_name;
    let options = Options { desktop: true, associate: true, path: true };
    if flag("/S") {
        let r = if remove { win::uninstall() } else { win::install(&folder, options, &|_, _| ()) };
        std::process::exit(match r {
            Ok(()) => 0,
            Err(e) => {
                let _ = std::fs::write(std::env::temp_dir().join("RyboCraft-setup.log"), e);
                1
            }
        });
    }
    let native = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([560.0, 360.0]).with_resizable(false).with_title("RyboCraft Setup"),
        ..Default::default()
    };
    let wizard = Wizard {
        remove,
        folder: folder.to_string_lossy().into_owned(),
        options,
        launch: true,
        page: Page::Ready,
        logo: None,
        progress: Arc::default(),
    };
    let r = eframe::run_native("RyboCraft Setup", native, Box::new(|_| Ok(Box::new(wizard))));
    if let Err(e) = r {
        rfd::MessageDialog::new().set_title("RyboCraft Setup").set_description(format!("The window could not open ({e}). Run the installer with /S to install without it.")).show();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_installs_under_rg_studios() {
        assert!(super::default_folder().ends_with(r"RG Studios\RyboCraft"));
    }
}
