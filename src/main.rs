#[macro_use]
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

    let (tx, rx) = mpsc::channel();
    let handle = create_thread(rx,
                               matches.is_present("progress"),
                               matches.value_of("progress-count")
                                   .unwrap()
                                   .to_string()
                                   .parse::<u64>()
                                   .unwrap_or(10000),
                               ProgressFormat::from(matches.value_of("progress-format")
                                   .unwrap()
                                   .to_string()));

    let dirs: Vec<_> = matches.values_of("DIR").unwrap().collect();
    wims::process(tx.clone(), &dirs);

    let _ = tx.send((MessageType::Exit, None));
    let _ = handle.join();
}

fn create_thread(rx: RxChannel,
                 progress: bool,
                 progress_count: u64,
                 progress_format: ProgressFormat)
                 -> thread::JoinHandle<()> {

    let mut stdout = io::stdout();
    let start = PreciseTime::now();

    thread::spawn(move || {
        let mut overall = OverallInfo {
            files: 0,
            dirs: 0,
        };

        let mut stack = FsStack::new();
        let mut dir_files = Vec::new();

        loop {
            match rx.recv() {
                Ok(received) => {
                    let data: (MessageType, Option<FsItemInfo>) = received;
                    let info = data.1.unwrap();
                    match data.0 {
                        MessageType::FsItem => {
                            handle_fs_item(&mut stack,
                                           &mut overall,
                                           &mut dir_files,
                                           &info,
                                           &progress,
                                           &progress_count,
                                           &progress_format,
                                           &mut stdout);
                        }
                        MessageType::Exit => {
                            handle_exit(&overall, &start, &progress, &progress_format);
                            break;
                        }
                    };
                }
                _ => {}
            }
        }
    })
}

fn handle_dir_enter(stack: &mut FsStack,
                    overall: &mut OverallInfo,
                    dir_files: &mut Vec<Vec<FsItemInfo>>,
                    info: &FsItemInfo) {
    dir_files.push(Vec::new());

    debug!("{:?}", info);

    stack.push(FsDirInfo {
        path: info.path.clone(),
        files: Vec::new(),
        files_size: 0,
    });
    overall.dirs += 1;
}

fn handle_dir_leave(stack: &mut FsStack, dir_files: &mut Vec<Vec<FsItemInfo>>, info: &FsItemInfo) {
    let files = dir_files.pop().unwrap();
    stack.last_mut().unwrap().files = files;
    stack.last_mut().unwrap().calculate_files_size();

    debug!("Stack when leaving {}: {:?}", info.path, stack);
    stack.pop();
}

fn handle_exit(overall: &OverallInfo,
               start: &PreciseTime,
               progress: &bool,
               progress_format: &ProgressFormat) {
    let diff = start.to(PreciseTime::now());
    let elapsed_secs = diff.num_seconds() as f64 + diff.num_milliseconds() as f64 * 0.001 +
                       diff.num_microseconds().unwrap() as f64 * 1e-6;

    print_stats(&overall, elapsed_secs, progress, progress_format);
}

fn handle_file(overall: &mut OverallInfo,
               dir_files: &mut Vec<Vec<FsItemInfo>>,
               info: &FsItemInfo,
               progress: &bool,
               progress_count: &u64,
               progress_format: &ProgressFormat)
               -> bool {
    overall.files += 1;
    let res = if *progress && (overall.files % progress_count) == 0 {
        match *progress_format {
            ProgressFormat::Path => {
                println!("{} {}", overall.files, info.path);
            }
            ProgressFormat::Dot => print!("."),
        }
        true

    } else {
        false
    };

    debug!("{:?}", info);
    dir_files.last_mut().unwrap().push(info.clone());

    res
}

fn handle_fs_item(stack: &mut FsStack,
                  overall: &mut OverallInfo,
                  dir_files: &mut Vec<Vec<FsItemInfo>>,
                  info: &FsItemInfo,
                  progress: &bool,
                  progress_count: &u64,
                  progress_format: &ProgressFormat,
                  stdout: &mut io::Stdout) {
    match info.event_type {
        EventType::DirEnter => {
            handle_dir_enter(stack, overall, dir_files, &info);
        }
        EventType::DirLeave => {
            handle_dir_leave(stack, dir_files, &info);
        }
        EventType::File => {
            if handle_file(overall,
                           dir_files,
                           &info,
                           &progress,
                           &progress_count,
                           &progress_format) {
                let _ = stdout.flush();
            }
        }
    };
}

fn print_stats(info: &OverallInfo,
               elapsed_secs: f64,
               progress: &bool,
               progress_format: &ProgressFormat) {

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

    if *progress {
        match *progress_format {
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
