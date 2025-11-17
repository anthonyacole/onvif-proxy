# ONVIF Proxy for Reolink Cameras

A Rust-based ONVIF proxy that translates non-compliant ONVIF responses from Reolink cameras into spec-compliant responses for NVR systems like iSpy Agent DVR.

## Problem

Reolink cameras have several ONVIF compliance issues:
- Wrong or missing XML namespaces
- Custom event topics for smart detection (person, vehicle, pet detection)
- Missing required ONVIF fields
- Non-standard namespace declarations

These issues prevent proper integration with NVR systems that expect standard ONVIF responses.

## Solution

This proxy acts as a gateway between your NVR and Reolink cameras:

```
iSpy Agent DVR <--ONVIF--> Proxy Gateway <---> Reolink Cameras
                          (Port 8000)          (192.168.x.x)
                          - Fix namespaces
                          - Translate events
                          - Route by camera ID
```

## Features

- **Single Gateway Architecture**: One proxy instance handles multiple cameras
- **Smart Detection Translation**: Converts Reolink's proprietary AI detection events to standard ONVIF events
- **Namespace Fixes**: Automatically adds missing XML namespaces
- **Service Support**:
  - Device Management (GetDeviceInformation, GetCapabilities)
  - Media Services (GetProfiles, GetStreamUri)
  - Events (PullPoint subscriptions, smart detection events)
- **WS-Security Authentication**: Handles ONVIF authentication with cameras
- **Configurable Quirks**: Per-camera translation rules

## Quick Start

### 1. Configure Your Cameras

Edit `config/cameras.yaml`:

```yaml
proxy:
  listen_address: "0.0.0.0:8000"
  base_path: "/onvif"
  log_level: "info"

cameras:
  - id: "camera-01"
    name: "Front Door Camera"
    address: "192.168.1.100:80"
    username: "admin"
    password: "your_password"
    model: "reolink"
    enable_smart_detection: true
    quirks:
      - fix_device_info_namespace
      - normalize_media_profiles
      - translate_smart_events
```

### 2. Build and Run

```bash
# Build the project
cargo build --release

# Run the proxy
cargo run --release

# Or run the binary directly
./target/release/onvif-proxy
```

### 3. Configure Your NVR

Point iSpy Agent DVR (or other NVR) to the proxy instead of the camera directly:

- **Camera URL**: `http://your-proxy-ip:8000/onvif/camera-01/device_service`
- **Username/Password**: Use the same credentials as configured for the camera

## Configuration

### Environment Variables

- `CONFIG_PATH`: Path to configuration file (default: `config/cameras.yaml`)
- `BASE_URL`: Public URL of the proxy (default: `http://{listen_address}`)
- `RUST_LOG`: Logging level (default: `info`)

Example:
```bash
export CONFIG_PATH=/etc/onvif-proxy/cameras.yaml
export BASE_URL=http://192.168.1.50:8000
export RUST_LOG=debug
cargo run --release
```

### Camera Quirks

Available quirks for fixing Reolink issues:

- `fix_device_info_namespace`: Adds missing namespaces to device info responses
- `normalize_media_profiles`: Fixes media profile structure
- `translate_smart_events`: Converts Reolink AI events to ONVIF events
- `add_missing_namespaces`: Adds all common ONVIF namespaces

## URL Structure

The proxy uses camera IDs in the URL path:

```
http://{proxy-host}:8000/onvif/{camera-id}/{service}
```

Examples:
- Device Service: `http://192.168.1.50:8000/onvif/camera-01/device_service`
- Media Service: `http://192.168.1.50:8000/onvif/camera-01/media_service`
- Events Service: `http://192.168.1.50:8000/onvif/camera-01/event_service`

## Smart Detection Events

The proxy translates Reolink's proprietary smart detection events to standard ONVIF motion events:

| Reolink Event | ONVIF Event |
|--------------|-------------|
| `PeopleDetect` | `RuleEngine/CellMotionDetector/Motion` |
| `VehicleDetect` | `RuleEngine/CellMotionDetector/Motion` |
| `DogCatDetect` | `RuleEngine/CellMotionDetector/Motion` |
| `FaceDetect` | `RuleEngine/CellMotionDetector/Motion` |

This allows iSpy Agent DVR to receive and process AI detection events from Reolink cameras.

## Supported ONVIF Operations

### Device Service
- `GetDeviceInformation`
- `GetCapabilities`
- `GetServices`

### Media Service
- `GetProfiles`
- `GetStreamUri`
- `GetSnapshotUri`

### Events Service
- `GetEventProperties`
- `CreatePullPointSubscription`
- `PullMessages`
- `Renew`
- `Unsubscribe`

## Troubleshooting

### Enable Debug Logging

```bash
RUST_LOG=debug cargo run --release
```

### Test Camera Connectivity

```bash
# Test if camera is reachable
curl -X POST http://192.168.1.100:80/onvif/device_service

# Test proxy
curl http://localhost:8000/health
```

### Common Issues

**NVR can't connect to proxy:**
- Ensure the proxy is running and accessible
- Check firewall rules
- Verify the URL format is correct

**Events not working:**
- Enable `translate_smart_events` quirk in camera config
- Check that `enable_smart_detection: true` is set
- Verify camera firmware supports AI detection

**Namespace errors in logs:**
- Add `add_missing_namespaces` to quirks
- Check if camera firmware is up to date

## Development

### Project Structure

```
onvif-proxy/
├── src/
│   ├── main.rs              # Entry point
│   ├── config.rs            # Configuration loading
│   ├── server/              # HTTP server
│   │   ├── http.rs
│   │   └── routes.rs        # Request routing
│   ├── camera/              # Camera management
│   │   ├── manager.rs
│   │   ├── client.rs
│   │   └── config.rs
│   ├── onvif/               # ONVIF protocol
│   │   ├── soap.rs          # SOAP parsing
│   │   ├── auth.rs          # WS-Security
│   │   ├── device.rs
│   │   ├── media.rs
│   │   └── events.rs
│   └── translator/          # Response translation
│       ├── response.rs
│       ├── rules.rs
│       └── reolink.rs       # Reolink-specific fixes
└── config/
    └── cameras.yaml         # Camera configuration
```

### Running Tests

```bash
cargo test
```

### Adding Support for Other Cameras

1. Create a new translator in `src/translator/`
2. Implement the translation logic similar to `reolink.rs`
3. Register it in `response.rs`

## License

MIT

## Contributing

Contributions welcome! Please open an issue or PR.

## Acknowledgments

Built with:
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [Tokio](https://tokio.rs/) - Async runtime
- [Quick-XML](https://github.com/tafia/quick-xml) - XML parsing
