use libcnb_data::buildpack::Buildpack;
use opentelemetry::{
    global,
    trace::{Span as SpanTrait, Status, Tracer, TracerProvider as TracerProviderTrait},
    KeyValue,
};
use opentelemetry_sdk::{
    trace::{Config, Span, TracerProvider},
    Resource,
};
use std::{io::BufWriter, path::Path};

// This is the directory in which `BuildpackTrace` stores Open Telemetry File
// Exports. Services which intend to export the tracing data from libcnb.rs
// (such as [cnb-otel-collector](https://github.com/heroku/cnb-otel-collector))
// should look for `.jsonl` file exports in this directory. This path was chosen
// to prevent conflicts with the CNB spec and /tmp is commonly available and
// writable on base images.
#[cfg(target_family = "unix")]
const TELEMETRY_EXPORT_ROOT: &str = "/tmp/libcnb-telemetry";

/// `BuildpackTrace` represents an Open Telemetry tracer provider and single span.
/// It's designed to support tracing a CNB build or detect phase as a singular
/// span.
pub(crate) struct BuildpackTrace {
    provider: TracerProvider,
    span: Span,
}

/// `start_trace` starts an Open Telemetry trace and span that exports to an
/// Open Telemetry file export. The resulting trace provider and span are
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
    let exporter = match std::fs::File::options()
        .create(true)
        .append(true)
        .open(&tracing_file_path)
    {
        // Write tracing data to a file, which may be read by other
        // services. Wrap with a BufWriter to prevent serde from sending each
        // JSON token to IO, and instead send entire JSON objects to IO.
        Ok(file) => opentelemetry_stdout::SpanExporter::builder()
            .with_writer(BufWriter::new(file))
            .build(),
        // Failed tracing shouldn't fail a build, and any logging here would
        // likely confuse the user, so send telemetry to /dev/null on errors.
        Err(_) => opentelemetry_stdout::SpanExporter::builder()
            .with_writer(std::io::sink())
            .build(),
    };

    let provider = TracerProvider::builder()
        .with_simple_exporter(exporter)
        .with_config(Config::default().with_resource(Resource::new(vec![
            // Associate the tracer provider with service attributes. The buildpack
            // name/version seems to map well to the suggestion
            // [here](https://opentelemetry.io/docs/specs/semconv/resource/#service).
            KeyValue::new("service.name", buildpack.id.to_string()),
            KeyValue::new("service.version", buildpack.version.to_string()),
        ])))
        .build();

    // Set the global tracer provider so that buildpacks may use it.
    global::set_tracer_provider(provider.clone());

    // Get a tracer identified by the instrumentation scope/library. The libcnb crate
    // name/version seems to map well to the suggestion
    // [here](https://opentelemetry.io/docs/specs/otel/trace/api/#get-a-tracer).
    let tracer = provider.versioned_tracer(
        option_env!("CARGO_PKG_NAME").unwrap_or("libcnb.rs"),
        option_env!("CARGO_PKG_VERSION"),
        None as Option<&str>,
        None,
    );

    let mut span = tracer.start(trace_name);
    span.set_attributes(vec![
        KeyValue::new("buildpack_id", buildpack.id.to_string().clone()),
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
    /// `set_error` sets the status for the underlying span to error, and
    /// also records an exception on the span.
    pub(crate) fn set_error(&mut self, err: &dyn std::error::Error) {
        self.span.set_status(Status::error(format!("{err:?}")));
        self.span.record_error(err);
    }
    /// `add_event` adds a named event to the underlying span.
    pub(crate) fn add_event(&mut self, name: &'static str) {
        self.span.add_event(name, vec![]);
    }
}

impl Drop for BuildpackTrace {
    fn drop(&mut self) {
        self.span.end();
        self.provider.force_flush();
        global::shutdown_tracer_provider();
    }
}

#[cfg(test)]
mod tests {
    use super::start_trace;
    use libcnb_data::buildpack::{Buildpack, BuildpackVersion};
    use serde_json::Value;
    use std::{
        collections::HashSet,
        fs,
        io::{Error, ErrorKind},
    };

    #[test]
    fn test_tracing() {
        let buildpack = Buildpack {
            id: "company.com/foo"
                .parse()
                .expect("Valid BuildpackId should parse"),
            version: BuildpackVersion::new(0, 0, 0),
            name: Some("Foo buildpack for company.com".to_string()),
            homepage: None,
            clear_env: false,
            description: None,
            keywords: vec![],
            licenses: vec![],
            sbom_formats: HashSet::new(),
        };
        let phase = "bar";
        let event = "baz-event";
        let error_message = "its broken";
        let telemetry_path = "/tmp/libcnb-telemetry/company_com_foo-bar.jsonl";
        _ = fs::remove_file(telemetry_path);

        {
            let mut trace = start_trace(&buildpack, phase);
            trace.add_event(event);
            trace.set_error(&Error::new(ErrorKind::Other, error_message));
        }
        let tracing_contents = fs::read_to_string(telemetry_path)
            .expect("Expected telemetry file to exist, but couldn't read it");

        println!("tracing_contents: {tracing_contents}");
        let _tracing_data: Value = serde_json::from_str(&tracing_contents)
            .expect("Expected tracing export file contents to be valid json");
        assert!(tracing_contents.contains(phase));
        assert!(tracing_contents.contains(event));
        assert!(tracing_contents.contains(error_message));
        assert!(tracing_contents.contains(buildpack.id.as_str()));
        assert!(tracing_contents.contains(&buildpack.version.to_string()));
        assert!(
            tracing_contents.contains(&buildpack.name.expect("Expected buildpack.name to exist"))
        );
    }
}
