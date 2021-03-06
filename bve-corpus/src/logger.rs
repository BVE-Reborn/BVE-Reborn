#![allow(clippy::mem_forget)]

use crate::{FileKind, FileResult, Options, ParseResult};
use crossbeam_channel::Receiver;
use log::{debug, info};
use serde::Serialize;
use std::{cmp::Reverse, collections::HashMap, fs::write, path::PathBuf};

#[derive(Debug, Default, Clone, Serialize)]
struct ResultCollection {
    file_types: HashMap<FileKind, SingleFileCollection>,
}

#[derive(Debug, Default, Clone, Serialize)]
struct SingleFileCollection {
    successes: Vec<PathBuf>,
    warnings: Vec<Failure>,
    errors: Vec<Failure>,
    panics: Vec<Panic>,
}

#[derive(Debug, Clone, Serialize)]
struct Failure {
    count: u64,
    path: PathBuf,
    issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct Panic {
    path: PathBuf,
    cause: String,
}

pub fn receive_results(options: &Options, result_source: Receiver<FileResult>) {
    let mut results = ResultCollection::default();

    while let Ok(result) = result_source.recv() {
        let single_file_result = results.file_types.entry(result.kind).or_default();
        match result.result {
            ParseResult::Success => single_file_result.successes.push(result.path),
            ParseResult::Issues { warnings, errors } => {
                if !warnings.is_empty() {
                    single_file_result.warnings.push(Failure {
                        count: warnings.len() as u64,
                        path: result.path.clone(),
                        issues: warnings
                            .into_iter()
                            .map(|err| {
                                format!(
                                    "{} - {}",
                                    err.line.map(|v| v.to_string()).as_deref().unwrap_or("None"),
                                    err.description_english
                                )
                            })
                            .collect(),
                    })
                }
                if !errors.is_empty() {
                    single_file_result.errors.push(Failure {
                        count: errors.len() as u64,
                        path: result.path.clone(),
                        issues: errors
                            .into_iter()
                            .map(|err| {
                                format!(
                                    "{} - {}",
                                    err.line.map(|v| v.to_string()).as_deref().unwrap_or("None"),
                                    err.description_english
                                )
                            })
                            .collect(),
                    })
                }
            }
            ParseResult::Panic { cause } => single_file_result.panics.push(Panic {
                cause,
                path: result.path,
            }),
            ParseResult::Finish => {
                std::mem::forget(result_source); // We're finishing, we don't actually care about if this is cleaned up, and this prevents a out-of-time panic cascade 
                break;
            }
        }
    }

    debug!("Summing result counts");
    let (panics, warnings, errors, successes) = results
        .file_types
        .values_mut()
        .map(|single| {
            single
                .warnings
                .sort_by_cached_key(|v| Reverse((v.count, v.path.clone())));
            single.errors.sort_by_cached_key(|v| Reverse((v.count, v.path.clone())));

            (
                single.panics.len(),
                single.warnings.iter().map(|f| f.count).sum(),
                single.errors.iter().map(|f| f.count).sum(),
                single.successes.len(),
            )
        })
        .fold(
            (0, 0, 0, 0),
            |(acc_p, acc_w, acc_e, acc_s): (usize, u64, u64, usize), (p, w, e, s): (usize, u64, u64, usize)| {
                (acc_p + p, acc_w + w, acc_e + e, acc_s + s)
            },
        );

    info!("Panics: {}", panics);
    info!("Warnings: {}", warnings);
    info!("Errors: {}", errors);
    info!("Successes: {}", successes);

    if let Some(output) = &options.output {
        info!("Serializing result");
        let serialized = serde_json::to_string_pretty(&results).unwrap();
        info!("Writing output to {}", output.display());
        write(output, serialized).unwrap();
    } else {
        info!("No output file, not writing results")
    }
}
