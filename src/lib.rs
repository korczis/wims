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

#[derive(Debug, Copy, Clone)]
pub struct OverallInfo {
    pub dirs: u64,
    pub files: u64,
}
