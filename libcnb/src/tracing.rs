use crate::Buildpack;
use libcnb_data::buildpack::{BuildpackId, ComponentBuildpackDescriptor};
use opentelemetry::{
    global::{self},
    sdk::trace::TracerProvider,
    trace::{SpanRef, Tracer, TracerProvider as TracerProviderImpl},
    Context, KeyValue,
};
use opentelemetry_stdout::SpanExporter;
use std::{
    fmt::Display,
    fs::{create_dir_all, File},
    io::sink,
    path::Path,
};

pub fn with_tracing<F, R, S>(step: S, bp_id: BuildpackId, f: F) -> R
where
    F: FnOnce(Context) -> R,
    S: Display,
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
    let result = tracer.in_span(format!("libcnb-{step}-{bp_slug}"), |trace_ctx| f(trace_ctx));
    provider.force_flush();
    global::shutdown_tracer_provider();
    result
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
            bp_descriptor
                .buildpack
                .name
                .clone()
                .unwrap_or_else(String::new),
        ),
        KeyValue::new("buildpack_api", bp_descriptor.api.to_string()),
    ]);
}
