use super::event_type::EventType;

#[derive(Debug)]
pub struct FsItemInfo {
    pub event_type: EventType,
    pub path: String,
    pub ino: u64,
    pub mtime: i64,
    pub size: u64,
}
