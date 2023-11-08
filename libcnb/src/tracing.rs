use libcnb_data::buildpack::Buildpack;
use opentelemetry::{
    global,
    sdk::{
        trace::{Config, Span, TracerProvider},
        Resource,
    },
    trace::{Span as SpanTrait, Status, Tracer, TracerProvider as TracerProviderTrait},
    KeyValue,
};
use std::path::Path;

pub(crate) struct BuildpackTrace {
    provider: TracerProvider,
    span: Span,
}

pub(crate) fn start_trace(buildpack: &Buildpack, phase_name: &'static str) -> BuildpackTrace {
    let trace_name = format!("{}-{phase_name}", buildpack.id.replace(['/', '.'], "_"));
    let tracing_file_path = Path::new("/tmp")
        .join("cnb-telemetry")
        .join(format!("{trace_name}.jsonl"));

    // Ensure tracing file path parent exists by creating it.
    if let Some(parent_dir) = tracing_file_path.parent() {
        let _ = std::fs::create_dir_all(parent_dir);
    }
    let exporter = match std::fs::File::options()
        .create(true)
        .append(true)
        .open(&tracing_file_path)
    {
        Ok(file) => opentelemetry_stdout::SpanExporter::builder()
            .with_writer(file)
            .build(),
        Err(_) => opentelemetry_stdout::SpanExporter::builder()
            .with_writer(std::io::sink())
            .build(),
    };

    let lib_name = option_env!("CARGO_PKG_NAME").unwrap_or("libcnb");

    let provider = opentelemetry::sdk::trace::TracerProvider::builder()
        .with_simple_exporter(exporter)
        .with_config(
            Config::default()
                .with_resource(Resource::new(vec![KeyValue::new("service.name", lib_name)])),
        )
        .build();

    global::set_tracer_provider(provider.clone());

    let tracer = provider.versioned_tracer(
        lib_name,
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
    pub(crate) fn set_error(&mut self, err: &dyn std::error::Error) {
        self.span.set_status(Status::error(err.to_string()));
        self.span.record_error(err);
    }
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
        let telemetry_path = "/tmp/cnb-telemetry/company_com_foo-bar.jsonl";
        _ = fs::remove_file(telemetry_path);

        {
            let mut trace = start_trace(&buildpack, &phase);
            trace.add_event(event);
            trace.set_error(&Error::new(ErrorKind::Other, error_message));
        }
        let tracing_contents = fs::read_to_string(telemetry_path)
            .expect("Expected telemetry file to exist, but couldn't read it");

        println!("tracing_contents: {tracing_contents}");
        assert!(tracing_contents.contains(phase));
        assert!(tracing_contents.contains(event));
        assert!(tracing_contents.contains(error_message));
        assert!(tracing_contents.contains(buildpack.id.as_str()));
        assert!(tracing_contents.contains(&buildpack.version.to_string()));
        assert!(
            tracing_contents.contains(&buildpack.name.expect("Expected buildpack.name to exist"))
        );
        assert!(tracing_contents.contains("something"));
    }
}
