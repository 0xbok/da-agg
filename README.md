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
