pub mod packets;

use dotenv::dotenv;
use ordb::ObliviousDB;
use reqwest::Client;
use reth_revm::primitives::{state, Bytes, B256, U256};
// use verified_contract_state::VerifiedContractState;
use axum::{
  extract::{Query, State},
  http::StatusCode,
  routing::{get, post},
  Json, Router,
};
use serde::Serialize;
use std::{
  env,
  net::SocketAddr,
  sync::{Arc, Mutex},
};
use verified_contract_state::{
  instantiations::{
    ierc20::CertainMemoryHandler, shib::SHIBMemoryUpdates, usdt::USDTMemoryUpdates,
    wbtc::WBTCMemoryUpdates,
  },
  tprintln,
  utils::ThreadSafeError,
  StateVerifier,
};

use packets::{
  DBState, MultiQuery, QueryResponseVec, QuotedResponse, SecureHash, SingleQuery, StatusResponse,
  StorageResult, TDXChallengeResponse,
};

#[derive(Clone)]
struct AppState {
  counter: Arc<Mutex<i64>>,
  state_verifier: Arc<Mutex<StateVerifier<CertainMemoryHandler<WBTCMemoryUpdates>>>>,
  db_state: Arc<Mutex<DBState>>,
  db: Arc<Mutex<ObliviousDB>>,
}

impl AppState {
  fn new(geth_url: String) -> Result<Self, ThreadSafeError> {
    Ok(Self {
      counter: Arc::new(Mutex::new(0)),
      state_verifier: Arc::new(Mutex::new(
        StateVerifier::<CertainMemoryHandler<WBTCMemoryUpdates>>::new(&geth_url)?,
      )),
      db_state: Arc::new(Mutex::new(DBState::new())),
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
    let mut db_state = state.db_state.lock().unwrap();
    let mut db = state.db.lock().unwrap();

    state_verifier.initialize().await?;
    tprintln!(
      "State verifier initialized with block_id: {}, storage_root: {}, size: {}",
      state_verifier.block_id,
      state_verifier.storage_root,
      state_verifier.mem.memory.len()
    );
    let mut cnt = 0;
    for (k, v) in state_verifier.mem.memory.iter() {
      db.insert(k.to_vec(), v.to_vec());
      cnt += 1;
      if cnt % 100000 == 0 {
        tprintln!("{} kv pairs inserted", cnt);
        db.print_meta_state();
      }
    }
    tprintln!("DB initialized");
    db_state.block_id = state_verifier.block_id;
    db_state.state_root = state_verifier.storage_root;
    db_state.contract_address = state_verifier.contract_address();
    tprintln!("DB state initialized");
  }
  let app = Router::new()
    .route("/status", get(status_handler))
    .route("/storage_at", post(query_handler))
    .route("/storage_at_mq", post(multiquery_handler))
    .route("/quoted/storage_at", post(query_handler_quoted))
    .route("/quoted/storage_at_mq", post(multiquery_handler_quoted))
    // .route("/update", post(update_handler))
    .with_state(state);

  let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
  println!("Server running at http://{}", listener.local_addr().unwrap());

  axum::serve(listener, app.into_make_service()).await.unwrap();
  Ok(())
}

#[derive(Serialize)]
struct ReportData {
  pub report_data: String,
}

async fn get_tdx_quote(hash: B256) -> Bytes {
  let client = Client::new();
  let report_data = ReportData { report_data: hash.to_string() };

  let response = client
    .get("http+unix://%2Fvar%2Frun%2Ftappd.sock/prpc/Tappd.TdxQuote?json")
    .json(&report_data)
    .send()
    .await;

  match response {
    Ok(response) => {
      let status = response.status();
      if status == StatusCode::OK {
        let body = response.text().await.unwrap();
        return Bytes::from(body);
      }
    }
    Err(e) => {
      eprintln!("Error: {:?}", e);
    }
  }

  Bytes::from("")
}

async fn status_handler(State(state): State<AppState>) -> Json<StatusResponse> {
  let response = StatusResponse {
    message: format!("All good!"),
    db_state: state.db_state.lock().unwrap().clone(),
  };
  Json(response)
}

async fn query_handler_base(state: &AppState, payload: &SingleQuery) -> QueryResponseVec {
  let db_state = state.db_state.lock().unwrap();
  let db = state.db.lock().unwrap();
  let addr = payload.addr;
  let v = db.get(&addr.0);
  let v = match v {
    Some(v) => B256::from_slice(&v),
    None => B256::default(),
  };
  QueryResponseVec {
    db_state: db_state.clone(),
    resps: vec![StorageResult { addr: payload.addr, value: v }],
  }
}

async fn query_handler(
  State(state): State<AppState>,
  Json(payload): Json<SingleQuery>,
) -> Json<QueryResponseVec> {
  let qrv = query_handler_base(&state, &payload).await;
  Json(qrv)
}

async fn query_handler_quoted(
  State(state): State<AppState>,
  Json(payload): Json<SingleQuery>,
) -> Json<QuotedResponse<QueryResponseVec>> {
  let qrv = query_handler_base(&state, &payload).await;
  let hash = qrv.secure_hash();
  let quote = get_tdx_quote(hash).await;

  Json(QuotedResponse { response: qrv, quote })
}

async fn multiquery_handler_base(state: &AppState, payload: &MultiQuery) -> QueryResponseVec {
  let db_state = state.db_state.lock().unwrap();
  let db = state.db.lock().unwrap();
  let mut resps = Vec::new();
  for addr in payload.queries.iter() {
    let v = db.get(&addr.0);
    let v = match v {
      Some(v) => B256::from_slice(&v),
      None => B256::default(),
    };
    resps.push(StorageResult { addr: addr.clone(), value: v });
  }
  QueryResponseVec { db_state: db_state.clone(), resps }
}

async fn multiquery_handler(
  State(state): State<AppState>,
  Json(payload): Json<MultiQuery>,
) -> Json<QueryResponseVec> {
  let qrv = multiquery_handler_base(&state, &payload).await;
  Json(qrv)
}

async fn multiquery_handler_quoted(
  State(state): State<AppState>,
  Json(payload): Json<MultiQuery>,
) -> Json<QuotedResponse<QueryResponseVec>> {
  let qrv = multiquery_handler_base(&state, &payload).await;
  let hash = qrv.secure_hash();
  let quote = get_tdx_quote(hash).await;

  Json(QuotedResponse { response: qrv, quote })
}

async fn update_handler(
  State(state): State<AppState>,
) -> Result<Json<StatusResponse>, ThreadSafeError> {
  let mut state_verifier = state.state_verifier.lock().unwrap();
  state_verifier.update().await?;

  // UNDONE(): update the rest of the state
  //
  let response = StatusResponse {
    message: "Updated".to_string(),
    db_state: state.db_state.lock().unwrap().clone(),
  };
  Ok(Json(response))
}

// Signs a public key along with a tdx signature.
// This can be used to announce that the ssl key used is assigned to the tdx enclave only.
// (iex: the nonce can be the sha256 of the public key)
//
async fn tdx_challenge(
  State(state): State<AppState>,
) -> Result<Json<TDXChallengeResponse>, ThreadSafeError> {
  // UNDONE():
  //
  Ok(Json(TDXChallengeResponse::default()))
}
