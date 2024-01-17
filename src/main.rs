#[macro_use]
extern crate serde;
use futures::TryStreamExt;
use pulsar::{
    message::proto::command_subscribe::SubType, message::Payload, Consumer,
    DeserializeMessage, Pulsar, TokioExecutor,
};
use tokio_postgres::NoTls;
use pulsar2db::*;
use std::path::Path;

#[derive(Serialize, Deserialize)]
struct TestData {
    data: String,
}

// Store our list of topics as an array of string slices.
// The order of the topics is the natural order of an event.
const TOPICS: [&'static str; 16] = [
    // All topics in de `sipin` namespace.
    "public/sipin/s3.object.create",
    "public/sipin/bag.transfer",
    "public/sipin/bag.unzip",
    "public/sipin/bag.validate",
    "public/sipin/sip.validate.xsd",
    "public/sipin/sip.loadgraph",
    "public/sipin/sip.validate.shacl",
    "public/sipin/mh-sip.create",
    "public/sipin/mh-sip.transfer",
    // All topics in the `default` namespace (legacy SIPIN)
    "public/default/be.meemoo.sipin.sip.create",
    "public/default/be.meemoo.sipin.bag.transfer",
    "public/default/be.meemoo.sipin.bag.unzip",
    "public/default/be.meemoo.sipin.bag.validate",
    "public/default/be.meemoo.sipin.sip.validate",
    "public/default/be.meemoo.sipin.aip.create",
    "public/default/be.meemoo.sipin.aip.transfer",
];

// Helper functions

/// Splits a string by the underscore character and returns the first
/// element.
///
/// Used to derive the "base-pid" from pids for collaterals, eg.:
/// `<pid>_srt`. Returns the original input string if it doesn't
/// contain an underscore.
fn split_pid_by_underscore(pid: &str) -> &str{
    let result: Vec<&str> = pid.split('_').collect();
    result[0]
}

/// Return the filename from a given path
fn filename_from_path(full_path: Option<&str>) -> Option<&str> {
    let path = Path::new(full_path.unwrap());
    let filename = path.file_name().unwrap();
    filename.to_str()
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

    log::info!("Connecting to Pulsar on {}: topics={:?}, subscription_name={}", &config.pulsar_host, &TOPICS, &config.pulsar_subscription_name);
    let addr = format_pulsar_connection_string(&config);
    let pulsar: Pulsar<_> = Pulsar::builder(addr, TokioExecutor).build().await?;

    // Pulsar consumer
    let mut consumer: Consumer<CloudEvent, _> = pulsar
        .consumer()
        .with_topics(&TOPICS)
        .with_consumer_name(&config.pulsar_consumer_name)
        .with_subscription_type(SubType::Exclusive)
        .with_subscription(&config.pulsar_subscription_name)
        .build()
        .await?;

    // Postgres client: non-blocking
    log::info!("Connecting to Postgres on {}", &config.postgres_host);
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
        log::trace!("got {} messages", counter);
        log::debug!("{:?}", &data);
        log::info!("insert into DB: {}, correlation_id: {}", &data.type_field.as_str(), &data.correlation_id.as_str());
        match data.type_field.as_str() {
            // Sipin S3 object create event: sip uploaded to S3
            "persistent://public/sipin/s3.object.create" => {
                let status: &str = "S3_OBJECT_CREATED";
                let res = client.execute(
                    "INSERT INTO sipin_sips (
                        correlation_id,
                        bag_name,
                        ingest_host,
                        ingest_bucket,
                        ingest_path_or_key,
                        first_event_date,
                        last_event_type,
                        last_event_date,
                        status)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)", &[
                        &data.correlation_id.as_str(),
                        &data.subject.as_str(),
                        &data.data["s3_message"]["Records"][0]["s3"]["domain"]["s3-endpoint"].as_str(),
                        &data.data["s3_message"]["Records"][0]["s3"]["bucket"]["name"].as_str(),
                        &data.data["s3_message"]["Records"][0]["s3"]["object"]["key"].as_str(),
                        &data.time,
                        &data.type_field.as_str(),
                        &data.time,
                        &status,
                    ],
                ).await;
                match res {
                    Ok(rows) => log::debug!("Rows created: {}", rows),
                    Err(error) => log::warn!("Problem: {:?}", error),
                };
            },
            // Legacy sip create event: sip created on FTP
            "be.meemoo.sipin.sip.create" => {
                let status: &str = "SIP_CREATED";
                let filename = filename_from_path(data.data["path"].as_str());
                let res = client.execute(
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
                        &filename.unwrap(),
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
                ).await;
                match res {
                    Ok(rows) => log::debug!("Rows created: {}", rows),
                    Err(error) => log::warn!("Problem: {:?}", error),
                };
            },
            // Legacy and new bag transfer events
            "be.meemoo.sipin.bag.transfer" | "persistent://public/default/be.meemoo.sipin.bag.transfer" => {
                let status: &str = "BAG_TRANSFERRED_TO_SIPIN";
                let res = client.execute(
                    "UPDATE sipin_sips SET last_event_type=$1, last_event_date=$2, status=$3
                    WHERE correlation_id=$4", &[
                        &data.type_field.as_str(),
                        &data.time,
                        &status,
                        &data.correlation_id.as_str(),
                    ],
                ).await;
                match res {
                    Ok(rows) => match rows {
                        0 => log::warn!("No rows updated for event {}! correlation_id {} not present?", &data.type_field.as_str(), &data.correlation_id.as_str()),
                        1 => log::debug!("Rows updated for event {}: {}", &data.type_field.as_str(), rows),
                        _ => log::warn!("Rows updated for event {}: {}. More then one record with correlation_id {}", &data.type_field.as_str(), rows, &data.correlation_id.as_str()),
                    },
                    Err(error) => log::warn!("Problem: {:?}", error),
                };
            },
            // Legacy and new bag unzip events
            "be.meemoo.sipin.bag.unzip" | "persistent://public/sipin/bag.unzip" => {
                let status: &str = "BAG_UNZIPPED";
                let res = client.execute(
                    "UPDATE sipin_sips SET last_event_type=$1, last_event_date=$2, status=$3
                    WHERE correlation_id=$4", &[
                        &data.type_field.as_str(),
                        &data.time,
                        &status,
                        &data.correlation_id.as_str(),
                    ],
                ).await;
                match res {
                    Ok(rows) => match rows {
                        0 => log::warn!("No rows updated for event {}! correlation_id {} not present?", &data.type_field.as_str(), &data.correlation_id.as_str()),
                        1 => log::debug!("Rows updated for event {}: {}", &data.type_field.as_str(), rows),
                        _ => log::warn!("Rows updated for event {}: {}. More then one record with correlation_id {}", &data.type_field.as_str(), rows, &data.correlation_id.as_str()),
                    },
                    Err(error) => log::warn!("Problem: {:?}", error),
                };
            },
            // Legacy and new bag validate events
            "be.meemoo.sipin.bag.validate" | "persistent://public/sipin/bag.validate" => {
                let status: &str = "BAG_VALIDATED";
                let res = client.execute(
                    "UPDATE sipin_sips SET last_event_type=$1, last_event_date=$2, status=$3
                    WHERE correlation_id=$4", &[
                        &data.type_field.as_str(),
                        &data.time,
                        &status,
                        &data.correlation_id.as_str(),
                    ],
                ).await;
                match res {
                    Ok(rows) => match rows {
                        0 => log::warn!("No rows updated for event {}! correlation_id {} not present?", &data.type_field.as_str(), &data.correlation_id.as_str()),
                        1 => log::debug!("Rows updated for event {}: {}", &data.type_field.as_str(), rows),
                        _ => log::warn!("Rows updated for event {}: {}. More then one record with correlation_id {}", &data.type_field.as_str(), rows, &data.correlation_id.as_str()),
                    },
                    Err(error) => log::warn!("Problem: {:?}", error),
                };
            },
            // Legacy sip validate event
            "be.meemoo.sipin.sip.validate" => {
                let status: &str = "SIP_VALIDATED";
                let res = client.execute(
                    "UPDATE sipin_sips SET last_event_type=$1, last_event_date=$2, status=$3
                    WHERE correlation_id=$4", &[
                        &data.type_field.as_str(),
                        &data.time,
                        &status,
                        &data.correlation_id.as_str(),
                    ],
                ).await;
                match res {
                    Ok(rows) => match rows {
                        0 => log::warn!("No rows updated for event {}! correlation_id {} not present?", &data.type_field.as_str(), &data.correlation_id.as_str()),
                        1 => log::debug!("Rows updated for event {}: {}", &data.type_field.as_str(), rows),
                        _ => log::warn!("Rows updated for event {}: {}. More then one record with correlation_id {}", &data.type_field.as_str(), rows, &data.correlation_id.as_str()),
                    },
                    Err(error) => log::warn!("Problem: {:?}", error),
                };
            },
            // Legacy aip (mh-sip) create event
            "be.meemoo.sipin.aip.create" => {
                let status: &str = "AIP_CREATED";
                let pid = split_pid_by_underscore(data.data["pid"].as_str().unwrap());
                let res = client.execute(
                    "UPDATE sipin_sips SET last_event_type=$1, last_event_date=$2, status=$3, cp_id=$4, pid=$5
                    WHERE correlation_id=$6", &[
                        &data.type_field.as_str(),
                        &data.time,
                        &status,
                        &data.data["cp_id"].as_str(),
                        &pid,
                        &data.correlation_id.as_str(),
                    ],
                ).await;
                match res {
                    Ok(rows) => match rows {
                        0 => log::warn!("No rows updated for event {}! correlation_id {} not present?", &data.type_field.as_str(), &data.correlation_id.as_str()),
                        1 => log::debug!("Rows updated for event {}: {}", &data.type_field.as_str(), rows),
                        _ => log::warn!("Rows updated for event {}: {}. More then one record with correlation_id {}", &data.type_field.as_str(), rows, &data.correlation_id.as_str()),
                    },
                    Err(error) => log::warn!("Problem: {:?}", error),
                };
            },
            // Sipin mh-sip create event
            "persistent://public/sipin/mh-sip.create" => {
                let status: &str = "MH-SIP_CREATED";
                let pid = split_pid_by_underscore(data.data["pid"].as_str().unwrap());
                let res = client.execute(
                    "UPDATE sipin_sips SET last_event_type=$1, last_event_date=$2, status=$3, cp_id=$4, pid=$5, sip_profile=$6
                    WHERE correlation_id=$7", &[
                        &data.type_field.as_str(),
                        &data.time,
                        &status,
                        &data.data["cp_id"].as_str(),
                        &pid,
                        &data.data["sip_profile"].as_str(),
                        &data.correlation_id.as_str(),
                    ],
                ).await;
                match res {
                    Ok(rows) => match rows {
                        0 => log::warn!("No rows updated for event {}! correlation_id {} not present?", &data.type_field.as_str(), &data.correlation_id.as_str()),
                        1 => log::debug!("Rows updated for event {}: {}", &data.type_field.as_str(), rows),
                        _ => log::warn!("Rows updated for event {}: {}. More then one record with correlation_id {}", &data.type_field.as_str(), rows, &data.correlation_id.as_str()),
                    },
                    Err(error) => log::warn!("Problem: {:?}", error),
                };
            },
            "be.meemoo.sipin.aip.transfer" => {
                let status: &str = "AIP_DELIVERED_TO_MAM";
                let res = client.execute(
                    "UPDATE sipin_sips SET last_event_type=$1, last_event_date=$2, status=$3
                    WHERE correlation_id=$4", &[
                        &data.type_field.as_str(),
                        &data.time,
                        &status,
                        &data.correlation_id.as_str(),
                    ],
                ).await;
                match res {
                    Ok(rows) => match rows {
                        0 => log::warn!("No rows updated for event {}! correlation_id {} not present?", &data.type_field.as_str(), &data.correlation_id.as_str()),
                        1 => log::debug!("Rows updated for event {}: {}", &data.type_field.as_str(), rows),
                        _ => log::warn!("Rows updated for event {}: {}. More then one record with correlation_id {}", &data.type_field.as_str(), rows, &data.correlation_id.as_str()),
                    },
                    Err(error) => log::warn!("Problem: {:?}", error),
                };
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
    fn split_pid_with_underscore() {
        let input_pid: &str = "a1b2c3d4e5_str";
        let result_pid: &str = "a1b2c3d4e5";
        let result: &str = split_pid_by_underscore(&input_pid);
        assert_eq!(&result, &result_pid);
    }
    #[test]
    fn split_pid_without_underscore() {
        let input_pid: &str = "a1b2c3d4e5";
        let result_pid: &str = "a1b2c3d4e5";
        let result: &str = split_pid_by_underscore(&input_pid);
        assert_eq!(&result, &result_pid);
    }
    #[test]
    fn split_pid_with_hyphen() {
        let input_pid: &str = "a1b2c3d4e5-str";
        let result_pid: &str = "a1b2c3d4e5-str";
        let result: &str = split_pid_by_underscore(&input_pid);
        assert_eq!(&result, &result_pid);
    }
    #[test]
    fn filename_from_path_with_filename() {
        let input_path = Some("/home/username/files/directory/filename-123.bag.zip");
        let expected_filename = "filename-123.bag.zip";
        let result = filename_from_path(input_path).unwrap();
        assert_eq!(&result, &expected_filename);
    }
}
