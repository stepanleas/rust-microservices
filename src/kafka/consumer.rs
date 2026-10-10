use kafka::client::{FetchOffset, GroupOffsetStorage, KafkaClient};
use kafka::consumer::{Consumer, MessageSet, MessageSets};

pub struct KafkaConsumer {
    consumer: Consumer,
}

impl KafkaConsumer {
    pub fn new(hosts: Vec<String>, group_id: String, topics: &[&str]) -> anyhow::Result<Self> {
        let topics: Vec<String> = topics.iter().map(|topic| topic.to_string()).collect();

        // Requesting metadata for explicit topics lets the broker auto-create missing ones.
        let mut client = KafkaClient::new(hosts);
        client.load_metadata(&topics)?;

        let mut builder = Consumer::from_client(client)
            .with_group(group_id)
            .with_fallback_offset(FetchOffset::Earliest)
            .with_offset_storage(GroupOffsetStorage::Kafka.into())
            .with_fetch_min_bytes(1);

        for topic in topics {
            builder = builder.with_topic(topic);
        }

        Ok(Self {
            consumer: builder.create()?,
        })
    }

    pub fn poll(&mut self) -> kafka::Result<MessageSets> {
        self.consumer.poll()
    }

    pub fn consume_messageset(&mut self, message_set: MessageSet) -> kafka::Result<()> {
        self.consumer.consume_messageset(message_set)
    }

    pub fn commit_consumed(&mut self) -> kafka::Result<()> {
        self.consumer.commit_consumed()
    }
}
