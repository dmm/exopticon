# EXOPTICON

## Installation
1. Pre-install requirements
* Network layout
* Install docker and docker-compose
* Create udev rule

If you're using the coral tpu, you'll have to add a udev rule to set
the correct permissions on the device. On Debian create a file called
/etc/udev/rules.d/60-coral.rules:

```
SUBSYSTEM=="usb",ATTRS{idVendor}=="1a6e",GROUP="plugdev",MODE="0666"
SUBSYSTEM=="usb",ATTRS{idVendor}=="18d1",GROUP="plugdev",MODE="0666"
```

Then run

```
# udevadm control --reload
```

as root to load the rule.

2. Fetch the sources

```bash
$ git clone --recurse-submodules https://gitlab.com/dmattli/exopticon.git
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
bash -ac 'source ../release-info && docker-compose -f docker-compose.db.yml -f docker-compose.yml -f docker-compose.cuda.yml -d'
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
exopticon@a08865046dd9:/exopticon$ cargo make ci-flow
.... build output ...
```

5. Run exopticon

### Build exopticon
```
exopticon@a08865046dd9:/exopticon$ cargo run

```
