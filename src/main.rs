#[macro_use]
extern crate log;
extern crate env_logger;

extern crate bincode;
extern crate clap;
extern crate wims;
extern crate time;

// use bincode::{serialize, deserialize, Bounded};
use clap::{App, Arg};
use std::collections::BTreeMap;
use std::io;
use std::io::Write;
use std::env;
use std::sync::mpsc;
use std::thread;
use time::PreciseTime;
use wims::*;

use self::types::*;

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
            .possible_values(&["dot", "path", "raw"])
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
    wims::process(&tx, &dirs);

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
        let mut pc: BTreeMap<String, PathCacheInfo> = BTreeMap::new();

        loop {
            match rx.recv() {
                Ok(received) => {
                    let data: (MessageType, Option<Box<FsItemInfo>>) = received;
                    match data.0 {
                        MessageType::FsItem => {
                            let info = data.1.unwrap();
                            // items.push(info.clone());
                            handle_fs_item(&mut stack,
                                           &mut pc,
                                           &mut overall,
                                           &mut dir_files,
                                           info,
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

        // println!("{:?}", pc);
    })
}

fn handle_dir_enter(stack: &mut FsStack,
                    pc: &mut BTreeMap<String, PathCacheInfo>,
                    overall: &mut OverallInfo,
                    dir_files: &mut Vec<Box<Vec<Box<FsItemInfo>>>>,
                    info: &Box<FsItemInfo>,
                    progress: &bool,
                    progress_count: &u64,
                    progress_format: &ProgressFormat)
                    -> bool {
    overall.dirs += 1;

    let mut parts = info.path
        .split("/")
        .map(|i| i.to_string())
        .collect::<Vec<String>>();
    parts.reverse();

    path_cache::construct(pc, &mut parts, &0);
    // path_cache::print(&pc, 0);

    let res = print_progress_if_needed(overall, info, progress, progress_count, progress_format);

    dir_files.push(Box::new(Vec::new()));

    debug!("{:?}", info);

    stack.push(FsDirInfo {
        path: info.path.clone(),
        files: Vec::new(),
        files_size: 0,
    });

    res
}

fn handle_dir_leave(stack: &mut FsStack,
                    dir_files: &mut Vec<Box<Vec<Box<FsItemInfo>>>>,
                    info: &Box<FsItemInfo>) {

    let _files = dir_files.pop().unwrap();
    // stack.last_mut().unwrap().files = files;
    stack.last_mut().unwrap().calculate_files_size();

    debug!("Stack when leaving {}: {:?}", info.path, stack);
    stack.pop();
}

fn handle_exit(overall: &OverallInfo,
               start: &PreciseTime,
               progress: &bool,
               progress_format: &ProgressFormat) {
    let diff = start.to(PreciseTime::now());
    let elapsed_secs = diff.num_nanoseconds().unwrap() as f64 * 1e-9;

    print_stats(&overall, elapsed_secs, progress, progress_format);
}

fn handle_file(pc: &mut BTreeMap<String, PathCacheInfo>,
               overall: &mut OverallInfo,
               dir_files: &mut Vec<Box<FsItemInfo>>,
               info: Box<FsItemInfo>,
               progress: &bool,
               progress_count: &u64,
               progress_format: &ProgressFormat)
               -> bool {
    overall.files += 1;

    let mut parts = info.path
        .split("/")
        .map(|i| i.to_string())
        .collect::<Vec<String>>();
    parts.reverse();

    path_cache::construct(pc, &mut parts, &0);
    // path_cache::print(&pc, 0);

    let res = print_progress_if_needed(overall, &info, progress, progress_count, progress_format);

    debug!("{:?}", info);
    dir_files.push(info);

    res
}

fn handle_fs_item(stack: &mut FsStack,
                  pc: &mut BTreeMap<String, PathCacheInfo>,
                  overall: &mut OverallInfo,
                  dir_files: &mut Vec<Box<Vec<Box<FsItemInfo>>>>,
                  info: Box<FsItemInfo>,
                  progress: &bool,
                  progress_count: &u64,
                  progress_format: &ProgressFormat,
                  stdout: &mut io::Stdout) {
    match info.event_type {
        EventType::DirEnter => {
            if handle_dir_enter(stack,
                                pc,
                                overall,
                                dir_files,
                                &info,
                                &progress,
                                &progress_count,
                                &progress_format) {
                let _ = stdout.flush();
            }
        }
        EventType::DirLeave => {
            handle_dir_leave(stack, dir_files, &info);
        }
        EventType::File => {
            if handle_file(pc,
                           overall,
                           dir_files.last_mut().unwrap(),
                           info,
                           &progress,
                           &progress_count,
                           &progress_format) {
                let _ = stdout.flush();
            }
        }
    };
}

fn print_progress(overall: &OverallInfo,
                  info: &Box<FsItemInfo>,
                  progress_format: &ProgressFormat) {
    match *progress_format {
        ProgressFormat::Dot => print!("."),
        ProgressFormat::Path => {
            println!("{} {}", overall.all(), info.path);
        }
        ProgressFormat::Raw => println!("{} {:?}", overall.all(), info),
    }
}

fn print_progress_if_needed(overall: &OverallInfo,
                            info: &Box<FsItemInfo>,
                            progress: &bool,
                            progress_count: &u64,
                            progress_format: &ProgressFormat)
                            -> bool {
    if *progress && (overall.all() % progress_count) == 0 {
        print_progress(&overall, &info, &progress_format);
        true

    } else {
        false
    }
}

fn print_stats(info: &OverallInfo,
               elapsed_secs: f64,
               progress: &bool,
               progress_format: &ProgressFormat) {

    let dirs_count = info.dirs;
    let files_count = info.files;
    let items_count = info.all();

    let fpd = if dirs_count > 0 {
        files_count as f64 / dirs_count as f64
    } else {
        0.0
    };

    let ips = if elapsed_secs > 0.0 {
        items_count as f64 / elapsed_secs
    } else {
        0.0
    };

    if *progress {
        match *progress_format {
            ProgressFormat::Dot => println!(""),
            _ => {}
        };
    }

    println!("Dirs: {}, Files: {}, Files Per Dir: {:.2}, Time: {:.2}, Speed: {:.2} ips",
             dirs_count,
             files_count,
             fpd,
             elapsed_secs,
             ips);
}
