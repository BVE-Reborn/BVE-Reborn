// Rust warnings
#![warn(unused)]
#![deny(future_incompatible)]
#![deny(nonstandard_style)]
#![deny(rust_2018_idioms)]
// Rustdoc Warnings
#![deny(intra_doc_link_resolution_failure)]
// Clippy warnings
#![warn(clippy::cargo)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#![warn(clippy::restriction)]
// Annoying regular clippy warnings
#![allow(clippy::cast_sign_loss)] // Annoying
#![allow(clippy::cast_precision_loss)] // Annoying
#![allow(clippy::cast_possible_truncation)] // Annoying
#![allow(clippy::cognitive_complexity)] // This is dumb
// Annoying/irrelevant clippy Restrictions
#![allow(clippy::as_conversions)]
#![allow(clippy::decimal_literal_representation)]
#![allow(clippy::else_if_without_else)]
#![allow(clippy::float_arithmetic)]
#![allow(clippy::float_cmp_const)]
#![allow(clippy::implicit_return)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::integer_arithmetic)]
#![allow(clippy::integer_division)]
#![allow(clippy::let_underscore_must_use)]
#![allow(clippy::match_bool)] // prettier
#![allow(clippy::missing_docs_in_private_items)]
#![allow(clippy::missing_inline_in_public_items)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::option_expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::print_stdout)] // This is a build script, not a fancy app
#![allow(clippy::result_expect_used)]
#![allow(clippy::result_unwrap_used)] // Doesn't play nice with structopt
#![allow(clippy::shadow_reuse)]
#![allow(clippy::shadow_same)]
#![allow(clippy::unreachable)]
#![allow(clippy::wildcard_enum_match_arm)]

use crate::enumeration::enumerate_all_files;
use crate::worker::create_worker_thread;
use anyhow::Result;
use crossbeam::channel::unbounded;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
pub use options::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use structopt::StructOpt;
use walkdir::{DirEntry, WalkDir};

mod enumeration;
mod options;
mod thread_kill;
mod worker;

#[derive(Debug, Default)]
pub struct Stats {
    finished: AtomicU64,
    total: AtomicU64,
}

pub struct File {
    path: PathBuf,
    kind: FileKind,
}

enum FileKind {
    AtsCfg,
    ExtensionsCfg,
    ModelAnimated,
    ModelB3d,
    ModelCsv,
    PanelCfg,
    PanelCfg2,
    RouteCsv,
    RouteRw,
    SoundCfg,
    TrainDat,
    TrainXML,
}

pub struct FileResult {
    path: PathBuf,
    kind: FileKind,
    result: Result<Duration>,
}

#[derive(Debug, Default)]
pub struct SharedData {
    total: Stats,
    ats_cfg: Stats,
    extensions_cfg: Stats,
    model_animated: Stats,
    model_b3d: Stats,
    model_csv: Stats,
    panel_cfg: Stats,
    panel_cfg2: Stats,
    route_csv: Stats,
    route_rw: Stats,
    sound_cfg: Stats,
    train_dat: Stats,
    train_xml: Stats,

    fully_loaded: AtomicBool,
}

fn main() {
    let options: Options = Options::from_args();

    let shared = Arc::new(SharedData::default());
    let (file_sink, file_source) = unbounded();
    let (result_sink, result_source) = unbounded();

    // Progress bars
    let mp = MultiProgress::new();
    let style = ProgressStyle::default_spinner()
        .template("Total: {wide_bar} {pos:>6}/{len:6} {elapsed_precise} (eta {eta_precise}) {msg}")
        .progress_chars("##-");

    let total_progress = mp.add(ProgressBar::new(0).with_style(style.clone()));

    let enumeration_thread = {
        let shared = Arc::clone(&shared);
        let options = options.clone();
        std::thread::spawn(move || enumerate_all_files(options, file_sink, shared))
    };

    let worker_thread = create_worker_thread(&file_source, &result_sink, &shared);

    let tui_progress_thread = std::thread::spawn(move || mp.join().unwrap());

    while shared.fully_loaded.load(Ordering::SeqCst) == false
        || (shared.total.total.load(Ordering::SeqCst) - shared.total.finished.load(Ordering::SeqCst)) != 0
    {
        total_progress.set_position(shared.total.finished.load(Ordering::SeqCst));
        total_progress.set_length(shared.total.total.load(Ordering::SeqCst));
        std::thread::sleep(Duration::from_millis(2));
    }

    total_progress.finish();

    enumeration_thread.join().unwrap(); // Closes down file_sink which shuts down the processing threads when done.
    tui_progress_thread.join().unwrap();

    worker_thread.handle.join();

    dbg!(shared);
}
