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

3. Configure docker compose by setting the variables exopticon/docker/.env:

* EXOPTICON_DB_PATH
  This is the path to store the database files.
* EXOPTICON_VIDEO_PATH
  This is the path where video files will be stored
* EXOPTICON_POSTGRES_PASSWORD
  Password for the postgres database. This really isn't used by you so
  just set it to something long.

4. Call docker compose

### Non-cuda
```bash
cd exopticon/docker/
docker-compose -f docker-compose.db.yml -f docker-compose.yml up -d
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

