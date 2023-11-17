//! All integration tests are skipped by default (using the `ignore` attribute)
//! since performing builds is slow. To run them use: `cargo test -- --ignored`.

// Enable Clippy lints that are disabled by default.
// https://rust-lang.github.io/rust-clippy/stable/index.html
#![warn(clippy::pedantic)]

use libcnb::data::buildpack_id;
use libcnb::data::layer_name;
use libcnb::data::sbom::SbomFormat;
use libcnb_test::{BuildConfig, SbomType, TestRunner};
use std::env::temp_dir;
use std::fs;

#[test]
#[ignore = "integration test"]
#[allow(clippy::too_many_lines)]
fn test() {
    let empty_app_dir = temp_dir().join("empty-app-dir");
    fs::create_dir_all(&empty_app_dir).unwrap();

    let buildpack_id = buildpack_id!("libcnb-test-buildpacks/sbom");
    let build_config = BuildConfig::new("heroku/builder:22", &empty_app_dir);

    TestRunner::default().build(&build_config, |context| {
        context.download_sbom_files(|sbom_files| {
            assert_eq!(
                fs::read(sbom_files.path_for(
                    &buildpack_id,
                    SbomType::Launch,
                    SbomFormat::CycloneDxJson
                ))
                .unwrap(),
                include_bytes!("../etc/cyclonedx_1.sbom.json")
            );

            assert_eq!(
                fs::read(sbom_files.path_for(
                    &buildpack_id,
                    SbomType::Launch,
                    SbomFormat::SpdxJson
                ))
                .unwrap(),
                include_bytes!("../etc/spdx_1.sbom.json")
            );

            assert_eq!(
                fs::read(sbom_files.path_for(
                    &buildpack_id,
                    SbomType::Launch,
                    SbomFormat::SyftJson
                ))
                .unwrap(),
                include_bytes!("../etc/syft_1.sbom.json")
            );

            assert_eq!(
                fs::read(sbom_files.path_for(
                    &buildpack_id,
                    SbomType::Layer(layer_name!("test")),
                    SbomFormat::CycloneDxJson
                ))
                .unwrap(),
                include_bytes!("../etc/cyclonedx_3.sbom.json")
            );

            assert_eq!(
                fs::read(sbom_files.path_for(
                    &buildpack_id,
                    SbomType::Layer(layer_name!("test")),
                    SbomFormat::SpdxJson
                ))
                .unwrap(),
                include_bytes!("../etc/spdx_3.sbom.json")
            );

            assert_eq!(
                fs::read(sbom_files.path_for(
                    &buildpack_id,
                    SbomType::Layer(layer_name!("test")),
                    SbomFormat::SyftJson
                ))
                .unwrap(),
                include_bytes!("../etc/syft_3.sbom.json")
            );

            assert_eq!(
                fs::read(sbom_files.path_for(
                    &buildpack_id,
                    SbomType::Layer(layer_name!("test2")),
                    SbomFormat::CycloneDxJson
                ))
                .unwrap(),
                include_bytes!("../etc/cyclonedx_2.sbom.json")
            );

            assert_eq!(
                fs::read(sbom_files.path_for(
                    &buildpack_id,
                    SbomType::Layer(layer_name!("test2")),
                    SbomFormat::SpdxJson
                ))
                .unwrap(),
                include_bytes!("../etc/spdx_2.sbom.json")
            );

            assert_eq!(
                fs::read(sbom_files.path_for(
                    &buildpack_id,
                    SbomType::Layer(layer_name!("test2")),
                    SbomFormat::SyftJson
                ))
                .unwrap(),
                include_bytes!("../etc/syft_2.sbom.json")
            );
        });

        context.rebuild(&build_config, |context| {
            context.download_sbom_files(|sbom_files| {
                // The 'test' layer does not return any SBOM files in its update implementation. This
                // must result in no SBOM files being used from previous build.
                assert!(!sbom_files
                    .path_for(
                        &buildpack_id,
                        SbomType::Layer(layer_name!("test")),
                        SbomFormat::CycloneDxJson,
                    )
                    .exists());

                assert!(!sbom_files
                    .path_for(
                        &buildpack_id,
                        SbomType::Layer(layer_name!("test")),
                        SbomFormat::SpdxJson,
                    )
                    .exists());

                assert!(!sbom_files
                    .path_for(
                        &buildpack_id,
                        SbomType::Layer(layer_name!("test")),
                        SbomFormat::SyftJson,
                    )
                    .exists());

                // The 'test2' layer will be cached between builds and should remain untouched, SBOM
                // files included.
                assert_eq!(
                    fs::read(sbom_files.path_for(
                        &buildpack_id,
                        SbomType::Layer(layer_name!("test2")),
                        SbomFormat::CycloneDxJson
                    ))
                    .unwrap(),
                    include_bytes!("../etc/cyclonedx_2.sbom.json")
                );

                assert_eq!(
                    fs::read(sbom_files.path_for(
                        &buildpack_id,
                        SbomType::Layer(layer_name!("test2")),
                        SbomFormat::SpdxJson
                    ))
                    .unwrap(),
                    include_bytes!("../etc/spdx_2.sbom.json")
                );

                assert_eq!(
                    fs::read(sbom_files.path_for(
                        &buildpack_id,
                        SbomType::Layer(layer_name!("test2")),
                        SbomFormat::SyftJson
                    ))
                    .unwrap(),
                    include_bytes!("../etc/syft_2.sbom.json")
                );
            });
        });
    });
}
