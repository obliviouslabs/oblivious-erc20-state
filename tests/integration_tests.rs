use reqwest::Client;
use reth_revm::primitives::B256;
use serde_json::json;
use macros_tests::test_with_server;
use futures_util::FutureExt;
use serial_test::serial;
use oblivious_erc20_state::packets::{DBState, SingleQuery, StorageResult, StatusResponse, QueryResponseVec};

#[test_with_server]
async fn test_status() {
  println!("Running test_status");
  let client = Client::new();
  let url = "http://127.0.0.1:3000/status";

  let response = client.get(url).send().await.unwrap();
  assert_eq!(response.status(), 200, "Unexpected status code");

  let response_body: StatusResponse = response.json().await.unwrap();
  assert_eq!(
      response_body.message, "All good!",
      "{:?}", response_body
  );
}

#[test_with_server]
async fn test_single_query() {
  println!("Running test_greet_handler");
  let client = Client::new();
  let query_addr = B256::default();
  let payload = SingleQuery { addr: query_addr };

  let url = "http://127.0.0.1:3000/storage_at";

  println!("Sending GET request to: {url}");
  let response = client.post(url).json(&payload).send().await.unwrap();
  assert_eq!(response.status(), 200, "Unexpected status code");
  let response_body: QueryResponseVec = response.json().await.unwrap();
  assert_eq!(response_body.resps.len(), 1);
  assert_eq!(response_body.resps[0].addr, B256::default());
}

