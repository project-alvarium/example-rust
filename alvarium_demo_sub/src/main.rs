#[macro_use] extern crate rocket;

pub mod errors;
pub mod logger;

use std::collections::HashSet;
use std::io::Write;
use std::str::FromStr;
use std::sync::{Arc};
use alvarium_annotator::{Annotation, AnnotationList, HashProvider, MessageWrapper};
use alvarium_sdk_rust::providers::hash_provider::Sha256Provider;
use base64::Engine;
use rocket::tokio::{self, sync::Mutex, time::Duration};
use reqwest;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Build, Rocket, State};
use rocket_dyn_templates::{context, Template};
use serde::{Deserialize, Serialize};
use streams::{Address, Message, User};
use streams::id::{Ed25519, Psk};
use streams::transport::utangle::Client;

const NODE_URL: &'static str = "http://nodes.02.demia-testing-domain.com:14102";


// Define your Reading and Annotation structs
#[derive(Clone, Serialize, Deserialize)]
struct Reading(String);

#[derive(Clone, Serialize, Deserialize)]
struct AnnotationWrap {
    reading_id: String,
    annotation: Annotation
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SensorReading {
    id: String,
    value: u8,
    timestamp: chrono::DateTime<chrono::Utc>
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ReadingWrap {
    id: String,
    address: String,
    reading: SensorReading
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DashboardContext {
    sensors: Vec<SensorDashboardContext>
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SensorDashboardContext {
    id: String,
    total: usize,
    avgcf: String,
    readings: Vec<ReadingDashboardContext>
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ReadingDashboardContext {
    id: String,
    address: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    value: u8,
    annotations: Vec<Annotation>,
    score: f32,
}

impl DashboardContext {
    fn new(messages: Vec<ReadingWrap>, annotations: Vec<AnnotationWrap>) -> Self {
        let mut ids = HashSet::new();
        messages.iter().for_each(|m| {
            ids.insert(m.reading.id.clone());
        });

        debug!("Ids: {:?}", ids);

        let mut sensors = Vec::new();
        for id in ids {
            let mut readings = Vec::new();
            messages.iter()
                .filter(|m| m.reading.id.eq(&id))
                .for_each(|reading| {
                    let mut score = 0_f32;
                    let annotations = annotations.iter()
                        .filter(|ann| ann.reading_id.eq(&reading.id))
                        .map(|ann| {
                            if ann.annotation.is_satisfied {
                                match ann.annotation.kind.kind() {
                                    "threshold" => score += 3.33333/10.0,
                                    "source" => score += 1.33333/10.0,
                                    "tls" => score += 2.00000/10.0,
                                    "pki" => score += 3.333333/10.0,
                                    _ => ()
                                }
                            }
                            ann.annotation.clone()
                        })
                        .collect::<Vec<Annotation>>();
                    debug!("Annotations for {}: {}", id, annotations.len());
                    let mut id = reading.id.clone();
                    id.truncate(10);



                    readings.push(ReadingDashboardContext {
                        id,
                        timestamp: reading.reading.timestamp,
                        value: reading.reading.value,
                        address: reading.address.clone(),
                        annotations,
                        score
                    })
                });
            info!("Readings: {}", readings.len());
            readings.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            let mut avgcf = 0_f32;
            let total = readings.iter().map(|r| avgcf += r.score ).count();
            let avgcf = ((avgcf/total as f32) * 1000.0).round() / 1000.0;
            debug!("Avgcf: {}", avgcf);
            debug!("Total: {}", total);

            readings.truncate(75);
            sensors.push(SensorDashboardContext {
                id,
                readings,
                total,
                avgcf: format!("{}", avgcf * 100.0),
            })
        }

        sensors.sort_by(|a, b| a.id.cmp(&b.id));

        DashboardContext {
            sensors
        }
    }
}



#[get("/")]
async fn index(state: &State<AppState>) -> Template {
    let messages = state.messages.lock().await.clone();
    let annotations = state.annotations.lock().await.clone();

    info!("Messages: {}, Annotations: {}", messages.len(), annotations.len());

    Template::render("index", DashboardContext::new(messages, annotations))
}


#[rocket::launch]
async fn rocket() -> _ {
    logger::init().unwrap();

    let psk = Psk::from_seed("A pre shared key seed");
    let announcement = fetch_user_announcement().await;
    let mut readings: Vec<ReadingWrap> = Vec::new();
    let mut annotations: Vec<AnnotationWrap> = Vec::new();

    let user = match std::fs::read("user.bin") {
        Err(_) => {
            let mut user = streams::User::builder()
                .with_identity(Ed25519::from_seed("Subscriber Seed"))
                .with_transport(Client::new(NODE_URL))
                .with_psk(psk.to_pskid(), psk)
                .build();
            user.receive_message(announcement).await.unwrap();
            let backup = user.backup("password").await.unwrap();
            std::fs::write("user.bin", backup).unwrap();
            user
        },
        Ok(bytes) => {
            let user = User::restore(bytes, "password", Client::new(NODE_URL)).await.unwrap();

            let reading_bytes = std::fs::read("readings.bin").unwrap();
            readings = serde_json::from_slice(&reading_bytes).unwrap();
            let annotation_bytes = std::fs::read("annotations.bin").unwrap();
            annotations = serde_json::from_slice(&annotation_bytes).unwrap();

            user
        }
    };

    info!("User received announcement");

    rocket::build()
        .attach(Template::fairing())
        .attach(MessageFetcher)
        .manage(AppState {
            messages: Arc::new(Mutex::new(readings)),
            annotations: Arc::new(Mutex::new(annotations)),
            user: Arc::new(Mutex::new(user))
        })
        .mount("/", routes![index])
        .mount("/static", rocket::fs::FileServer::from("./static"))
}

struct AppState {
    messages: Arc<Mutex<Vec<ReadingWrap>>>,
    annotations: Arc<Mutex<Vec<AnnotationWrap>>>,
    user: Arc<Mutex<User<Client>>>
}

struct MessageFetcher;

async fn unpack_message(
    messages: Arc<Mutex<Vec<ReadingWrap>>>,
    annotations: Arc<Mutex<Vec<AnnotationWrap>>>,
    msg: Message
) {
    let address = msg.address.to_blake2b();
    if let streams::MessageContent::SignedPacket(msg) = msg.content {
        match serde_json::from_slice::<SensorReading>(&msg.masked_payload) {
            Ok(reading) => {
                let id = Sha256Provider::new().derive(&msg.masked_payload);
                info!("Found reading: {}", id);
                let reading = ReadingWrap { id, reading, address: hex::encode(address)  };

                messages.lock().await.push(reading)
            },
            Err(_) => {
                match serde_json::from_slice::<MessageWrapper>(&msg.masked_payload) {
                    Ok(annotation) => {
                        let content = base64::engine::general_purpose::STANDARD.decode(annotation.content).unwrap();
                        unpack_annotations(annotations, content).await
                    },
                    Err(_) => {
                        error!("Not a known message type")
                    },
                }
            }
        }
    }
}

async fn unpack_annotations(annotations: Arc<Mutex<Vec<AnnotationWrap>>>, content: Vec<u8>) {
    match serde_json::from_slice::<AnnotationList>(&content) {
        Ok(annotation_list) => {
            let mut anns = String::new();
            anns.push_str(&format!("Found annotations for {}: ", annotation_list.items[0].key));
            for annotation in annotation_list.items {
                anns.push_str(&format!("{}  ", annotation.kind.0));
                let reading_id = annotation.key.clone();
                let annotation = AnnotationWrap { reading_id, annotation };

                annotations.lock().await.push(annotation)
            }
            info!("{}", anns);
        },
        Err(_) => error!("failed to parse annotation list")
    }
}


#[rocket::async_trait]
impl Fairing for MessageFetcher {
    fn info(&self) -> Info {
        Info {
            name: "Message Fetcher",
            kind: Kind::Ignite,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> rocket::fairing::Result {
        let state = rocket.state::<AppState>().unwrap();
        let messages = state.messages.clone();
        let annotations = state.annotations.clone();
        let user = state.user.clone();

        tokio::spawn(async move {
            loop {
                let mut user = user.lock().await;
                if let Some(msg) = user.messages().next().await {
                    if let Ok(msg) = msg {
                        unpack_message(messages.clone(), annotations.clone(), msg).await
                    }
                } else {
                    tokio::time::sleep(Duration::from_secs(1)).await
                };

                let backup = user.backup("password").await.unwrap();
                std::fs::write("user.bin", backup).unwrap();

                let readings = messages.lock().await.clone().into_iter().collect::<Vec<ReadingWrap>>();
                std::fs::write("readings.bin",serde_json::to_vec(&readings).unwrap()).unwrap();
                let annotations = annotations.lock().await.clone().into_iter().collect::<Vec<AnnotationWrap>>();
                std::fs::write("annotations.bin",serde_json::to_vec(&annotations).unwrap()).unwrap();

            }
        });

        Ok(rocket)
    }
}


async fn fetch_user_announcement() -> Address {
    let response = reqwest::get("http://localhost:8900/get_announcement_id")
        .await
        .map_err(|_| "Failed to query the provider".to_string())
        .unwrap()
        .text()
        .await
        .map_err(|_| "Failed to get response text".to_string())
        .unwrap();

    #[derive(serde::Serialize, serde::Deserialize)]
    struct AnnouncementResponse {
        announcement_id: String
    }

    let address = serde_json::from_str::<AnnouncementResponse>(&response).unwrap();
    Address::from_str(&address.announcement_id).unwrap()
}