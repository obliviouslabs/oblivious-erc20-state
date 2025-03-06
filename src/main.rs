pub mod packets;

use axum::{
  extract::State,
  http::StatusCode,
  routing::{get, post},
  Json, Router,
};
use dotenv::dotenv;
use http_body_util::{BodyExt, Full};
use hyper::Uri as HyperUri;
use hyper::{body::Buf, Method, Request};
use hyper_util::client::legacy::Client;
use hyperlocal::{UnixClientExt, UnixConnector, Uri};
use ordb::ObliviousDB;
use reth_revm::primitives::{Bytes, B256};
use serde::Serialize;
use serde_json::json;
use std::error::Error;
use std::{env, io::Read, sync::Arc};
use tokio::io;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};

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
  StorageResult,
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
    let mut state_verifier = state.state_verifier.lock().await;
    let mut db_state = state.db_state.lock().await;
    let db = state.db.lock().await;

    state_verifier.initialize().await?;
    tprintln!(
      "State verifier initialized with block_id: {}, storage_root: {}, size: {}",
      state_verifier.block_id,
      state_verifier.storage_root,
      state_verifier.mem.memory.len()
    );
    let mut cnt = 0;
    for (k, v) in state_verifier.pending_updates.iter() {
      db.insert(k.to_vec(), v.to_vec());
      cnt += 1;
      if cnt % 100000 == 0 {
        tprintln!("{} kv pairs inserted", cnt);
        db.print_meta_state();
      }
    }
    state_verifier.pending_updates.clear();
    tprintln!("DB initialized");
    db_state.block_id = state_verifier.block_id;
    db_state.state_root = state_verifier.storage_root;
    db_state.contract_address = state_verifier.contract_address();
    tprintln!("DB state initialized");
  }
  let cors = CorsLayer::new()
    .allow_origin(Any)
    .allow_methods(Any)
    .allow_headers(Any);

  let app = Router::new()
    .route("/status", get(status_handler))
    .route("/update", get(update_handler))
    .route("/storage_at", post(query_handler))
    .route("/storage_at_mq", post(multiquery_handler))
    .route("/quoted/status", get(status_handler_quoted))
    .route("/quoted/storage_at", post(query_handler_quoted))
    .route("/quoted/storage_at_mq", post(multiquery_handler_quoted))
    .with_state(state)
    .layer(cors);

  let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
  println!("Server running at http://{}", listener.local_addr().unwrap());

  axum::serve(listener, app.into_make_service()).await.unwrap();
  Ok(())
}

#[derive(Serialize)]
struct ReportData {
  pub report_data: String,
}

async fn get_tdx_quote_wrapper(hash: B256) -> Result<Bytes, Box<dyn Error>> {
  let report_data = ReportData { report_data: hash.to_string() };
  let uri: HyperUri = Uri::new("/var/run/tappd.sock", "localhost/prpc/Tappd.TdxQuote?json").into();
  // let uri: HyperUri  = Uri::new("/home/xtrm0/ol/erc20-status/tappd-simulator/tappd.sock", "localhost/prpc/Tappd.TdxQuote?json").into();
  let client: Client<UnixConnector, Full<Bytes>> = Client::unix();
  let rd = ReportData { report_data: hash.to_string() };
  let body_rd = json!(rd);
  let request = Request::builder()
    .method(Method::POST)
    .uri(uri)
    .header("Content-Type", "application/json")
    .body(Full::new(Bytes::from(body_rd.to_string())))?;
  let response = client.request(request).await?;

  let status = response.status();
  if status == StatusCode::OK {
    let body = response.collect().await?.aggregate();
    let mut buf = vec![];
    body.reader().read_to_end(&mut buf)?;
    return Ok(Bytes::from(buf));
  } else {
    println!("Error: {:?}", status);
    return Err(Box::new(io::Error::new(io::ErrorKind::Other, "Error")));
  }
}

async fn get_tdx_quote(hash: B256) -> Bytes {
  let res = get_tdx_quote_wrapper(hash).await;
  match res {
    Ok(b) => return b,
    Err(e) => {
      eprintln!("Error: {:?}", e);
    }
  }
  Bytes::from("")
}

async fn status_handler(State(state): State<AppState>) -> Json<StatusResponse> {
  let response =
    StatusResponse { message: format!("All good!"), db_state: state.db_state.lock().await.clone() };
  Json(response)
}

async fn status_handler_quoted(
  State(state): State<AppState>,
) -> Json<QuotedResponse<StatusResponse>> {
  let db_state = state.db_state.lock().await.clone();
  let response = StatusResponse { message: "All good!".to_string(), db_state: db_state.clone() };

  let hash = response.secure_hash();
  let quote = get_tdx_quote(hash).await;

  Json(QuotedResponse { response, quote })
}

async fn query_handler_base(state: &AppState, payload: &SingleQuery) -> QueryResponseVec {
  let db_state = state.db_state.lock().await;
  let db = state.db.lock().await;
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
  let db_state = state.db_state.lock().await;
  let db = state.db.lock().await;
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

#[axum::debug_handler]
async fn update_handler(State(state): State<AppState>) -> Json<StatusResponse> {
  let mut state_verifier = state.state_verifier.lock().await;
  {
    let updated_status = state_verifier.update().await;

    if updated_status.is_err() {
      let response = StatusResponse {
        message: "Error updating state".to_string(),
        db_state: state.db_state.lock().await.clone(),
      };
      return Json(response);
    }
  }

  let mut db_state = state.db_state.lock().await;
  let db = state.db.lock().await;

  let mut cnt = 0;
  tprintln!("Updating DB");
  for (k, v) in state_verifier.pending_updates.iter() {
    db.insert(k.to_vec(), v.to_vec());
    cnt += 1;
    if cnt % 10000 == 0 {
      tprintln!("{} kv updates done", cnt);
      db.print_meta_state();
    }
  }
  state_verifier.pending_updates.clear();

  tprintln!("DB updated");
  db_state.block_id = state_verifier.block_id;
  db_state.state_root = state_verifier.storage_root;
  db_state.contract_address = state_verifier.contract_address();
  tprintln!("DB state updated");

  let response =
    StatusResponse { message: format!("Updated {cnt} addresses"), db_state: db_state.clone() };
  Json(response)
}
