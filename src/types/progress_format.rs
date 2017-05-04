#[derive(Debug, Clone, Copy)]
pub enum ProgressFormat {
    Dot,
    Path,
    Raw,
}

impl From<String> for ProgressFormat {
    fn from(val: String) -> ProgressFormat {
        let val = val.to_lowercase();
        if val == String::from("path") {
            ProgressFormat::Path
        } else if val == String::from("raw") {
            ProgressFormat::Raw
        } else if val == String::from("dot") {
            ProgressFormat::Dot
        } else {
            warn!("Invalid format specified - {:?} - using Progress::Dot", val);
            ProgressFormat::Dot
        }
    }
}
