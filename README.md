# FamilyPhotos

## How to set up
Install all the libraries on your system and setup the PostgresSQL service.

### Needed libraries and programs:
- postgres (for the database)
- openssl (for HTTPS support)
- libwebp (Optional - for any kind of thumbnail generation)
- libheif (Optional - for HEIC/HEIF thumbnail generation)
- ffmpegthumbnailer (Optional - for video thumbnail generation)

While some of these are optional, it is recommended to install them all.

### Configuration
The server can be configured using a .env file located in the same folder as the executable or setting environment variables.<br>

- SERVER_PORT: The port the server should listen on
- DATABASE_URL (eg: postgres://username:password@localhost/database?sslmode=disable)
- STORAGE_PATH: The path to the folder where the photos will be stored
- USE_HTTPS: Run the web server in HTTPS Mode, recommended if you don't have a reverse proxy (default: false)
- SSL_PRIVATE_KEY_PATH (default: none)
- SSL_CERTS_PATH (default: none)
- SKIP_SCANNING: Start scanning the storage path for changes (for e.g. new/deleted photos) (default: false)
- RUST_LOG: Specifies the Rust log level (default: none)
- GENERATE_THUMBNAILS_BACKGROUND: Generate thumbnails for all photos on background thread (on app startup), as opposed to only lazily generating when needed (default: false)

### Creating user accounts
On your first run, the server will generate a user account with the username "public" and a random password that will be printed in the console.<br>
Knowing this password is not relevant as this user is only used for photos that belong to everyone.<br><br>
To create new user accounts only through the CLI:<br>
```
familyphotos user create -u <user_name> -d <display_name> -p <password>
```
This will generate a new user with the given username, display name and password or a random one if not provided.<br>

## Folder structure
The server will generate the following folder structure in the STORAGE_PATH:
```
├───.thumbnail/ # Folder for thumbnails
├───public/ # The folder of the "public" user, alas photos who belong to everyone
│   ├───<album_name>/ # Folder for albums aka "folders"
│   │   └───<photo_name> # Photo files
│   └───<photo_name> # Photo files
└───<user_name>/ # Folder for each individual user
    ├───<album_name>/ # Folder for albums aka "folders"
    │   └───<photo_name> # Photo files
    └───<photo_name> # Photo files
```