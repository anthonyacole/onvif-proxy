#!/bin/bash
# Test events directly on camera

echo "Creating subscription..."
SUB_RESP=$(curl -s -X POST "http://192.168.30.11:8000/onvif/event_service" -u "stream:111111" --digest -H "Content-Type: application/soap+xml" -d '<?xml version="1.0"?><SOAP-ENV:Envelope xmlns:SOAP-ENV="http://www.w3.org/2003/05/soap-envelope" xmlns:tev="http://www.onvif.org/ver10/events/wsdl"><SOAP-ENV:Body><tev:CreatePullPointSubscription><tev:InitialTerminationTime>PT600S</tev:InitialTerminationTime></tev:CreatePullPointSubscription></SOAP-ENV:Body></SOAP-ENV:Envelope>')

SUB_URL=$(echo "$SUB_RESP" | grep -oP '<wsa5:Address>\K[^<]+' | head -1)
echo "Camera subscription URL: $SUB_URL"

echo "Testing PullMessages..."
curl -s -X POST "$SUB_URL" -u "stream:111111" --digest -H "Content-Type: application/soap+xml" -d '<?xml version="1.0"?><SOAP-ENV:Envelope xmlns:SOAP-ENV="http://www.w3.org/2003/05/soap-envelope" xmlns:tev="http://www.onvif.org/ver10/events/wsdl"><SOAP-ENV:Body><tev:PullMessages><tev:Timeout>PT1S</tev:Timeout><tev:MessageLimit>10</tev:MessageLimit></tev:PullMessages></SOAP-ENV:Body></SOAP-ENV:Envelope>' | xmllint --format - 2>/dev/null | head -30
