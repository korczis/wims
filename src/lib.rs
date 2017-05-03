#[macro_use]
extern crate log;
extern crate env_logger;

use std::fs::{self, DirEntry};
use std::io;
use std::path::Path;
use std::sync::mpsc;

pub mod types;

use types::dir_info::FsDirInfo;
use types::event_type::EventType;
use types::item_info::FsItemInfo;
use types::message_type::MessageType;

pub type FsStack = Vec<FsDirInfo>;

pub type RxChannel = mpsc::Receiver<(MessageType, Option<Box<FsItemInfo>>)>;
pub type TxChannel = mpsc::Sender<(MessageType, Option<Box<FsItemInfo>>)>;

pub fn get_file_info(event_type: &EventType, path: &Path, entry: &DirEntry) -> Box<FsItemInfo> {
    let md = Box::new(entry.metadata().unwrap()) as Box<std::os::unix::fs::MetadataExt>;

    Box::new(FsItemInfo {
        event_type: *event_type,
        path: path.to_str().unwrap().to_string(),
        ino: md.ino(),
        mtime: md.mtime(),
        size: md.size(),
    })
}

pub fn process(tx: &TxChannel, dirs: &Vec<&str>) {
    for dir in dirs.iter() {
        let _ = self::visit_dir(tx, Path::new(dir));
    }
}

pub fn visit_dir(tx: &TxChannel, dir: &Path) -> io::Result<()> {
    debug!("Entering directory {:?}", dir);

    let metadata = fs::symlink_metadata(dir)?;
    let file_type = metadata.file_type();

    if dir.is_dir() && !file_type.is_symlink() {
        let dir_meta = Box::new(dir.metadata().unwrap()) as Box<std::os::unix::fs::MetadataExt>;

        let _ = tx.send((MessageType::FsItem,
                         Some(Box::new(FsItemInfo {
            event_type: EventType::DirEnter,
            path: dir.to_str().unwrap().to_string(),
            ino: dir_meta.ino(),
            mtime: dir_meta.mtime(),
            size: dir_meta.size(),
        }))));

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries {
                debug!("Processing entry {:?}", entry);

                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_dir() {
                        debug!("Processing directory: {:?}", &path);
                        let _ = self::visit_dir(tx, &path);

                    } else if path.is_file() {
                        debug!("Processing file: {:?}", &path);

                        let _ =
                            tx.send((MessageType::FsItem,
                                     Some(self::get_file_info(&EventType::File, &path, &entry))));
                    }
                }
            }
        }

        let _ = tx.send((MessageType::FsItem,
                         Some(Box::new(FsItemInfo {
            event_type: EventType::DirLeave,
            path: dir.to_str().unwrap().to_string(),
            ino: dir_meta.ino(),
            mtime: dir_meta.mtime(),
            size: dir_meta.size(),
        }))));
    }

    debug!("Leaving directory {:?}", dir);
    Ok(())
}
