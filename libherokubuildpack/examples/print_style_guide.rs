use ascii_table::AsciiTable;
use fun_run::CommandWithName;
use indoc::formatdoc;
use libherokubuildpack::output::style::{self, DEBUG_INFO, HELP};
#[allow(clippy::wildcard_imports)]
use libherokubuildpack::output::{
    build_log::BuildLog,
    section_log::{log_step, log_step_stream, log_step_timed},
};
use std::io::stdout;
use std::process::Command;

// Avoid cargo-clippy warnings: "external crate X unused in `print_style_guide`"
use const_format as _;
use crossbeam_utils as _;
use flate2 as _;
use lazy_static as _;
use libcnb as _;
use libcnb_test as _;
use pathdiff as _;
use pretty_assertions as _;
use regex as _;
use sha2 as _;
use tar as _;
use tempfile as _;
use termcolor as _;
use thiserror as _;
use toml as _;
use ureq as _;

#[allow(clippy::too_many_lines)]
fn main() {
    println!(
        "{}",
        formatdoc! {"

            Living build output style guide
            ===============================
        "}
    );

    {
        let mut log = BuildLog::new(stdout()).buildpack_name("Section logging features");
        log = log
            .section("Section heading example")
            .step("step example")
            .step("step example two")
            .end_section();

        log = log
            .section("Section and step description")
            .step(
                "A section should be a noun i.e. 'Ruby Version', consider this the section topic.",
            )
            .step("A step should be a verb i.e. 'Downloading'")
            .step("Related verbs should be nested under a single section")
            .step(
                formatdoc! {"
                Steps can be multiple lines long
                However they're best as short, factual,
                descriptions of what the program is doing.
            "}
                .trim(),
            )
            .step("Prefer a single line when possible")
            .step("Sections and steps are sentence cased with no ending puncuation")
            .step(&format!("{HELP} capitalize the first letter"))
            .end_section();

        let mut command = Command::new("bash");
        command.args(["-c", "ps aux | grep cargo"]);

        let mut stream = log.section("Timer steps")
        .step("Long running code should execute with a timer printing to the UI, to indicate the progam did not hang.")
        .step("Example:")
        .step_timed("Background progress timer")
        .finish_timed_step()
        .step("Output can be streamed. Mostly from commands. Example:")
        .step_timed_stream(&format!("Running {}", style::command(command.name())));

        command
            .stream_output(stream.io(), stream.io())
            .expect("Implement real error handling in real apps");
        stream.finish_timed_stream().end_section();
    }

    {
        let mut log = BuildLog::new(stdout()).buildpack_name("Section log functions");
        log = log
            .section("Logging inside a layer")
            .step(
                formatdoc! {"
                Layer interfaces are neither mutable nor consuming i.e.

                    ```
                    fn create(
                        &self,
                        _context: &BuildContext<Self::Buildpack>,
                        layer_path: &Path,
                    ) -> Result<LayerResult<Self::Metadata>, RubyBuildpackError>
                    ```

                To allow logging within a layer you can use the `output::section_log` interface.
            "}
                .trim_end(),
            )
            .step("This `section_log` inteface allows you to log without state")
            .step("That means you're responsonsible creating a section before calling it")
            .step("Here's an example")
            .end_section();

        let section_log = log.section("Example:");

        log_step("log_step()");
        log_step_timed("log_step_timed()", || {
            // do work here
        });
        log_step_stream("log_step_stream()", |stream| {
            Command::new("bash")
                .args(["-c", "ps aux | grep cargo"])
                .stream_output(stream.io(), stream.io())
                .expect("Implement Error handling in real apps")
        });
        log_step(formatdoc! {"
            If you want to help make sure you're within a section then you can require your layer
            takes a reference to `&'a dyn SectionLogger`
        "});
        section_log.end_section();
    }

    {
        #[allow(clippy::unwrap_used)]
        let cmd_error = Command::new("iDoNotExist").named_output().err().unwrap();

        let mut log = BuildLog::new(stdout()).buildpack_name("Error and warnings");
        log = log
            .section("Debug information")
            .step("Should go above errors in section/step format")
            .end_section();

        log = log
            .section(DEBUG_INFO)
            .step(&cmd_error.to_string())
            .end_section();

        log.announce()
            .warning(&formatdoc! {"
                Warning: This is a warning header

                This is a warning body. Warnings are for when we know for a fact a problem exists
                but it's not bad enough to abort the build.
            "})
            .important(&formatdoc! {"
                Important: This is important

                Important is for when there's critical information that needs to be read
                however it may or may not be a problem. If we know for a fact that there's
                a problem then use a warning instead.

                An example of something that is important but might not be a problem is
                that an application owner upgraded to a new stack.
            "})
            .error(&formatdoc! {"
                Error: This is an error header

                This is the error body. Use an error for when the build cannot continue.
                An error should include a header with a short description of why it cannot continue.

                The body should include what error state was observed, why that's a problem, and
                what remediation steps an application owner using the buildpack to deploy can
                take to solve the issue.
            "});
    }

    {
        let log = BuildLog::new(stdout()).buildpack_name("Formatting helpers");
        log.section("The style module")
            .step(&formatdoc! {"
                Formatting helpers can be used to enhance log output:
            "})
            .end_section();

        let mut table = AsciiTable::default();
        table.set_max_width(240);
        table.column(0).set_header("Example");
        table.column(1).set_header("Code");
        table.column(2).set_header("When to use");

        let data: Vec<Vec<String>> = vec![
            vec![
                style::value("2.3.4"),
                "style::value(\"2.3.f\")".to_string(),
                "With versions, file names or other important values worth highlighting".to_string(),
            ],
            vec![
                style::url("https://www.schneems.com"),
                "style::url(\"https://www.schneems.com\")".to_string(),
                "With urls".to_string(),
            ],
            vec![
                style::command("bundle install"),
                "style::command(command.name())".to_string(),
                "With commands (alongside of `fun_run::CommandWithName`)".to_string(),
            ],
            vec![
                style::details("extra information"),
                "style::details(\"extra information\")".to_string(),
                "Add specific information at the end of a line i.e. 'Cache cleared (ruby version changed)'".to_string()
            ],
            vec![
                style::HELP.to_string(),
                "style::HELP.to_string()".to_string(),
                "A help prefix, use it in a step or section title".to_string()
            ],
            vec![
                style::DEBUG_INFO.to_string(),
                "style::DEBUG_INFO.to_string()".to_string(),
                "A debug prefix, use it in a step or section title".to_string()
            ]
        ];

        table.print(data);
    }
}
