use bollard::models::{PortBinding, PortMap};
use std::collections::HashMap;
use std::net::{AddrParseError, SocketAddr};
use std::num::ParseIntError;

pub(crate) fn parse_port_map(
    port_map: &bollard::models::PortMap,
) -> Result<HashMap<u16, SocketAddr>, PortMapParseError> {
    port_map.iter().map(parse_port_map_entry).collect()
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

#[derive(Debug, Eq, PartialEq)]
pub enum PortMapParseError {
    UnexpectedOrMissingContainerPortSuffix,
    ContainerPortParseError(ParseIntError),
    NoBindingForPort(u16),
    MissingHostIpAddress,
    MissingHostPort,
    HostIpAddressParseError(AddrParseError),
    HostPortParseError(ParseIntError),
}

pub(crate) fn simple_tcp_port_map(ports: &[u16]) -> PortMap {
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
        .collect::<PortMap>()
}

#[cfg(test)]
mod test {
    use super::*;
    use bollard::models::PortMap;

    #[test]
    fn simple() {
        let mut port_map: PortMap = HashMap::new();
        port_map.insert(
            String::from("12345/tcp"),
            Some(vec![PortBinding {
                host_ip: Some(String::from("0.0.0.0")),
                host_port: Some(String::from("57233")),
            }]),
        );

        port_map.insert(
            String::from("80/tcp"),
            Some(vec![PortBinding {
                host_ip: Some(String::from("127.0.0.1")),
                host_port: Some(String::from("51039")),
            }]),
        );

        let mut expected_result: HashMap<u16, SocketAddr> = HashMap::new();
        expected_result.insert(12345, "0.0.0.0:57233".parse().unwrap());
        expected_result.insert(80, "127.0.0.1:51039".parse().unwrap());

        assert_eq!(parse_port_map(&port_map).unwrap(), expected_result)
    }

    #[test]
    fn multiple_bindings() {
        let mut port_map: PortMap = HashMap::new();
        port_map.insert(
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
        );

        let mut expected_result: HashMap<u16, SocketAddr> = HashMap::new();
        expected_result.insert(12345, "0.0.0.0:57233".parse().unwrap());

        assert_eq!(parse_port_map(&port_map).unwrap(), expected_result)
    }

    #[test]
    fn non_tcp_port_binding() {
        let mut port_map: PortMap = HashMap::new();
        port_map.insert(
            String::from("12345/udp"),
            Some(vec![PortBinding {
                host_ip: Some(String::from("0.0.0.0")),
                host_port: Some(String::from("57233")),
            }]),
        );

        assert_eq!(
            parse_port_map(&port_map),
            Err(PortMapParseError::UnexpectedOrMissingContainerPortSuffix)
        )
    }
}
