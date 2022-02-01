use bollard::container::Config;
use bollard::models::{HostConfig, PortBinding, PortMap};
use std::collections::HashMap;
use std::net::{AddrParseError, SocketAddr};
use std::num::ParseIntError;

/// Parse a Bollard [`PortMap`](bollard::models::PortMap) into a simpler, better typed, form.
pub(crate) fn parse_port_map(
    port_map: &bollard::models::PortMap,
) -> Result<HashMap<u16, SocketAddr>, PortMapParseError> {
    port_map.iter().map(parse_port_map_entry).collect()
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum PortMapParseError {
    UnexpectedOrMissingContainerPortSuffix,
    ContainerPortParseError(ParseIntError),
    NoBindingForPort(u16),
    MissingHostIpAddress,
    MissingHostPort,
    HostIpAddressParseError(AddrParseError),
    HostPortParseError(ParseIntError),
}

fn parse_port_map_entry(
    port_mapping: (&String, &Option<Vec<PortBinding>>),
) -> Result<(u16, SocketAddr), PortMapParseError> {
    let (container_port, mappings) = port_mapping;

    let container_port = container_port
        .strip_suffix("/tcp")
        .ok_or(PortMapParseError::UnexpectedOrMissingContainerPortSuffix)
        .and_then(|port_string| {
            port_string
                .parse()
                .map_err(PortMapParseError::ContainerPortParseError)
        })?;

    let port_binding = mappings
        .clone()
        .unwrap_or_default()
        .first()
        .cloned()
        .ok_or(PortMapParseError::NoBindingForPort(container_port))?;

    let host_address = port_binding
        .host_ip
        .ok_or(PortMapParseError::MissingHostIpAddress)
        .and_then(|host_ip| {
            host_ip
                .parse()
                .map_err(PortMapParseError::HostIpAddressParseError)
        })?;

    let host_port = port_binding
        .host_port
        .ok_or(PortMapParseError::MissingHostPort)
        .and_then(|host_port| {
            host_port
                .parse()
                .map_err(PortMapParseError::HostPortParseError)
        })?;

    Ok((container_port, SocketAddr::new(host_address, host_port)))
}

/// Create a new Bollard container config with the given exposed ports.
///
/// The exposed ports will be forwarded to random ports on the host.
pub(crate) fn port_mapped_container_config(ports: &[u16]) -> bollard::container::Config<String> {
    Config {
        host_config: Some(HostConfig {
            port_bindings: Some(
                ports
                    .iter()
                    .map(|port| {
                        (
                            format!("{}/tcp", port),
                            Some(vec![PortBinding {
                                host_ip: None,
                                host_port: None,
                            }]),
                        )
                    })
                    .collect::<PortMap>(),
            ),
            ..HostConfig::default()
        }),
        exposed_ports: Some(
            ports
                .iter()
                .map(|port| {
                    (
                        format!("{}/tcp", port),
                        #[allow(clippy::zero_sized_map_values)]
                        HashMap::new(), // Bollard requires this zero sized value map,
                    )
                })
                .collect(),
        ),
        ..Config::default()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bollard::models::PortMap;

    #[test]
    fn parse_port_map_simple() {
        let port_map: PortMap = HashMap::from([
            (
                String::from("12345/tcp"),
                Some(vec![PortBinding {
                    host_ip: Some(String::from("0.0.0.0")),
                    host_port: Some(String::from("57233")),
                }]),
            ),
            (
                String::from("80/tcp"),
                Some(vec![PortBinding {
                    host_ip: Some(String::from("127.0.0.1")),
                    host_port: Some(String::from("51039")),
                }]),
            ),
        ]);

        let expected_result: HashMap<u16, SocketAddr> = HashMap::from([
            (12345, "0.0.0.0:57233".parse().unwrap()),
            (80, "127.0.0.1:51039".parse().unwrap()),
        ]);

        assert_eq!(parse_port_map(&port_map).unwrap(), expected_result);
    }

    #[test]
    fn parse_port_map_multiple_bindings() {
        let port_map: PortMap = HashMap::from([(
            String::from("12345/tcp"),
            Some(vec![
                PortBinding {
                    host_ip: Some(String::from("0.0.0.0")),
                    host_port: Some(String::from("57233")),
                },
                PortBinding {
                    host_ip: Some(String::from("0.0.0.0")),
                    host_port: Some(String::from("57234")),
                },
            ]),
        )]);

        let expected_result: HashMap<u16, SocketAddr> =
            HashMap::from([(12345, "0.0.0.0:57233".parse().unwrap())]);

        assert_eq!(parse_port_map(&port_map).unwrap(), expected_result);
    }

    #[test]
    fn parse_port_map_non_tcp_port_binding() {
        let port_map: PortMap = HashMap::from([(
            String::from("12345/udp"),
            Some(vec![PortBinding {
                host_ip: Some(String::from("0.0.0.0")),
                host_port: Some(String::from("57233")),
            }]),
        )]);

        assert_eq!(
            parse_port_map(&port_map),
            Err(PortMapParseError::UnexpectedOrMissingContainerPortSuffix)
        );
    }

    #[test]
    fn port_mapped_container_config_simple() {
        let config = port_mapped_container_config(&[80, 443, 22]);

        assert_eq!(
            config.exposed_ports,
            Some(HashMap::from([
                (
                    String::from("80/tcp"),
                    #[allow(clippy::zero_sized_map_values)]
                    HashMap::new()
                ),
                (
                    String::from("443/tcp"),
                    #[allow(clippy::zero_sized_map_values)]
                    HashMap::new()
                ),
                (
                    String::from("22/tcp"),
                    #[allow(clippy::zero_sized_map_values)]
                    HashMap::new()
                )
            ]))
        );

        assert_eq!(
            config.host_config.unwrap().port_bindings,
            Some(HashMap::from([
                (
                    String::from("80/tcp"),
                    Some(vec![PortBinding {
                        host_ip: None,
                        host_port: None,
                    }]),
                ),
                (
                    String::from("443/tcp"),
                    Some(vec![PortBinding {
                        host_ip: None,
                        host_port: None,
                    }]),
                ),
                (
                    String::from("22/tcp"),
                    Some(vec![PortBinding {
                        host_ip: None,
                        host_port: None,
                    }]),
                )
            ]))
        );
    }
}
