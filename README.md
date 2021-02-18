# EXOPTICON

## Installation
1. Pre-install requirements
* Network layout
* Install docker and docker-compose
2. Fetch the sources

```bash
$ git clone https://gitlab.com/dmattli/exopticon.git
```

This will fetch master which should contain the latest
release. Alternatively you could checkout a specific release:

```bash
cd exopticon/
git checkout v0.11.0
```

3. Configure docker compose by setting the variables in exopticon/docker/.env .

4. Call docker compose

### Non-cuda
```bash
cd exopticon/docker/
bash -ac 'source ../release_info && docker-compose -f docker-compose.db.yml -f docker-compose.yml up -d'
```

### cuda
```bash
cd exopticon/docker/
./gen-cuda.bash |  docker-compose -f docker-compose.db.yml -f docker-compose.yml -f /dev/stdin up -d
```

5. Create initial user

```bash
docker exec -it exopticon_exopticon_1 /exopticon/exopticon --add-user
```

## Development environment

1. Follow steps 1 & 2 & 3 from the Installation instructions

2. Call docker compose to start the dev environment.

### Build dev environment
```bash
cd exopticon/docker
docker-compose -f docker-compose.db.yml -f docker-compose.dev.yml up
```

That will build the dev docker image and start the dev environment container.

3. Connect to the dev container

In a new terminal run docker exec to connect to your dev environment.

### Start dev container
```bash
docker exec -it exopticon_exopticon_1 /bin/bash
```

4. Build exopticon

### Build exopticon
```
exopticon@a08865046dd9:/exopticon$ cargo make
.... build output ...
```

5. Run exopticon

### Build exopticon
```
exopticon@a08865046dd9:/exopticon$ cargo run

```
