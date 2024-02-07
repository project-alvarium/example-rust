use std::net::SocketAddr;
use std::sync::{Arc};
use hyper::{Body, header, Request, Response, Server, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use streams::{Address, User};
use streams::id::{Permissioned, Psk};
use streams::transport::utangle::Client;
use crate::BASE_TOPIC;
use std::str::FromStr;
use tokio::sync::Mutex;


type GenericError = Box<dyn std::error::Error + Send + Sync>;

/// Starts an http server for receiving subscription requests
pub async fn start(user: Arc<Mutex<User<Client>>>) -> Result<(), GenericError> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8900));

    let service = make_service_fn(move |_| {
        let user = user.clone();
        async {
            Ok::<_, GenericError>(service_fn(move |req| {
                handle_request(req, user.clone())
            }))
        }
    });

    let server = Server::bind(&addr).serve(service);
    server.await?;

    Ok(())
}

// Handler to manage the get_announcement_id() and subscribe() api calls
async fn handle_request(req: Request<Body>, user: Arc<Mutex<User<Client>>>) -> Result<Response<Body>, GenericError> {
    match req.uri().path() {
        // Returns the announcement id of the stream created by the publisher instance
        "/get_announcement_id" => {
            // Generate a new announcement ID and send it through the channel.
            let announcement_id = user.lock().await.stream_address().unwrap();

            #[derive(serde::Serialize, serde::Deserialize)]
            struct AnnouncementResponse {
                announcement_id: String
            }

            let announcement = serde_json::to_vec(&AnnouncementResponse {
                announcement_id: announcement_id.to_string()
            }).unwrap();

            // Respond with the announcement ID.
            let response = Response::new(Body::from(announcement));
            Ok(response)
        },
        // Adds subscriber to publisher
        "/subscribe" => subscribe_response(req, user).await,
        _ => {
            // Respond with a 404 Not Found for other paths.
            let response = Response::builder()
                .status(404)
                .body(Body::empty())
                .unwrap();
            Ok(response)
        }
    }
}

// Attempts to unpack a subscription request, if successful the subscription message will be
// retrieved from the distributed network, and once processed, a new branch will be created for the
pub async fn subscribe_response(
    req: Request<Body>,
    user: Arc<Mutex<User<Client>>>,
) -> Result<Response<Body>, GenericError> {
    let data = hyper::body::to_bytes(req.into_body()).await?;

    let response;
    let json_data: serde_json::Result<SubscriptionRequest> = serde_json::from_slice(&data);
    match json_data {
        Ok(sub_req) => {
            let mut user = user.lock().await;
            let sub_address = Address::from_str(&sub_req.address).unwrap();
            let msg = user.receive_message(sub_address).await.unwrap();
            let sub = msg.as_subscription().unwrap();
            let _ = user.new_branch(BASE_TOPIC, sub_req.topic.as_str()).await;
            let psk = Psk::from_seed("A pre shared key seed").to_pskid();
            let keyload = user.send_keyload(
                sub_req.topic.as_str(),
                vec![Permissioned::Admin(sub.subscriber_identifier.clone())],
                vec![psk]
            )
                .await
                .unwrap();

            response = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .header("Access-Control-Allow-Origin", "*")
                .body(Body::from("Subscription processed, keyload link: ".to_owned() + &keyload.address().to_string()))
                .unwrap();
        },
        Err(e) => {
            dbg!("Error in formatting: {:?}", e);
            response = Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .header("Access-Control-Allow-Origin", "*")
                .body(Body::from("Malformed json request"))
                .unwrap();
        }
    }

    Ok(response)
}


// Subscription Request as
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct SubscriptionRequest {
    address: String,
    identifier: String,
    #[serde(rename="idType")]
    id_type: u8,
    topic: String,
}