#[macro_use]
extern crate log;
extern crate env_logger;

use std::fs::{self, DirEntry};
use std::io;
use std::path::Path;
use std::sync::mpsc;

#[derive(Copy, Clone, Debug)]
pub enum EventType {
    DirEnter,
    DirLeave,
    File,
}

pub enum MessageType {
    FsItem,
    Exit,
}

#[derive(Copy, Clone)]
pub enum ProgressFormat {
    Dot,
    Path,
    Raw,
}

impl From<String> for ProgressFormat {
    fn from(val: String) -> ProgressFormat {
        let val = val.to_lowercase();
        if val == String::from("path") {
            ProgressFormat::Path
        } else if val == String::from("raw") {
            ProgressFormat::Raw
        } else if val == String::from("dot") {
            ProgressFormat::Dot
        } else {
            warn!("Invalid format specified - {:?} - using Progress::Dot", val);
            ProgressFormat::Dot
        }
    }
}

#[derive(Debug, Clone)]
pub struct FsItemInfo {
    pub event_type: EventType,
    pub path: String,
    pub ino: u64,
    pub mtime: i64,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct FsDirInfo {
    pub path: String,
    // pub dirs: Vec<FsDirInfo>,
    pub files: Vec<FsItemInfo>,
    pub files_size: u64,
}

impl FsDirInfo {
    pub fn calculate_files_size(&mut self) {
        for file in self.files.iter() {
            self.files_size += file.size;
        }
    }
}

pub type FsStack = Vec<FsDirInfo>;

#[derive(Debug, Copy, Clone)]
pub struct OverallInfo {
    pub dirs: u64,
    pub files: u64,
}

impl OverallInfo {
    pub fn all(&self) -> u64 {
        self.dirs + self.files
    }
}

pub type RxChannel = mpsc::Receiver<(MessageType, Option<FsItemInfo>)>;
pub type TxChannel = mpsc::Sender<(MessageType, Option<FsItemInfo>)>;

pub fn get_file_info(event_type: &EventType, path: &Path, entry: &DirEntry) -> FsItemInfo {
    let md = Box::new(entry.metadata().unwrap()) as Box<std::os::unix::fs::MetadataExt>;

    let res = FsItemInfo {
        event_type: *event_type,
        path: path.to_str().unwrap().to_string(),
        ino: md.ino(),
        mtime: md.mtime(),
        size: md.size(),
    };

    return res;
}

pub fn process(tx: TxChannel, dirs: &Vec<&str>) {
    let mut visitor = |item: &FsItemInfo| {
        let _ = tx.send((MessageType::FsItem, Some(item.clone())));
    };

    for dir in dirs.iter() {
        let _ = self::visit_dir(Path::new(dir), &mut visitor);
    }
}

pub fn visit_dir(dir: &Path, cb: &mut FnMut(&FsItemInfo)) -> io::Result<()> {
    debug!("Entering directory {:?}", dir);

    let metadata = fs::symlink_metadata(dir)?;
    let file_type = metadata.file_type();

    if dir.is_dir() && !file_type.is_symlink() {
        let dir_meta = Box::new(dir.metadata().unwrap()) as Box<std::os::unix::fs::MetadataExt>;

        cb(&FsItemInfo {
            event_type: EventType::DirEnter,
            path: dir.to_str().unwrap().to_string(),
            ino: dir_meta.ino(),
            mtime: dir_meta.mtime(),
            size: dir_meta.size(),
        });

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries {
                debug!("Processing entry {:?}", entry);

                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_dir() {
                        debug!("Processing directory: {:?}", &path);
                        let _ = self::visit_dir(&path, cb);

                    } else if path.is_file() {
                        debug!("Processing file: {:?}", &path);

                        let entry_info = self::get_file_info(&EventType::File, &path, &entry);
                        cb(&entry_info);
                    }
                }
            }
        }

        cb(&FsItemInfo {
            event_type: EventType::DirLeave,
            path: dir.to_str().unwrap().to_string(),
            ino: dir_meta.ino(),
            mtime: dir_meta.mtime(),
            size: dir_meta.size(),
        });
    }

    debug!("Leaving directory {:?}", dir);
    Ok(())
}
