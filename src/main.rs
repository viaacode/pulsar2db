#[macro_use]
extern crate serde;
use futures::TryStreamExt;
use pulsar::{
    message::proto::command_subscribe::SubType, message::Payload, Consumer,
    DeserializeMessage, Pulsar, TokioExecutor,
};
use tokio_postgres::NoTls;
use regex::Regex;
use pulsar2db::*;

#[derive(Serialize, Deserialize)]
struct TestData {
    data: String,
}


#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();

    // Get our configuration from the environment
    // The necessary environment variables can be found in the `.env` file
    let config = match envy::from_env::<Config>() {
       Ok(config) => config,
       Err(error) => panic!("{:#?}", error)
    };

    log::debug!("Connecting to Pulsar on {}", &config.pulsar_host);
    let addr = format_pulsar_connection_string(&config);
    let pulsar: Pulsar<_> = Pulsar::builder(addr, TokioExecutor).build().await?;

    // Pulsar consumer
    let mut consumer: Consumer<CloudEvent, _> = pulsar
        .consumer()
        .with_topic_regex(Regex::new(&config.pulsar_topics).unwrap())
        .with_consumer_name(&config.pulsar_consumer_name)
        .with_subscription_type(SubType::Exclusive)
        .with_subscription(&config.pulsar_subscription_name)
        .build()
        .await?;

    // Postgres client: non-blocking
    log::debug!("Connecting to Postgres on {}", &config.postgres_host);
    let connection_string = format_postgres_connection_string(&config);
    let (client, connection) = tokio_postgres::connect(&connection_string, NoTls).await?;

    // The connection object performs the actual communication with the database,
    // so spawn it off to run on its own.
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });
    
    let mut counter = 0usize;
    while let Some(msg) = consumer.try_next().await? {
        consumer.ack(&msg).await?;
        let data = match msg.deserialize() {
            Ok(data) => data,
            Err(e) => {
                log::error!("could not deserialize message: {:?}", e);
                break;
            }
        };

        counter += 1;
        log::info!("got {} messages", counter);
        log::debug!("{:?}", &data);
        log::info!("insert {} into DB", &data.type_field.as_str());
        match data.type_field.as_str() {
            "be.meemoo.sipin.sip.create" => {
                let status: &str = "SIP_CREATED";
                let _rows = client.execute(
                    "INSERT INTO sipin_sips (
                        correlation_id,
                        bag_name,
                        cp_id,
                        local_id,
                        md5_hash_essence_manifest,
                        md5_hash_essence_sidecar,
                        essence_filename,
                        essence_filesize,
                        ingest_host,
                        ingest_path_or_key,
                        bag_filesize,
                        first_event_date,
                        last_event_type,
                        last_event_date,
                        status)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
                            $13, $14, $15)", &[
                        &data.correlation_id.as_str(),
                        &data.data["path"].as_str(),
                        &data.data["cp_id"].as_str(),
                        &data.data["local_id"].as_str(),
                        &data.data["md5_hash_essence_manifest"].as_str(),
                        &data.data["md5_hash_essence_sidecar"].as_str(),
                        &data.data["essence_filename"].as_str(),
                        &data.data["essence_filesize"].as_i64(),
                        &data.data["host"].as_str(),
                        &data.data["path"].as_str(),
                        &data.data["bag_filesize"].as_i64(),
                        &data.time,
                        &data.type_field.as_str(),
                        &data.time,
                        &status,
                    ],
                ).await?;
            },
            "be.meemoo.sipin.bag.transfer" => {
                let status: &str = "BAG_TRANSFERRED_TO_SIPIN";
                let _rows = client.execute(
                    "UPDATE sipin_sips SET last_event_type=$1, last_event_date=$2, status=$3
                    WHERE correlation_id=$4", &[
                        &data.type_field.as_str(),
                        &data.time,
                        &status,
                        &data.correlation_id.as_str(),
                    ],
                ).await?;
            },
            "be.meemoo.sipin.bag.unzip" => {
                let status: &str = "BAG_UNZIPPED";
                let _rows = client.execute(
                    "UPDATE sipin_sips SET last_event_type=$1, last_event_date=$2, status=$3
                    WHERE correlation_id=$4", &[
                        &data.type_field.as_str(),
                        &data.time,
                        &status,
                        &data.correlation_id.as_str(),
                    ],
                ).await?;
            },
            "be.meemoo.sipin.bag.validate" => {
                let status: &str = "BAG_VALIDATED";
                let _rows = client.execute(
                    "UPDATE sipin_sips SET last_event_type=$1, last_event_date=$2, status=$3
                    WHERE correlation_id=$4", &[
                        &data.type_field.as_str(),
                        &data.time,
                        &status,
                        &data.correlation_id.as_str(),
                    ],
                ).await?;
            },
            "be.meemoo.sipin.sip.validate" => {
                let status: &str = "SIP_VALIDATED";
                let _rows = client.execute(
                    "UPDATE sipin_sips SET last_event_type=$1, last_event_date=$2, status=$3
                    WHERE correlation_id=$4", &[
                        &data.type_field.as_str(),
                        &data.time,
                        &status,
                        &data.correlation_id.as_str(),
                    ],
                ).await?;
            },
            "be.meemoo.sipin.aip.create" => {
                let status: &str = "AIP_CREATED";
                let _rows = client.execute(
                    "UPDATE sipin_sips SET last_event_type=$1, last_event_date=$2, status=$3, cp_id=$4, pid=$5
                    WHERE correlation_id=$6", &[
                        &data.type_field.as_str(),
                        &data.time,
                        &status,
                        &data.data["cp_id"].as_str(),
                        &data.data["pid"].as_str(),
                        &data.correlation_id.as_str(),
                    ],
                ).await?;
            },
            "be.meemoo.sipin.aip.transfer" => {
                let status: &str = "AIP_DELIVERED_TO_MAM";
                let _rows = client.execute(
                    "UPDATE sipin_sips SET last_event_type=$1, last_event_date=$2, status=$3
                    WHERE correlation_id=$4", &[
                        &data.type_field.as_str(),
                        &data.time,
                        &status,
                        &data.correlation_id.as_str(),
                    ],
                ).await?;
            },
            _ => {
                log::warn!("Unknown event type: {:#?}", &data.type_field.as_str());
            },
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
