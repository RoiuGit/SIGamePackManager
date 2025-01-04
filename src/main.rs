use clap::{Parser, ValueEnum};
use std::collections::HashMap;
use std::error::Error;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::process::exit;

static STORE: &str = "pack_store.pkgst";

static FILE_EXTENSION: &str = "siq";

type StoreMap = HashMap<String, Vec<String>>;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    file: Option<PathBuf>,
    #[arg(short, long)]
    dir: Option<PathBuf>,
    #[arg(value_enum, required = true)]
    action: Option<Action>,
}

#[derive(Debug, ValueEnum, Clone, PartialEq, Eq)]
enum Action {
    Add,
    Check,
    Remove,
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

fn handle_check_dir(dirname: &Path, store_map: &StoreMap) -> Result<(), Box<dyn Error>> {
    let files = std::fs::read_dir(dirname)?;
    for file in files {
        let file = file?;
        let filename = file.path();
        if !is_siq_file(&filename) {
            handle_check_file(&filename, store_map)?;
        }
    }
    Ok(())
}

fn handle_add_dir(dirname: &Path, store_map: &mut StoreMap) -> Result<(), Box<dyn Error>> {
    let files = std::fs::read_dir(dirname)?;
    for file in files {
        let file = file?;
        let filename = file.path();
        if !is_siq_file(&filename) {
            handle_add_file(&filename, store_map)?;
        }
    }
    Ok(())
}

fn handle_check_file(filename: &Path, store_map: &StoreMap) -> Result<(), Box<dyn Error>> {
    let file = File::open(filename)?;
    let pack_name = filename
        .file_name()
        .ok_or("Invalid file name")?
        .to_str()
        .ok_or("Invalid file name")?;
    let hash = hash_entry(&file)?;
    if store_map.contains_key(&hash) {
        println!("SIGame pack {} was played before", pack_name);
    } else {
        println!("SIGame pack {} wasn't played before", pack_name);
    }
    Ok(())
}

fn handle_add_file(filename: &Path, store_map: &mut StoreMap) -> Result<(), Box<dyn Error>> {
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
        println!("SIGame pack {} was added to the store", pack_name);
    } else {
        println!("SIGame pack {} is present in the store", pack_name);
    }
    Ok(())
}
fn handle_remove_file(filename: &Path, store_map: &mut StoreMap) -> Result<(), Box<dyn Error>> {
    let file = File::open(filename)?;
    let hash = hash_entry(&file)?;
    let pack_name = filename
        .file_name()
        .ok_or("Invalid file name")?
        .to_str()
        .ok_or("Invalid file name")?;
    if store_map.contains_key(&hash) {
        store_map.remove(&hash);
        println!("SIGame pack {} was removed from the store", pack_name);
    } else {
        println!("SIGame pack {} not found in the store", pack_name);
    }
    Ok(())
}

fn handle_remove_dir(dirname: &Path, store_map: &mut StoreMap) -> Result<(), Box<dyn Error>> {
    let files = std::fs::read_dir(dirname)?;
    for file in files {
        let file = file?;
        let filename = file.path();
        if is_siq_file(&filename) {
            handle_remove_file(&filename, store_map)?;
        }
    }
    Ok(())
}

fn is_siq_file(filename: &PathBuf) -> bool {
    filename.extension().unwrap_or(OsStr::new("")) == FILE_EXTENSION
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

fn main() {
    let args = Args::parse();

    if args.dir.is_some() && args.file.is_some() {
        eprintln!("Cannot specify both a file and a directory");
        exit(1);
    }

    let action = args.action.unwrap_or_else(|| {
        eprintln!("No action specified");
        exit(1);
    });

    let mut store_map = file_to_map(STORE).unwrap_or_default();

    if args.file.is_some() {
        let file = args.file.unwrap_or_else(|| {
            eprintln!("No file or directory specified");
            exit(1);
        });
        if !is_siq_file(&file) {
            eprintln!("File must be a .siq file");
            exit(1);
        }
        match action {
            Action::Add => handle_add_file(&file, &mut store_map).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                exit(1);
            }),
            Action::Check => handle_check_file(&file, &store_map).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                exit(1);
            }),
            Action::Remove => handle_remove_file(&file, &mut store_map).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                exit(1);
            }),
        }
    } else if args.dir.is_some() {
        let dir = args.dir.unwrap_or_else(|| {
            eprintln!("No file or directory specified");
            exit(1);
        });
        match action {
            Action::Add => handle_add_dir(&dir, &mut store_map).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                exit(1);
            }),
            Action::Check => handle_check_dir(&dir, &store_map).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                exit(1);
            }),
            Action::Remove => handle_remove_dir(&dir, &mut store_map).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                exit(1);
            }),
        }
    }

    map_to_file(&store_map, STORE).unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        exit(1);
    });
}
