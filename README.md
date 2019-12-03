# EXOPTICON

Video Surveillance

## Installation
1. Fetch the sources

    $ git clone https://gitlab.com/dmattli/exopticon.git

2. Grab the exopticon-build image

    $ docker pull dmattli/exopticon-build:latest

3. Start the build

    $ cd exopticon && docker run -v "$(pwd)":/exopticon -w /exopticon dmattli/exopticon-build:latest sh -c 'cargo build --release'

4. Prepare database


    $ sudo -u postgres psql # Run psql as user that can create users and databases

    # Create the database
    postgres=# CREATE DATABASE exopticon;

    # Create the database user
    postgres=# CREATE USER exopticon_user WITH PASSWORD 'change_this_password;

    # Grant access to the database
    postgres=# GRANT ALL PRIVILEGES ON DATABASE exopticon TO exopticon_user;

    postgres=# \q

    # Now run the diesel migrations
    $  docker run -it --rm -v "$(pwd)":/exopticon \
       -e DATABASE_URL=postgres://exopticon_user:'change_this_password'@10.0.0.2/exopticon \ # SET THIS DB CONNECTION STRING
       -w /exopticon dmattli/exopticon-build:latest sh -c 'diesel setup'
    Running migration 2018-10-22-125302_initial_schema
    Running migration 2018-12-11-144804_users
    Running migration 2019-06-28-192842_create_observations

5. Create initial user and camera group

    $ docker run -it --rm -v "$(pwd)":/exopticon \
       -e DATABASE_URL=postgres://exopticon_user:'change_this_password'@10.0.0.2/exopticon \ # SET THIS DB CONNECTION STRING
       -w /exopticon dmattli/exopticon-build:latest sh -c 'target/release/exopticon --add-user'
    Enter username for initial user: user1
    Enter password for initial user: [hidden]
    Created User!

    $ docker run -it --rm -v "$(pwd)":/exopticon \
      -e DATABASE_URL=postgres://exopticon_user:'change_this_password'@10.0.0.2/exopticon \ # SET THIS DB CONNECTION STRING
      -w /exopticon dmattli/exopticon-build:latest sh -c 'target/release/exopticon --add-camera-group'
    Enter storage path for recorded video: /tank/video
    Enter max space used at this path, in megabytes: 2000


