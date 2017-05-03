#[derive(Copy, Clone, Debug)]
pub enum EventType {
    DirEnter,
    DirLeave,
    File,
}
