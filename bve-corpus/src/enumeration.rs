use crate::*;
use crossbeam_channel::Sender;
use log::info;

fn enumerate(path: impl AsRef<Path>, mut func: impl FnMut(PathBuf, DirEntry)) -> usize {
    let mut count = 0_usize;
    for entry in WalkDir::new(path.as_ref()).follow_links(true).same_file_system(false) {
        if let Ok(entry) = entry {
            if !entry.file_type().is_dir() {
                let mut file_path = path.as_ref().to_path_buf();
                file_path.push(entry.path());
                func(file_path, entry);
            }
        }
        count += 1;
    }
    count
}

fn add_file(file_sink: &Sender<File>, shared: &SharedData, stats: &Stats, file: File) {
    file_sink.send(file).unwrap();
    shared.total.total.fetch_add(1, Ordering::SeqCst);
    stats.total.fetch_add(1, Ordering::SeqCst);
}

fn should_add_file(allowed: Option<FileType>, desired: FileType) -> bool {
    match allowed {
        Some(t) => t == desired,
        None => true,
    }
}

#[allow(clippy::logic_bug, clippy::shadow_unrelated)] // Remove when `&& false` removed
pub fn enumerate_all_files(options: &Options, file_sink: &Sender<File>, shared: &SharedData) {
    let mut entry_func = |path: PathBuf, _entry: DirEntry| {
        match path.file_name().map(|v| v.to_string_lossy().to_lowercase()) {
            p if p == Some("train.dat".into()) && should_add_file(options.file_types, FileType::TrainDat) => {
                add_file(file_sink, shared, &shared.train_dat, File {
                    path,
                    kind: FileKind::TrainDat,
                })
            }
            p if p == Some("train.xml".into()) && false => add_file(file_sink, shared, &shared.train_xml, File {
                path,
                kind: FileKind::TrainXML,
            }),
            p if p == Some("extensions.cfg".into()) && should_add_file(options.file_types, FileType::ExtensionsCfg) => {
                add_file(file_sink, shared, &shared.extensions_cfg, File {
                    path,
                    kind: FileKind::ExtensionsCfg,
                })
            }
            p if p == Some("panel.cfg".into()) && should_add_file(options.file_types, FileType::PanelCfg) => {
                add_file(file_sink, shared, &shared.panel1_cfg, File {
                    path,
                    kind: FileKind::Panel1Cfg,
                })
            }
            p if p == Some("panel2.cfg".into()) && should_add_file(options.file_types, FileType::Panel2Cfg) => {
                add_file(file_sink, shared, &shared.panel2_cfg, File {
                    path,
                    kind: FileKind::Panel2Cfg,
                })
            }
            p if p == Some("sound.cfg".into()) && should_add_file(options.file_types, FileType::SoundCfg) => {
                add_file(file_sink, shared, &shared.sound_cfg, File {
                    path,
                    kind: FileKind::SoundCfg,
                })
            }
            p if p == Some("ats.cfg".into()) && should_add_file(options.file_types, FileType::AtsCfg) => {
                add_file(file_sink, shared, &shared.ats_cfg, File {
                    path,
                    kind: FileKind::AtsCfg,
                })
            }
            _ => match path.extension().map(|v| v.to_string_lossy().to_lowercase()) {
                ext if ext == Some("b3d".into()) && should_add_file(options.file_types, FileType::B3D) => {
                    add_file(file_sink, shared, &shared.model_b3d, File {
                        path,
                        kind: FileKind::ModelB3d,
                    })
                }
                ext if ext == Some("csv".into()) && should_add_file(options.file_types, FileType::CSV) => {
                    add_file(file_sink, shared, &shared.model_csv, File {
                        path,
                        kind: FileKind::ModelCsv,
                    })
                }
                ext if ext == Some("animated".into()) && should_add_file(options.file_types, FileType::Animated) => {
                    add_file(file_sink, shared, &shared.model_animated, File {
                        path,
                        kind: FileKind::ModelAnimated,
                    })
                }
                Some(_ext) => {
                    // unrecognized
                }
                None => {}
            },
        }
    };

    let mut path = options.path.clone();
    path.push("LegacyContent");
    path.push("Railway");
    path.push("Object");

    info!("Scanning {}", path.display());
    let count = enumerate(path, &mut entry_func);
    info!("Scanned {} files", count);

    let mut path = options.path.clone();
    path.push("LegacyContent");
    path.push("Train");

    info!("Scanning {}", path.display());
    let count = enumerate(path, &mut entry_func);
    info!("Scanned {} files", count);

    let mut path = options.path.clone();
    path.push("LegacyContent");
    path.push("Railway");
    path.push("Route");

    info!("Scanning {}", path.display());
    let count = enumerate(path, |path: PathBuf, _entry| {
        match path.extension().map(|v| v.to_string_lossy().to_lowercase()) {
            ext if ext == Some("csv".into()) && should_add_file(options.file_types, FileType::RouteCSV) => {
                add_file(file_sink, shared, &shared.route_csv, File {
                    path,
                    kind: FileKind::RouteCsv,
                })
            }
            ext if ext == Some("rw".into()) && false => add_file(file_sink, shared, &shared.route_rw, File {
                path,
                kind: FileKind::RouteRw,
            }),
            Some(_ext) => {
                // unrecognized
            }
            None => {}
        }
    });
    info!("Scanned {} files", count);

    shared.fully_loaded.store(true, Ordering::SeqCst);
}
