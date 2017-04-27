extern crate log;
extern crate env_logger;

extern crate clap;
extern crate wims;
extern crate time;

use clap::{App, Arg};
use std::io;
use std::io::Write;
use std::env;
use std::sync::mpsc;
use std::thread;
use time::PreciseTime;
use wims::*;

const AUTHOR: &'static str = env!("CARGO_PKG_AUTHORS");
const DESCRIPTION: &'static str = env!("CARGO_PKG_DESCRIPTION");
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    let matches = App::new(DESCRIPTION)
        .version(VERSION)
        .author(AUTHOR)
        .about("Disk Usage Information")
        .arg(Arg::with_name("verbose")
            .help("Verbose mode")
            .short("v")
            .long("verbose")
            .multiple(true))
        .arg(Arg::with_name("progress")
            .help("Show progress")
            .short("p")
            .long("progress"))
        .arg(Arg::with_name("progress-count")
            .help("Progress count")
            .short("c")
            .long("progress-count")
            .default_value("10000"))
        .arg(Arg::with_name("progress-format")
            .help("Progress format")
            .short("f")
            .long("progress-format")
            .possible_values(&["dot", "path"])
            .default_value("path"))
        .arg(Arg::with_name("DIR")
            .help("Directories to process")
            .index(1)
            .required(true)
            .multiple(true))
        .get_matches();

    match matches.occurrences_of("verbose") {
        0 => {}
        1 => env::set_var("RUST_LOG", "warn"),
        2 => env::set_var("RUST_LOG", "info"),
        _ => env::set_var("RUST_LOG", "debug"),
    }

    env_logger::init().unwrap();

    let progress = matches.is_present("progress");
    let progress_count =
        matches.value_of("progress-count").unwrap().to_string().parse::<u64>().unwrap_or(10000);
    let progress_format =
        ProgressFormat::from(matches.value_of("progress-format").unwrap().to_string());

    fn print_stats(info: &OverallInfo,
                   elapsed_secs: f64,
                   progress: bool,
                   progress_format: ProgressFormat) {

        let dirs_count = info.dirs;
        let files_count = info.files;

        let fpd = if dirs_count > 0 {
            files_count / dirs_count
        } else {
            0
        };

        let fps = if elapsed_secs > 0.0 {
            (files_count as f64 / elapsed_secs) as u64
        } else {
            0
        };

        if progress {
            match progress_format {
                ProgressFormat::Dot => println!(""),
                _ => {}
            };
        }

        println!("Dirs: {}, Files: {}, Files Per Dir: {}, Time: {}, Speed: {} fps",
                 dirs_count,
                 files_count,
                 fpd,
                 elapsed_secs,
                 fps);
    }

    let mut stdout = io::stdout();
    let (tx, rx) = mpsc::channel();
    let start = PreciseTime::now();
    let handle = thread::spawn(move || {
        let mut info = OverallInfo {
            files: 0,
            dirs: 0,
        };

        loop {
            match rx.recv() {
                Ok(received) => {
                    let data: (MessageType, Option<OverallInfo>, Option<String>) = received;

                    match data.0 {
                        MessageType::Entry => {
                            info = data.1.unwrap();
                            let path = data.2.unwrap();

                            if progress && (info.files % progress_count) == 0 {
                                match progress_format {
                                    ProgressFormat::Path => println!("{} {}", info.files, path),
                                    ProgressFormat::Dot => print!("."),
                                }
                                let _ = stdout.flush();
                            }
                        }
                        MessageType::Exit => {
                            let diff = start.to(PreciseTime::now());
                            let elapsed_secs = diff.num_seconds() as f64 +
                                               diff.num_milliseconds() as f64 * 0.001 +
                                               diff.num_microseconds().unwrap() as f64 * 1e-6;

                            print_stats(&info, elapsed_secs, progress, progress_format);
                            break;
                        }
                    };
                }
                _ => {}
            }
        }
    });

    let dirs: Vec<_> = matches.values_of("DIR").unwrap().collect();
    wims::process(tx.clone(), &dirs);

    let _ = tx.send((MessageType::Exit, None, None));
    let _ = handle.join();
}
