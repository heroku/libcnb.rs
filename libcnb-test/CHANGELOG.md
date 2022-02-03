# Changelog

## [Unreleased]

- Use the `DOCKER_HOST` environment variable to determine the Docker connection strategy, adding support for HTTPS 
connections in addition to local UNIX sockets. This enables using `libcnb-test` in more complex setups like on CircleCI 
where the Docker deamon is on a remote machine.

## [0.1.0] 2022-02-02

- Initial release ([#277](https://github.com/Malax/libcnb.rs/pull/277)).
