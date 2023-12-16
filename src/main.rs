mod model;
mod routes;

use async_graphql::{EmptySubscription, Schema};
use axum::{extract::Extension, routing::get, Router, Server};

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

// struct ApiContext {
//     eigen_client: DisperserClient<Channel>,
// }

// Runtime to run our server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription).finish();
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
