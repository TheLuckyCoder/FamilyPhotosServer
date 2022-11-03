# FamilyPhotos

### How to set up

### Needed libraries and programs:
- postgres
- libwebp (Optional - for any kind of thumbnail)
- libheif (Optional - for HEIC/HEIF thumbnails)
- ffmpegthumbnailer (Optional - for video thumbnails)
- openssl (Optional - for HTTPS support)

### Configuration
The server can be configured using a .env file place in the same folder as the executable or using environment variables.<br>
- SERVER_PORT
- DATABASE_URL (eg: postgres://username:password@localhost/database?sslmode=disable)
- STORAGE_PATH 
- USE_HTTPS (default: false)
- SSL_PRIVATE_KEY_PATH (default: none)
- SSL_CERTS_PATH (default: none)
- SKIP_SCANNING (default: false)
- RUST_LOG (default: none)
