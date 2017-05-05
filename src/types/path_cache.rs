use serde::ser::{Serialize, Serializer, SerializeStruct};

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::Debug;

use super::event_type::EventType;
use super::formatter::human_format;
use super::item_info::FsItemInfo;
use super::item_info::ItemSize;

#[derive(Debug, Clone)]
pub struct PathCache<T>
    where T: Clone + Copy + Debug + ItemSize + Serialize
{
    pub path: String,
    pub data: Option<T>,
    pub dirs_size: u64,
    pub files_size: u64,
    pub total_size: u64,
    pub childs: Option<BTreeMap<String, PathCache<T>>>,
}

impl<T> PathCache<T>
    where T: Clone + Copy + Debug + ItemSize + Serialize
{
    pub fn dirs_size(&self) -> u64 {
        self.dirs_size
    }

    pub fn files_size(&self) -> u64 {
        self.files_size
    }

    pub fn total_size(&self) -> u64 {
        self.total_size
    }

    pub fn calculate_size(&mut self) {
        self.dirs_size = 0;
        self.files_size = 0;
        self.total_size = 0;

        if self.childs.is_some() {
            for (_k, v) in self.childs.as_mut().unwrap().iter_mut() {
                v.calculate_size();

                if let Some(data) = v.data {
                    match data.event_type() {
                        &EventType::File => self.files_size += data.size(),
                        &EventType::DirEnter => self.dirs_size += v.files_size,
                        _ => {}
                    }
                }
            }
        }

        self.total_size = self.dirs_size + self.files_size;
    }
}

pub type PathCacheInfo = PathCache<FsItemInfo>;

impl<T> Eq for PathCache<T> where T: Clone + Copy + Debug + ItemSize + Serialize {}

impl<T> Ord for PathCache<T>
    where T: Clone + Copy + Debug + ItemSize + Serialize
{
    fn cmp(&self, other: &PathCache<T>) -> Ordering {
        self.path.cmp(&other.path)
    }
}

impl<T> PartialEq for PathCache<T>
    where T: Clone + Copy + Debug + ItemSize + Serialize
{
    fn eq(&self, other: &PathCache<T>) -> bool {
        self.path == other.path
    }
}

impl<T> PartialOrd for PathCache<T>
    where T: Clone + Copy + Debug + ItemSize + Serialize
{
    fn partial_cmp(&self, other: &PathCache<T>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn construct<T>(pc: &mut BTreeMap<String, PathCache<T>>, parts: &mut Vec<String>, data: &T)
    where T: Clone + Copy + Debug + ItemSize + Serialize
{
    if let Some(part) = parts.pop() {
        let node_data = if parts.len() == 0 {
            Some(data.clone())
        } else {
            None
        };

        let mut tmp: BTreeMap<String, PathCache<T>> = BTreeMap::new();
        if parts.len() > 0 {
            construct(&mut tmp, parts, data);
        };

        let key = part.clone();
        let has_key = pc.contains_key(&key);

        if has_key {
            let item = pc.get_mut(&key).unwrap();
            if item.childs == None {
                item.childs = Some(tmp);
            } else {
                merge(item.childs.as_mut().unwrap(), &mut tmp);
            }
        } else {
            pc.insert(key,
                      PathCache {
                          path: part.clone(),
                          childs: if parts.len() > 0 {
                              Some(tmp)
                          } else {
                              Some(tmp)
                          },
                          data: node_data,
                          dirs_size: 0,
                          files_size: 0,
                          total_size: 0,
                      });
        }
    }
}

impl<T> Serialize for PathCache<T>
    where T: Clone + Copy + Debug + ItemSize + Serialize
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let mut s = serializer.serialize_struct("PathCache", 5)?;
        s.serialize_field("path", &self.path)?;
        s.serialize_field("data", &self.data)?;
        s.serialize_field("dirs_size", &self.dirs_size)?;
        s.serialize_field("files_size", &self.files_size)?;
        s.serialize_field("total_size", &self.total_size)?;
        s.serialize_field("childs", &self.childs)?;
        s.end()
    }
}

pub fn merge<T>(left: &mut BTreeMap<String, PathCache<T>>,
                right: &mut BTreeMap<String, PathCache<T>>)
    where T: Clone + Copy + Debug + ItemSize + Serialize
{
    for (k, v) in right.iter_mut() {
        if !left.contains_key(k) {
            left.insert(k.clone(), v.clone());
        } else {
            let left_has_childs = left.get(k).as_ref().unwrap().childs.as_ref() != None;
            let right_has_childs = v.childs != None;

            if right_has_childs {
                if !left_has_childs {
                    left.get_mut(k).unwrap().childs = v.childs.clone();
                } else {
                    merge(left.get_mut(k).unwrap().childs.as_mut().unwrap(),
                          v.childs.as_mut().unwrap());
                }
            }
        }
    }
}

fn human_format_if_needed(size: u64, human_readable: bool) -> String {
    match human_readable {
        true => {
            let (val, unit) = human_format(size as f32);
            if val == val.floor() {
                format!("{}{}B", val as u64, unit)
            } else {
                format!("{:.2}{}B", val, unit)
            }

        }
        false => format!("{}", size),
    }
}

pub fn print<T>(pc: &BTreeMap<String, PathCache<T>>, depth: usize, human_readable: bool)
    where T: Clone + Copy + Debug + ItemSize + Serialize
{
    for (_k, ref v) in pc {
        // print!("{:?}", v);

        print!("{}", String::from("  ").repeat(depth));
        if let Some(data) = v.data {
            let size = match data.event_type() {
                &EventType::DirEnter => {
                    format!("{} / {} / {}",
                            human_format_if_needed(v.files_size(), human_readable),
                            human_format_if_needed(v.dirs_size(), human_readable),
                            human_format_if_needed(v.total_size(), human_readable))
                }
                &EventType::File => human_format_if_needed(data.size(), human_readable),
                _ => format!(""),
            };

            println!("{} ({})", v.path, size);
        } else {
            println!("{}", v.path);
        }

        if v.childs != None {
            print(v.childs.as_ref().unwrap(), depth + 1, human_readable);
        }
    }
}
