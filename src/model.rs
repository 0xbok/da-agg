use std::{str::FromStr, sync::Arc};

use async_graphql::{Context, EmptySubscription, Enum, Object, Schema};
use ethers::{
    middleware::SignerMiddleware,
    providers::{Http, Provider},
    signers::{LocalWallet, Signer},
    types::Address,
};
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
        disperser_client::DisperserClient, BlobStatus as EigenBlobStatus, BlobStatusRequest,
        DisperseBlobReply, DisperseBlobRequest, RetrieveBlobRequest, SecurityParams,
    },
    hash_data, ApiContext, Data, EigenObj, MapContract, AVAIL_SEED, AVAIL_SERVER, EIGEN_SERVER,
    NEAR_ACCOUNT_ID, NEAR_SECRET, OPSEP_CONTRACT, OPSEP_RPC, OPSET_SEED,
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
    index: Option<[u8; 32]>,
}

#[Object]
impl BlobStatus {
    async fn status(&self) -> &str {
        &self.status
    }

    async fn index(&self) -> &Option<[u8; 32]> {
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
                status: "Not found".to_string(),
                index: None,
            };
        }

        let data = data.unwrap().clone();
        drop(map);
        match data {
            Data::EigenDA(eigen_da) => {
                if eigen_da.status == *"FINALIZED" || eigen_da.status == *"CONFIRMED" {
                    return BlobStatus {
                        status: eigen_da.status,
                        index: eigen_da.op_index,
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
                match status {
                    EigenBlobStatus::Finalized | EigenBlobStatus::Confirmed => {
                        let info = response
                            .info
                            .unwrap_or_default()
                            .blob_verification_proof
                            .unwrap_or_default();

                        let mut hash = info.batch_metadata.unwrap_or_default().batch_header_hash;
                        let mut index = Vec::from(info.blob_index.to_le_bytes());
                        let owner = OPSET_SEED.parse::<LocalWallet>().unwrap();
                        let op_provider = Provider::<Http>::try_from(OPSEP_RPC).unwrap();

                        let op_client =
                            SignerMiddleware::new(op_provider, owner.with_chain_id(11155420u64));
                        let contract_address = OPSEP_CONTRACT.parse::<Address>().unwrap();
                        let contract = MapContract::new(contract_address, Arc::new(op_client));
                        hash.insert(0, DA::EigenDA as u8);
                        hash.append(&mut index);
                        let tx = contract
                            .save(hash.into())
                            .send()
                            .await
                            .unwrap()
                            .await
                            .unwrap()
                            .unwrap();

                        let op_index = tx.logs[0].topics[1].into();

                        let mut map = api_context.map.write().await;
                        map.insert(
                            id,
                            Data::EigenDA(EigenObj {
                                status: status.as_str_name().to_string(),
                                request_id: id.into(),
                                op_index: Some(op_index),
                            }),
                        );
                        drop(map);

                        BlobStatus {
                            status: status.as_str_name().to_string(),
                            index: Some(op_index),
                        }
                    }
                    _ => BlobStatus {
                        status: status.as_str_name().to_string(),
                        index: None,
                    },
                }
            }
        }
    }

    async fn get_blob_data(&self, _ctx: &Context<'_>, id: [u8; 32]) -> String {
        let op_provider = Provider::<Http>::try_from(OPSEP_RPC).unwrap();

        let owner = OPSET_SEED.parse::<LocalWallet>().unwrap();
        let op_client = SignerMiddleware::new(op_provider, owner.with_chain_id(11155420u64));
        let contract_address = OPSEP_CONTRACT.parse::<Address>().unwrap();
        let contract = MapContract::new(contract_address, Arc::new(op_client));
        let data = contract.get(id.into()).call().await.unwrap();
        let ptr: Vec<u8> = data.0.into();

        if ptr[0] == DA::Near as u8 {
            let cryptohash: [u8; 32] = ptr[1..33].try_into().unwrap();
            let near_client = Client::new(&Config {
                key: KeyType::SecretKey(NEAR_ACCOUNT_ID.to_string(), NEAR_SECRET.to_string()),
                network: Network::Testnet,
                namespace: Namespace::new(1, 1),
                contract: NEAR_ACCOUNT_ID.to_string(),
            });

            let data = near_client
                .get(CryptoHash(cryptohash))
                .await
                .unwrap()
                .0
                .data;
            String::from_utf8(data).unwrap()
        } else if ptr[0] == DA::Avail as u8 {
            let hash: [u8; 32] = ptr[1..33].try_into().unwrap();
            let index: [u8; 4] = ptr[33..].try_into().unwrap();
            let client = build_client(AVAIL_SERVER, true).await.unwrap();

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
                .nth(u32::from_le_bytes(index) as usize)
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
        } else if ptr[0] == DA::EigenDA as u8 {
            let hash: [u8; 32] = ptr[1..33].try_into().unwrap();
            let index: [u8; 4] = ptr[33..].try_into().unwrap();

            let request = RetrieveBlobRequest {
                batch_header_hash: hash.into(),
                blob_index: u32::from_le_bytes(index),
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
                    Status::internal(format!("Failed to send request to external service: {}", e))
                })
                .unwrap();

            let response = response.into_inner();
            String::from_utf8(response.data).unwrap()
        } else {
            "Not found".to_string()
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
                let mut key: Vec<u8> = CryptoHash::from_str(&response.0).unwrap().0.into();

                let owner = OPSET_SEED.parse::<LocalWallet>().unwrap();
                let op_provider = Provider::<Http>::try_from(OPSEP_RPC).unwrap();

                let op_client =
                    SignerMiddleware::new(op_provider, owner.with_chain_id(11155420u64));
                let contract_address = OPSEP_CONTRACT.parse::<Address>().unwrap();
                let contract = MapContract::new(contract_address, Arc::new(&op_client));
                key.insert(0, DA::Near as u8);
                let id = contract
                    .save(key.into())
                    .send()
                    .await
                    .unwrap()
                    .await
                    .unwrap()
                    .unwrap();

                id.logs[0].topics[1].into() // topics[1] is the map index in the contract at which data was stored.
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
                let key = hash_data(&response.request_id);
                let v = Data::EigenDA(EigenObj {
                    status: "Processing".to_owned(),
                    request_id: response.request_id.clone(),
                    ..Default::default()
                });
                let mut map = api_context.map.write().await;
                map.insert(key, v);
                drop(map);
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

                // let mut map = api_context.map.write().await;
                let block_hash = h.block_hash();
                let hash = block_hash.as_bytes();
                let index = h.extrinsic_index().to_le_bytes();
                let mut key = concatenate_slices(hash, &index);

                let owner = OPSET_SEED.parse::<LocalWallet>().unwrap();
                let op_provider = Provider::<Http>::try_from(OPSEP_RPC).unwrap();

                let op_client =
                    SignerMiddleware::new(op_provider, owner.with_chain_id(11155420u64));
                let contract_address = OPSEP_CONTRACT.parse::<Address>().unwrap();
                let contract = MapContract::new(contract_address, Arc::new(op_client));
                key.insert(0, DA::Avail as u8);
                let id = contract
                    .save(key.into())
                    .send()
                    .await
                    .unwrap()
                    .await
                    .unwrap()
                    .unwrap();
                id.logs[0].topics[1].into() // topics[1] is the map index in the contract at which data was stored.            }
            }
        }
    }
}
