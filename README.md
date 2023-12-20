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
