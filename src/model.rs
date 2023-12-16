use async_graphql::{Context, EmptySubscription, Enum, Object, Schema};
use serde::{Deserialize, Serialize};
use tonic::Status;

use crate::{
    disperser::{
        disperser_client::DisperserClient, BlobStatusRequest, DisperseBlobReply,
        DisperseBlobRequest, RetrieveBlobRequest, SecurityParams,
    },
    AVAIL_SEED, AVAIL_SERVER, EIGEN_SERVER,
};

use avail_subxt::{
    api::{
        self,
        runtime_types::{
            bounded_collections::bounded_vec::BoundedVec, da_control::pallet::Call as DaCall,
        },
    },
    avail::{AppUncheckedExtrinsic, Pair},
    build_client,
    primitives::AvailExtrinsicParams,
    Call, Opts,
};

use sp_core::{Pair as _, H256};
use subxt::tx::PairSigner;

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum DA {
    EigenDA,
    Avail,
    // hawk current pony echo horse belt drill ceiling film theory guitar mind
}

#[derive(Serialize, Deserialize, Clone)]
struct BlobStatus {
    status: String,
    batch_header_hash: Vec<u8>,
    blob_index: u32,
}

#[Object]
impl BlobStatus {
    async fn status(&self) -> &str {
        &self.status
    }

    async fn batch_header_hash(&self) -> &Vec<u8> {
        &self.batch_header_hash
    }

    async fn blob_index(&self) -> &u32 {
        &self.blob_index
    }
}

pub(crate) type ServiceSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub(crate) struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn get_blob_status(&self, _ctx: &Context<'_>, id: Vec<u8>, da: DA) -> BlobStatus {
        match da {
            DA::EigenDA => {
                let request = BlobStatusRequest { request_id: id };

                let mut client = DisperserClient::connect(EIGEN_SERVER)
                    .await
                    .map_err(|e| {
                        Status::internal(format!("Failed to connect to external service: {}", e))
                    })
                    .unwrap();

                let response = client
                    .get_blob_status(request)
                    .await
                    .map_err(|e| {
                        Status::internal(format!(
                            "Failed to send request to external service: {}",
                            e
                        ))
                    })
                    .unwrap();

                let response = response.into_inner();

                let status = response.status();

                let info = response
                    .info
                    .unwrap_or_default()
                    .blob_verification_proof
                    .unwrap_or_default();
                BlobStatus {
                    status: status.as_str_name().into(),
                    batch_header_hash: info.batch_metadata.unwrap_or_default().batch_header_hash,
                    blob_index: info.blob_index,
                }
            }
            _ => panic!("not supported"),
        }
    }

    async fn get_blob_data(
        &self,
        _ctx: &Context<'_>,
        hash: Vec<u8>,
        index: u32,
        da: DA,
    ) -> Vec<u8> {
        match da {
            DA::EigenDA => {
                let request = RetrieveBlobRequest {
                    batch_header_hash: hash,
                    blob_index: index,
                };

                let mut client = DisperserClient::connect(EIGEN_SERVER)
                    .await
                    .map_err(|e| {
                        Status::internal(format!("Failed to connect to external service: {}", e))
                    })
                    .unwrap();

                // Send the request to the external service
                let response = client
                    .retrieve_blob(request)
                    .await
                    .map_err(|e| {
                        Status::internal(format!(
                            "Failed to send request to external service: {}",
                            e
                        ))
                    })
                    .unwrap();

                let response = response.into_inner();
                response.data
            }
            DA::Avail => {
                let client = build_client(AVAIL_SERVER, true).await.unwrap();

                let hash: [u8; 32] = hash.try_into().ok().unwrap();

                let submitted_block = client
                    .rpc()
                    .block(Some(H256::from(hash)))
                    .await
                    .unwrap()
                    .unwrap();

                let x = submitted_block
                    .block
                    .extrinsics
                    .into_iter()
                    .nth(index as usize)
                    .map(|chain_block_ext| {
                        AppUncheckedExtrinsic::try_from(chain_block_ext)
                            .map(|ext| ext.function)
                            .ok()
                    })
                    .unwrap()
                    .unwrap();

                match x {
                    Call::DataAvailability(DaCall::submit_data { data }) => data.0,
                    _ => vec![],
                }
            }
        }
    }
}

pub(crate) struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn store_blob(&self, _ctx: &Context<'_>, data: Vec<u8>, da: DA) -> Vec<u8> {
        dbg!(&data);
        // let api_context = ctx.data_unchecked::<ApiContext>();
        match da {
            DA::EigenDA => {
                let request = DisperseBlobRequest {
                    data,
                    security_params: vec![SecurityParams {
                        quorum_id: 0,
                        adversary_threshold: 25,
                        quorum_threshold: 50,
                    }],
                };

                let mut client = DisperserClient::connect(EIGEN_SERVER)
                    .await
                    .map_err(|e| {
                        Status::internal(format!("Failed to connect to external service: {}", e))
                    })
                    .unwrap();

                let response = client
                    .disperse_blob(request)
                    .await
                    .map_err(|e| {
                        Status::internal(format!(
                            "Failed to send request to external service: {}",
                            e
                        ))
                    })
                    .unwrap();

                let response: DisperseBlobReply = response.into_inner();
                response.request_id
            }
            DA::Avail => {
                let client = build_client(AVAIL_SERVER, true).await.unwrap();
                let pair = Pair::from_string_with_seed(AVAIL_SEED, None).unwrap();
                let signer = PairSigner::new(pair.0);

                let data_transfer = api::tx()
                    .data_availability()
                    .submit_data(BoundedVec(data.clone()));
                let extrinsic_params = AvailExtrinsicParams::new_with_app_id(1.into());

                println!("Sending example data...");
                let h = client
                    .tx()
                    .sign_and_submit_then_watch(&data_transfer, &signer, extrinsic_params)
                    .await
                    .unwrap()
                    .wait_for_finalized_success()
                    .await
                    .unwrap();

                dbg!(&h.extrinsic_index());
                dbg!(&h.block_hash());
                h.block_hash().as_fixed_bytes().into()
            }
        }
    }
}
