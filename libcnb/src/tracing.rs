use futures_core::future::BoxFuture;
use libcnb_data::buildpack::Buildpack;
use opentelemetry::{
    InstrumentationScope, KeyValue,
    global::{self, BoxedSpan},
    trace::{Span as SpanTrait, Status, Tracer, TracerProvider as TracerProviderTrait},
};
use opentelemetry_proto::transform::trace::tonic::group_spans_by_resource_and_scope;
use opentelemetry_proto::{
    tonic::trace::v1::TracesData, transform::common::tonic::ResourceAttributesWithSchema,
};
use opentelemetry_sdk::{
    Resource,
    error::{OTelSdkError, OTelSdkResult},
    trace::SdkTracerProvider,
    trace::SpanExporter,
};
use std::{
    fmt::Debug,
    io::{LineWriter, Write},
    path::Path,
    sync::{Arc, Mutex},
};

// This is the directory in which `BuildpackTrace` stores OpenTelemetry File
// Exports. Services which intend to export the tracing data from libcnb.rs
// (such as https://github.com/heroku/cnb-otel-collector)
// should look for `.jsonl` file exports in this directory. This path was chosen
// to prevent conflicts with the CNB spec and /tmp is commonly available and
// writable on base images.
#[cfg(target_family = "unix")]
const TELEMETRY_EXPORT_ROOT: &str = "/tmp/libcnb-telemetry";

/// Represents an OpenTelemetry tracer provider and single span tracing
/// a single CNB build or detect phase.
pub(crate) struct BuildpackTrace {
    provider: SdkTracerProvider,
    span: BoxedSpan,
}

/// Start an OpenTelemetry trace and span that exports to an
/// OpenTelemetry file export. The resulting trace provider and span are
/// enriched with data from the buildpack and the rust environment.
pub(crate) fn start_trace(buildpack: &Buildpack, phase_name: &'static str) -> BuildpackTrace {
    let trace_name = format!(
        "{}-{phase_name}",
        buildpack.id.replace(['/', '.', '-'], "_")
    );
    let tracing_file_path = Path::new(TELEMETRY_EXPORT_ROOT).join(format!("{trace_name}.jsonl"));

    // Ensure tracing file path parent exists by creating it.
    if let Some(parent_dir) = tracing_file_path.parent() {
        let _ = std::fs::create_dir_all(parent_dir);
    }

    let resource = Resource::builder()
        // Define a resource that defines the trace provider.
        // The buildpack name/version seems to map well to the suggestion here
        // https://opentelemetry.io/docs/specs/semconv/resource/#service.
        .with_attributes([
            KeyValue::new("service.name", buildpack.id.to_string()),
            KeyValue::new("service.version", buildpack.version.to_string()),
        ])
        .build();

    let provider_builder = SdkTracerProvider::builder().with_resource(resource.clone());

    let provider = match std::fs::File::options()
        .create(true)
        .append(true)
        .open(&tracing_file_path)
        .map(|file| FileExporter::new(file, resource))
    {
        // Write tracing data to a file, which may be read by other services
        Ok(exporter) => provider_builder.with_batch_exporter(exporter),
        // Failed tracing shouldn't fail a build, and any export logging here
        // would likely confuse the user; don't export when the file has IO errors
        Err(_) => provider_builder,
    }
    .build();

    // Set the global tracer provider so that buildpacks may use it.
    global::set_tracer_provider(provider.clone());

    // Get a tracer identified by the instrumentation scope/library. The libcnb
    // crate name/version seems to map well to the suggestion here:
    // https://opentelemetry.io/docs/specs/otel/trace/api/#get-a-tracer.
    let tracer = global::tracer_provider().tracer_with_scope(
        InstrumentationScope::builder(env!("CARGO_PKG_NAME"))
            .with_version(env!("CARGO_PKG_VERSION"))
            .build(),
    );

    let mut span = tracer.start(trace_name);
    span.set_attributes([
        KeyValue::new("buildpack_id", buildpack.id.to_string()),
        KeyValue::new("buildpack_name", buildpack.name.clone().unwrap_or_default()),
        KeyValue::new("buildpack_version", buildpack.version.to_string()),
        KeyValue::new(
            "buildpack_homepage",
            buildpack.homepage.clone().unwrap_or_default(),
        ),
    ]);
    BuildpackTrace { provider, span }
}

impl BuildpackTrace {
    /// Set the status for the underlying span to error, and record
    /// an exception on the span.
    pub(crate) fn set_error(&mut self, err: &dyn std::error::Error) {
        self.span.set_status(Status::error(format!("{err:?}")));
        self.span.record_error(err);
    }
    /// Add a named event to the underlying span.
    pub(crate) fn add_event(&mut self, name: &'static str) {
        self.span.add_event(name, Vec::new());
    }
}

impl Drop for BuildpackTrace {
    fn drop(&mut self) {
        self.span.end();
        self.provider.force_flush().ok();
        self.provider.shutdown().ok();
    }
}

#[derive(Debug)]
struct FileExporter<W: Write + Send + Debug> {
    writer: Arc<Mutex<LineWriter<W>>>,
    resource: Resource,
}

impl<W: Write + Send + Debug> FileExporter<W> {
    fn new(writer: W, resource: Resource) -> Self {
        Self {
            writer: Arc::new(Mutex::new(LineWriter::new(writer))),
            resource,
        }
    }
}

impl<W: Write + Send + Debug> SpanExporter for FileExporter<W> {
    fn export(
        &mut self,
        batch: Vec<opentelemetry_sdk::trace::SpanData>,
    ) -> BoxFuture<'static, OTelSdkResult> {
        let resource = ResourceAttributesWithSchema::from(&self.resource);
        let resource_spans = group_spans_by_resource_and_scope(batch, &resource);
        let data = TracesData { resource_spans };

        let mut writer = match self.writer.lock() {
            Ok(f) => f,
            Err(e) => {
                return Box::pin(std::future::ready(Err(OTelSdkError::InternalFailure(
                    e.to_string(),
                ))));
            }
        };
        Box::pin(std::future::ready(
            serde_json::to_writer(writer.get_mut(), &data)
                .map_err(|e| OTelSdkError::InternalFailure(e.to_string())),
        ))
    }

    fn force_flush(&mut self) -> OTelSdkResult {
        let mut writer = self
            .writer
            .lock()
            .map_err(|e| OTelSdkError::InternalFailure(e.to_string()))?;

        writer
            .flush()
            .map_err(|e| OTelSdkError::InternalFailure(e.to_string()))
    }

    fn set_resource(&mut self, res: &opentelemetry_sdk::Resource) {
        self.resource = res.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::start_trace;
    use libcnb_data::{
        buildpack::{Buildpack, BuildpackVersion},
        buildpack_id,
    };
    use serde_json::Value;
    use std::{
        collections::HashSet,
        fs,
        io::{Error, ErrorKind},
    };

    #[test]
    fn test_tracing() {
        let buildpack = Buildpack {
            id: buildpack_id!("company.com/foo"),
            version: BuildpackVersion::new(0, 0, 99),
            name: Some("Foo buildpack for company.com".to_string()),
            homepage: None,
            clear_env: false,
            description: None,
            keywords: Vec::new(),
            licenses: Vec::new(),
            sbom_formats: HashSet::new(),
        };
        let telemetry_path = "/tmp/libcnb-telemetry/company_com_foo-bar.jsonl";
        _ = fs::remove_file(telemetry_path);

        {
            let mut trace = start_trace(&buildpack, "bar");
            trace.add_event("baz-event");
            trace.set_error(&Error::new(ErrorKind::Other, "it's broken"));
        }
        let tracing_contents = fs::read_to_string(telemetry_path)
            .expect("Expected telemetry file to exist, but couldn't read it");

        println!("tracing_contents: {tracing_contents}");
        let _tracing_data: Value = serde_json::from_str(&tracing_contents)
            .expect("Expected tracing export file contents to be valid json");

        // Check top level structure
        assert!(tracing_contents.contains("{\"resourceSpans\":[{\"resource\":"));

        // Check resource attributes
        assert!(tracing_contents.contains(
            "{\"key\":\"service.name\",\"value\":{\"stringValue\":\"company.com/foo\"}}"
        ));
        assert!(
            tracing_contents
                .contains("{\"key\":\"service.version\",\"value\":{\"stringValue\":\"0.0.99\"}}")
        );

        // Check span name
        assert!(tracing_contents.contains("\"name\":\"company_com_foo-bar\""));

        // Check span attributes
        assert!(tracing_contents.contains(
            "{\"key\":\"buildpack_id\",\"value\":{\"stringValue\":\"company.com/foo\"}}"
        ));
        assert!(
            tracing_contents
                .contains("{\"key\":\"buildpack_version\",\"value\":{\"stringValue\":\"0.0.99\"}}")
        );
        assert!(tracing_contents.contains(
                "{\"key\":\"buildpack_name\",\"value\":{\"stringValue\":\"Foo buildpack for company.com\"}}"
        ));

        // Check event name
        assert!(tracing_contents.contains("\"name\":\"baz-event\""));

        // Check exception event
        assert!(tracing_contents.contains("\"name\":\"exception\""));
        assert!(tracing_contents.contains(
            "{\"key\":\"exception.message\",\"value\":{\"stringValue\":\"it's broken\"}}"
        ));

        // Check error status
        assert!(
            tracing_contents
                .contains("\"message\":\"Custom { kind: Other, error: \\\"it's broken\\\" }")
        );
        assert!(tracing_contents.contains("\"code\":2"));
    }
}
