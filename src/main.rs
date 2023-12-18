mod model;
mod routes;

use async_graphql::{EmptySubscription, Schema};
use axum::{extract::Extension, http::StatusCode, response::Html, routing::get, Router, Server};

use std::sync::Arc;
use std::{collections::HashMap, fs};
use tokio::sync::RwLock;

use sha2::{Digest, Sha256};

type SharedMap = Arc<RwLock<HashMap<[u8; 32], Data>>>;

#[derive(Clone)]
struct EigenObj {
    status: String,
    request_id: Vec<u8>,
    hash: Option<Vec<u8>>,
    index: Option<u32>,
}

#[derive(Clone)]
struct AvailObj {
    hash: Option<Vec<u8>>,
    index: Option<u32>,
}

#[derive(Clone)]
enum Data {
    EigenDA(EigenObj),
    Avail(AvailObj),
}

struct ApiContext {
    map: SharedMap,
}

async fn index_html() -> Result<Html<String>, (StatusCode, &'static str)> {
    match fs::read_to_string("static/index.html") {
        Ok(contents) => Ok(Html(contents)),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Could not read file")),
    }
}

use model::{MutationRoot, QueryRoot};
use routes::{graphql_handler, graphql_playground, health};

// Import the generated proto-rust file into a module
pub mod disperser {
    tonic::include_proto!("disperser");
}

// Implement the service skeleton for the "Greeter" service
// defined in the proto
#[derive(Debug, Default)]
pub struct MyDisperser {}
const EIGEN_SERVER: &str = "https://disperser-goerli.eigenda.xyz:443";
const AVAIL_SERVER: &str = "wss://goldberg.avail.tools:443/ws";
const AVAIL_SEED: &str = "hawk current pony echo horse belt drill ceiling film theory guitar mind";

fn hash_data(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    result.into()
}

// Runtime to run our server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let map: SharedMap = Arc::new(RwLock::new(HashMap::new()));
    let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(ApiContext { map })
        .finish();
    let app = Router::new()
        .route("/", get(index_html).post(graphql_handler))
        .route("/playground", get(graphql_playground).post(graphql_handler))
        .route("/health", get(health))
        .layer(Extension(schema));

    Server::bind(&"0.0.0.0:8000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}
