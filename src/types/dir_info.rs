use super::item_info::FsItemInfo;

#[derive(Debug)]
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
