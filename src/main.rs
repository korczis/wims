#[macro_use]
extern crate log;
extern crate env_logger;

extern crate clap;
extern crate wims;

use clap::{App, Arg};
use std::env;

use std::io;
use std::io::Write;
use std::fs::{self, DirEntry};
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::SystemTime;
use wims::*;

const AUTHOR: &'static str = env!("CARGO_PKG_AUTHORS");
const DESCRIPTION: &'static str = env!("CARGO_PKG_DESCRIPTION");
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    let mut stdout = io::stdout();

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

    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        loop {
            match rx.recv() {
                Ok(received) => {
                    let data: (MessageType, Option<OverallInfo>, Option<String>) = received;

                    match data.0 {
                        MessageType::Entry => {
                            let info = data.1.unwrap();
                            let path = data.2.unwrap();

                            if progress && (info.files % progress_count) == 0 {
                                match progress_format {
                                    ProgressFormat::Path => println!("{} - {}", info.files, path),
                                    ProgressFormat::Dot => print!("."),
                                }
                                let _ = stdout.flush();
                            }
                        }
                        MessageType::Exit => {
                            break;
                        }
                    };
                }
                _ => {}
            }
        }
    });

    let mut info = OverallInfo {
        dirs: 0,
        files: 0,
    };

    let now = SystemTime::now();
    {
        type FsStack = Vec<FsDirInfo>;

        let mut dir_stack: FsStack = FsStack::new();

        fn get_file_info(path: &Path, entry: &DirEntry) -> FsFileInfo {
            let md = Box::new(entry.metadata().unwrap()) as Box<std::os::unix::fs::MetadataExt>;

            let res = FsFileInfo {
                path: path.to_str().unwrap().to_string(),
                ino: md.ino(),
                mtime: md.mtime(),
                size: md.size(),
            };

            debug!("{:?}", res);

            return res;
        };

        // one possible implementation of walking a directory only visiting files
        fn visit_dir(stack: &mut FsStack,
                     dir: &Path,
                     cb: &mut FnMut(&Path, &DirEntry))
                     -> io::Result<()> {

            debug!("Entering directory {:?}", dir);

            let mut stat = OverallInfo {
                dirs: 0,
                files: 0,
            };

            let metadata = fs::symlink_metadata(dir)?;
            let file_type = metadata.file_type();

            if dir.is_dir() && !file_type.is_symlink() {
                // let _dir_meta = dir.metadata();
                let dir_info = FsDirInfo {
                    path: dir.to_str().unwrap().to_string(),
                    // dirs: Vec::new(),
                    files: Vec::new(),
                    files_size: 0,
                };

                stack.push(dir_info);

                let mut dir_files = Vec::new();

                if let Ok(entries) = fs::read_dir(dir) {
                    for entry in entries {
                        debug!("Processing entry {:?}", entry);

                        if let Ok(entry) = entry {
                            let path = entry.path();
                            if path.is_dir() {
                                debug!("Found directory: {:?}", &path);

                                stat.dirs += 1;
                                let _ = visit_dir(stack, &path, cb);
                                cb(&path, &entry);
                            } else if path.is_file() {
                                debug!("Found file: {:?}", &path);

                                stat.files += 1;

                                dir_files.push(get_file_info(&path, &entry));
                                cb(&path, &entry);
                            }
                        }
                    }
                }

                stack.last_mut().unwrap().files = dir_files;
                stack.last_mut().unwrap().calculate_files_size();

                debug!("{:?}", stack);
                stack.pop();
            } else {
                debug!("{:?}", stack);
            }

            debug!("Leaving directory {:?}", dir);
            Ok(())
        }

        let mut visitor = |path: &Path, _entry: &DirEntry| {
            if path.is_dir() {
                info.dirs += 1;
            } else if path.is_file() {
                info.files += 1;

                let _ = tx.send((MessageType::Entry,
                                 Some(info.clone()),
                                 Some(path.to_str().unwrap().to_string())));
            }
        };

        let dirs: Vec<_> = matches.values_of("DIR").unwrap().collect();
        for dir in dirs.iter() {
            let _ = visit_dir(&mut dir_stack, Path::new(dir), &mut visitor);
        }
    }

    let elapsed = now.elapsed();
    match elapsed {
        Ok(elapsed) => {
            let dirs_count = info.dirs;
            let files_count = info.files;

            let elapsed_secs = (elapsed.as_secs() as f64) +
                               ((elapsed.subsec_nanos() as f64) * 1e-9);

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
        _ => {}
    };

    let _ = tx.send((MessageType::Exit, None, None));
    let _ = handle.join();
}
