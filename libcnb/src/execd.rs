use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::os::unix::io::FromRawFd;

/// # Panics
pub fn execd_binary<K: Into<String>, V: Into<String>, M: IntoIterator<Item = (K, V)>>(env_vars: M) {
    let toml_string = toml::to_string(&toml::value::Value::Table(
        env_vars
            .into_iter()
            .map(|(key, value)| (K::into(key), toml::value::Value::String(V::into(value))))
            .collect(),
    ))
    .unwrap();

    writeln!(
        BufWriter::new(unsafe { File::from_raw_fd(3) }),
        "{}",
        toml_string
    )
    .unwrap();
}

#[macro_export]
macro_rules! execd_binary_main {
    ($env_vars:expr) => {
        fn main() {
            ::libcnb::execd::execd_binary($env_vars);
        }
    };
}
