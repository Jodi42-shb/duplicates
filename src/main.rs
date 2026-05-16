use eframe::egui;
use std::{collections::HashMap, fs::{self, File}, io::{BufReader, Read}, path::{Path, PathBuf}, sync::{Arc, Mutex}};
use walkdir::WalkDir;

struct DuplicateItem {
    path: PathBuf,
    original: PathBuf,
    status: String,
}

struct GuiApp {
    target_path: PathBuf,
    duplicates: Arc<Mutex<Vec<DuplicateItem>>>,
    is_scanning: Arc<Mutex<bool>>,
    status_msg: String,
}

fn get_file_hash(path: &Path) -> std::io::Result<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut context = md5::Context::new();
    let mut buffer = [0; 8192];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 { break; }
        context.consume(&buffer[..count]);
    }
    Ok(format!("{:x}", context.compute()))
}

fn run_scan_async(target: PathBuf, duplicates_lock: Arc<Mutex<Vec<DuplicateItem>>>, scanning_lock: Arc<Mutex<bool>>, ctx: egui::Context) {
    tokio::spawn(async move {
        let mut hashes: HashMap<String, PathBuf> = HashMap::new();
        let mut found_dups = Vec::new();

        for entry in WalkDir::new(target).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let path = entry.into_path();
                if let Ok(hash) = get_file_hash(&path) {
                    if let Some(original_path) = hashes.get(&hash) {
                        found_dups.push(DuplicateItem {
                            path: path.clone(),
                            original: original_path.clone(),
                            status: "Pending".to_string(),
                        });
                    } else {
                        hashes.insert(hash, path);
                    }
                }
            }
        }
        *duplicates_lock.lock().unwrap() = found_dups;
        *scanning_lock.lock().unwrap() = false;
        ctx.request_repaint();
    });
}

impl GuiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        Self {
            target_path: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            duplicates: Arc::new(Mutex::new(Vec::new())),
            is_scanning: Arc::new(Mutex::new(false)),
            status_msg: "Ready.".to_string(),
        }
    }
}

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let scanning = *self.is_scanning.lock().unwrap();
        let has_duplicates = {
            let dups = self.duplicates.lock().unwrap();
            dups.iter().any(|item| item.status == "Pending")
        };

        egui::TopBottomPanel::top("top_layout").show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("📊 Cryptographic Duplicate Cleanup Engine");
            });
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("Scan Location: ");
                ui.colored_label(egui::Color32::from_rgb(130, 190, 250), self.target_path.to_string_lossy());

                if ui.add_enabled(!scanning, egui::Button::new("📁 Browse...")).clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.target_path = path;
                    }
                }

                if ui.add_enabled(!scanning, egui::Button::new("🔍 Analyze")).clicked() {
                    *self.is_scanning.lock().unwrap() = true;
                    self.status_msg = "Scanning directory tree layout...".to_string();
                    run_scan_async(self.target_path.clone(), self.duplicates.clone(), self.is_scanning.clone(), ctx.clone());
                }

                ui.separator();

                    // --- Bulk Actions Layout ---
                    let trash_all_btn = egui::Button::new("🗑 Trash All Pending"); // Drop the .text_style() modifier
                    if ui.add_enabled(has_duplicates && !scanning, trash_all_btn).clicked() {                // --- Bulk Actions Layout ---
                    let mut dups = self.duplicates.lock().unwrap();
                    let mut count = 0;
                    for item in dups.iter_mut().filter(|i| i.status == "Pending") {
                        if trash::delete(&item.path).is_ok() {
                            item.status = "Trashed".to_string();
                            count += 1;
                        }
                    }
                    self.status_msg = format!("Bulk Operation: Successfully moved {} files to system trash.", count);
                }

                let delete_all_btn = egui::Button::new("💥 Delete All Pending");
                if ui.add_enabled(has_duplicates && !scanning, delete_all_btn).clicked() {
                    let mut dups = self.duplicates.lock().unwrap();
                    let mut count = 0;
                    for item in dups.iter_mut().filter(|i| i.status == "Pending") {
                        if fs::remove_file(&item.path).is_ok() {
                            item.status = "Deleted".to_string();
                            count += 1;
                        }
                    }
                    self.status_msg = format!("Bulk Operation: Permanently deleted {} duplicate instances.", count);
                }
            });
            ui.add_space(8.0);
        });

        egui::TopBottomPanel::bottom("bottom_layout").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if scanning { ui.spinner(); }
                ui.label(&self.status_msg);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Target Mapped Workspace Files:");
            ui.separator();

            let mut dups = self.duplicates.lock().unwrap();
            if dups.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label("No active duplicate instances found or loaded.");
                });
            } else {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for item in dups.iter_mut() {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(format!("Duplicate: {}", item.path.file_name().unwrap().to_string_lossy()));
                                    ui.small(format!("Path: {}", item.path.display()));
                                    ui.small(format!("Matches Original: {}", item.original.display()));
                                });

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if item.status != "Pending" {
                                        let text_color = match item.status.as_str() {
                                            "Deleted" => egui::Color32::from_rgb(240, 100, 100),
                                            "Trashed" => egui::Color32::from_rgb(240, 200, 100),
                                            _ => egui::Color32::from_rgb(120, 240, 120),
                                        };
                                        ui.colored_label(text_color, &item.status);
                                    } else {
                                        if ui.button("🗑 Trash").clicked() {
                                            if trash::delete(&item.path).is_ok() { item.status = "Trashed".to_string(); }
                                        }
                                        if ui.button("❌ Delete").clicked() {
                                            if fs::remove_file(&item.path).is_ok() { item.status = "Deleted".to_string(); }
                                        }
                                        if ui.button("📁 Move").clicked() {
                                            let mut dest = item.path.parent().unwrap().to_path_buf();
                                            dest.push("duplicates");
                                            let _ = fs::create_dir_all(&dest);
                                            dest.push(item.path.file_name().unwrap());
                                            if fs::rename(&item.path, dest).is_ok() { item.status = "Moved".to_string(); }
                                        }
                                    }
                                });
                            });
                        });
                        ui.add_space(4.0);
                    }
                });
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let _guard = rt.enter();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Duplicate Analyzer Engine",
        native_options,
        Box::new(|cc| Box::new(GuiApp::new(cc)) as Box<dyn eframe::App>),
    )
}
