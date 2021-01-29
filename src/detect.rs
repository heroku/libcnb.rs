use crate::{
    data::build_plan::BuildPlan,
    platform::{GenericPlatform, Platform},
    shared::write_toml_file,
};
use std::{
    env,
    path::{Path, PathBuf},
    process,
};

pub fn cnb_runtime_detect<
    P: Platform,
    E: std::fmt::Display,
    F: FnOnce(DetectContext<P>) -> Result<DetectOutcome, E>,
>(
    detect_fn: F,
) {
    let app_dir = env::current_dir().expect("Could not determine current working directory!");

    let buildpack_dir = env::var("CNB_BUILDPACK_DIR")
        .expect("Could not determine buildpack directory!")
        .into();

    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: detect <platform_dir> <buildplan>");
        process::exit(1);
    }

    let platform = {
        let platform_dir = PathBuf::from(args.get(1).unwrap());

        if !platform_dir.is_dir() {
            eprintln!("First argument must be a readable platform directory!");
            process::exit(1);
        }

        match P::from_path(platform_dir.as_path()) {
            Ok(platform) => platform,
            Err(error) => {
                eprintln!(
                    "Could not create platform from platform directory: {}",
                    error
                );
                process::exit(1);
            }
        }
    };

    let build_plan_path: PathBuf = PathBuf::from(args.get(2).unwrap());

    let detect_context = DetectContext {
        app_dir,
        buildpack_dir,
        platform,
    };

    match detect_fn(detect_context) {
        Ok(DetectOutcome::Fail) | Err(_) => process::exit(100),
        Ok(DetectOutcome::Error(code)) => process::exit(code),
        Ok(DetectOutcome::Pass(build_plan)) => {
            if let Err(error) = write_toml_file(&build_plan, build_plan_path) {
                eprintln!("Could not write buildplan to disk: {}", error);
                process::exit(1);
            }

            process::exit(0)
        }
    };
}

pub struct DetectContext<P: Platform> {
    app_dir: PathBuf,
    buildpack_dir: PathBuf,
    pub platform: P,
}

impl<P: Platform> DetectContext<P> {
    pub fn app_dir(&self) -> &Path {
        self.app_dir.as_path()
    }

    pub fn buildpack_dir(&self) -> &Path {
        self.buildpack_dir.as_path()
    }
}

pub type GenericDetectContext = DetectContext<GenericPlatform>;

#[derive(Debug)]
pub enum DetectOutcome {
    Pass(BuildPlan),
    Fail,
    Error(i32),
}
