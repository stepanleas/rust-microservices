use rdkafka::ClientConfig;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::client::DefaultClientContext;
use rdkafka::types::RDKafkaErrorCode;
use std::time::Duration;

/// Creates the topics that don't exist yet (replication factor 1). Blocking.
///
/// The services rely on topic auto-creation or on `kafka-init`/Strimzi; this is for tests and
/// tools that need a topic with a known partition count before producing to it.
pub fn ensure_topics(
    hosts: &[String],
    topics: &[&str],
    partitions: i32,
    timeout: Duration,
) -> anyhow::Result<()> {
    let admin: AdminClient<DefaultClientContext> = ClientConfig::new()
        .set("bootstrap.servers", hosts.join(","))
        .create()?;
    let new_topics: Vec<NewTopic> = topics
        .iter()
        .map(|topic| NewTopic::new(topic, partitions, TopicReplication::Fixed(1)))
        .collect();
    let options = AdminOptions::new()
        .operation_timeout(Some(timeout))
        .request_timeout(Some(timeout));

    let results = futures::executor::block_on(admin.create_topics(&new_topics, &options))?;
    for result in results {
        match result {
            Ok(_) | Err((_, RDKafkaErrorCode::TopicAlreadyExists)) => {}
            Err((topic, error)) => anyhow::bail!("Failed to create topic {topic}: {error}"),
        }
    }
    Ok(())
}
