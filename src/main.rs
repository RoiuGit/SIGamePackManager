use eframe::Storage;
use native_dialog::FileDialog;
use std::collections::HashMap;
use std::error::Error;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};

static FILE_EXTENSION: &str = "siq";

type StoreMap = HashMap<String, Vec<String>>;

#[derive(Default)]
struct AppState {
    store_map: StoreMap,
    selected_action: Action,
    selected_type: SelectionType,
    selected_path: Option<PathBuf>,
    selected_index: String,
    output: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum Action {
    Add,
    #[default]
    Check,
    Remove,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum SelectionType {
    #[default]
    File,
    Directory,
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("SIGame Pack Manager");

            // Type selector
            ui.horizontal(|ui| {
                ui.label("Type:");
                ui.selectable_value(&mut self.selected_type, SelectionType::File, "File");
                ui.selectable_value(&mut self.selected_type, SelectionType::Directory, "Directory");
            });

            // Action selector
            ui.horizontal(|ui| {
                ui.label("Action:");
                ui.selectable_value(&mut self.selected_action, Action::Add, "Add");
                ui.selectable_value(&mut self.selected_action, Action::Check, "Check");
                ui.selectable_value(&mut self.selected_action, Action::Remove, "Remove");
                ui.text_edit_singleline(&mut self.selected_index);
            });

            ui.horizontal(|ui| {

                // File browser button
                if ui.button("Browse").clicked() {
                    let path = if self.selected_type == SelectionType::File {
                        FileDialog::new()
                            .add_filter("SIGame files", &[FILE_EXTENSION])
                            .show_open_single_file()
                    } else {
                        FileDialog::new()
                            .show_open_single_dir()
                    };

                    match path {
                        Ok(Some(path)) => {
                            self.selected_path = Some(path);
                        }
                        Ok(None) => self.output = "No file or directory selected".to_string(),
                        Err(e) => self.output = format!("Error: {}", e),
                    }
                }

                // Execute button
                if ui.button("Execute").clicked() {
                    if let Some(path) = &self.selected_path.clone() {
                        match self.execute_action(path) {
                            Ok(msg) => self.output = msg,
                            Err(e) => self.output = format!("Error: {}", e),
                        }
                    } else {
                        if !self.selected_index.is_empty() {
                            self.output = remove_by_index(&self.selected_index, &mut self.store_map).unwrap_or_else(|e| {
                                format!("Error: {}", e)
                            });
                        }
                        else {
                            self.output = "Please select a file or directory first.".to_string();
                        }
                    }
                }

                // See store button
                if ui.button("See store").clicked() {
                    self.see_store_action();
                }
            });

            //Selected
            ui.separator();
            ui.label(&format!("Selected: {}", self.selected_path.as_ref()
                .unwrap_or(&PathBuf::new())
                .to_str().unwrap_or("")));

            // Output window
            ui.separator();
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2]) // ensure it doesn't shrink vertically
                .show(ui, |ui| {
                    ui.label(&self.output);
                });
            //ui.text_edit_multiline(&mut self.output);
        });
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        if let Ok(serialized) = serde_json::to_string(&self.store_map) {
            storage.set_string("store_map", serialized);
        }
    }

}

impl AppState {

    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut store_map = HashMap::new();
        if let Some(storage) = cc.storage {
            if let Some(serialized) = storage.get_string("store_map") {
                if let Ok(map) = serde_json::from_str(&serialized) {
                    store_map = map;
                }
            }
        }

        AppState {
            store_map,
            selected_action: Action::default(),
            selected_type: SelectionType::default(),
            selected_path: None,
            selected_index: "".to_string(),
            output: "Welcome to SIGame Pack Manager!".to_string(),
        }
    }

    fn see_store_action(&mut self) {
        let mut store = String::new();
        for (i, (_hash, values)) in self.store_map.iter().enumerate() {
            store.push_str(&format!("Pack {}:\n", i));
            for value in values {
                store.push_str(&format!("  {}\n", value));
            }
        }
        self.output = store;
    }

    fn execute_action(&mut self, path: &Path) -> Result<String, Box<dyn Error>> {
        // Dummy implementation for now
        let output: String = match self.selected_action {
            Action::Add => {
                match self.selected_type {
                    SelectionType::File => handle_add_file(path, &mut self.store_map)?,
                    SelectionType::Directory => handle_add_dir(path, &mut self.store_map)?,
                }
            },
            Action::Check => {
                match self.selected_type {
                    SelectionType::File => handle_check_file(path, &self.store_map)?,
                    SelectionType::Directory => handle_check_dir(path, &self.store_map)?,
                }
                },
            Action::Remove => {
                let mut index = self.selected_index.clone();
                if index.is_empty() {
                    match self.selected_type {
                        SelectionType::File => handle_remove_file(path, &mut self.store_map)?,
                        SelectionType::Directory => handle_remove_dir(path, &mut self.store_map)?,
                    }
                } else {
                    remove_by_index(&index, &mut self.store_map)?
                }
            },
        };

        Ok(output)
    }
}

fn file_to_map(file: &str) -> Result<StoreMap, Box<dyn Error>> {
    let mut map = HashMap::new();
    let mut file = File::open(file)?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    for line in buf.lines() {
        let mut parts = line.split(',');
        let key = parts.next().ok_or("Invalid line")?;
        let values = map.entry(key.to_string()).or_insert(Vec::new());
        for value in parts {
            values.push(value.to_string());
        }
    }
    Ok(map)
}

fn map_to_file(map: &StoreMap, file: &str) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(file)?;
    for key in map.keys() {
        file.write_all(key.as_bytes())?;
        file.write_all(b",")?;
        let values = map.get(key).unwrap();
        for (i, value) in values.iter().enumerate() {
            file.write_all(value.as_bytes())?;
            if i < values.len() - 1 {
                file.write_all(b",")?;
            }
        }
        file.write_all(b"\n")?;
    }
    Ok(())
}

fn handle_check_dir(dirname: &Path, store_map: &StoreMap) -> Result<String, Box<dyn Error>> {
    let files = std::fs::read_dir(dirname)?;
    let mut output = String::new();
    for file in files {
        let file = file?;
        let filename = file.path();
        if is_siq_file(&filename) {
            output.push_str(&handle_check_file(&filename, store_map)?);
        }
    }
    Ok(output)
}

fn handle_check_file(filename: &Path, store_map: &StoreMap) -> Result<String, Box<dyn Error>> {
    let file = File::open(filename)?;
    let pack_name = filename
        .file_name()
        .ok_or("Invalid file name")?
        .to_str()
        .ok_or("Invalid file name")?;
    let hash = hash_entry(&file)?;
    if store_map.contains_key(&hash) {
        Ok(format!("SIGame pack {} was played before\n", pack_name))
    } else {
        Ok(format!("SIGame pack {} wasn't played before\n", pack_name))
    }
}

fn handle_add_dir(dirname: &Path, store_map: &mut StoreMap) -> Result<String, Box<dyn Error>> {
    let files = std::fs::read_dir(dirname)?;
    let mut output = String::new();
    for file in files {
        let file = file?;
        let filename = file.path();
        if is_siq_file(&filename) {
            output.push_str(&handle_add_file(&filename, store_map)?);
        }
    }
    Ok(output)
}

fn handle_add_file(filename: &Path, store_map: &mut StoreMap) -> Result<String, Box<dyn Error>> {
    let file = File::open(filename)?;
    let pack_name = filename
        .file_name()
        .ok_or("Invalid file name")?
        .to_str()
        .ok_or("Invalid file name")?;
    let hash = hash_entry(&file)?;
    let values = store_map.entry(hash).or_default();
    if !values.contains(&pack_name.to_string()) {
        values.push(pack_name.to_string());
        Ok(format!("SIGame pack {} was added to the store\n", pack_name))
    } else {
        Ok(format!("SIGame pack {} is already present in the store\n", pack_name))
    }
}
fn handle_remove_dir(dirname: &Path, store_map: &mut StoreMap) -> Result<String, Box<dyn Error>> {
    let files = std::fs::read_dir(dirname)?;
    let mut output = String::new();
    for file in files {
        let file = file?;
        let filename = file.path();
        if is_siq_file(&filename) {
            output.push_str(&handle_remove_file(&filename, store_map)?);
        }
    }
    Ok(output)
}

fn handle_remove_file(filename: &Path, store_map: &mut StoreMap) -> Result<String, Box<dyn Error>> {
    let file = File::open(filename)?;
    let hash = hash_entry(&file)?;
    let pack_name = filename
        .file_name()
        .ok_or("Invalid file name")?
        .to_str()
        .ok_or("Invalid file name")?;
    if store_map.contains_key(&hash) {
        store_map.remove(&hash);
        Ok(format!("SIGame pack {} was removed from the store\n", pack_name))
    } else {
        Ok(format!("SIGame pack {} not found in the store\n", pack_name))
    }
}

fn remove_by_index(index: &str, store_map: &mut StoreMap) -> Result<String, Box<dyn Error>> {
    let index = index.trim();
    let index = index.parse::<usize>()?;
    let map_copy = store_map.clone();
    for (i, (hash, _)) in map_copy.iter().enumerate() {
        if i == index {
            store_map.remove(hash);
            return Ok(format!("Pack {} removed\n", index));
        }
    }
    Ok(format!("Pack {} not found\n", index))
}
fn is_siq_file(filename: &PathBuf) -> bool {
    let extension = filename.extension().unwrap_or(OsStr::new(""));
    extension == FILE_EXTENSION
}

fn hash_entry<R>(mut file: R) -> Result<String, Box<dyn Error>>
where
    R: Read + Seek,
{
    // hash the file
    file.seek(std::io::SeekFrom::Start(0))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    let hash = sha256::digest(buf.as_slice());
    Ok(hash)
}


fn main() -> Result<(), Box<dyn Error>> {
    let options = eframe::NativeOptions {
        ..Default::default()
    };
    println!("Starting SIGame Pack Manager");
    eframe::run_native(
        "SIGame Pack Manager",
        options,
        Box::new(|cc| Ok(Box::new(AppState::new(cc)))),
    )?;
    println!("Exiting SIGame Pack Manager");
    Ok(())
}
