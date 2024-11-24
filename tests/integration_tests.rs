use std::sync::Arc;

use alloy_rpc_types::{serde_helpers::JsonStorageKey, BlockId};
use dotenv::dotenv;
use futures_util::FutureExt;
use jsonrpsee::http_client::HttpClientBuilder;
use lazy_static::lazy_static;
use macros_tests::test_with_server;
use oblivious_erc20_state::packets::{
  DBState, MultiQuery, QueryResponseVec, SingleQuery, StatusResponse, StorageResult,
};
use rand::seq::SliceRandom;
use reqwest::Client;
use reth_revm::primitives::B256;
use serde_json::json;
use serial_test::serial;
use verified_contract_state::{
  solidity_memory::get_mapping_address, storage_utils::get_storage_proof, utils::b256,
};

lazy_static! {
  static ref TEST_ADDRESSES: [B256; 6] = [
    B256::from_slice(&[0u8; 32]),
    "0x0000000000000000000000008b41783ad99fcbeb8d575fa7a7b5a04fa0b8d80b".parse::<B256>().unwrap(),
    "0x000000000000000000000000ca06411bd7a7296d7dbdd0050dfc846e95febeb7".parse::<B256>().unwrap(),
    "0x00000000000000000000000036928500bc1dcd7af6a2b4008875cc336b927d57".parse::<B256>().unwrap(),
    "0x000000000000000000000000c6cde7c39eb2f0f0095f41570af89efc2c1ea828".parse::<B256>().unwrap(),
    "0x000000000000000000000000B8F226DDB7BC672E27DFFB67E4ADABFA8C0DFA08".parse::<B256>().unwrap(),
  ];
  static ref STATUS_URL: String = "status".to_string();
  static ref STORAGE_AT_URL: String = "storage_at".to_string();
  static ref STORAGES_AT_URL: String = "storage_at_mq".to_string();
  static ref BASE_URL: String = "http://127.0.0.1:3000".to_string();
  static ref FULL_STATUS_URL: String = format!("{}/{}", *BASE_URL, *STATUS_URL);
  static ref FULL_STORAGE_AT_URL: String = format!("{}/{}", *BASE_URL, *STORAGE_AT_URL);
  static ref FULL_STORAGES_AT_URL: String = format!("{}/{}", *BASE_URL, *STORAGES_AT_URL);
}

#[test_with_server]
async fn test_status() {
  let client = Client::new();

  let response = client.get(FULL_STATUS_URL.as_str()).send().await.unwrap();
  assert_eq!(response.status(), 200, "Unexpected status code");

  let response_body: StatusResponse = response.json().await.unwrap();
  assert_eq!(response_body.message, "All good!", "{:?}", response_body);
}

#[test_with_server]
async fn test_single_query() {
  dotenv().ok();

  let geth_url = std::env::var("GETH_URL").unwrap();
  let geth_client = Arc::new(HttpClientBuilder::default().build(geth_url).unwrap());

  let client = Client::new();

  let status: StatusResponse =
    client.get(FULL_STATUS_URL.as_str()).send().await.unwrap().json().await.unwrap();
  let contract_address = status.db_state.contract_address;
  let block_number = status.db_state.block_id;
  let block_id = BlockId::from(block_number);

  for query_addr in TEST_ADDRESSES.iter() {
    for slot in [0u64, 1u64, 2u64] {
      let slot = b256(slot);
      let actual_addr = get_mapping_address(query_addr, &slot);
      let addresses = vec![JsonStorageKey::from(actual_addr)];
      let payload = SingleQuery { addr: actual_addr };
      let expected = get_storage_proof(geth_client.clone(), contract_address, block_id, addresses)
        .await
        .unwrap()
        .1[0]
        .value;
      let expected_b256 = B256::from(expected);

      let response = client.post(FULL_STORAGE_AT_URL.as_str()).json(&payload).send().await.unwrap();
      assert_eq!(response.status(), 200, "Unexpected status code");
      let response_body: QueryResponseVec = response.json().await.unwrap();
      assert_eq!(response_body.resps.len(), 1);
      assert_eq!(response_body.resps[0].addr, actual_addr);
      assert_eq!(response_body.resps[0].value, expected_b256);
    }
  }
}

#[test_with_server]
fn test_multi_query() {
  dotenv().ok();
  let geth_url = std::env::var("GETH_URL").unwrap();
  let geth_client = Arc::new(HttpClientBuilder::default().build(geth_url).unwrap());

  let client = Client::new();

  let status: StatusResponse =
    client.get(FULL_STATUS_URL.as_str()).send().await.unwrap().json().await.unwrap();
  let contract_address = status.db_state.contract_address;
  let block_number = status.db_state.block_id;
  let block_id = BlockId::from(block_number);
  let mut full_samples = vec![];
  for addr in TEST_ADDRESSES.iter() {
    for slot in [0u64, 1u64, 2u64] {
      let slot = b256(slot);
      let actual_addr = get_mapping_address(addr, &slot);
      full_samples.push(actual_addr);
    }
  }
  for _ in 0..50 {
    let sample = full_samples.choose_multiple(&mut rand::thread_rng(), 4).collect::<Vec<_>>();
    let addresses = sample.iter().map(|addr| JsonStorageKey::from(**addr)).collect::<Vec<_>>();
    let payload = MultiQuery { queries: sample.iter().map(|addr| **addr).collect() };
    let response = client.post(FULL_STORAGES_AT_URL.as_str()).json(&payload).send().await.unwrap();
    let expected = get_storage_proof(geth_client.clone(), contract_address, block_id, addresses)
      .await
      .unwrap()
      .1;

    assert_eq!(response.status(), 200, "Unexpected status code");
    let response_body: QueryResponseVec = response.json().await.unwrap();
    assert_eq!(response_body.resps.len(), expected.len());
    for (i, expected) in expected.iter().enumerate() {
      assert_eq!(response_body.resps[i].addr, expected.key.as_b256());
      assert_eq!(response_body.resps[i].value, B256::from(expected.value));
    }
  }
}
