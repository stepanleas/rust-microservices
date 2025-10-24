#!/bin/bash
set -e

echo "⏳ Waiting for Kafka to be ready..."

BROKER=${KAFKA_BROKER:-kafka:9092}
TOPIC_DIR="/topics"

create_topic() {
  local topic=$1
  local partitions=$2
  local replication=$3

  echo "📘 Creating topic: $topic"
  kafka-topics.sh --create \
    --if-not-exists \
    --topic "$topic" \
    --bootstrap-server "$BROKER" \
    --partitions "$partitions" \
    --replication-factor "$replication"
}

for file in "$TOPIC_DIR"/*.json; do
  echo "🔍 Processing file: $file"

  # Clean up JSON: remove spaces and newlines
  json=$(tr -d '\n' < "$file" | tr -d '[:space:]')

  # Extract blocks like: {"topic":"customer.created","num_partitions":3,"replication_factor":1}
  echo "$json" | grep -o '{[^}]*}' | while IFS= read -r block; do
    topic=$(echo "$block" | sed -n 's/.*"topic":"\([^"]*\)".*/\1/p')
    partitions=$(echo "$block" | sed -n 's/.*"num_partitions":\([0-9]*\).*/\1/p')
    replication=$(echo "$block" | sed -n 's/.*"replication_factor":\([0-9]*\).*/\1/p')

    if [[ -n "$topic" && -n "$partitions" && -n "$replication" ]]; then
      create_topic "$topic" "$partitions" "$replication"
    else
      echo "⚠️  Skipping invalid block: $block"
    fi
  done
done

echo "✅ Kafka topics created successfully!"
