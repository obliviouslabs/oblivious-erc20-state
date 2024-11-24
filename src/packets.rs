use reth_revm::primitives::B256;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SingleQuery {
  pub addr: B256
}

#[derive(Serialize, Deserialize)]
pub struct StorageResult {
  pub addr: B256,
  pub value: B256
}




#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct DBState {
  pub block_id: u64,
  pub state_root: B256,
}

impl DBState {
  pub fn new() -> Self {
    Self {
      block_id: 0,
      state_root: B256::default(),
    }
  }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StatusResponse {
	pub message: String,
  pub db_state: DBState
}

#[derive(Serialize, Deserialize)]
pub struct QueryResponseVec {
  pub db_state: DBState,
  pub resps: Vec<StorageResult>
}