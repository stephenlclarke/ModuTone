// 64-bit SFX stub for ModuTone installer and model addons.
//
// Supports two payload sources:
//
// **External payload** (preferred for large installers >4 GiB total):
//   Looks for {exe_stem}.7z next to this exe. No PE size limit.
//
// **Embedded payload** (for self-contained SFX <4 GiB):
//   [this PE binary]
//   [7z archive]
//   [8-byte LE: offset where the 7z archive starts]
//
// After extracting the archive, two modes based on contents:
//
// **Installer mode** (archive contains *-setup.exe):
//   1. Extracts archive to temp
//   2. Runs the NSIS installer (POSTINSTALL hook copies models)
//   3. Cleans up
//
// **Addon mode** (archive contains only model files, no installer):
//   1. Extracts archive to temp
//   2. Finds ModuTone install directory (registry or default paths)
//   3. Copies model files to {install_dir}/models/
//   4. Cleans up

#![windows_subsystem = "windows"]

use std::env;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Standalone 7za.exe (64-bit) embedded at compile time.
const SEVEN_ZA: &[u8] = include_bytes!("../../7za.exe");

fn main() {
    // Log to file for debugging
    let log_path = env::temp_dir().join("modutone-sfx-log.txt");
    let _ = fs::write(&log_path, format!("SFX stub started at {:?}\n", std::time::SystemTime::now()));

    match run() {
        Ok(()) => {
            log(&log_path, "SFX completed successfully");
        }
        Err(e) => {
            log(&log_path, &format!("SFX FAILED: {e}"));
            show_error(&format!("ModuTone Setup failed:\n\n{e}"));
            std::process::exit(1);
        }
    }
}

fn log(path: &Path, msg: &str) {
    use std::io::Write;
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{msg}");
    }
}

/// Look for a companion .7z file next to the exe.
fn find_external_archive(exe_path: &Path) -> Option<PathBuf> {
    let stem = exe_path.file_stem()?.to_str()?;
    let dir = exe_path.parent()?;
    let archive_path = dir.join(format!("{stem}.7z"));
    if archive_path.exists() {
        Some(archive_path)
    } else {
        None
    }
}

/// Check if this exe has an embedded payload (8-byte LE offset trailer).
fn find_embedded_payload(exe_path: &Path) -> Option<(u64, u64)> {
    let mut file = fs::File::open(exe_path).ok()?;
    let file_size = file.metadata().ok()?.len();
    if file_size < 16 {
        return None;
    }
    file.seek(SeekFrom::End(-8)).ok()?;
    let mut offset_buf = [0u8; 8];
    file.read_exact(&mut offset_buf).ok()?;
    let archive_offset = u64::from_le_bytes(offset_buf);
    if archive_offset == 0 || archive_offset >= file_size - 8 {
        return None;
    }
    let archive_size = file_size - 8 - archive_offset;
    Some((archive_offset, archive_size))
}

fn run() -> Result<(), String> {
    let log_path = env::temp_dir().join("modutone-sfx-log.txt");
    let exe_path = env::current_exe().map_err(|e| format!("Cannot find own path: {e}"))?;
    log(&log_path, &format!("Exe path: {}", exe_path.display()));

    // Create temp directory
    let temp_base = env::temp_dir();
    let temp_dir = temp_base.join(format!("ModuTone-Setup-{}", std::process::id()));
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Cannot create temp directory: {e}"))?;

    log(&log_path, &format!("Temp dir: {}", temp_dir.display()));

    let _cleanup = CleanupGuard(temp_dir.clone());

    // Write embedded 7za.exe to temp
    let seven_za_path = temp_dir.join("7za.exe");
    fs::write(&seven_za_path, SEVEN_ZA)
        .map_err(|e| format!("Cannot write 7za.exe: {e}"))?;

    // Determine archive source: external .7z file or embedded payload
    let archive_path;

    if let Some(ext_archive) = find_external_archive(&exe_path) {
        // External payload mode — companion .7z file next to exe
        log(&log_path, &format!("External payload: {}", ext_archive.display()));
        archive_path = ext_archive;
    } else if let Some((offset, size)) = find_embedded_payload(&exe_path) {
        // Embedded payload mode — extract from own exe
        log(&log_path, &format!("Embedded payload: offset={offset}, size={size}"));
        let mut file = fs::File::open(&exe_path)
            .map_err(|e| format!("Cannot open own exe: {e}"))?;
        let temp_archive = temp_dir.join("payload.7z");
        extract_payload(&mut file, offset, size, &temp_archive)?;
        log(&log_path, "Extracted embedded payload to temp");
        archive_path = temp_archive;
    } else {
        return Err(
            "No installer payload found.\n\n\
             If this is an external-payload installer, make sure the .7z file \
             is in the same folder as this .exe."
                .into(),
        );
    }

    // Run 7za.exe to extract the archive
    let extract_dir = temp_dir.join("contents");
    fs::create_dir_all(&extract_dir)
        .map_err(|e| format!("Cannot create extract dir: {e}"))?;

    log(&log_path, &format!("Extracting {} ...", archive_path.display()));

    let status = Command::new(&seven_za_path)
        .args([
            "x",
            archive_path.to_str().unwrap_or("payload.7z"),
            &format!("-o{}", extract_dir.display()),
            "-y",
        ])
        .creation_flags(0x08000000)
        .status()
        .map_err(|e| format!("Cannot run 7za.exe: {e}"))?;

    log(&log_path, &format!("7za exit code: {:?}", status.code()));

    if !status.success() {
        return Err(format!(
            "Archive extraction failed (exit code: {:?})",
            status.code()
        ));
    }

    // List extracted contents
    if let Ok(entries) = fs::read_dir(&extract_dir) {
        for entry in entries.flatten() {
            log(&log_path, &format!("  extracted: {}", entry.path().display()));
        }
    }

    // Decide mode based on extracted contents
    if let Some(installer) = find_installer(&extract_dir) {
        log(&log_path, &format!("Installer mode: {}", installer.display()));
        // ── Installer mode ──
        let install_status = Command::new(&installer)
            .current_dir(&extract_dir)
            .status()
            .map_err(|e| format!("Cannot run installer: {e}"))?;

        if !install_status.success() {
            // Non-zero from NSIS is usually "user cancelled" — not fatal
        }
    } else {
        // ── Addon mode ── (model files only)
        install_addon_models(&extract_dir)?;
    }

    Ok(())
}

/// Stream the archive portion from the exe to a temp file.
fn extract_payload(
    file: &mut fs::File,
    offset: u64,
    size: u64,
    dest: &PathBuf,
) -> Result<(), String> {
    file.seek(SeekFrom::Start(offset))
        .map_err(|e| format!("Seek to payload failed: {e}"))?;

    let mut out =
        fs::File::create(dest).map_err(|e| format!("Cannot create payload file: {e}"))?;

    let mut remaining = size;
    let mut buf = vec![0u8; 1024 * 1024];

    while remaining > 0 {
        let to_read = remaining.min(buf.len() as u64) as usize;
        let n = file
            .read(&mut buf[..to_read])
            .map_err(|e| format!("Read payload failed: {e}"))?;
        if n == 0 {
            return Err("Unexpected end of file while reading payload.".into());
        }
        out.write_all(&buf[..n])
            .map_err(|e| format!("Write payload failed: {e}"))?;
        remaining -= n as u64;
    }

    Ok(())
}

/// Find an NSIS installer in the extracted directory.
fn find_installer(dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            let lower = name.to_lowercase();
            if lower.ends_with("-setup.exe") || lower.ends_with("_setup.exe") {
                return Some(path);
            }
        }
    }
    None
}

/// Addon mode: copy extracted model files to the ModuTone install directory.
fn install_addon_models(extract_dir: &Path) -> Result<(), String> {
    let install_dir = find_install_dir()?;
    let models_dir = install_dir.join("models");

    fs::create_dir_all(&models_dir)
        .map_err(|e| format!("Cannot create models directory: {e}"))?;

    // Look for .gguf files in extract_dir and extract_dir/models/
    let mut copied = 0u32;
    for search_dir in [extract_dir.to_path_buf(), extract_dir.join("models")] {
        if let Ok(entries) = fs::read_dir(&search_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.to_lowercase().ends_with(".gguf") {
                        let dest = models_dir.join(name);
                        fs::copy(&path, &dest).map_err(|e| {
                            format!("Cannot copy {name} to {}: {e}", dest.display())
                        })?;
                        copied += 1;
                    }
                }
            }
        }
    }

    if copied == 0 {
        return Err("No model files found in the archive.".into());
    }

    show_info(&format!(
        "Successfully installed {copied} model file(s) to:\n{}",
        models_dir.display()
    ));
    Ok(())
}

/// Find the ModuTone installation directory.
fn find_install_dir() -> Result<PathBuf, String> {
    // Check common install locations
    let candidates = [
        PathBuf::from(r"C:\Program Files\ModuTone"),
        // Per-user install location (Tauri NSIS)
        env::var("LOCALAPPDATA")
            .map(|la| PathBuf::from(la).join("ModuTone"))
            .unwrap_or_default(),
        env::var("LOCALAPPDATA")
            .map(|la| PathBuf::from(la).join("Programs").join("ModuTone"))
            .unwrap_or_default(),
    ];

    for path in &candidates {
        if path.as_os_str().is_empty() {
            continue;
        }
        // Check for the main executable as proof of installation
        if path.join("modutone-app.exe").exists() || path.join("ModuTone.exe").exists() {
            return Ok(path.clone());
        }
    }

    Err(
        "ModuTone installation not found.\n\n\
         Please install ModuTone first, then run this addon installer."
            .into(),
    )
}

/// Show a Windows message box for errors.
fn show_error(msg: &str) {
    show_message_box(msg, "ModuTone Setup - Error", 0x10); // MB_ICONERROR
}

/// Show a Windows message box for informational messages.
fn show_info(msg: &str) {
    show_message_box(msg, "ModuTone Setup", 0x40); // MB_ICONINFORMATION
}

fn show_message_box(msg: &str, title: &str, flags: u32) {
    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use std::ptr;

        #[link(name = "user32")]
        extern "system" {
            fn MessageBoxW(
                hwnd: *mut (),
                text: *const u16,
                caption: *const u16,
                utype: u32,
            ) -> i32;
        }

        let text: Vec<u16> = OsStr::new(msg).encode_wide().chain(Some(0)).collect();
        let caption: Vec<u16> = OsStr::new(title).encode_wide().chain(Some(0)).collect();

        unsafe {
            MessageBoxW(ptr::null_mut(), text.as_ptr(), caption.as_ptr(), flags);
        }
    }

    #[cfg(not(windows))]
    {
        let _ = (title, flags);
        eprintln!("{msg}");
    }
}

/// Best-effort cleanup of the temp directory on drop.
struct CleanupGuard(PathBuf);

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Extension trait to add creation_flags for Windows process creation.
trait CommandExt {
    fn creation_flags(&mut self, flags: u32) -> &mut Self;
}

impl CommandExt for Command {
    #[cfg(windows)]
    fn creation_flags(&mut self, flags: u32) -> &mut Self {
        use std::os::windows::process::CommandExt as WinCommandExt;
        WinCommandExt::creation_flags(self, flags);
        self
    }

    #[cfg(not(windows))]
    fn creation_flags(&mut self, _flags: u32) -> &mut Self {
        self
    }
}
