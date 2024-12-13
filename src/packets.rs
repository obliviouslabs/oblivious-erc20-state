use reth_revm::primitives::keccak256;
use reth_revm::primitives::{Address, Bytes, B256};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SingleQuery {
  pub addr: B256,
}

#[derive(Serialize, Deserialize)]
pub struct MultiQuery {
  pub queries: Vec<B256>,
}

#[derive(Serialize, Deserialize)]
pub struct StorageResult {
  pub addr: B256,
  pub value: B256,
}

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct DBState {
  pub block_id: u64,
  pub state_root: B256,
  pub contract_address: Address,
}

impl DBState {
  pub fn new() -> Self {
    Self { block_id: 0, state_root: B256::default(), contract_address: Address::default() }
  }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StatusResponse {
  pub message: String,
  pub db_state: DBState,
}

#[derive(Serialize, Deserialize)]
pub struct QueryResponseVec {
  pub db_state: DBState,
  pub resps: Vec<StorageResult>,
}

pub trait SecureHash {
  fn secure_hash(&self) -> B256;
}

#[derive(Serialize, Deserialize)]
pub struct QuotedResponse<T>
where
  T: SecureHash,
{
  pub response: T,
  pub quote: Bytes,
}

impl SecureHash for DBState {
  fn secure_hash(&self) -> B256 {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&self.block_id.to_be_bytes());
    bytes.extend_from_slice("|".as_bytes());
    bytes.extend_from_slice(&self.state_root.as_slice());
    bytes.extend_from_slice("|".as_bytes());
    bytes.extend_from_slice(&self.contract_address.as_slice());
    keccak256(&bytes)
  }
}

impl SecureHash for QueryResponseVec {
  fn secure_hash(&self) -> B256 {
    let v = self.db_state.secure_hash();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&v.as_slice());
    for r in self.resps.iter() {
      bytes.extend_from_slice(&r.addr.as_slice());
      bytes.extend_from_slice(&r.value.as_slice());
    }
    keccak256(&bytes)
  }
}

#[derive(Serialize, Deserialize, Default)]
pub struct TDXChallengeResponse {
  pub public_key: Vec<u8>,
}
