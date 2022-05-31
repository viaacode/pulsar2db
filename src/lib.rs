use std::str;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use pulsar::{message::Payload, DeserializeMessage};

#[derive(Deserialize, Debug)]
pub struct Config {
    // Pulsar
    #[serde(default="default_user_pass")]
    pub pulsar_user: String,
    #[serde(default="default_user_pass")]
    pub pulsar_passwd: String,
    #[serde(default="default_host")]
    pub pulsar_host: String,
    #[serde(default="default_port")]
    pub pulsar_port: String,
    pub pulsar_topics: String,
    #[serde(default="default_consumer_name")]
    pub pulsar_consumer_name: String,
    #[serde(default="default_subscription_name")]
    pub pulsar_subscription_name: String,
    // Postgres
    #[serde(default="default_user_pass")]
    pub postgres_user: String,
    #[serde(default="default_user_pass")]
    pub postgres_passwd: String,
    #[serde(default="default_host")]
    pub postgres_host: String,
    #[serde(default="default_database")]
    pub postgres_database: String,
}

fn default_user_pass() -> String  {
  String::from("admin")
}

fn default_host() -> String  {
  String::from("localhost")
}

fn default_port() -> String  {
  String::from("5672")
}

fn default_database() -> String  {
  String::from("postgres")
}

fn default_consumer_name() -> String  {
  String::from("pulsar2db")
}

fn default_subscription_name() -> String  {
  String::from("pulsar2db_subscription")
}

// TODO: These 2 conn string fn's can become methods on their respective configs
pub fn format_pulsar_connection_string(config: &Config) -> String {
    format!("pulsar://{}:{}",
        config.pulsar_host,
        config.pulsar_port
    )
}

pub fn format_postgres_connection_string(config: &Config) -> String {
    format!("postgresql://{}:{}@{}/{}",
        config.postgres_user,
        config.postgres_passwd,
        config.postgres_host,
        config.postgres_database
    )
}

#[derive(Serialize, Deserialize, Debug)]
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
    //~ #[serde(skip_deserializing)]
    //~ pub data: Data,
    pub data: ::serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    pub path: String,
    pub host: String,
    pub outcome: String,
    pub message: String,
}

impl DeserializeMessage for CloudEvent {
    type Output = Result<CloudEvent, serde_json::Error>;

    fn deserialize_message(payload: &Payload) -> Self::Output {
        serde_json::from_slice(&payload.data)
    }
}
