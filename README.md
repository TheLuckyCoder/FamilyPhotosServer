# FamilyPhotos Server

An open source self-hosted photo and video server for your family written in Rust.

## How to set up
Install all the libraries on your system and setup the PostgresSQL service.

### Needed libraries and programs:
- postgres (for the database)
- libwebp (for thumbnail generation)
- libheif (Optional - thumbnail generation for HEIC/HEIF images)
- ffmpegthumbnailer (Optional - for video thumbnail generation)

While some of these are optional, it is recommended to install them all.

### Configuration
The server can be configured using a .env file located in the same folder as the executable or setting environment variables.<br>
Variables in bold **must** be specified.
- **SERVER_PORT**: The port the server should listen on
- **DATABASE_URL** (eg: postgres://username:password@localhost/database?sslmode=disable)
- **SESSION_SECRET**: Used to hash the session cookie with. Must be at least 64 bytes long (64 chars)
- **STORAGE_PATH**: The path to the folder where the photos will be stored
- THUMBNAIL_PATH: The path to the folder where the thumbnails of the photos will be generated (default: in STORAGE_PATH/.thumbnail)
- USE_HTTPS: Run the web server in HTTPS Mode, recommended if you don't have a reverse proxy (default: false)
- SSL_PRIVATE_KEY_PATH: expects a PKCS8 file path (default: none)
- SSL_CERTS_PATH (default: none)
- SCAN_NEW_FILES: Scan the storage for external changes at startup (default: true)
- GENERATE_THUMBNAILS_BACKGROUND: Generate thumbnails for all photos on background thread (on startup), as opposed to only lazily generating when needed (default: false)
- RUST_LOG: Specifies the Rust log level (default: none)

### Creating user accounts
On your first run, the server will generate a user account with the username "public" and a random password that will be printed in the console.<br>
Knowing this password is not relevant as this user is only used for photos that belong to everyone.<br><br>
To create new user accounts run the following command using the CLI:<br>
```commandline
familyphotos user create -u <user_name> -d <display_name> [-p <password>]
```
This will generate a new user with the given username, display name and password or a random one if not provided.<br>

## Folder structure
The server will generate the following folder structure in the STORAGE_PATH:
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

## Running the server
To run the server, simply execute the binary for your chosen architecture.<br>

If you want the server to run in the background and automatically start on boot you might want to try setting up service in systemd such as below.<br>
Place the following file in `/etc/systemd/system/familyphotos.service`
```
[Unit]
Description=Family Photos Server
Wants=network.target
After=network.target

[Service]
WorkingDirectory=/path/to/executable
ExecStart=familyphotos
User=...
Restart=on-failure
RestartSec=20
SuccessExitStatus=0

[Install]
WantedBy=multi-user.target
```
Now run the following to reload the systemd daemon and enable the service:
```commandline
sudo systemctl daemon-reload
sudo systemctl enable --now familyphotos
```