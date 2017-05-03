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
