# Reolink Camera ONVIF Quirks

This document details the known ONVIF compliance issues with Reolink cameras and how this proxy fixes them.

## Namespace Issues

### Missing XML Namespaces

**Problem:** Reolink cameras often return SOAP responses with missing namespace declarations, even though they use prefixed elements in the response body.

**Example:**
```xml
<SOAP-ENV:Envelope>
  <SOAP-ENV:Body>
    <tds:GetDeviceInformationResponse>
      <tt:Manufacturer>Reolink</tt:Manufacturer>  <!-- tt: prefix used but not declared -->
    </tds:GetDeviceInformationResponse>
  </SOAP-ENV:Body>
</SOAP-ENV:Envelope>
```

**Fix:** The proxy automatically detects missing namespace declarations and adds them:
- `xmlns:tds="http://www.onvif.org/ver10/device/wsdl"`
- `xmlns:trt="http://www.onvif.org/ver10/media/wsdl"`
- `xmlns:tev="http://www.onvif.org/ver10/events/wsdl"`
- `xmlns:tt="http://www.onvif.org/ver10/schema"`
- `xmlns:tns1="http://www.onvif.org/ver10/topics"`

**Quirk Flag:** `fix_device_info_namespace`, `add_missing_namespaces`

## Smart Detection Events

### Custom Event Topics

**Problem:** Reolink uses proprietary event topics for AI-based smart detection features:
- `tns1:RuleEngine/MyRuleDetector/PeopleDetect` (person detection)
- `tns1:RuleEngine/MyRuleDetector/VehicleDetect` (vehicle detection)
- `tns1:RuleEngine/MyRuleDetector/DogCatDetect` (pet detection)

These topics are not part of the ONVIF specification and most NVR systems don't recognize them.

**Fix:** The proxy translates these custom events to standard ONVIF motion detection events:
- All AI detection events → `tns1:RuleEngine/CellMotionDetector/Motion`

This allows NVRs like iSpy Agent DVR to receive and process the smart detection events as motion events.

**Quirk Flag:** `translate_smart_events`

### Missing Event Data Fields

**Problem:** Event messages sometimes lack required SimpleItem fields that NVRs expect, such as:
- `State` (true/false for motion state)
- `IsMotion` (boolean indicating motion detection)

**Fix:** The proxy adds these fields automatically when translating events.

**Quirk Flag:** `translate_smart_events`

## Media Profiles

### Non-standard Profile Structure

**Problem:** Media profiles sometimes have incomplete or non-standard structures, missing fields like:
- Video encoder configuration details
- PTZ configuration (even when camera doesn't have PTZ)

**Fix:** The proxy normalizes profile structures and adds sensible defaults for missing fields.

**Quirk Flag:** `normalize_media_profiles`

## Service URLs (XAddr)

### Internal IP Addresses

**Problem:** Reolink cameras return their internal IP addresses in service URLs (XAddr fields in GetCapabilities response), which may not be accessible from the NVR's network.

**Fix:** The proxy rewrites all XAddr URLs to point back to the proxy itself, ensuring the NVR always communicates through the proxy:

Before:
```xml
<XAddr>http://192.168.1.100/onvif/media_service</XAddr>
```

After:
```xml
<XAddr>http://proxy-ip:8000/onvif/camera-01/media_service</XAddr>
```

**Note:** This is handled automatically for all cameras.

## Available Quirk Flags

Configure these in your `cameras.yaml` file:

### `fix_device_info_namespace`
Adds missing namespace declarations to device information responses.

### `normalize_media_profiles`
Fixes media profile structure and adds missing fields.

### `translate_smart_events`
Translates Reolink AI detection events to standard ONVIF motion events.

### `add_missing_namespaces`
Adds all common ONVIF namespace declarations if any are referenced but missing.

## Recommended Configuration

For most Reolink cameras with smart detection, use:

```yaml
quirks:
  - fix_device_info_namespace
  - normalize_media_profiles
  - translate_smart_events
```

## Testing Smart Detection

To verify smart detection is working:

1. Enable AI detection on your Reolink camera (Person, Vehicle, Pet)
2. Configure the camera in the proxy with `enable_smart_detection: true`
3. Add the `translate_smart_events` quirk
4. In iSpy Agent DVR, set up the camera and enable motion detection
5. Trigger an AI detection event (walk in front of camera)
6. Check iSpy logs - you should see motion events triggered

## Known Limitations

- **Event Types:** All AI detection types (person, vehicle, pet) are mapped to generic motion events. The NVR won't be able to distinguish between them.
- **Event Metadata:** Rich metadata from Reolink's AI (bounding boxes, confidence scores) is not preserved in the translation.
- **Real-time Events:** PullPoint subscriptions work, but WebSocket/WS-BaseNotification are not supported.

## Future Improvements

- Support for preserving AI event types using ONVIF Analytics metadata
- Custom event topic mappings via configuration
- Support for Reolink's HTTP API for richer event data
