// Hide the background command prompt console window on Windows release builds
#![windows_subsystem = "windows"]

use eframe::egui;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

#[derive(Clone, Debug)]
struct CleanupItem {
    path: String,
    size: u64,
    category: String,
    reason: String,
    selected: bool,
}

#[derive(Clone, Debug)]
struct PurgeResultItem {
    path: String,
    success: bool,
    error_message: Option<String>,
}

enum AppMessage {
    AuditFinished(Vec<CleanupItem>),
    PurgeFinished(Vec<PurgeResultItem>),
}

struct SystemCleanerApp {
    user_profile: String,
    system_drive: String,
    items: Vec<CleanupItem>,
    purge_results: Vec<PurgeResultItem>,
    is_working: bool,
    status_message: String,
    require_admin: bool,
    auto_purge_triggered: bool,
    rx: Receiver<AppMessage>,
    tx: Sender<AppMessage>,
}

impl Default for SystemCleanerApp {
    fn default() -> Self {
        let user_profile = std::env::var("USERPROFILE")
            .unwrap_or_else(|_| "C:\\Users\\Default".to_string());

        let system_drive = std::env::var("SystemDrive")
            .unwrap_or_else(|_| "C:".to_string());

        let (tx, rx) = channel();

        Self {
            user_profile,
            system_drive,
            items: Vec::new(),
            purge_results: Vec::new(),
            is_working: false,
            status_message: "Ready. Double-click any row to open folder in Explorer.".to_string(),
            require_admin: true,
            auto_purge_triggered: false,
            rx,
            tx,
        }
    }
}

impl eframe::App for SystemCleanerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_rgb(13, 13, 16);
        visuals.window_fill = egui::Color32::from_rgb(18, 18, 22);
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(24, 24, 29);
        visuals.widgets.noninteractive.rounding = egui::Rounding::same(4.0);
        visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);
        visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
        visuals.widgets.active.rounding = egui::Rounding::same(4.0);
        visuals.selection.bg_fill = egui::Color32::from_rgb(0, 122, 204);
        ctx.set_visuals(visuals);

        if !self.auto_purge_triggered {
            let args: Vec<String> = std::env::args().collect();
            if args.iter().any(|arg| arg == "--auto-purge") {
                self.auto_purge_triggered = true;
                self.run_audit_and_purge_all();
            }
        }

        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                AppMessage::AuditFinished(results) => {
                    let count = results.len();
                    self.items = results;
                    self.status_message = format!("Audit successful. Discovered {} cleanable items.", count);
                    self.is_working = false;
                }
                AppMessage::PurgeFinished(results) => {
                    let failed_count = results.iter().filter(|r| !r.success).count();
                    let success_count = results.len() - failed_count;
                    self.purge_results = results;
                    self.status_message = format!("Purge complete: {} successfully removed, {} failed.", success_count, failed_count);
                    self.is_working = false;
                    self.run_audit();
                }
            }
        }

        egui::TopBottomPanel::top("header_panel").show(ctx, |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.heading(egui::RichText::new("OrphanCleaner // SYSTEM RESIDUAL & SPACE AUDITOR").color(egui::Color32::from_rgb(70, 180, 240)).strong());
                    ui.add_space(2.0);
                    ui.label(egui::RichText::new(format!("System Drive: {}  •  User Profile: {}", self.system_drive, self.user_profile)).size(11.0).color(egui::Color32::GRAY));
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(8.0);
                    if self.is_working {
                        ui.spinner();
                        ui.label(egui::RichText::new("Scanning disk structures...").color(egui::Color32::YELLOW));
                    } else {
                        let scan_btn = ui.add_sized(
                            [160.0, 32.0],
                            egui::Button::new(egui::RichText::new("Run System Audit").strong().color(egui::Color32::WHITE))
                                .fill(egui::Color32::from_rgb(0, 110, 190))
                        );
                        if scan_btn.clicked() {
                            self.run_audit();
                        }
                    }
                });
            });
            ui.add_space(8.0);
        });

        egui::TopBottomPanel::bottom("footer_panel").show(ctx, |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let total_bytes: u64 = self.items.iter().filter(|i| i.selected).map(|i| i.size).sum();
                ui.label(egui::RichText::new(format!("Reclaimable Space Selected: {:.2} MB", total_bytes as f64 / (1024.0 * 1024.0))).strong().color(egui::Color32::from_rgb(100, 240, 140)));

                ui.separator();
                ui.checkbox(&mut self.require_admin, "Request UAC Elevation (Admin Mode)");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(8.0);
                    let purge_btn = ui.add_sized(
                        [210.0, 34.0],
                        egui::Button::new(egui::RichText::new("Purge Selected & Hunt Locks").strong().color(egui::Color32::WHITE))
                            .fill(egui::Color32::from_rgb(190, 45, 45))
                    );
                    if purge_btn.on_hover_text("Automatically terminates locking processes, clears attributes, and permanently deletes items.").clicked() {
                        self.purge_selected();
                    }
                });
            });
            ui.add_space(8.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&self.status_message).color(egui::Color32::from_rgb(210, 225, 245)));
            });
            ui.add_space(6.0);

            // Full-Width Slim Compact Table Header Bar
            let header_bg = egui::Color32::from_rgb(28, 28, 35);
            let full_w = ui.available_width();
            let (rect, _resp) = ui.allocate_exact_size(egui::vec2(full_w, 20.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 0.0, header_bg);

            let mut header_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center))
            );
            header_ui.add_space(4.0);
            header_ui.add_sized([full_w * 0.05, 18.0], egui::Label::new(egui::RichText::new("Sel").strong().size(11.0)));
            header_ui.add_sized([full_w * 0.22, 18.0], egui::Label::new(egui::RichText::new("Category").strong().size(11.0)));
            header_ui.add_sized([full_w * 0.15, 18.0], egui::Label::new(egui::RichText::new("Size (MB)").strong().size(11.0)));
            header_ui.add_sized([full_w * 0.55, 18.0], egui::Label::new(egui::RichText::new("Target Path & Description (Double-click any row to open folder)").strong().size(11.0)));

            let available_height = ui.available_height();
            let upper_height = if self.purge_results.iter().any(|r| !r.success) { available_height * 0.55 } else { available_height };

            // Full-Width Scrollable Grid Rows
            egui::ScrollArea::vertical().id_salt("audit_scroll").max_height(upper_height).auto_shrink([false, false]).show(ui, |ui| {
                if self.items.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label(egui::RichText::new("No system leftovers or residuals found. Click 'Run System Audit' to scan.").color(egui::Color32::GRAY));
                    });
                } else {
                    let full_width = ui.available_width();
                    for (index, item) in self.items.iter_mut().enumerate() {
                        let row_bg = if index % 2 == 0 {
                            egui::Color32::from_rgb(18, 18, 22)
                        } else {
                            egui::Color32::from_rgb(23, 23, 28)
                        };

                        let row_size = egui::vec2(full_width, 32.0);
                        let (rect, response) = ui.allocate_exact_size(row_size, egui::Sense::click());

                        ui.painter().rect_filled(rect, 0.0, row_bg);

                        let mut child_ui = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(rect)
                                .layout(egui::Layout::left_to_right(egui::Align::Center))
                        );

                        child_ui.add_space(4.0);

                        child_ui.add_sized([full_width * 0.05, 24.0], egui::Checkbox::new(&mut item.selected, ""));

                        child_ui.add_sized([full_width * 0.22, 24.0], egui::Label::new(
                            egui::RichText::new(&item.category).color(egui::Color32::from_rgb(110, 190, 255)).size(11.5).strong()
                        ));

                        let size_mb = item.size as f64 / (1024.0 * 1024.0);
                        child_ui.add_sized([full_width * 0.15, 24.0], egui::Label::new(
                            egui::RichText::new(format!("{:.2}", size_mb)).monospace().size(11.5).color(egui::Color32::from_rgb(240, 220, 120))
                        ));

                        child_ui.add_sized([full_width * 0.55, 24.0], |ui: &mut egui::Ui| {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(&item.path).monospace().size(11.0).color(egui::Color32::from_rgb(235, 235, 235)));
                                ui.label(egui::RichText::new(&item.reason).size(9.5).color(egui::Color32::GRAY));
                            }).response
                        });

                        if response.double_clicked() {
                            open_in_explorer(&item.path);
                        }
                    }
                }
            });

            // Error Logging Panel
            let failures: Vec<&PurgeResultItem> = self.purge_results.iter().filter(|r| !r.success).collect();
            if !failures.is_empty() {
                ui.add_space(8.0);
                ui.colored_label(egui::Color32::LIGHT_RED, format!("Deletion Errors / Locks Encountered ({} items failed):", failures.len()));

                egui::ScrollArea::vertical().id_salt("error_scroll").max_height(available_height * 0.35).show(ui, |ui| {
                    for fail in failures {
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::RED, "✖");
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(&fail.path).monospace().size(11.0).color(egui::Color32::LIGHT_RED));
                                if let Some(ref err) = fail.error_message {
                                    ui.label(egui::RichText::new(format!("Reason: {}", err)).size(10.0).color(egui::Color32::YELLOW));
                                }
                            });
                        });
                        ui.separator();
                    }
                });
            }
        });
    }
}

fn open_in_explorer(path_str: &str) {
    #[cfg(target_os = "windows")]
    {
        let path = Path::new(path_str);
        if path.is_file() {
            let _ = std::process::Command::new("explorer")
                .args(&[format!("/select,{}", path_str)])
                .spawn();
        } else {
            let _ = std::process::Command::new("explorer")
                .arg(path_str)
                .spawn();
        }
    }
}

fn hunt_and_kill_locking_processes(target_path: &str) {
    #[cfg(target_os = "windows")]
    {
        if let Some(folder_name) = Path::new(target_path).file_name().and_then(|n| n.to_str()) {
            if folder_name.len() > 2 {
                let _ = std::process::Command::new("powershell")
                    .args(&["-Command", &format!(
                        "Get-Process | Where-Object {{ $_.Path -like '*{0}*' }} | Stop-Process -Force -ErrorAction SilentlyContinue",
                        folder_name
                    )])
                    .status();
            }
        }
    }
}

impl SystemCleanerApp {
    fn run_audit(&mut self) {
        if self.is_working { return; }
        self.is_working = true;
        self.status_message = "Scanning system directories and residuals...".to_string();

        let profile_path = PathBuf::from(&self.user_profile);
        let tx = self.tx.clone();

        thread::spawn(move || {
            let discovered = scan_all_targets(&profile_path);
            let _ = tx.send(AppMessage::AuditFinished(discovered));
        });
    }

    fn run_audit_and_purge_all(&mut self) {
        if self.is_working { return; }
        self.is_working = true;
        self.status_message = "Elevated mode: hunting locks and purging...".to_string();

        let profile_path = PathBuf::from(&self.user_profile);
        let tx = self.tx.clone();

        thread::spawn(move || {
            let discovered = scan_all_targets(&profile_path);
            let paths_to_remove: Vec<String> = discovered.iter().map(|i| i.path.clone()).collect();

            let mut results = Vec::new();
            for p in paths_to_remove {
                hunt_and_kill_locking_processes(&p);
                let path = Path::new(&p);
                if path.is_file() {
                    if let Ok(metadata) = fs::metadata(path) {
                        let mut perms = metadata.permissions();
                        if perms.readonly() {
                            perms.set_readonly(false);
                            let _ = fs::set_permissions(path, perms);
                        }
                    }
                    match fs::remove_file(path) {
                        Ok(_) => results.push(PurgeResultItem { path: p, success: true, error_message: None }),
                        Err(e) => results.push(PurgeResultItem { path: p, success: false, error_message: Some(e.to_string()) }),
                    }
                } else if path.is_dir() {
                    let _ = clear_readonly_recursive(path);
                    match fs::remove_dir_all(path) {
                        Ok(_) => results.push(PurgeResultItem { path: p, success: true, error_message: None }),
                        Err(e) => results.push(PurgeResultItem { path: p, success: false, error_message: Some(e.to_string()) }),
                    }
                }
            }

            let _ = tx.send(AppMessage::AuditFinished(discovered));
            let _ = tx.send(AppMessage::PurgeFinished(results));
        });
    }

    fn purge_selected(&mut self) {
        if self.is_working { return; }
        let to_remove: Vec<String> = self.items.iter().filter(|i| i.selected).map(|i| i.path.clone()).collect();
        if to_remove.is_empty() { return; }

        #[cfg(target_os = "windows")]
        if self.require_admin {
            if let Ok(exe) = std::env::current_exe() {
                let exe_str = exe.to_string_lossy();
                let ps_command = format!("Start-Process -FilePath '{}' -ArgumentList '--auto-purge' -Verb RunAs", exe_str);
                let status = std::process::Command::new("powershell")
                    .args(&["-Command", &ps_command])
                    .status();

                if status.is_ok() && status.unwrap().success() {
                    std::process::exit(0);
                }
            }
        }

        self.is_working = true;
        self.status_message = "Terminating locks and purging selected items...".to_string();
        let tx = self.tx.clone();

        thread::spawn(move || {
            let mut results = Vec::new();

            for p in to_remove {
                hunt_and_kill_locking_processes(&p);

                let path = Path::new(&p);
                if path.is_file() {
                    if let Ok(metadata) = fs::metadata(path) {
                        let mut perms = metadata.permissions();
                        if perms.readonly() {
                            perms.set_readonly(false);
                            let _ = fs::set_permissions(path, perms);
                        }
                    }

                    match fs::remove_file(path) {
                        Ok(_) => results.push(PurgeResultItem { path: p, success: true, error_message: None }),
                        Err(e) => results.push(PurgeResultItem { path: p, success: false, error_message: Some(e.to_string()) }),
                    }
                } else if path.is_dir() {
                    let _ = clear_readonly_recursive(path);
                    match fs::remove_dir_all(path) {
                        Ok(_) => results.push(PurgeResultItem { path: p, success: true, error_message: None }),
                        Err(e) => results.push(PurgeResultItem { path: p, success: false, error_message: Some(e.to_string()) }),
                    }
                }
            }
            let _ = tx.send(AppMessage::PurgeFinished(results));
        });
    }
}

fn scan_all_targets(profile_path: &Path) -> Vec<CleanupItem> {
    let mut discovered = Vec::new();

    let cache_dir = profile_path.join(".cache");
    if cache_dir.exists() {
        scan_safe_recursive(&cache_dir, "User Cache", "Unused local runtime cache", 0, &mut discovered);
    }

    let appdata_local = profile_path.join("AppData").join("Local");
    let appdata_roaming = profile_path.join("AppData").join("Roaming");
    scan_orphaned_appdata(&appdata_local, "Local AppLeftover", &mut discovered);
    scan_orphaned_appdata(&appdata_roaming, "Roaming AppLeftover", &mut discovered);

    let local_programs = appdata_local.join("Programs");
    if local_programs.exists() {
        scan_orphaned_appdata(&local_programs, "Programs Leftover", &mut discovered);
    }

    let program_data = std::env::var("ProgramData")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("C:\\ProgramData"));

    if program_data.exists() {
        scan_orphaned_programdata(&program_data, &mut discovered);
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(windir) = std::env::var("WINDIR") {
            let windir_path = PathBuf::from(&windir);

            let sw_download = windir_path.join("SoftwareDistribution").join("Download");
            if sw_download.exists() {
                scan_safe_recursive(&sw_download, "Update Download Cache", "Stale Windows update download payload", 0, &mut discovered);
            }

            let installer_dir = windir_path.join("Installer");
            if installer_dir.exists() {
                scan_installer_backlogs(&installer_dir, &mut discovered);
            }
        }
    }

    discovered
}

fn scan_installer_backlogs(dir: &Path, list: &mut Vec<CleanupItem>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(meta) = fs::metadata(&path) {
                if meta.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if ext.eq_ignore_ascii_case("msp") || ext.eq_ignore_ascii_case("msi") {
                            list.push(CleanupItem {
                                path: path.to_string_lossy().to_string(),
                                size: meta.len(),
                                category: "Version Patch Backlog".to_string(),
                                reason: "Orphaned software version patch file in Windows Installer cache".to_string(),
                                selected: true,
                            });
                        }
                    }
                }
            }
        }
    }
}

fn scan_safe_recursive(dir: &Path, category: &str, reason: &str, depth: usize, list: &mut Vec<CleanupItem>) {
    if depth > 8 { return; }
    if let Ok(meta) = fs::symlink_metadata(dir) {
        if meta.file_type().is_symlink() { return; }
    }

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(meta) = fs::symlink_metadata(&path) {
                if meta.file_type().is_symlink() { continue; }
                if meta.is_file() {
                    list.push(CleanupItem {
                        path: path.to_string_lossy().to_string(),
                        size: meta.len(),
                        category: category.to_string(),
                        reason: reason.to_string(),
                        selected: true,
                    });
                } else if meta.is_dir() {
                    scan_safe_recursive(&path, category, reason, depth + 1, list);
                }
            }
        }
    }
}

fn is_system_or_active_folder(name: &str) -> bool {
    let lower = name.to_lowercase();
    let ignored = [
        "temp", "tmp", "packages", "comms", "connecteddevicesplatform",
        "usoprivate", "microsoft", "windows", "windowsapps", "package cache",
        "assembly", "crest", "history", "etin", "cache", "google", "mozilla",
        "clients", "dxcache", "ShaderCache", "nvidia corporation", "amd",
        "inflight", "winsxs"
    ];
    ignored.contains(&lower.as_str())
}

fn scan_orphaned_appdata(base_dir: &Path, category: &str, list: &mut Vec<CleanupItem>) {
    if let Ok(entries) = fs::read_dir(base_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Ok(meta) = fs::symlink_metadata(&path) {
                    if meta.file_type().is_symlink() { continue; }
                }

                let folder_name = path.file_name().unwrap_or_default().to_string_lossy();
                if is_system_or_active_folder(&folder_name) {
                    continue;
                }

                if folder_name.eq_ignore_ascii_case("Programs") {
                    scan_orphaned_appdata(&path, "Programs Leftover", list);
                    continue;
                }

                // Fully dynamic: build Program Files paths relative to system drive
                let system_drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
                let pf1 = PathBuf::from(format!("{}\\Program Files", system_drive)).join(&*folder_name);
                let pf2 = PathBuf::from(format!("{}\\Program Files (x86)", system_drive)).join(&*folder_name);

                if !pf1.exists() && !pf2.exists() {
                    let mut dir_size = 0;
                    get_dir_size_safe(&path, 0, &mut dir_size);
                    if dir_size > 500 * 1024 {
                        list.push(CleanupItem {
                            path: path.to_string_lossy().to_string(),
                            size: dir_size,
                            category: category.to_string(),
                            reason: format!("Orphaned uninstalled folder: '{}'", folder_name),
                            selected: true,
                        });
                    }
                }
            }
        }
    }
}

fn scan_orphaned_programdata(base_dir: &Path, list: &mut Vec<CleanupItem>) {
    if let Ok(entries) = fs::read_dir(base_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Ok(meta) = fs::symlink_metadata(&path) {
                    if meta.file_type().is_symlink() { continue; }
                }

                let folder_name = path.file_name().unwrap_or_default().to_string_lossy();
                if is_system_or_active_folder(&folder_name) {
                    continue;
                }

                // Fully dynamic: build Program Files paths relative to system drive
                let system_drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
                let pf1 = PathBuf::from(format!("{}\\Program Files", system_drive)).join(&*folder_name);
                let pf2 = PathBuf::from(format!("{}\\Program Files (x86)", system_drive)).join(&*folder_name);

                if !pf1.exists() && !pf2.exists() {
                    let mut dir_size = 0;
                    get_dir_size_safe(&path, 0, &mut dir_size);
                    if dir_size > 2 * 1024 * 1024 {
                        list.push(CleanupItem {
                            path: path.to_string_lossy().to_string(),
                            size: dir_size,
                            category: "ProgramData Residual".to_string(),
                            reason: format!("Orphaned system folder '{}'", folder_name),
                            selected: true,
                        });
                    }
                }
            }
        }
    }
}

fn get_dir_size_safe(dir: &Path, depth: usize, size: &mut u64) {
    if depth > 6 { return; }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(meta) = fs::symlink_metadata(&path) {
                if meta.file_type().is_symlink() { continue; }
                if meta.is_file() {
                    *size += meta.len();
                } else if meta.is_dir() {
                    get_dir_size_safe(&path, depth + 1, size);
                }
            }
        }
    }
}

fn clear_readonly_recursive(dir: &Path) -> std::io::Result<()> {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(meta) = fs::symlink_metadata(&path) {
                if meta.file_type().is_symlink() { continue; }
                if meta.is_file() {
                    let mut perms = meta.permissions();
                    if perms.readonly() {
                        perms.set_readonly(false);
                        let _ = fs::set_permissions(&path, perms);
                    }
                } else if meta.is_dir() {
                    let _ = clear_readonly_recursive(&path);
                }
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), eframe::Error> {
    let icon_image = {
        let width = 32;
        let height = 32;
        let mut pixels = vec![0u8; width * height * 4];
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) * 4;
                let dx = x as i32 - 16;
                let dy = y as i32 - 16;
                if dx * dx + dy * dy <= 140 {
                    pixels[idx] = 70;
                    pixels[idx + 1] = 180;
                    pixels[idx + 2] = 240;
                    pixels[idx + 3] = 255;
                }
            }
        }
        egui::viewport::IconData {
            rgba: pixels,
            width: width as u32,
            height: height as u32,
        }
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1150.0, 740.0])
            .with_title("OrphanCleaner: Process Hunter & Residual Auditor")
            .with_icon(icon_image),
        ..Default::default()
    };

    eframe::run_native(
        "System Cleaner",
        options,
        Box::new(|_| Ok(Box::new(SystemCleanerApp::default()))),
    )
}