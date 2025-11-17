# Getting Started with ONVIF Proxy

This guide will help you set up the ONVIF proxy to integrate your Reolink cameras with iSpy Agent DVR (or other NVR systems).

## Prerequisites

- Rust toolchain installed (1.70 or later)
- Reolink camera(s) with ONVIF support
- Network connectivity between proxy, cameras, and NVR
- OpenSSL development libraries (`libssl-dev` on Ubuntu/Debian)

## Installation

### 1. Install Dependencies

**Ubuntu/Debian:**
```bash
sudo apt-get update
sudo apt-get install -y libssl-dev pkg-config build-essential
```

**Other Systems:**
- Make sure you have OpenSSL development libraries installed
- Install the Rust toolchain from https://rustup.rs/

### 2. Clone and Build

```bash
cd /path/to/onvif-proxy
cargo build --release
```

The compiled binary will be at `./target/release/onvif-proxy`

## Configuration

### 1. Create Configuration File

Copy the example configuration:

```bash
cp config/cameras.yaml.example config/cameras.yaml
```

### 2. Edit Configuration

Edit `config/cameras.yaml` with your camera details:

```yaml
proxy:
  listen_address: "0.0.0.0:8000"  # IP and port the proxy listens on
  base_path: "/onvif"
  log_level: "info"               # debug, info, warn, error

cameras:
  - id: "front-door"              # Unique ID for this camera
    name: "Front Door Camera"     # Human-readable name
    address: "192.168.1.100:80"   # Camera IP address and port
    username: "admin"             # Camera ONVIF username
    password: "your_password"     # Camera ONVIF password
    model: "reolink"              # Camera model (currently only "reolink")
    enable_smart_detection: true  # Enable AI detection event translation
    quirks:                       # Fixes to apply
      - fix_device_info_namespace
      - normalize_media_profiles
      - translate_smart_events
```

**Important Notes:**
- Each camera needs a unique `id`
- The `address` should include the port (usually 80 for Reolink)
- Use the ONVIF credentials, which may differ from the web UI credentials
- Keep `config/cameras.yaml` secure - it contains passwords

### 3. Test Camera Connectivity

Before starting the proxy, verify you can reach your cameras:

```bash
# Test if camera responds to ONVIF requests
curl -X POST http://192.168.1.100:80/onvif/device_service \
  -H "Content-Type: application/soap+xml" \
  -d '<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
<s:Body><GetDeviceInformation xmlns="http://www.onvif.org/ver10/device/wsdl"/></s:Body>
</s:Envelope>'
```

You should get an XML response with device information.

## Running the Proxy

### Development Mode

```bash
cargo run --release
```

### Production Mode

```bash
# Run the compiled binary
./target/release/onvif-proxy

# Or with custom config path
CONFIG_PATH=/etc/onvif-proxy/cameras.yaml ./target/release/onvif-proxy

# With custom base URL (if proxy is behind a reverse proxy)
BASE_URL=http://my-proxy.example.com:8000 ./target/release/onvif-proxy

# With debug logging
RUST_LOG=debug ./target/release/onvif-proxy
```

### Running as a Service (systemd)

Create `/etc/systemd/system/onvif-proxy.service`:

```ini
[Unit]
Description=ONVIF Proxy for Reolink Cameras
After=network.target

[Service]
Type=simple
User=onvif
WorkingDirectory=/opt/onvif-proxy
ExecStart=/opt/onvif-proxy/target/release/onvif-proxy
Environment="CONFIG_PATH=/etc/onvif-proxy/cameras.yaml"
Environment="BASE_URL=http://your-proxy-ip:8000"
Environment="RUST_LOG=info"
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable onvif-proxy
sudo systemctl start onvif-proxy
sudo systemctl status onvif-proxy
```

## Configuring iSpy Agent DVR

### 1. Add Camera

1. Open iSpy Agent DVR
2. Click "+" to add a new camera
3. Select "ONVIF" as the source type

### 2. Configure Connection

- **ONVIF URL:** `http://proxy-ip:8000/onvif/front-door/device_service`
  - Replace `proxy-ip` with your proxy's IP address
  - Replace `front-door` with your camera's ID from config
- **Username:** Same as configured in `cameras.yaml`
- **Password:** Same as configured in `cameras.yaml`

### 3. Test Connection

Click "Test" or "Get Profiles" - you should see:
- Device information retrieved successfully
- Media profiles listed
- Stream URLs available

### 4. Enable Events/Motion Detection

1. In camera settings, go to "Alerts" tab
2. Enable "Motion Detection"
3. Select "ONVIF Events" as the motion detection method
4. The camera should now receive motion events from Reolink's AI detection

## Verification

### Check Proxy is Running

```bash
curl http://localhost:8000/health
# Should return: OK
```

### Check Logs

```bash
# If running with cargo
cargo run --release

# If running as service
sudo journalctl -u onvif-proxy -f

# Look for lines like:
# INFO  onvif_proxy] Starting ONVIF Proxy for Reolink Cameras
# INFO  onvif_proxy] Added camera: front-door
# INFO  onvif_proxy] Starting ONVIF proxy server on 0.0.0.0:8000
```

### Test ONVIF Operations

```bash
# Get device information
curl -X POST http://localhost:8000/onvif/front-door/device_service \
  -H "Content-Type: application/soap+xml" \
  -d '<SOAP-ENV:Envelope xmlns:SOAP-ENV="http://www.w3.org/2003/05/soap-envelope">
<SOAP-ENV:Body><tds:GetDeviceInformation xmlns:tds="http://www.onvif.org/ver10/device/wsdl"/>
</SOAP-ENV:Body></SOAP-ENV:Envelope>'
```

## Troubleshooting

### Proxy Won't Start

**Error:** `Failed to bind to address`
- Another service is using port 8000
- Change `listen_address` in config or stop the conflicting service

**Error:** `Failed to load configuration`
- Check YAML syntax in `cameras.yaml`
- Verify file path is correct

### Camera Not Found

**Error:** `Camera not found: xyz`
- Verify camera ID matches exactly in config
- Check that config file is loaded correctly

### NVR Can't Connect

1. **Verify proxy is reachable:**
   ```bash
   # From NVR machine
   curl http://proxy-ip:8000/health
   ```

2. **Check firewall rules:**
   - Allow inbound TCP port 8000 on proxy

3. **Verify URL format:**
   - Should be: `http://proxy-ip:8000/onvif/{camera-id}/device_service`
   - Not: `http://camera-ip:80/onvif/device_service`

### Events Not Working

1. **Enable debug logging:**
   ```bash
   RUST_LOG=debug cargo run --release
   ```

2. **Check camera supports AI detection:**
   - Enable Person/Vehicle/Pet detection in Reolink app
   - Verify `enable_smart_detection: true` in config
   - Add `translate_smart_events` to quirks

3. **Check event subscription:**
   - Look for log lines about CreatePullPointSubscription
   - Verify PullMessages requests are happening

### Camera Authentication Fails

**Error:** `401 Unauthorized` in logs
- Verify username/password are correct
- Try ONVIF credentials (may differ from web UI)
- Check camera has ONVIF enabled

## Next Steps

- Read [REOLINK_QUIRKS.md](REOLINK_QUIRKS.md) for details on fixes applied
- Set up multiple cameras by adding more entries to `cameras.yaml`
- Configure reverse proxy (nginx/Apache) if needed
- Set up SSL/TLS for secure communication

## Getting Help

- Check logs with `RUST_LOG=debug`
- Verify camera firmware is up to date
- Review [REOLINK_QUIRKS.md](REOLINK_QUIRKS.md) for known issues
- Open an issue on GitHub with debug logs
