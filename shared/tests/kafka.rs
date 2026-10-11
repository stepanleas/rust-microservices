//! `shared::kafka` against a real Kafka 4 broker (testcontainers, needs Docker).

use shared::kafka::{
    KafkaConsumer, KafkaMessage, KafkaProducer, MessageHandler, RetryPolicy, dead_letter_topic,
    ensure_topics,
};
use shared::shutdown::Shutdown;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::kafka::apache::{KAFKA_PORT, Kafka};

const KAFKA_VERSION: &str = "4.3.1";
const POLL: Duration = Duration::from_millis(100);

async fn start_kafka() -> anyhow::Result<(ContainerAsync<Kafka>, String)> {
    let container = Kafka::default().with_tag(KAFKA_VERSION).start().await?;
    let host = format!(
        "127.0.0.1:{}",
        container.get_host_port_ipv4(KAFKA_PORT).await?
    );
    Ok((container, host))
}

async fn create_topic(host: &str, topic: &str, partitions: i32) -> anyhow::Result<()> {
    let hosts = vec![host.to_string()];
    let topic = topic.to_string();
    tokio::task::spawn_blocking(move || {
        ensure_topics(&hosts, &[&topic], partitions, Duration::from_secs(10))
    })
    .await?
}

fn group() -> String {
    format!("test-{}", uuid::Uuid::new_v4())
}

/// One poll. Like the listener, treats errors as transient: librdkafka reports some through
/// poll while the group coordinator starts up (e.g. while `__consumer_offsets` is created).
fn poll(consumer: &KafkaConsumer) -> Option<KafkaMessage> {
    match consumer.poll(POLL) {
        Ok(message) => message,
        Err(error) => {
            eprintln!("transient poll error: {error}");
            None
        }
    }
}

/// Polls until `count` messages arrived or the deadline passed.
fn receive(consumer: &KafkaConsumer, count: usize) -> anyhow::Result<Vec<KafkaMessage>> {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut messages = vec![];
    while messages.len() < count && Instant::now() < deadline {
        if let Some(message) = poll(consumer) {
            messages.push(message);
        }
    }
    Ok(messages)
}

#[tokio::test(flavor = "multi_thread")]
async fn test_round_trip_keeps_key_payload_and_headers() -> anyhow::Result<()> {
    let (_kafka, host) = start_kafka().await?;
    let topic = "round-trip";
    create_topic(&host, topic, 3).await?;

    let producer_host = host.clone();
    tokio::task::spawn_blocking(move || {
        KafkaProducer::new(vec![producer_host]).send_with_headers(
            topic,
            "order-1",
            b"payload",
            &[("traceparent", b"00-abc-def-01")],
        )
    })
    .await??;

    let message = tokio::task::spawn_blocking(move || -> anyhow::Result<KafkaMessage> {
        let consumer = KafkaConsumer::new(vec![host], group(), &[topic])?;
        receive(&consumer, 1)?
            .pop()
            .ok_or_else(|| anyhow::anyhow!("no message received"))
    })
    .await??;

    assert_eq!(b"order-1".as_slice(), message.key);
    assert_eq!(b"payload".as_slice(), message.payload);
    assert_eq!(
        Some(b"00-abc-def-01".as_slice()),
        message.header("traceparent")
    );
    Ok(())
}

/// Two replicas of a service (same group id) split the partitions: every message is handled
/// exactly once overall, and both replicas get a share.
#[tokio::test(flavor = "multi_thread")]
async fn test_consumers_of_one_group_share_the_partitions() -> anyhow::Result<()> {
    let (_kafka, host) = start_kafka().await?;
    let topic = "shared-load";
    create_topic(&host, topic, 12).await?;

    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let group = group();
        let first = KafkaConsumer::new(vec![host.clone()], group.clone(), &[topic])?;
        let second = KafkaConsumer::new(vec![host.clone()], group, &[topic])?;

        // Polling drives the group protocol; wait until both members own partitions.
        let deadline = Instant::now() + Duration::from_secs(60);
        while Instant::now() < deadline {
            assert!(poll(&first).is_none());
            assert!(poll(&second).is_none());
            let (a, b) = (first.assignment()?, second.assignment()?);
            if !a.is_empty() && !b.is_empty() && a.len() + b.len() == 12 {
                break;
            }
        }
        let first_partitions: HashSet<_> = first.assignment()?.into_iter().collect();
        let second_partitions: HashSet<_> = second.assignment()?.into_iter().collect();
        assert!(first_partitions.is_disjoint(&second_partitions));
        assert_eq!(
            12,
            first_partitions.len() + second_partitions.len(),
            "{first_partitions:?} / {second_partitions:?}"
        );

        let mut producer = KafkaProducer::new(vec![host]);
        for i in 0..120 {
            producer.send(topic, &format!("key-{i}"), format!("{i}").as_bytes())?;
        }

        let mut received: HashMap<String, usize> = HashMap::new();
        let (mut by_first, mut by_second) = (0, 0);
        let deadline = Instant::now() + Duration::from_secs(30);
        while received.len() < 120 && Instant::now() < deadline {
            if let Some(message) = poll(&first) {
                by_first += 1;
                *received
                    .entry(String::from_utf8(message.payload)?)
                    .or_default() += 1;
            }
            if let Some(message) = poll(&second) {
                by_second += 1;
                *received
                    .entry(String::from_utf8(message.payload)?)
                    .or_default() += 1;
            }
        }

        assert_eq!(120, received.len());
        assert!(received.values().all(|count| *count == 1), "duplicates");
        assert!(by_first > 0 && by_second > 0, "{by_first} / {by_second}");
        Ok(())
    })
    .await?
}

/// A restarted consumer resumes after the last stored offset instead of re-reading everything.
#[tokio::test(flavor = "multi_thread")]
async fn test_stored_offsets_are_committed() -> anyhow::Result<()> {
    let (_kafka, host) = start_kafka().await?;
    let topic = "offsets";
    create_topic(&host, topic, 1).await?;

    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let mut producer = KafkaProducer::new(vec![host.clone()]);
        for i in 0..3 {
            producer.send(topic, "key", format!("{i}").as_bytes())?;
        }
        let group = group();

        let consumer = KafkaConsumer::new(vec![host.clone()], group.clone(), &[topic])?;
        let first_two = receive(&consumer, 2)?;
        assert_eq!(2, first_two.len());
        consumer.store_offset(&first_two[1])?;
        consumer.commit()?;
        drop(consumer);

        let consumer = KafkaConsumer::new(vec![host], group, &[topic])?;
        let rest = receive(&consumer, 1)?;
        assert_eq!(b"2".as_slice(), rest[0].payload);
        Ok(())
    })
    .await?
}

struct AlwaysFails;

impl MessageHandler for AlwaysFails {
    fn topics(&self) -> Vec<&'static str> {
        vec!["poison"]
    }

    fn handle(&self, _: &str, _: &[u8]) -> anyhow::Result<()> {
        anyhow::bail!("cannot handle")
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_listener_moves_failing_messages_to_the_dead_letter_topic() -> anyhow::Result<()> {
    let (_kafka, host) = start_kafka().await?;
    create_topic(&host, "poison", 1).await?;
    create_topic(&host, &dead_letter_topic("poison"), 1).await?;

    let listener_host = host.clone();
    std::thread::spawn(move || {
        shared::kafka::listen(
            vec![listener_host],
            group(),
            Arc::new(AlwaysFails),
            RetryPolicy {
                max_attempts: 2,
                initial_backoff: Duration::from_millis(10),
                max_backoff: Duration::from_millis(10),
            },
            &Shutdown::new(),
        )
    });

    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        KafkaProducer::new(vec![host.clone()]).send("poison", "order-9", b"bad")?;

        let consumer = KafkaConsumer::new(vec![host], group(), &["poison.dlq"])?;
        let dead = receive(&consumer, 1)?;
        assert_eq!(1, dead.len(), "nothing in the dead letter topic");
        assert_eq!(b"order-9".as_slice(), dead[0].key);
        assert_eq!(b"bad".as_slice(), dead[0].payload);
        Ok(())
    })
    .await?
}

struct Recording {
    handled: Arc<std::sync::Mutex<Vec<Vec<u8>>>>,
}

impl MessageHandler for Recording {
    fn topics(&self) -> Vec<&'static str> {
        vec!["late"]
    }

    fn handle(&self, _: &str, payload: &[u8]) -> anyhow::Result<()> {
        self.handled.lock().unwrap().push(payload.to_vec());
        Ok(())
    }
}

/// In Kubernetes a service often starts before Kafka. The errors librdkafka queues meanwhile
/// must not keep the listener from consuming once Kafka is up.
#[tokio::test(flavor = "multi_thread")]
async fn test_listener_consumes_once_a_late_kafka_is_up() -> anyhow::Result<()> {
    let port = std::net::TcpListener::bind("127.0.0.1:0")?
        .local_addr()?
        .port();
    let host = format!("127.0.0.1:{port}");
    let handled = Arc::new(std::sync::Mutex::new(vec![]));

    let listener_host = host.clone();
    let recording = Recording {
        handled: handled.clone(),
    };
    std::thread::spawn(move || {
        shared::kafka::listen(
            vec![listener_host],
            group(),
            Arc::new(recording),
            RetryPolicy::default(),
            &Shutdown::new(),
        )
    });

    // Let the listener collect connection errors for a while.
    tokio::time::sleep(Duration::from_secs(15)).await;
    let _kafka = Kafka::default()
        .with_tag(KAFKA_VERSION)
        .with_mapped_port(port, KAFKA_PORT)
        .start()
        .await?;
    create_topic(&host, "late", 1).await?;

    let deadline = Instant::now() + Duration::from_secs(30);
    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        KafkaProducer::new(vec![host]).send("late", "key", b"hello")?;
        while handled.lock().unwrap().is_empty() {
            anyhow::ensure!(Instant::now() < deadline, "listener did not consume");
            std::thread::sleep(Duration::from_millis(200));
        }
        Ok(())
    })
    .await?
}
