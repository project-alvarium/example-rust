mod custom_annotator;
mod mock_sensor;
mod http;
mod errors;
mod logger;

use std::fs;
use std::sync::{Arc};
use tokio::sync::Mutex;
use std::time::Duration;
use alvarium_annotator::{Annotator, SignProvider};
use alvarium_sdk_rust::config::{SdkInfo, Signable, StreamConfig};
use alvarium_sdk_rust::factories::{new_annotator, new_signature_provider};
use alvarium_sdk_rust::providers::stream_provider::DemiaPublisher;
use alvarium_sdk_rust::sdk::SDK;
use crypto::signatures::ed25519::SecretKey;
use streams::id::{Ed25519, Psk};
use streams::transport::utangle::Client;
use streams::User;
use crate::custom_annotator::ThresholdAnnotator;
use crate::mock_sensor::Sensor;

pub const BASE_TOPIC: &'static str = "Base Topic";
pub const SENSOR_TOPIC: &'static str = "Sensor Topic";

#[tokio::main]
async fn main() {
    // Set up logger
    logger::init().unwrap();

    // Get configurations from the static configuration bytes
    let sdk_info: SdkInfo = serde_json::from_slice(CONFIG_BYTES.as_slice()).unwrap();

    // Create a new stream instance, or retrieve an existing one
    let (user, retrieved) = create_stream(&sdk_info).await;
    let stream_author = Arc::new(Mutex::new(user));
    // Start the api server
    tokio::spawn(http::start(stream_author.clone()));
    // Prepare the signature provider
    let signature_provider = new_signature_provider( &sdk_info.signature).unwrap();

    // Create a vector of annotators for the alvarium sdk instance
    let mut annotators: Vec<Box<dyn Annotator<Error = alvarium_sdk_rust::errors::Error> + '_>> = Vec::new();
    for ann in &sdk_info.annotators {
        match ann.0.as_str() {
            // if the annotation type is the custom "threshold" then create a new custom ThresholdAnnotator
            "threshold" => annotators.push(Box::new(ThresholdAnnotator::new(&sdk_info, 180..200).unwrap())),
            // else generate a new annotator from the sdk factory
            _ => annotators.push(new_annotator(ann.clone(), sdk_info.clone()).unwrap()),
        }
    }

    // Create the alvarium SDK instance to annotate sensor data
    let mut sdk: SDK<'_, DemiaPublisher> = SDK::new(sdk_info, annotators.as_mut_slice()).await
        .map_err(|e| {
            // print out any error that might be occurring in SDK generation
            log::error!("Error: {}", e);
            e
        })
        .unwrap();

    // Create 2 mock sensors
    let sensor1 = Sensor("Flow_Sensor_1".to_string());
    let sensor2 = Sensor("Flow_Sensor_2".to_string());

    // If the user instance is new, make sure to create a new branch for each data source
    if !retrieved {
        stream_author.lock().await.new_branch(BASE_TOPIC, sensor1.0.clone()).await.unwrap();
        stream_author.lock().await.new_branch(BASE_TOPIC, sensor2.0.clone()).await.unwrap();
    }

    // Main sensor loop
    loop {
        // Generate readings to send
        let val = sensor1.new_reading();
        let val2 = sensor2.bad_reading();

        let val_bytes = serde_json::to_vec(&val).unwrap();
        let val2_bytes = serde_json::to_vec(&val2).unwrap();

        log::info!("Sensor {} reading: {}", val.id, val.value);
        log::info!("Sensor {} reading: {}", val2.id, val2.value);

        // Send sensor data
        stream_author.lock().await.message()
            .with_topic(val.id.clone())
            .with_payload(val_bytes.as_slice())
            .signed()
            .send()
            .await
            .unwrap();

        stream_author.lock().await.message()
            .with_topic(val2.id.clone())
            .with_payload(val2_bytes.as_slice())
            .signed()
            .send()
            .await
            .unwrap();
        backup(stream_author.clone()).await;

        // Create a signable object for sensor 1 that provides a proper signature
        let sig = signature_provider.sign(&serde_json::to_vec(&val).unwrap()).unwrap();
        let data = Signable::new(serde_json::to_string(&val).unwrap(), sig);

        // Create a signable object for sensor 2 that provides an improper signature
        let sig = hex::encode([0u8; crypto::signatures::ed25519::SIGNATURE_LENGTH]);
        let data2 = Signable::new(serde_json::to_string(&val2).unwrap(), sig);

        // Annotate the messages
        sdk.create(data.to_bytes().as_slice()).await.unwrap();
        sdk.create(data2.to_bytes().as_slice()).await.unwrap();

        backup(stream_author.clone()).await;
        // Wait for 10 seconds and repeat
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}


/// Create or fetch an existing streams instance from backup
async fn create_stream(config: &SdkInfo) -> (User<Client>, bool) {
    if let StreamConfig::DemiaStreams(streams_config) = &config.stream.config {
        let client: Client = Client::new(&streams_config.tangle_node.uri());
        match fs::read("user.bin") {
            Ok(user_bytes) => {
                // User instance was found so restore it and return "true" for restored boolean
                (User::restore(user_bytes, "unique password", client).await.unwrap(), true)
            }
            Err(_) => {
                // User instance was not found, so a new one needs to be made
                let sk = SecretKey::generate().unwrap();
                // Save secret key and public key to files
                fs::write(&config.signature.private_key_info.path, hex::encode(sk.as_slice())).unwrap();
                fs::write(&config.signature.public_key_info.path, hex::encode(sk.public_key().as_slice())).unwrap();

                let psk = Psk::from_seed("A pre shared key seed");
                let mut streams_author = User::builder()
                    .with_transport(client)
                    .with_identity(Ed25519::new(sk))
                    .with_psk(psk.to_pskid(), psk)
                    .lean()
                    .build();
                log::info!("Log set up");
                let announcement = streams_author.create_stream(BASE_TOPIC).await.unwrap();

                log::info!("Stream started: {}", announcement.address());
                (streams_author, false)
            }
        }
    } else {
        panic!("Test configuration is not correct, should be DemiaStreams config")
    }
}

async fn backup(author: Arc<Mutex<User<Client>>>) {
    // Backup the user instance
    let backup = author.lock().await.backup("unique password").await.unwrap();
    fs::write("user.bin", backup).unwrap();
}


#[macro_use]
extern crate lazy_static;
extern crate core;
// Creates a static CONFIG_BYTES value from the ./config.json file if it exists
lazy_static! {
    pub static ref CONFIG_BYTES: Vec<u8> = {
        match std::fs::read("config/config.json") {
            Ok(config_bytes) => config_bytes,
            Err(_) => vec![]
        }
    };
}



