use std::str::FromStr;

use async_graphql::{Context, EmptySubscription, Enum, Object, Schema};
use near_da_rpc::{
    near::{
        config::{Config, KeyType, Network},
        Client,
    },
    Blob, CryptoHash, DataAvailability, Namespace,
};
use serde::{Deserialize, Serialize};
use tonic::Status;

use crate::{
    disperser::{
        disperser_client::DisperserClient, BlobStatusRequest, DisperseBlobReply,
        DisperseBlobRequest, RetrieveBlobRequest, SecurityParams,
    },
    hash_data, ApiContext, AvailObj, Data, EigenObj, NearObj, AVAIL_SEED, AVAIL_SERVER,
    EIGEN_SERVER, NEAR_ACCOUNT_ID, NEAR_SECRET,
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
    Call,
};

use sp_core::{Pair as _, H256};
use subxt::tx::PairSigner;

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum DA {
    Avail,
    EigenDA,
    Near, // hawk current pony echo horse belt drill ceiling film theory guitar mind
}

#[derive(Serialize, Deserialize, Clone)]
struct BlobStatus {
    status: String,
    hash: Vec<u8>,
    index: u32,
}

#[Object]
impl BlobStatus {
    async fn status(&self) -> &str {
        &self.status
    }

    async fn hash(&self) -> &Vec<u8> {
        &self.hash
    }

    async fn index(&self) -> &u32 {
        &self.index
    }
}

fn concatenate_slices(slice1: &[u8], slice2: &[u8]) -> Vec<u8> {
    let mut concatenated = Vec::with_capacity(slice1.len() + slice2.len());
    concatenated.extend_from_slice(slice1);
    concatenated.extend_from_slice(slice2);
    concatenated
}

pub(crate) type ServiceSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub(crate) struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn get_blob_status(&self, ctx: &Context<'_>, id: [u8; 32]) -> BlobStatus {
        let api_context = ctx.data_unchecked::<ApiContext>();
        let map = api_context.map.read().await;
        let data = map.get(&id);

        if data.is_none() {
            return BlobStatus {
                status: "Not found".to_owned(),
                hash: vec![],
                index: 0,
            };
        }

        let data = data.unwrap().clone();
        drop(map);
        match data {
            Data::Near(near) => BlobStatus {
                status: "FINALIZED".to_owned(),
                hash: near.hash.into(),
                index: 0,
            },
            Data::Avail(avail) => BlobStatus {
                status: "FINALIZED".to_owned(),
                hash: avail.hash.unwrap_or_default(),
                index: avail.index.unwrap_or_default(),
            },
            Data::EigenDA(eigen_da) => {
                if eigen_da.status == *"FINALIZED" {
                    return BlobStatus {
                        status: eigen_da.status.clone(),
                        hash: eigen_da.hash.clone().unwrap(),
                        index: eigen_da.index.unwrap(),
                    };
                }
                let request_id = eigen_da.request_id;

                let request = BlobStatusRequest {
                    request_id: request_id.clone(),
                };

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

                let hash = info.batch_metadata.unwrap_or_default().batch_header_hash;
                let index = info.blob_index;
                let value = Data::EigenDA(EigenObj {
                    status: status.as_str_name().into(),
                    request_id,
                    hash: Some(hash.clone()),
                    index: Some(index),
                });
                let mut map = api_context.map.write().await;
                map.insert(id, value);
                BlobStatus {
                    status: status.as_str_name().into(),
                    hash,
                    index,
                }
            }
        }
    }

    async fn get_blob_data(&self, ctx: &Context<'_>, id: [u8; 32]) -> String {
        let api_context = ctx.data_unchecked::<ApiContext>();
        let map = api_context.map.read().await;
        let value = map.get(&id);
        match value {
            None => "Invalid Id".to_owned(),
            Some(Data::Near(near)) => {
                let near_client = Client::new(&Config {
                    key: KeyType::SecretKey(NEAR_ACCOUNT_ID.to_string(), NEAR_SECRET.to_string()),
                    network: Network::Testnet,
                    namespace: Namespace::new(1, 1),
                    contract: NEAR_ACCOUNT_ID.to_string(),
                });

                let data = near_client.get(CryptoHash(near.hash)).await.unwrap().0.data;
                String::from_utf8(data).unwrap()
            }
            Some(Data::EigenDA(eigen_da)) => {
                let request = RetrieveBlobRequest {
                    batch_header_hash: eigen_da.hash.clone().unwrap(),
                    blob_index: eigen_da.index.unwrap(),
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
                String::from_utf8(response.data).unwrap()
            }
            Some(Data::Avail(avail)) => {
                let client = build_client(AVAIL_SERVER, true).await.unwrap();

                let hash: [u8; 32] = avail.hash.clone().unwrap().try_into().ok().unwrap();

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
                    .nth(avail.index.unwrap() as usize)
                    .map(|chain_block_ext| {
                        AppUncheckedExtrinsic::try_from(chain_block_ext)
                            .map(|ext| ext.function)
                            .ok()
                    })
                    .unwrap()
                    .unwrap();

                match x {
                    Call::DataAvailability(DaCall::submit_data { data }) => {
                        String::from_utf8(data.0).unwrap()
                    }
                    _ => "".to_owned(),
                }
            }
        }
    }
}

pub(crate) struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn store_blob(&self, ctx: &Context<'_>, data: String, da: DA) -> [u8; 32] {
        let data = data.into_bytes();
        let api_context = ctx.data_unchecked::<ApiContext>();

        match da {
            DA::Near => {
                let near_client = Client::new(&Config {
                    key: KeyType::SecretKey(NEAR_ACCOUNT_ID.to_string(), NEAR_SECRET.to_string()),
                    network: Network::Testnet,
                    namespace: Namespace::new(1, 1),
                    contract: NEAR_ACCOUNT_ID.to_string(),
                });

                let blobs = [Blob::new_v0(Namespace::new(0, 0), data)];
                let response = near_client.submit(&blobs).await.unwrap();
                let key = CryptoHash::from_str(&response.0).unwrap().0;
                let mut map = api_context.map.write().await;
                let v = Data::Near(NearObj { hash: key });
                map.insert(key, v);
                key
            }
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
                let mut map = api_context.map.write().await;
                let key = hash_data(&response.request_id);
                let v = Data::EigenDA(EigenObj {
                    status: "Processing".to_owned(),
                    request_id: response.request_id.clone(),
                    hash: None,
                    index: None,
                });
                map.insert(key, v);
                key
            }
            DA::Avail => {
                let client = build_client(AVAIL_SERVER, true).await.unwrap();
                let pair = Pair::from_string_with_seed(AVAIL_SEED, None).unwrap();
                let signer = PairSigner::new(pair.0);

                let data_transfer = api::tx()
                    .data_availability()
                    .submit_data(BoundedVec(data.clone()));
                let extrinsic_params = AvailExtrinsicParams::new_with_app_id(1.into());

                let h = client
                    .tx()
                    .sign_and_submit_then_watch(&data_transfer, &signer, extrinsic_params)
                    .await
                    .unwrap()
                    .wait_for_finalized_success()
                    .await
                    .unwrap();

                let mut map = api_context.map.write().await;
                let block_hash = h.block_hash();
                let hash = block_hash.as_bytes();
                let index = h.extrinsic_index().to_le_bytes();
                let key = concatenate_slices(hash, &index);
                let key = hash_data(&key);
                map.insert(
                    key,
                    Data::Avail(AvailObj {
                        hash: Some(block_hash.as_bytes().to_vec()),
                        index: Some(h.extrinsic_index()),
                    }),
                );
                dbg!(&h.extrinsic_index());
                dbg!(&h.block_hash());
                key
            }
        }
    }
}
