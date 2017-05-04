
use clap::ArgMatches;

use super::progress_format::ProgressFormat;

#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub print_progress: bool,
    pub progress_count: u64,
    pub progress_format: ProgressFormat,
}

impl<'a> From<&'a ArgMatches<'a>> for Options {
    fn from(matches: &ArgMatches) -> Options {
        println!("Parsing options");
        Options {
            print_progress: matches.is_present("progress"),
            progress_count: matches.value_of("progress-count")
                .unwrap()
                .to_string()
                .parse::<u64>()
                .unwrap_or(10000),
            progress_format: ProgressFormat::from(matches.value_of("progress-format")
                .unwrap()
                .to_string()),
        }
    }
}
