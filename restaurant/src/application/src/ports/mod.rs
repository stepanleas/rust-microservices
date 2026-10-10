/// Driving ports: what the messaging adapter calls. (HTTP controllers call the use cases directly.)
pub mod r#in;
/// Driven ports: what the use cases need from the infrastructure.
pub mod out;
