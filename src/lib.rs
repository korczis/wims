#[macro_use]
extern crate log;
extern crate env_logger;

use std::fs::{self, DirEntry};
use std::io;
use std::path::Path;
use std::sync::mpsc;

pub enum MessageType {
    Entry,
    Exit,
}

#[derive(Copy, Clone)]
pub enum ProgressFormat {
    Dot,
    Path,
}

impl From<String> for ProgressFormat {
    fn from(val: String) -> ProgressFormat {
        let val = val.to_lowercase();
        if val == String::from("path") {
            ProgressFormat::Path
        } else {
            ProgressFormat::Dot
        }
    }
}

#[derive(Debug, Clone)]
pub struct FsFileInfo {
    pub path: String,
    pub ino: u64,
    pub mtime: i64,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct FsDirInfo {
    pub path: String,
    // pub dirs: Vec<FsDirInfo>,
    pub files: Vec<FsFileInfo>,
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

pub type RxChannel = mpsc::Receiver<(MessageType, Option<OverallInfo>, Option<String>)>;
pub type TxChannel = mpsc::Sender<(MessageType, Option<OverallInfo>, Option<String>)>;

pub fn get_file_info(path: &Path, entry: &DirEntry) -> FsFileInfo {
    let md = Box::new(entry.metadata().unwrap()) as Box<std::os::unix::fs::MetadataExt>;

    let res = FsFileInfo {
        path: path.to_str().unwrap().to_string(),
        ino: md.ino(),
        mtime: md.mtime(),
        size: md.size(),
    };

    debug!("{:?}", res);

    return res;
}

pub fn process(tx: TxChannel, dirs: &Vec<&str>) {
    let mut info = OverallInfo {
        dirs: 0,
        files: 0,
    };

    let mut dir_stack: FsStack = FsStack::new();
    let mut visitor = |path: &Path, _entry: &DirEntry| {
        if path.is_dir() {
            info.dirs += 1;
        } else if path.is_file() {
            info.files += 1;

            let _ = tx.send((MessageType::Entry,
                             Some(info.clone()),
                             Some(path.to_str().unwrap().to_string())));
        }
    };

    for dir in dirs.iter() {
        let _ = self::visit_dir(&mut dir_stack, Path::new(dir), &mut visitor);
    }
}

pub fn visit_dir(stack: &mut FsStack,
                 dir: &Path,
                 cb: &mut FnMut(&Path, &DirEntry))
                 -> io::Result<()> {

    debug!("Entering directory {:?}", dir);

    let mut stat = OverallInfo {
        dirs: 0,
        files: 0,
    };

    let metadata = fs::symlink_metadata(dir)?;
    let file_type = metadata.file_type();

    if dir.is_dir() && !file_type.is_symlink() {
        // let _dir_meta = dir.metadata();
        let dir_info = FsDirInfo {
            path: dir.to_str().unwrap().to_string(),
            // dirs: Vec::new(),
            files: Vec::new(),
            files_size: 0,
        };

        stack.push(dir_info);

        let mut dir_files = Vec::new();

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries {
                debug!("Processing entry {:?}", entry);

                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_dir() {
                        debug!("Found directory: {:?}", &path);

                        stat.dirs += 1;
                        let _ = self::visit_dir(stack, &path, cb);
                        cb(&path, &entry);
                    } else if path.is_file() {
                        debug!("Found file: {:?}", &path);

                        stat.files += 1;

                        dir_files.push(self::get_file_info(&path, &entry));
                        cb(&path, &entry);
                    }
                }
            }
        }

        stack.last_mut().unwrap().files = dir_files;
        stack.last_mut().unwrap().calculate_files_size();

        debug!("{:?}", stack);
        stack.pop();
    } else {
        debug!("{:?}", stack);
    }

    debug!("Leaving directory {:?}", dir);
    Ok(())
}
