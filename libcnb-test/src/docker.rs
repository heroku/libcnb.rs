use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

/// Represents a `docker run` command.
#[derive(Clone, Debug)]
pub(crate) struct DockerRunCommand {
    command: Option<Vec<String>>,
    container_name: String,
    detach: bool,
    entrypoint: Option<String>,
    env: BTreeMap<String, String>,
    exposed_ports: BTreeSet<u16>,
    image_name: String,
    platform: Option<String>,
    remove: bool,
}

impl DockerRunCommand {
    pub fn new(image_name: impl Into<String>, container_name: impl Into<String>) -> Self {
        Self {
            command: None,
            container_name: container_name.into(),
            detach: false,
            entrypoint: None,
            env: BTreeMap::new(),
            exposed_ports: BTreeSet::new(),
            image_name: image_name.into(),
            platform: None,
            remove: false,
        }
    }

    pub fn command<I: IntoIterator<Item = S>, S: Into<String>>(&mut self, command: I) -> &mut Self {
        self.command = Some(command.into_iter().map(S::into).collect());
        self
    }

    pub fn detach(&mut self, detach: bool) -> &mut Self {
        self.detach = detach;
        self
    }

    pub fn entrypoint(&mut self, entrypoint: impl Into<String>) -> &mut Self {
        self.entrypoint = Some(entrypoint.into());
        self
    }

    pub fn env(&mut self, k: impl Into<String>, v: impl Into<String>) -> &mut Self {
        self.env.insert(k.into(), v.into());
        self
    }

    pub fn expose_port(&mut self, port: u16) -> &mut Self {
        self.exposed_ports.insert(port);
        self
    }

    pub fn platform(&mut self, platform: impl Into<String>) -> &mut Self {
        self.platform = Some(platform.into());
        self
    }

    pub fn remove(&mut self, remove: bool) -> &mut Self {
        self.remove = remove;
        self
    }
}

impl From<DockerRunCommand> for Command {
    fn from(docker_run_command: DockerRunCommand) -> Self {
        let mut command = Command::new("docker");
        command.args(["run", "--name", &docker_run_command.container_name]);

        if docker_run_command.detach {
            command.arg("--detach");
        }

        if docker_run_command.remove {
            command.arg("--rm");
        }

        if let Some(platform) = docker_run_command.platform {
            command.args(["--platform", &platform]);
        }

        if let Some(entrypoint) = docker_run_command.entrypoint {
            command.args(["--entrypoint", &entrypoint]);
        }

        for (env_key, env_value) in &docker_run_command.env {
            command.args(["--env", &format!("{env_key}={env_value}")]);
        }

        for port in &docker_run_command.exposed_ports {
            command.args(["--publish", &format!("127.0.0.1::{port}")]);
        }

        command.arg(docker_run_command.image_name);

        if let Some(container_command) = docker_run_command.command {
            command.args(container_command);
        }

        command
    }
}

/// Represents a `docker exec` command.
#[derive(Clone, Debug)]
pub(crate) struct DockerExecCommand {
    command: Vec<String>,
    container_name: String,
}

impl DockerExecCommand {
    pub fn new<I: IntoIterator<Item = S>, S: Into<String>>(
        container_name: impl Into<String>,
        command: I,
    ) -> Self {
        Self {
            command: command.into_iter().map(S::into).collect(),
            container_name: container_name.into(),
        }
    }
}

impl From<DockerExecCommand> for Command {
    fn from(docker_exec_command: DockerExecCommand) -> Self {
        let mut command = Command::new("docker");
        command
            .args(["exec", &docker_exec_command.container_name])
            .args(docker_exec_command.command);
        command
    }
}

/// Represents a `docker logs` command.
#[derive(Clone, Debug)]
pub(crate) struct DockerLogsCommand {
    container_name: String,
    follow: bool,
}

impl DockerLogsCommand {
    pub fn new(container_name: impl Into<String>) -> Self {
        Self {
            container_name: container_name.into(),
            follow: false,
        }
    }

    pub fn follow(&mut self, follow: bool) -> &mut Self {
        self.follow = follow;
        self
    }
}

impl From<DockerLogsCommand> for Command {
    fn from(docker_logs_command: DockerLogsCommand) -> Self {
        let mut command = Command::new("docker");
        command.args(["logs", &docker_logs_command.container_name]);

        if docker_logs_command.follow {
            command.arg(String::from("--follow"));
        }

        command
    }
}

/// Represents a `docker port` command.
#[derive(Clone, Debug)]
pub(crate) struct DockerPortCommand {
    container_name: String,
    port: u16,
}

impl DockerPortCommand {
    pub fn new(container_name: impl Into<String>, port: u16) -> Self {
        Self {
            container_name: container_name.into(),
            port,
        }
    }
}

impl From<DockerPortCommand> for Command {
    fn from(docker_port_command: DockerPortCommand) -> Self {
        let mut command = Command::new("docker");
        command.args([
            "port",
            &docker_port_command.container_name,
            &docker_port_command.port.to_string(),
        ]);
        command
    }
}

/// Represents a `docker rm` command.
#[derive(Clone, Debug)]
pub(crate) struct DockerRemoveContainerCommand {
    container_name: String,
    force: bool,
}

impl DockerRemoveContainerCommand {
    pub fn new(container_name: impl Into<String>) -> Self {
        Self {
            container_name: container_name.into(),
            force: true,
        }
    }
}

impl From<DockerRemoveContainerCommand> for Command {
    fn from(docker_remove_container_command: DockerRemoveContainerCommand) -> Self {
        let mut command = Command::new("docker");
        command.args(["rm", &docker_remove_container_command.container_name]);

        if docker_remove_container_command.force {
            command.arg("--force");
        }

        command
    }
}

/// Represents a `docker rmi` command.
#[derive(Clone, Debug)]
pub(crate) struct DockerRemoveImageCommand {
    force: bool,
    image_name: String,
}

impl DockerRemoveImageCommand {
    pub fn new(container_name: impl Into<String>) -> Self {
        Self {
            force: true,
            image_name: container_name.into(),
        }
    }
}

impl From<DockerRemoveImageCommand> for Command {
    fn from(docker_remove_image_command: DockerRemoveImageCommand) -> Self {
        let mut command = Command::new("docker");
        command.args(["rmi", &docker_remove_image_command.image_name]);

        if docker_remove_image_command.force {
            command.arg("--force");
        }

        command
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn from_docker_run_command_to_command() {
        let mut docker_run_command = DockerRunCommand::new("my-image", "my-container");

        // Default usage
        let command: Command = docker_run_command.clone().into();
        assert_eq!(command.get_program(), "docker");
        assert_eq!(
            command.get_args().collect::<Vec<&OsStr>>(),
            ["run", "--name", "my-container", "my-image"]
        );

        // With optional flag/arguments set
        docker_run_command.command(["echo", "hello"]);
        docker_run_command.detach(true);
        docker_run_command.entrypoint("/usr/bin/bash");
        docker_run_command.env("BAR", "2");
        docker_run_command.env("FOO", "1");
        docker_run_command.expose_port(12345);
        docker_run_command.expose_port(55555);
        docker_run_command.platform("linux/amd64");
        docker_run_command.remove(true);

        let command: Command = docker_run_command.clone().into();
        assert_eq!(
            command.get_args().collect::<Vec<&OsStr>>(),
            [
                "run",
                "--name",
                "my-container",
                "--detach",
                "--rm",
                "--platform",
                "linux/amd64",
                "--entrypoint",
                "/usr/bin/bash",
                "--env",
                "BAR=2",
                "--env",
                "FOO=1",
                "--publish",
                "127.0.0.1::12345",
                "--publish",
                "127.0.0.1::55555",
                "my-image",
                "echo",
                "hello",
            ]
        );
    }

    #[test]
    fn from_docker_exec_command_to_command() {
        let docker_exec_command = DockerExecCommand::new("my-container", ["ps"]);
        let command: Command = docker_exec_command.into();
        assert_eq!(command.get_program(), "docker");
        assert_eq!(
            command.get_args().collect::<Vec<&OsStr>>(),
            ["exec", "my-container", "ps"]
        );
    }

    #[test]
    fn from_docker_logs_command_to_command() {
        let mut docker_logs_command = DockerLogsCommand::new("my-container");

        // Default usage
        let command: Command = docker_logs_command.clone().into();
        assert_eq!(command.get_program(), "docker");
        assert_eq!(
            command.get_args().collect::<Vec<&OsStr>>(),
            ["logs", "my-container"]
        );

        // With optional flag/arguments set
        docker_logs_command.follow(true);

        let command: Command = docker_logs_command.clone().into();
        assert_eq!(
            command.get_args().collect::<Vec<&OsStr>>(),
            ["logs", "my-container", "--follow"]
        );
    }

    #[test]
    fn from_docker_port_command_to_command() {
        let docker_port_command = DockerPortCommand::new("my-container", 12345);
        let command: Command = docker_port_command.into();
        assert_eq!(command.get_program(), "docker");
        assert_eq!(
            command.get_args().collect::<Vec<&OsStr>>(),
            ["port", "my-container", "12345"]
        );
    }

    #[test]
    fn from_docker_remove_container_command_to_command() {
        let docker_remove_container_command = DockerRemoveContainerCommand::new("my-container");
        let command: Command = docker_remove_container_command.into();
        assert_eq!(command.get_program(), "docker");
        assert_eq!(
            command.get_args().collect::<Vec<&OsStr>>(),
            ["rm", "my-container", "--force"]
        );
    }

    #[test]
    fn from_docker_remove_image_command_to_command() {
        let docker_remove_image_command = DockerRemoveImageCommand::new("my-image");
        let command: Command = docker_remove_image_command.into();
        assert_eq!(command.get_program(), "docker");
        assert_eq!(
            command.get_args().collect::<Vec<&OsStr>>(),
            ["rmi", "my-image", "--force"]
        );
    }
}
