
use clap::ArgMatches;

use super::progress_format::ProgressFormat;

#[derive(Debug, Clone, Copy)]
pub struct OptionsCache {
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct OptionsHuman {
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct OptionsProgress {
    pub enabled: bool,
    pub count: u64,
    pub format: ProgressFormat,
}

#[derive(Debug, Clone, Copy)]
pub struct OptionsStats {
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct OptionsTree {
    pub enabled: bool,
    pub max_depth: u16,
    pub only_dirs: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub cache: OptionsCache,
    pub human: OptionsHuman,
    pub progress: OptionsProgress,
    pub stats: OptionsStats,
    pub tree: OptionsTree,
}

impl<'a> From<&'a ArgMatches<'a>> for Options {
    fn from(matches: &ArgMatches) -> Options {
        debug!("Parsing options");
        Options {
            cache: OptionsCache { enabled: matches.is_present("cache") },
            human: OptionsHuman { enabled: matches.is_present("human") },
            progress: OptionsProgress {
                enabled: matches.is_present("progress"),
                count: matches.value_of("progress-count")
                    .unwrap()
                    .to_string()
                    .parse::<u64>()
                    .unwrap_or(10000),
                format: ProgressFormat::from(matches.value_of("progress-format")
                    .unwrap()
                    .to_string()),
            },
            stats: OptionsStats { enabled: matches.is_present("stats") },
            tree: OptionsTree {
                enabled: matches.is_present("tree"),
                max_depth: matches.value_of("tree-depth")
                    .unwrap()
                    .to_string()
                    .parse::<u16>()
                    .unwrap_or(0),
                only_dirs: matches.is_present("tree-only-dirs"),
            },
        }
    }
}
