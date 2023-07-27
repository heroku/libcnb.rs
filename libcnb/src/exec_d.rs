use libcnb_data::exec_d::ExecDProgramOutput;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;

/// Writes the output of a CNB exec.d program in a spec compliant way.
///
/// # Panics
///
/// Panics if there was an error serializing the TOML output or writing to FD 3.
pub fn write_exec_d_program_output<O: Into<ExecDProgramOutput>>(o: O) {
    // Allow compilation of exec.d programs under windows, but fail at runtime:
    #[cfg(target_family = "windows")]
    unimplemented!("libcnb.rs does not support running in Windows containers yet!");

    #[cfg(target_family = "unix")]
    {
        use std::os::unix::io::FromRawFd;

        // The spec requires writing the TOML to fd3:
        // https://github.com/buildpacks/spec/blob/main/buildpack.md#execd
        //
        // Using a file descriptor by id is an unsafe operation since Rust cannot guarantee it's
        // actually mapped to something. Since we're implementing the CNB spec and it explicitly
        // tells us to write to that file descriptor, this is safe to do without additional
        // validation in this context.
        let output_file = unsafe { File::from_raw_fd(3) };

        let serialized_output =
            toml::to_string(&o.into()).expect("Could not TOML serialize exec.d program output: ");

        BufWriter::new(output_file)
            .write_all(serialized_output.as_bytes())
            .expect("Could not write exec.d program output: ");
    }
}
