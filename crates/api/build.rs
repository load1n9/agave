use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::io::Write;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated_disk.rs");
    let mut f = fs::File::create(&dest_path).unwrap();

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let disk_dir = Path::new(&manifest_dir).join("../../disk");
    
    println!("cargo:rerun-if-changed={}", disk_dir.display());

    writeln!(f, "pub const DISK_FILES: &[(&str, &[u8])] = &[").unwrap();

    if disk_dir.exists() {
        let mut paths_to_visit = vec![disk_dir.clone()];
        while let Some(path) = paths_to_visit.pop() {
            if let Ok(entries) = fs::read_dir(&path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        paths_to_visit.push(path);
                    } else if path.is_file() {
                        let rel_path = path.strip_prefix(&disk_dir).unwrap();
                        let file_name_str = rel_path.to_str().unwrap().replace("\\", "/");
                        
                        let abs_path = path.canonicalize().unwrap();
                        let abs_path_str = abs_path.to_str().unwrap().replace("\\", "\\\\");
                        
                        writeln!(f, "    (\"/{file_name_str}\", include_bytes!(\"{abs_path_str}\")),").unwrap();
                    }
                }
            }
        }
    }

    writeln!(f, "];").unwrap();
}
