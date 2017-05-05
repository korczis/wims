#[macro_use]
extern crate log;
extern crate env_logger;

extern crate bincode;
extern crate clap;
extern crate serde;
extern crate wims;
extern crate time;

use bincode::{serialize, Infinite};
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
        .arg(Arg::with_name("cache")
            .help("Cache items to disk")
            .long("cache"))
        .arg(Arg::with_name("human")
            .help("Human readable sizes")
            .short("h")
            .long("human"))
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
        .arg(Arg::with_name("stats")
            .help("Print overall stats at exit")
            .short("s")
            .long("stats"))
        .arg(Arg::with_name("tree")
            .help("Show FS tree")
            .short("t")
            .long("tree"))
        .arg(Arg::with_name("DIR")
            .help("Directories to process")
            .index(1)
            .required(false)
            .multiple(true))
        .get_matches();

    let opts = Options::from(&matches);

    match matches.occurrences_of("verbose") {
        0 => {}
        1 => env::set_var("RUST_LOG", "warn"),
        2 => env::set_var("RUST_LOG", "info"),
        _ => env::set_var("RUST_LOG", "debug"),
    }

    env_logger::init().unwrap();

    let (tx, rx) = mpsc::channel();
    let handle = create_thread(rx, &opts);

    let dirs: Vec<_> = match matches.values_of("DIR") {
        Some(dirs) => dirs.map(|d| d.trim_right_matches('/').to_string()).collect(),
        _ => vec![String::from(".")],
    };

    wims::process(&tx, &dirs);

    let _ = tx.send((MessageType::Exit, None, None));
    let _ = handle.join();
}

fn create_thread(rx: RxChannel, opts: &Options) -> thread::JoinHandle<()> {
    let mut stdout = io::stdout();
    let start = PreciseTime::now();

    let opts = opts.clone();

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
                    let data: (MessageType, Option<String>, Option<Box<FsItemInfo>>) = received;
                    match data.0 {
                        MessageType::FsItem => {
                            let info = data.2.unwrap();
                            // items.push(info.clone());
                            handle_fs_item(&mut stack,
                                           &mut pc,
                                           &mut overall,
                                           &mut dir_files,
                                           data.1.unwrap(),
                                           info,
                                           &opts,
                                           &mut stdout);
                        }
                        MessageType::Exit => {
                            for (k, v) in pc.iter_mut() {
                                debug!("Calculating {:?}", k);
                                v.calculate_size();
                                debug!("Calculated total_size of topmost directory is {:?}",
                                       v.total_size());
                            }

                            if opts.cache.enabled {
                                let encoded: Vec<u8> = serialize(&pc, Infinite).unwrap();
                                println!("{:?}", encoded);

                                // let decoded: Option<BTreeMap<String, PathCacheInfo>> = deserialize(&encoded[..]).unwrap();
                            }

                            if opts.tree.enabled {
                                print(&pc, 0, opts.human.enabled);
                            }

                            handle_exit(&overall, &start, &opts);
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
                    pc: &mut BTreeMap<String, PathCacheInfo>,
                    overall: &mut OverallInfo,
                    dir_files: &mut Vec<Box<Vec<Box<FsItemInfo>>>>,
                    path: &String,
                    info: &Box<FsItemInfo>,
                    opts: &Options)
                    -> bool {
    overall.dirs += 1;

    let mut parts = path.split("/")
        .map(|i| i.to_string())
        .collect::<Vec<String>>();
    parts.reverse();

    path_cache::construct(pc, &mut parts, &info.clone());
    // path_cache::print(&pc, 0);

    let res = print_progress_if_needed(overall, path, info, opts);

    dir_files.push(Box::new(Vec::new()));

    debug!("{:?}", info);

    stack.push(FsDirInfo {
        path: path.clone(),
        files: Vec::new(),
        files_size: 0,
    });

    res
}

fn handle_dir_leave(stack: &mut FsStack,
                    dir_files: &mut Vec<Box<Vec<Box<FsItemInfo>>>>,
                    path: &String,
                    _info: &Box<FsItemInfo>) {

    let _files = dir_files.pop().unwrap();
    // stack.last_mut().unwrap().files = files;
    stack.last_mut().unwrap().calculate_files_size();

    debug!("Stack when leaving {}: {:?}", path, stack);
    stack.pop();
}

fn handle_exit(overall: &OverallInfo, start: &PreciseTime, opts: &Options) {
    if opts.stats.enabled {
        let diff = start.to(PreciseTime::now());
        let elapsed_secs = diff.num_nanoseconds().unwrap() as f64 * 1e-9;
        print_stats(&overall, elapsed_secs, &opts);
    }
}

fn handle_file(pc: &mut BTreeMap<String, PathCacheInfo>,
               overall: &mut OverallInfo,
               dir_files: &mut Vec<Box<FsItemInfo>>,
               path: &String,
               info: Box<FsItemInfo>,
               opts: &Options)
               -> bool {
    overall.files += 1;

    let mut parts = path.split("/")
        .map(|i| i.to_string())
        .collect::<Vec<String>>();
    parts.reverse();

    path_cache::construct(pc, &mut parts, &info.clone());

    let res = print_progress_if_needed(overall, &path, &info, &opts);

    debug!("{:?}", info);
    dir_files.push(info);

    res
}

fn handle_fs_item(stack: &mut FsStack,
                  pc: &mut BTreeMap<String, PathCacheInfo>,
                  overall: &mut OverallInfo,
                  dir_files: &mut Vec<Box<Vec<Box<FsItemInfo>>>>,
                  path: String,
                  info: Box<FsItemInfo>,
                  opts: &Options,
                  stdout: &mut io::Stdout) {
    match info.event_type {
        EventType::DirEnter => {
            if handle_dir_enter(stack, pc, overall, dir_files, &path, &info, &opts) {
                let _ = stdout.flush();
            }
        }
        EventType::DirLeave => {
            handle_dir_leave(stack, dir_files, &path, &info);
        }
        EventType::File => {
            if handle_file(pc,
                           overall,
                           dir_files.last_mut().unwrap(),
                           &path,
                           info,
                           &opts) {
                let _ = stdout.flush();
            }
        }
    };
}

fn print_progress(overall: &OverallInfo, path: &String, info: &Box<FsItemInfo>, opts: &Options) {
    match opts.progress.format {
        ProgressFormat::Dot => print!("."),
        ProgressFormat::Path => {
            println!("{} {}", overall.all(), path);
        }
        ProgressFormat::Raw => println!("{} {:?}", overall.all(), info),
    }
}

fn print_progress_if_needed(overall: &OverallInfo,
                            path: &String,
                            info: &Box<FsItemInfo>,
                            opts: &Options)
                            -> bool {
    if opts.progress.enabled && (overall.all() % opts.progress.count) == 0 {
        print_progress(&overall, &path, &info, &opts);
        true
    } else {
        false
    }
}

fn print_stats(info: &OverallInfo, elapsed_secs: f64, opts: &Options) {
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

    if opts.progress.enabled {
        match opts.progress.format {
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
