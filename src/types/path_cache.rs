use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct PathCache<T>
    where T: Clone + Copy + Debug
{
    pub path: String,
    pub childs: Option<BTreeMap<String, PathCache<T>>>,
    pub data: Option<T>,
}

pub type PathCacheInfo = PathCache<usize>;

impl<T> Eq for PathCache<T> where T: Clone + Copy + Debug {}

impl<T> Ord for PathCache<T>
    where T: Clone + Copy + Debug
{
    fn cmp(&self, other: &PathCache<T>) -> Ordering {
        self.path.cmp(&other.path)
    }
}

impl<T> PartialEq for PathCache<T>
    where T: Clone + Copy + Debug
{
    fn eq(&self, other: &PathCache<T>) -> bool {
        self.path == other.path
    }
}

impl<T> PartialOrd for PathCache<T>
    where T: Clone + Copy + Debug
{
    fn partial_cmp(&self, other: &PathCache<T>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn construct<T>(pc: &mut BTreeMap<String, PathCache<T>>, parts: &mut Vec<String>, data: &T)
    where T: Clone + Copy + Debug
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
                      });
        }
    }
}

pub fn merge<T>(left: &mut BTreeMap<String, PathCache<T>>,
                right: &mut BTreeMap<String, PathCache<T>>)
    where T: Clone + Copy + Debug
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

pub fn print<T>(pc: &BTreeMap<String, PathCache<T>>, depth: usize)
    where T: Clone + Copy + Debug
{
    for (_k, ref v) in pc {
        print!("{}", String::from("  ").repeat(depth));
        println!("{}", v.path);
        if v.childs != None {
            print(v.childs.as_ref().unwrap(), depth + 1);
        }
    }
}
