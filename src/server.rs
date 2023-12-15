use disperser::disperser_client::DisperserClient;
use tonic::{transport::Server, Request, Response, Status};

use disperser::disperser_server::{Disperser, DisperserServer};
use disperser::{
    BlobStatusReply, BlobStatusRequest, DisperseBlobReply, DisperseBlobRequest, RetrieveBlobReply,
    RetrieveBlobRequest,
};

// Import the generated proto-rust file into a module
pub mod disperser {
    tonic::include_proto!("disperser");
}

// Implement the service skeleton for the "Greeter" service
// defined in the proto
#[derive(Debug, Default)]
pub struct MyDisperser {}
const EIGEN_SERVER: &str = "https://disperser-goerli.eigenda.xyz:443";

// Implement the service function(s) defined in the proto
// for the Greeter service (SayHello...)
#[tonic::async_trait]
impl Disperser for MyDisperser {
    async fn disperse_blob(
        &self,
        request: Request<DisperseBlobRequest>,
    ) -> Result<Response<DisperseBlobReply>, Status> {
        // Create a client to communicate with the external gRPC service
        let mut client = DisperserClient::connect(EIGEN_SERVER).await.map_err(|e| {
            Status::internal(format!("Failed to connect to external service: {}", e))
        })?;

        // Send the request to the external service
        let response = client.disperse_blob(request).await.map_err(|e| {
            Status::internal(format!("Failed to send request to external service: {}", e))
        })?;
        // Send the request to the external service

        Ok(response)
    }

    async fn get_blob_status(
        &self,
        request: Request<BlobStatusRequest>,
    ) -> Result<Response<BlobStatusReply>, Status> {
        // Create a client to communicate with the external gRPC service
        let mut client = DisperserClient::connect(EIGEN_SERVER).await.map_err(|e| {
            Status::internal(format!("Failed to connect to external service: {}", e))
        })?;

        // Send the request to the external service
        let response = client.get_blob_status(request).await.map_err(|e| {
            Status::internal(format!("Failed to send request to external service: {}", e))
        })?;

        Ok(response)
    }

    async fn retrieve_blob(
        &self,
        request: Request<RetrieveBlobRequest>,
    ) -> Result<Response<RetrieveBlobReply>, Status> {
        // Create a client to communicate with the external gRPC service
        let mut client = DisperserClient::connect(EIGEN_SERVER).await.map_err(|e| {
            Status::internal(format!("Failed to connect to external service: {}", e))
        })?;

        // Send the request to the external service
        let response = client.retrieve_blob(request).await.map_err(|e| {
            Status::internal(format!("Failed to send request to external service: {}", e))
        })?;

        Ok(response)
    }
}

// Runtime to run our server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let disperser = MyDisperser::default();

    println!("Starting gRPC Server...");
    Server::builder()
        .add_service(DisperserServer::new(disperser))
        .serve(addr)
        .await?;

    Ok(())
}
