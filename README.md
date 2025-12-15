# Mostro Push Backend

Privacy-preserving push notification server for Mostro P2P trades. Inspired by [MIP-05](https://github.com/marmot-chat/mips), this server enables targeted push notifications without exposing user data to Firebase/Apple.

## How It Works

```
┌─────────────────┐     1. Register token      ┌──────────────────┐
│  Mostro Mobile  │ ─────────────────────────► │  Push Server     │
│                 │     (encrypted)            │                  │
│  trade_pubkey   │                            │  Stores:         │
│  + FCM token    │                            │  trade_pubkey →  │
└─────────────────┘                            │  device_token    │
                                               └────────┬─────────┘
                                                        │
┌─────────────────┐     2. Publishes event     ┌────────▼─────────┐
│  Mostro Daemon  │ ─────────────────────────► │  Nostr Relay     │
│                 │     kind 1059              │                  │
│  (no changes    │     p: trade_pubkey        │                  │
│   required)     │                            └────────┬─────────┘
└─────────────────┘                                     │
                                                        │ 3. Server observes
                                               ┌────────▼─────────┐
                                               │  Push Server     │
                                               │                  │
                                               │  Extracts p tag  │
                                               │  Looks up token  │
                                               │  Sends push      │
                                               └────────┬─────────┘
                                                        │
                                               ┌────────▼─────────┐
                                               │  FCM / APNs      │
                                               │  (silent push)   │
                                               └────────┬─────────┘
                                                        │
                                               ┌────────▼─────────┐
                                               │  Mostro Mobile   │
                                               │  wakes up,       │
                                               │  fetches events  │
                                               └──────────────────┘
```

## Privacy Properties

- **Server knows**: Temporary mapping of `trade_pubkey` → `device_token`, timing of events
- **Server does NOT know**: User identity (npub), message content, which orders belong to whom
- **Firebase/Apple know**: A notification occurred (unavoidable with push)
- **Firebase/Apple do NOT know**: Nostr identity, trade details, message content

## Features

- **Targeted notifications**: Only the recipient device receives the push (not broadcast)
- **Encrypted token registration**: Device tokens are encrypted with server's public key
- **Privacy-first**: No persistent storage, tokens auto-expire
- Firebase Cloud Messaging (FCM) support
- UnifiedPush support (GrapheneOS, LineageOS)
- Automatic relay reconnection
- HTTP API for token management

## Requirements

- Rust 1.75 or higher
- Access to a Nostr relay
- (Optional) Firebase account with service account for FCM

## Installation

### 1. Clone the repository

```bash
git clone <repository-url>
cd mostro-push-server
```

### 2. Configure environment variables

```bash
cp .env.example .env
nano .env
```

Edit the `.env` file with your configurations:

```bash
NOSTR_RELAYS=wss://relay.mostro.network
SERVER_HOST=0.0.0.0
SERVER_PORT=8080
FCM_ENABLED=true
UNIFIEDPUSH_ENABLED=true
FIREBASE_PROJECT_ID=your-project
RUST_LOG=info
```

### 3. Run in development mode

```bash
cargo run
```

### 4. Build for production

```bash
cargo build --release
./target/release/mostro-push-backend
```

## Docker Usage

### Build

```bash
docker build -t mostro-push-backend .
```

### Run

```bash
docker-compose up -d
```

## API Endpoints

### Health Check

```bash
curl http://localhost:8080/api/health
```

Response:
```json
{"status":"ok"}
```

### Server Info

Get the server's public key (needed by clients to encrypt tokens):

```bash
curl http://localhost:8080/api/info
```

Response:
```json
{
  "server_pubkey": "02abc123...",
  "version": "0.2.0",
  "encrypted_token_size": 281
}
```

### Status

```bash
curl http://localhost:8080/api/status
```

Response:
```json
{
  "status": "running",
  "version": "0.2.0",
  "server_pubkey": "02abc123...",
  "tokens": {
    "total": 42,
    "android": 35,
    "ios": 7
  }
}
```

### Register Token

Register an encrypted device token for a trade. The client must encrypt the token using the server's public key.

```bash
curl -X POST http://localhost:8080/api/register \
  -H "Content-Type: application/json" \
  -d '{
    "trade_pubkey": "abc123...def456",
    "encrypted_token": "<base64-encoded-encrypted-token>"
  }'
```

Response:
```json
{
  "success": true,
  "message": "Token registered successfully",
  "platform": "android"
}
```

### Unregister Token

```bash
curl -X POST http://localhost:8080/api/unregister \
  -H "Content-Type: application/json" \
  -d '{
    "trade_pubkey": "abc123...def456"
  }'
```

## Architecture

```
┌─────────────────┐
│  Nostr Relays   │
│ (kind 1059)     │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Rust Backend   │
│  - WebSocket    │
│  - Event batch  │
│  - HTTP API     │
└────────┬────────┘
         │
    ┌────┴────┐
    │         │
    ▼         ▼
┌─────┐   ┌──────────┐
│ FCM │   │UnifiedPush│
└──┬──┘   └────┬─────┘
   │           │
   ▼           ▼
[Android]   [GrapheneOS]
```

## Project Structure

```
mostro-push-backend/
├── Cargo.toml
├── .env.example
├── src/
│   ├── main.rs              # Entry point
│   ├── config.rs            # Configuration
│   ├── nostr/
│   │   ├── mod.rs
│   │   └── listener.rs      # Nostr event listener
│   ├── push/
│   │   ├── mod.rs           # PushService trait
│   │   ├── fcm.rs           # FCM implementation
│   │   └── unifiedpush.rs   # UnifiedPush implementation
│   ├── api/
│   │   ├── mod.rs
│   │   └── routes.rs        # HTTP endpoints
│   └── utils/
│       ├── mod.rs
│       └── batching.rs      # Batching management
├── Dockerfile
├── docker-compose.yml
└── README.md
```

## Firebase (FCM) Configuration

To use FCM, you need:

1. Create a project in [Firebase Console](https://console.firebase.google.com/)
2. Download the service account JSON file
3. Configure the environment variables:

```bash
FIREBASE_PROJECT_ID=your-project-id
FIREBASE_SERVICE_ACCOUNT_PATH=/path/to/service-account.json
FCM_ENABLED=true
```

## Monitoring

The backend logs detailed information that you can monitor:

```bash
# Production logs
tail -f /var/log/mostro-push-backend/app.log

# Docker logs
docker-compose logs -f push-backend
```

Important events:
- Connection to Nostr relays
- Receipt of kind 1059 events
- Notification sending
- Connection errors

## Development

### Run tests

```bash
cargo test
```

### Linting

```bash
cargo clippy
```

### Formatting

```bash
cargo fmt
```

## Testing

A test script is provided to verify all endpoints:

```bash
# Start the server
RUST_LOG=info cargo run

# In another terminal, run tests
./test_server.sh
```

The test script will:
1. Check health endpoint
2. Verify status endpoint
3. Register a test UnifiedPush endpoint
4. Verify persistence (check data/unifiedpush_endpoints.json)
5. Unregister the endpoint
6. Test the notification system

## Implementation Status

### ✅ Completed
- [x] Nostr listener with Mostro pubkey filtering (Option B: Silent Push Global)
- [x] UnifiedPush endpoint registration/unregistration
- [x] Persistent storage for UnifiedPush endpoints (JSON file)
- [x] FCM OAuth2 token refresh with JWT signing
- [x] Intelligent notification batching (5s delay, 60s cooldown)
- [x] HTTP API for endpoint management
- [x] Automatic relay reconnection

### 🔄 TODO
- [ ] Implement retry logic for failed push deliveries
- [ ] Add metrics and monitoring (Prometheus)
- [ ] Implement authentication for API endpoints
- [ ] Support for multiple Mostro instances
- [ ] Integration tests with mock Nostr relay
- [ ] Docker deployment configuration

## License

See [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome. Please open an issue first to discuss the changes you would like to make.

## Resources

- [UnifiedPush Spec](https://unifiedpush.org/developers/spec/)
- [Nostr SDK Rust](https://docs.rs/nostr-sdk/)
- [Actix Web](https://actix.rs/docs/)
- [FCM v1 API](https://firebase.google.com/docs/cloud-messaging/migrate-v1)
