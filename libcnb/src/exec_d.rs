use libcnb_data::exec_d::ExecDProgramOutput;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;

/// Writes the output of a CNB exec.d program in a spec compliant way.
pub fn write_exec_d_program_output<O: Into<ExecDProgramOutput>>(o: O) {
    // Allow compilation of exec.d programs under windows, but fail at runtime:
    #[cfg(target_family = "windows")]
    unimplemented!("libcnb.rs does not support running in Windows containers yet!");

    #[cfg(target_family = "unix")]
    {
        use std::os::unix::io::FromRawFd;

        // https://github.com/buildpacks/spec/blob/main/buildpack.md#execd
        let output_file = unsafe { File::from_raw_fd(3) };

        let serialized_output =
            toml::to_string(&o.into()).expect("Could not TOML serialize exec.d program output: ");

        write!(BufWriter::new(output_file), "{}", serialized_output)
            .expect("Could not write exec.d program output: ");
    }
}
