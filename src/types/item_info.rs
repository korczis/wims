use serde::ser::{Serialize, Serializer, SerializeStruct};

use super::event_type::EventType;

pub trait ItemSize {
    fn event_type(&self) -> &EventType;
    fn size(&self) -> u64;
}

#[derive(Debug, Clone, Copy)]
pub struct FsItemInfo {
    pub event_type: EventType,
    pub ino: u64,
    pub mtime: i64,
    pub size: u64,
}

impl ItemSize for FsItemInfo {
    fn event_type(&self) -> &EventType {
        &self.event_type
    }

    fn size(&self) -> u64 {
        self.size
    }
}

impl Serialize for FsItemInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let mut s = serializer.serialize_struct("FsItemInfo", 3)?;
        s.serialize_field("ino", &self.ino)?;
        s.serialize_field("mtime", &self.mtime)?;
        s.serialize_field("size", &self.size)?;
        s.end()
    }
}
