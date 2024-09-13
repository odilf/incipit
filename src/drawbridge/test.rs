use color_eyre::eyre;
use futures::{SinkExt as _, TryStreamExt as _};
use rand::{distributions::WeightedIndex, prelude::Distribution as _, SeedableRng as _};
use reqwest_websocket::{Message, RequestBuilderExt as _};
use serial_test::serial;

use crate::util;

use super::{mapping::Target, HostMapping};

fn example_mapping() -> impl Fn(&str) -> Target {
    |host| {
        util::test::services()
            .into_iter()
            .find(|service| service.config.host == host)
            .map(|service| Target::port(service.config.port))
            .unwrap_or(Target::Unknown)
    }
}

#[test]
fn mapping_from_config() {
    let config = util::test::example_config();
    let mapping = example_mapping();

    for service in util::test::services() {
        let host = &service.config.host;
        assert_eq!(mapping.route(host), config.route(host));
    }
}

#[tokio::test]
#[serial]
async fn forward_http_request_to_correct_server() -> eyre::Result<()> {
    let (services, _) = util::test::scaffold().await?;

    let response = util::test::fetch("service0.example.com", "/").await?;

    assert_eq!(services[0].server.history.lock().unwrap().len(), 1);
    assert_eq!(services[1].server.history.lock().unwrap().len(), 0);
    assert_eq!(services[2].server.history.lock().unwrap().len(), 0);

    assert_eq!(response, "Hello world");

    Ok(())
}

#[tokio::test]
#[serial]
async fn forward_http_request_preserves_path() -> eyre::Result<()> {
    let (services, _) = util::test::scaffold().await?;

    let path = "/this/is/a/path";

    let response = util::test::fetch("service1.example.com", path).await?;

    assert_eq!(services[0].server.history.lock().unwrap().len(), 0);
    assert_eq!(services[1].server.history.lock().unwrap().len(), 1);
    assert_eq!(services[2].server.history.lock().unwrap().len(), 0);

    assert_eq!(response, format!("Hello path: {path}"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn forward_http_request_preserves_other_data() -> eyre::Result<()> {
    let (services, _) = util::test::scaffold().await?;

    let path = "/this/is/a/path";
    let cookies = "cookie1=1; cookie2=2";
    let headers = vec![
        ("Accept", "application/json"),
        ("User-Agent", "test"),
        ("Referer", "http://example.com"),
    ];

    let header_map = {
        let mut map = reqwest::header::HeaderMap::new();
        for (header, value) in &headers {
            map.insert(*header, value.parse()?);
        }
        map.insert("Cookie", cookies.parse()?);

        map
    };

    let _response = util::test::client::builder("service1.example.com", path)
        .headers(header_map)
        .send()
        .await?;

    let (request, _) = services[1].server.history.lock().unwrap().pop().unwrap();

    assert_eq!(request.uri().path(), path);
    assert_eq!(request.headers().get("Cookie").unwrap(), cookies);
    for (header, value) in headers {
        assert_eq!(request.headers().get(header).unwrap(), value);
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn handle_a_bunch_of_concurrent_requests() -> eyre::Result<()> {
    let (services, _) = util::test::scaffold().await?;

    let weights = [0.01, 0.09, 0.9]; // Lopsided, why not?
    let total_requests = 1000;

    // Seeded rng for consistency
    let mut rng = rand::rngs::StdRng::seed_from_u64(6942);
    let dist = WeightedIndex::new(&weights).unwrap();

    let mut expected_counts = vec![0; 3];

    for _ in 0..total_requests {
        let i = dist.sample(&mut rng);
        expected_counts[i] += 1;

        let host = &services[i].config.host;
        let _response = util::test::fetch(host, "/").await?;
    }

    let actual_counts = services
        .iter()
        .map(|service| service.server.history.lock().unwrap().len())
        .collect::<Vec<_>>();

    for (actual, expected) in actual_counts.iter().zip(&expected_counts) {
        assert_eq!(
            actual, expected,
            "Mismatch in counts (actual: {actual_counts:?}, expected: {expected_counts:?})"
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore = "not implemented"]
async fn forward_websockets() -> eyre::Result<()> {
    let (_services, _) = util::test::scaffold().await?;

    // Creates a GET request, upgrades and sends it.
    let response = reqwest::Client::default()
        .get("ws://localhost/")
        .header("Host", "service0.example.com")
        .upgrade()
        // Prepares the WebSocket upgrade.
        .send()
        .await?;

    // Turns the response into a WebSocket stream.
    let mut websocket = response.into_websocket().await?;

    // The WebSocket implements `Sink<Message>`.
    websocket.send(Message::Text("Hello, World".into())).await?;

    // The WebSocket is also a `TryStream` over `Message`s.
    while let Some(message) = websocket.try_next().await? {
        if let Message::Text(text) = message {
            println!("received: {text}")
        }
    }

    todo!();
}
