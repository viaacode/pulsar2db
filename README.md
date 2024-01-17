# pulsar2db

Subscribe to Pulsar topics and build final state (or a materialized view) for a
certain object or key.

## Description

This service subscribes to Pulsar topics and builds final state based on
information in all events/messages pertaining to a certain key. In this case,
this key is the `correlation_id`.

This means that all events/messages on topics to which this service subscribes
that carry the same `correlation_id` will be condensed to a single row/record
in the database.

Every Pulsar message is deserialized to a CloudEvent as such:

```rust
#[serde(rename_all = "camelCase")]
pub struct CloudEvent {
    #[serde(rename = "type")]
    pub type_field: String,
    pub source: String,
    #[serde(rename = "correlation_id")]
    pub correlation_id: String,
    #[serde(rename = "content_type")]
    pub content_type: String,
    pub time: DateTime<Utc>,
    pub datacontenttype: String,
    pub outcome: String,
    pub specversion: String,
    pub id: String,
    pub subject: String,
    pub data: ::serde_json::Value,
}
```

Since the `data`-field is deserialized via `::serde_json::Value`, every message
conforming to the CloudEvents structure can be deserialized.

And, since a state building application is, in effect, a tight coupling between
its input (CloudEvents in Pulsar, in this case) and its output (a Postgres
table), a SQL DDL file for the target table is included.

## Prerequisites

- Rust toolchain: see [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install).
- Cargo (should be installed along with the Rust toolchain)

## Usage

- Clone this repository.
- Fill in and source an `.env` file (Example in `.env.example`):
  ```bash
  $ export $(grep -v '^#' .env | xargs)
  ```
- Run with `cargo run`.
