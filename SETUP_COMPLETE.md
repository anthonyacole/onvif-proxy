# ONVIF Proxy Setup Complete! 🎉

Your ONVIF proxy for Reolink cameras is now built and running.

## Current Status

✓ **Proxy Running** on port 8080
✓ **Camera Configured**: Reolink Camera at 192.168.30.11:8000
✓ **Smart Detection Translation** enabled

## Quick Access URLs

Your camera is accessible through the proxy at:

### For iSpy Agent DVR Configuration:

**Device Service URL:**
```
http://YOUR_PROXY_IP:8080/onvif/camera-01/device_service
```

**Credentials:**
- Username: `stream`
- Password: `111111`

Replace `YOUR_PROXY_IP` with the IP address of this machine (192.168.1.194 based on auto-detection).

### Service Endpoints:

- **Health Check:** http://localhost:8080/health
- **Device Service:** http://localhost:8080/onvif/camera-01/device_service
- **Media Service:** http://localhost:8080/onvif/camera-01/media_service
- **Events Service:** http://localhost:8080/onvif/camera-01/event_service

## Testing the Proxy

### 1. Test Connection to Camera

```bash
# Test if proxy can reach your camera
curl -X POST http://localhost:8080/onvif/camera-01/device_service \
  -H "Content-Type: application/soap+xml" \
  -d '<SOAP-ENV:Envelope xmlns:SOAP-ENV="http://www.w3.org/2003/05/soap-envelope">
<SOAP-ENV:Body><tds:GetDeviceInformation xmlns:tds="http://www.onvif.org/ver10/device/wsdl"/>
</SOAP-ENV:Body></SOAP-ENV:Envelope>'
```

You should see an XML response with camera information.

### 2. View Live Logs

```bash
# In another terminal, view logs in real-time
tail -f /var/log/syslog | grep onvif
# Or if running with ./run.sh, logs are printed to stdout
```

## Configure iSpy Agent DVR

### Step 1: Add Camera

1. Open iSpy Agent DVR web interface
2. Click "+" to add a new device
3. Select "ONVIF" as the source type

### Step 2: Enter Connection Details

- **Name:** Reolink Camera
- **ONVIF URL:** `http://192.168.1.194:8080/onvif/camera-01/device_service`
- **Username:** `stream`
- **Password:** `111111`
- Click "Test Connection" - should succeed

### Step 3: Configure Video Stream

- Click "Get Profiles" to retrieve available video profiles
- Select the desired profile (usually "Profile_0" or "Profile_1")
- iSpy will automatically configure the RTSP stream URL

### Step 4: Enable Motion Detection (Smart Events)

1. Go to camera settings → "Alerts" tab
2. Enable "Motion Detection"
3. Select "ONVIF Events" as the trigger source
4. Configure alert actions as desired

Now your Reolink's AI detection (person, vehicle, pet) will trigger motion events in iSpy!

## Managing the Proxy

### Start Proxy (Manual)

```bash
cd /home/acole/onvif-proxy
./run.sh
```

### Stop Proxy

```bash
# Find the process
ps aux | grep onvif-proxy

# Kill it
kill <PID>
```

### Run with Debug Logging

```bash
RUST_LOG=debug ./run.sh
```

### Run as Background Service

See `docs/GETTING_STARTED.md` for systemd service setup instructions.

## Adding More Cameras

Edit `config/cameras.yaml` and add additional cameras:

```yaml
cameras:
  - id: "camera-01"
    name: "Reolink Camera"
    address: "192.168.30.11:8000"
    username: "stream"
    password: "111111"
    model: "reolink"
    enable_smart_detection: true
    quirks:
      - fix_device_info_namespace
      - normalize_media_profiles
      - translate_smart_events

  - id: "camera-02"
    name: "Another Camera"
    address: "192.168.30.12:8000"
    username: "stream"
    password: "password"
    model: "reolink"
    enable_smart_detection: true
    quirks:
      - fix_device_info_namespace
      - translate_smart_events
```

Then restart the proxy.

## What the Proxy Does

### ONVIF Compliance Fixes

✓ **Namespace Fixes:** Adds missing XML namespace declarations
✓ **Smart Event Translation:** Converts Reolink's AI events to standard ONVIF motion
✓ **URL Rewriting:** Makes all services accessible through the proxy
✓ **Media Profile Normalization:** Ensures profiles have all required fields

### Event Translation

| Reolink Event | Translated To |
|--------------|---------------|
| Person Detection | ONVIF Motion Event |
| Vehicle Detection | ONVIF Motion Event |
| Pet Detection | ONVIF Motion Event |

This allows iSpy Agent DVR to receive and process smart detection events.

## Troubleshooting

### iSpy Can't Connect

- Verify proxy is running: `curl http://localhost:8080/health`
- Check firewall allows port 8080
- Confirm URL format: `http://IP:8080/onvif/camera-01/device_service`

### No Smart Detection Events

- Ensure AI detection is enabled in Reolink app
- Verify `enable_smart_detection: true` in config
- Check `translate_smart_events` is in quirks list
- View proxy logs: `RUST_LOG=debug ./run.sh`

### Camera Authentication Fails

- Verify username/password match camera ONVIF credentials
- Test direct camera access with curl
- Check camera's ONVIF port (you're using 8000)

## Next Steps

1. **Test with iSpy:** Add the camera using the URLs above
2. **Monitor Logs:** Watch for any errors or warnings
3. **Test Smart Detection:** Walk in front of camera and verify motion triggers
4. **Set up as Service:** Make the proxy start automatically (see docs/GETTING_STARTED.md)

## Documentation

- **Getting Started Guide:** `docs/GETTING_STARTED.md`
- **Reolink Quirks Explained:** `docs/REOLINK_QUIRKS.md`
- **Full README:** `README.md`

## Support

If you encounter issues:

1. Enable debug logging: `RUST_LOG=debug ./run.sh`
2. Check proxy logs for error messages
3. Verify camera firmware is up to date
4. Test direct ONVIF access to camera

---

**Your proxy is ready to use!** 🚀

Point iSpy Agent DVR to `http://192.168.1.194:8080/onvif/camera-01/device_service` and enjoy your Reolink smart detection integration!
