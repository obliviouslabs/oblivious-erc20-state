pub mod packets;

use dotenv::dotenv;
use ordb::ObliviousDB;
use reth_revm::primitives::{state, B256, U256};
// use verified_contract_state::VerifiedContractState;
use verified_contract_state::{instantiations::{ierc20::CertainMemoryHandler, usdt::USDTMemoryUpdates, wbtc::WBTCMemoryUpdates, shib::SHIBMemoryUpdates}, utils::ThreadSafeError, StateVerifier};
use axum::{
	extract::{Query, State},
	routing::{get, post},
	Json, Router,
  http::StatusCode
};
use std::{env, net::SocketAddr, sync::{Arc, Mutex}};

use packets::{DBState, SingleQuery, StorageResult, StatusResponse, QueryResponseVec};

#[derive(Clone)]
struct AppState {
	counter: Arc<Mutex<i64>>,
  db_state: Arc<Mutex<DBState>>,
  state_verifier: Arc<Mutex<StateVerifier<CertainMemoryHandler<WBTCMemoryUpdates>>>>,
  db: Arc<Mutex<ObliviousDB>>,
}

impl AppState {
  fn new(geth_url: String) -> Result<Self, ThreadSafeError> {
    Ok(Self {
      counter: Arc::new(Mutex::new(0)),
      db_state: Arc::new(Mutex::new(DBState::new())),
      state_verifier: Arc::new(Mutex::new(StateVerifier::<CertainMemoryHandler<WBTCMemoryUpdates>>::new(&geth_url)?)),
      db: Arc::new(Mutex::new(ObliviousDB::new())),
    })
  }
}


#[tokio::main]
async fn main() -> Result<(), ThreadSafeError> {
  dotenv().ok(); // Load the .env file
	let geth_url = env::var("GETH_URL").expect("Infura URL must be set");
	let state = AppState::new(geth_url)?; //Arc::new(Mutex::new(AppState::default()));
	{
    let mut state_verifier = state.state_verifier.lock().unwrap();
    state_verifier.initialize().await?;
    let mut db = state.db.lock().unwrap();
    for (k, v) in state_verifier.mem.memory.iter() {
      db.insert(k, v);
    }
    let mut db_state = state.db_state.lock().unwrap();
    db_state.block_id = state_verifier.block_id;
    db_state.state_root = state_verifier.storage_root;
  }
	let app = Router::new()
    .route("/status", get(status_handler))
		.route("/storage_at", post(query_handler))
    // .route("/update", post(update_handler))
    .with_state(state);
  
	let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
	println!("Server running at http://{}", listener.local_addr().unwrap());

  axum::serve(listener, app.into_make_service()).await.unwrap();
  Ok(())
}

async fn status_handler(
	State(state): State<AppState>
) -> Json<StatusResponse> {
	let response = StatusResponse {
		message: format!("All good!"),
    db_state: state.db_state.lock().unwrap().clone()
	};
	Json(response)
}

async fn query_handler(
	State(state): State<AppState>,
  Json(payload): Json<SingleQuery>
) -> Json<QueryResponseVec> {
  let db_state = state.db_state.lock().unwrap();
  let db = state.db.lock().unwrap();
  let addr = payload.addr;
  let v = db.get(&addr.0);
  let v = match v {
    Some(v) => B256::from_slice(&v),
    None => B256::default()
  };
  Json(QueryResponseVec{
    db_state: db_state.clone(),
    resps: vec![StorageResult{
      addr: payload.addr,
      value: v
    }]
  })
}

async fn update_handler(
	State(state): State<AppState>,
) -> Result<Json<StatusResponse>, ThreadSafeError> {
  let mut state_verifier = state.state_verifier.lock().unwrap();
  state_verifier.update().await?;
  let response = StatusResponse {
    message: "Updated".to_string(),
    db_state: state.db_state.lock().unwrap().clone()
  };
  Ok(Json(response))
}