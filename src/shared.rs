use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

pub trait Platform {
    fn get_env_var(&self, key: &str) -> Option<&str>;
    fn get_env_vars(&self) -> &HashMap<String, String>;

    fn has_env_var(&self, key: &str) -> bool {
        self.get_env_var(key).is_some()
    }
}

pub trait BuildFromPath
where
    Self: Sized,
{
    fn build_from_path(path: &Path) -> io::Result<Self>;
}

pub struct GenericPlatform {
    env_vars: HashMap<String, String>,
}

impl Platform for GenericPlatform {
    fn get_env_var(&self, key: &str) -> Option<&str> {
        self.env_vars.get(key).map(|value| &value[..])
    }

    fn get_env_vars(&self) -> &HashMap<String, String> {
        &self.env_vars
    }
}

impl BuildFromPath for GenericPlatform {
    fn build_from_path(path: &Path) -> Result<Self, io::Error> {
        let env_path = path.join("env");
        let mut env_vars: HashMap<String, String> = HashMap::new();

        for entry in fs::read_dir(env_path)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(file_name) = path.file_name().and_then(|os_str| os_str.to_str()) {
                let file_contents = fs::read_to_string(&path)?;
                env_vars.insert(String::from(file_name), file_contents);
            }
        }

        Ok(GenericPlatform { env_vars })
    }
}

pub fn write_toml_file(value: &impl Serialize, path: impl AsRef<Path>) -> io::Result<()> {
    // TODO: Fix Result type, remove unwrap
    fs::write(path, toml::to_string(value).unwrap())
}

pub fn read_toml_file<A: DeserializeOwned>(path: impl AsRef<Path>) -> io::Result<A> {
    // TODO: Fix Result type, remove unwrap
    let file_contents = fs::read_to_string(path)?;
    Ok(toml::from_str(file_contents.as_str()).unwrap())
}
