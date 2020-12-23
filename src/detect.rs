use crate::data::build_plan::BuildPlan;
use crate::shared::{write_toml_file, BuildFromPath, Platform};
use std::env;
use std::path::PathBuf;
use std::process;

pub fn cnb_runtime_detect<P: Platform + BuildFromPath>(
    detect_fn: impl Fn(DetectContext<P>) -> DetectResult,
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

        match P::build_from_path(platform_dir.as_path()) {
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
        DetectResult::Fail => process::exit(100),
        DetectResult::Error(code) => process::exit(code),
        DetectResult::Pass(build_plan) => {
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

#[derive(Debug)]
pub enum DetectResult {
    Pass(BuildPlan),
    Fail,
    Error(i32),
}
