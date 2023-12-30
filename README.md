## Build and deploy locally
```
cargo build
cargo run
```
Open localhost:8000 or localhost:8000/playground


## Dockerize
This is specific to deploying on render.com. `docker build` is resource intensive. Increase memory and swap space on docker UI to the max and wait.
```
export DOCKER_DEFAULT_PLATFORM=linux/amd64
docker build -t <docker_id>/da:v1.0.0 .
docker push <docker_id>/da:v1.0.0 # may need to do docker login first
```

## NEAR
1. Create keypair using near cli
2. Build and deploy Blob contract using the created account: https://docs.near.org/data-availability/integrations.
   `NEAR_CONTRACT` has to be set to account_id (the user provided string, similar to any user handle).
3. Deploying doesn't call `new()` function. Run `near contract call-function as-transaction daaggregator.testnet new text-args '' prepaid-gas '100.0 Tgas' attached-deposit '0 NEAR' sign-as daaggregator.testnet network-config testnet sign-with-keychain send`

## Celestia
https://docs.celestia.org/nodes/celestia-node
- Install celestia-node (instead of `celestia version`, run `./build/celestia version`).
https://docs.celestia.org/nodes/light-node#start-the-light-node
- `celestia light init --p2p.network mocha`. Save its output somewhere.

Get rpc endpoint here: https://docs.celestia.org/nodes/mocha-testnet#rpc-endpoints
- `celestia light start --core.ip <rpc> --p2p.network mocha`
https://docs.celestia.org/developers/submit-data#grpc-to-a-consensus-node-via-the-user-package
https://docs.celestia.org/nodes/instantiate-testnet#celestia-app-installation

```sh
VALIDATOR_NAME=dagg_celestia
CHAIN_ID=testnet
KEY_NAME=dagg1
```

- We used `github.com/eigenco/lumina` for rust client.
  - Refer to https://github.com/Ferret-san/celestiabox for a working example of golang code to submit blobs to a light client. Although in go, this helps in seeing how to call a light client.
