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
docker build -t gmridul111/da:v1.0.0 .
docker push gmridul111/da:v1.0.0 # may need to do docker login first
```

## NEAR
1. Create keypair using near cli
2. Build and deploy Blob contract using the created account: https://docs.near.org/data-availability/integrations.
   `NEAR_CONTRACT` has to be set to account_id (the user provided string, similar to any user handle).
