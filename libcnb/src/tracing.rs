use crate::Buildpack;
use libcnb_data::buildpack::{BuildpackId, ComponentBuildpackDescriptor};
use opentelemetry::{
    global::{self},
    sdk::trace::TracerProvider,
    trace::{SpanRef, TraceContextExt, Tracer, TracerProvider as TracerProviderImpl},
    Context, KeyValue,
};
use opentelemetry_stdout::SpanExporter;
use std::{
    fmt::Display,
    fs::{create_dir_all, File},
    io::sink,
    path::Path,
};

pub fn with_tracing<B>(
    phase: impl Display,
    bp_id: &BuildpackId,
    f: impl FnOnce(&Context) -> crate::Result<i32, B::Error>,
) -> crate::Result<i32, B::Error>
where
    B: Buildpack,
{
    let bp_slug = bp_id.replace(['/', '.'], "_");
    let provider = init_tracing(&bp_slug);
    global::set_tracer_provider(provider.clone());
    let tracer = provider.versioned_tracer(
        option_env!("CARGO_PKG_NAME").unwrap_or("libcnb"),
        option_env!("CARGO_PKG_VERSION"),
        None as Option<&str>,
        None,
    );
    let outer_result = tracer.in_span(format!("libcnb-{bp_slug}-{phase}"), |trace_ctx| {
        let inner_result = f(&trace_ctx);
        if let Err(err) = &inner_result {
            let span = trace_ctx.span();
            // span.record_error(err) would make more sense than an event here,
            // but Buildpack::Error doesn't implement std::error::Error.
            // Should it?
            span.add_event(format!("{phase}-error"), vec![]);
            span.set_status(opentelemetry::trace::Status::Error {
                description: std::borrow::Cow::Owned(format!("{err:?}")),
            });
        };
        inner_result
    });
    provider.force_flush();
    global::shutdown_tracer_provider();
    outer_result
}

fn init_tracing(bp_id: &str) -> TracerProvider {
    let tracing_file_path = Path::new("/tmp")
        .join("cnb-telemetry")
        .join(format!("{bp_id}.jsonl"));

    // Ensure tracing file path parent exists by creating it.
    if let Some(parent_dir) = tracing_file_path.parent() {
        let _ = create_dir_all(parent_dir);
    }
    let exporter = match File::options()
        .create(true)
        .append(true)
        .open(&tracing_file_path)
    {
        Ok(file) => SpanExporter::builder().with_writer(file).build(),
        Err(_) => SpanExporter::builder().with_writer(sink()).build(),
    };

    TracerProvider::builder()
        .with_simple_exporter(exporter)
        .build()
}

pub fn set_buildpack_span_attributes<B: Buildpack + ?Sized>(
    span: &SpanRef,
    bp_descriptor: &ComponentBuildpackDescriptor<B::Metadata>,
) {
    span.set_attributes(vec![
        KeyValue::new("buildpack_id", bp_descriptor.buildpack.id.to_string()),
        KeyValue::new(
            "buildpack_version",
            bp_descriptor.buildpack.version.to_string(),
        ),
        KeyValue::new(
            "buildpack_name",
            bp_descriptor.buildpack.name.clone().unwrap_or_default(),
        ),
        KeyValue::new("buildpack_api", bp_descriptor.api.to_string()),
    ]);
}

#[cfg(test)]
mod tests {
    use super::with_tracing;
    use crate::{
        build::{BuildContext, BuildResult, BuildResultBuilder},
        detect::{DetectContext, DetectResult, DetectResultBuilder},
        generic::{GenericMetadata, GenericPlatform},
        Buildpack, Error,
    };
    use libcnb_data::buildpack::BuildpackId;
    use opentelemetry::trace::TraceContextExt;
    use std::fs;

    struct TestBuildpack;

    impl Buildpack for TestBuildpack {
        type Platform = GenericPlatform;
        type Metadata = GenericMetadata;
        type Error = TestBuildpackError;

        fn detect(
            &self,
            _context: DetectContext<Self>,
        ) -> crate::Result<DetectResult, Self::Error> {
            DetectResultBuilder::pass().build()
        }

        fn build(&self, _context: BuildContext<Self>) -> crate::Result<BuildResult, Self::Error> {
            BuildResultBuilder::new().build()
        }
    }

    #[derive(Debug)]
    struct TestBuildpackError;

    #[test]
    fn with_tracing_ok() {
        let buildpack_id: BuildpackId = "heroku/foo-engine"
            .parse()
            .expect("Expected to parse this buildpack id");
        let telemetry_path = "/tmp/cnb-telemetry/heroku_foo-engine.jsonl";

        with_tracing::<TestBuildpack>("detect", &buildpack_id, |trace_ctx| {
            trace_ctx.span().add_event("realigning-splines", vec![]);
            Ok(0)
        })
        .expect("Expected tracing result to be Ok, but was an Err");

        let tracing_contents = fs::read_to_string(telemetry_path)
            .expect("Expected telemetry file to exist, but couldn't read it");
        _ = fs::remove_file(telemetry_path);

        println!("tracing_contents: {tracing_contents}");
        assert!(tracing_contents.contains("libcnb-heroku_foo-engine-detect"));
        assert!(tracing_contents.contains("realigning-splines"));
    }

    #[test]
    fn with_tracing_err() {
        let buildpack_id: BuildpackId = "heroku/bar-engine"
            .parse()
            .expect("Expected to parse this buildpack id");
        let telemetry_path = "/tmp/cnb-telemetry/heroku_bar-engine.jsonl";

        with_tracing::<TestBuildpack>("build", &buildpack_id, |_| {
            Err(Error::BuildpackError(TestBuildpackError))
        })
        .expect_err("Expected tracing result to be an Err, but was Ok");

        let tracing_contents = fs::read_to_string(telemetry_path)
            .expect("Expected telemetry file to exist, but couldn't read it");
        _ = fs::remove_file(telemetry_path);

        println!("tracing_contents: {tracing_contents}");
        assert!(tracing_contents.contains("build-error"));
        assert!(tracing_contents.contains("TestBuildpackError"));
        assert!(tracing_contents.contains("libcnb-heroku_bar-engine-build"));
    }
}
