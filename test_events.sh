#!/bin/bash
# ONVIF Events Diagnostic Script for Reolink RLC-810A
# This script tests event functionality step by step

PROXY_URL="http://localhost:8080/onvif/camera-01"
USERNAME="stream"
PASSWORD="111111"

echo "===== ONVIF Events Diagnostic Test ====="
echo ""

# Test 1: GetEventProperties
echo "Test 1: GetEventProperties - Check what events the camera supports"
echo "--------------------------------------------------------------------"
curl -s -X POST "${PROXY_URL}/event_service" \
  -H "Content-Type: application/soap+xml; charset=utf-8" \
  -d '<?xml version="1.0" encoding="UTF-8"?>
<SOAP-ENV:Envelope xmlns:SOAP-ENV="http://www.w3.org/2003/05/soap-envelope" xmlns:tev="http://www.onvif.org/ver10/events/wsdl">
<SOAP-ENV:Body>
<tev:GetEventProperties/>
</SOAP-ENV:Body>
</SOAP-ENV:Envelope>' | xmllint --format - 2>/dev/null || cat
echo ""
echo ""

# Test 2: CreatePullPointSubscription
echo "Test 2: CreatePullPointSubscription - Create an event subscription"
echo "--------------------------------------------------------------------"
SUBSCRIPTION_RESPONSE=$(curl -s -X POST "${PROXY_URL}/event_service" \
  -H "Content-Type: application/soap+xml; charset=utf-8" \
  -d '<?xml version="1.0" encoding="UTF-8"?>
<SOAP-ENV:Envelope xmlns:SOAP-ENV="http://www.w3.org/2003/05/soap-envelope" xmlns:tev="http://www.onvif.org/ver10/events/wsdl">
<SOAP-ENV:Body>
<tev:CreatePullPointSubscription>
  <tev:InitialTerminationTime>PT600S</tev:InitialTerminationTime>
</tev:CreatePullPointSubscription>
</SOAP-ENV:Body>
</SOAP-ENV:Envelope>')

echo "$SUBSCRIPTION_RESPONSE" | xmllint --format - 2>/dev/null || echo "$SUBSCRIPTION_RESPONSE"
echo ""

# Extract subscription URL (supports both <Address> and <wsa5:Address>)
SUB_URL=$(echo "$SUBSCRIPTION_RESPONSE" | grep -oP '<(wsa5:|wsa:)?Address>\K[^<]+' | head -1)
if [ -n "$SUB_URL" ]; then
    echo "Subscription created: $SUB_URL"
else
    echo "ERROR: Could not create subscription"
    echo "Response was: $SUBSCRIPTION_RESPONSE"
    exit 1
fi
echo ""

# Test 3: PullMessages (immediate)
echo "Test 3: PullMessages - Pull any pending events (no wait)"
echo "--------------------------------------------------------------------"
curl -s -X POST "$SUB_URL" \
  -H "Content-Type: application/soap+xml; charset=utf-8" \
  -d '<?xml version="1.0" encoding="UTF-8"?>
<SOAP-ENV:Envelope xmlns:SOAP-ENV="http://www.w3.org/2003/05/soap-envelope" xmlns:tev="http://www.onvif.org/ver10/events/wsdl">
<SOAP-ENV:Body>
<tev:PullMessages>
  <tev:Timeout>PT0S</tev:Timeout>
  <tev:MessageLimit>10</tev:MessageLimit>
</tev:PullMessages>
</SOAP-ENV:Body>
</SOAP-ENV:Envelope>' | xmllint --format - 2>/dev/null || cat
echo ""
echo ""

# Test 4: PullMessages with wait
echo "Test 4: PullMessages - Wait for events (5 second timeout)"
echo "--------------------------------------------------------------------"
echo "Waiting for events... (trigger motion/AI detection on camera now)"
curl -s -X POST "$SUB_URL" \
  -H "Content-Type: application/soap+xml; charset=utf-8" \
  -d '<?xml version="1.0" encoding="UTF-8"?>
<SOAP-ENV:Envelope xmlns:SOAP-ENV="http://www.w3.org/2003/05/soap-envelope" xmlns:tev="http://www.onvif.org/ver10/events/wsdl">
<SOAP-ENV:Body>
<tev:PullMessages>
  <tev:Timeout>PT5S</tev:Timeout>
  <tev:MessageLimit>10</tev:MessageLimit>
</tev:PullMessages>
</SOAP-ENV:Body>
</SOAP-ENV:Envelope>' | xmllint --format - 2>/dev/null || cat
echo ""
echo ""

echo "===== Diagnostic Complete ====="
echo ""
echo "Next Steps:"
echo "1. Check if GetEventProperties shows supported event topics"
echo "2. If no events received in Test 4, check camera settings:"
echo "   - Enable AI detection (Person/Vehicle/Pet) in camera web UI"
echo "   - Enable 'Push Notifications' or 'ONVIF Events' if available"
echo "   - Trigger motion by waving in front of camera"
echo "3. Check proxy debug logs: tail -f /tmp/proxy_debug.log"
