mod model;
mod routes;

use async_graphql::{EmptySubscription, Schema};
use axum::{extract::Extension, routing::get, Router, Server};

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use sha2::{Digest, Sha256};

type SharedMap = Arc<RwLock<HashMap<[u8; 32], Data>>>;

struct EigenDA {
    status: String,
    request_id: Option<Vec<u8>>,
    hash: Option<Vec<u8>>,
    index: Option<u32>,
}

struct Avail {
    hash: Option<Vec<u8>>,
    index: Option<u32>,
}

struct Data {
    eigen_da: Option<EigenDA>,
    avail: Option<Avail>,
}

struct ApiContext {
    map: SharedMap,
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
        .route("/", get(graphql_playground).post(graphql_handler))
        .route("/health", get(health))
        .layer(Extension(schema));

    Server::bind(&"0.0.0.0:8000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}
