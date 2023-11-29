# FamilyPhotos Server

An open source self-hosted photo and video server for your family written in Rust.

## How to set up
A empty Postgres Database must be set up be it as a system library or Docker image.<br>

It's also expected that you run a proxy like Nginx to handle load balancing and TLS.

### Docker
Clone the repository and run the following command to build a docker image (for the x86_64 architecture):
```shell
docker build -t familyphotos .
```

### Docker Compose
Here is an example of a Docker compose file

```
version: '3'

services:
   familyphotos:
     container_name: familyphotos
     image: familyphotos
     volumes:
       - /path/to/photos/folder/:/opt/photos/
     restart: always
     network_mode: "host"
     environment:
       SCAN_NEW_FILES: true
       GENERATE_THUMBNAILS_BACKGROUND: false
       RUST_LOG: info
       SERVER_PORT: 3000
       DATABASE_URL: postgres://username:password@localhost/database?sslmode=disable
       STORAGE_PATH: /opt/photos/
```

Below you can see all the environment variables that can be configured

### Env Variables
Variables in bold **must** be specified.
- **SERVER_PORT**: The port the server should listen on
- **DATABASE_URL** (eg: postgres://username:password@localhost/database?sslmode=disable)
- **STORAGE_PATH**: The path to the folder where the photos will be stored
- THUMBNAIL_PATH: Alternative storage path for photo thumbnails (this is useful for example when you want to store the photos on a HDD but the thumbnails on an SSD so that they load faster) [default: in STORAGE_PATH/.thumbnail]
- SCAN_NEW_FILES: Scan the storage for external changes at startup [default: true]
- GENERATE_THUMBNAILS_BACKGROUND: Generate thumbnails for all photos on background thread (on startup), as opposed to only lazily generating when needed [default: false]
- RUST_LOG: Specifies the log level, it's recommended to set it to info [default: none]

### Creating user accounts
On your first run, the server will generate a user account with the username "public" and a random password that will be printed in the console.<br>
Knowing this password is not relevant as this user is only used for photos that belong to everyone.<br><br>
To create new user accounts run the following command using the CLI:<br>
```shell
familyphotos user create -u <user_name> -d <display_name> [-p <password>]
```
This will generate a new user with the given username, display name and password or a random one if not provided.<br>

## Folder structure
The server will generate the following folder structure in the STORAGE_PATH folder:
```
├───.thumbnail/ # Folder for thumbnails (if not specified elsewhere)
│
├───public/ # The folder of the "public" user, alas photos who belong to everyone
│   ├───<album_name>/ # Folder for albums aka "folders"
│   │   └───<photo_name> # Photo files
│   └───<photo_name> # Photo files
│
└───<user_name>/ # Folder for each individual user
    ├───<album_name>/ # Folder for albums aka "folders"
    │   └───<photo_name> # Photo files
    └───<photo_name> # Photo files
```
