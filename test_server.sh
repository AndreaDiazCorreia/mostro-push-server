#!/bin/bash

# Test script for mostro-push-server

echo "🧪 Testing Mostro Push Server"
echo "================================"

# Test 1: Health check
echo ""
echo "1️⃣ Testing /api/health endpoint..."
curl -s http://localhost:8080/api/health | jq . || echo "❌ Health check failed"

# Test 2: Status
echo ""
echo "2️⃣ Testing /api/status endpoint..."
curl -s http://localhost:8080/api/status | jq . || echo "❌ Status check failed"

# Test 3: Register UnifiedPush endpoint
echo ""
echo "3️⃣ Testing /api/register endpoint..."
curl -s -X POST http://localhost:8080/api/register \
  -H "Content-Type: application/json" \
  -d '{"device_id":"test-device-123","endpoint_url":"https://push.example.com/test"}' | jq . || echo "❌ Register failed"

# Test 4: Check if data file was created
echo ""
echo "4️⃣ Checking if endpoints were persisted..."
if [ -f "data/unifiedpush_endpoints.json" ]; then
    echo "✅ Endpoints file exists:"
    cat data/unifiedpush_endpoints.json | jq .
else
    echo "❌ Endpoints file not found"
fi

# Test 5: Unregister endpoint
echo ""
echo "5️⃣ Testing /api/unregister endpoint..."
curl -s -X POST http://localhost:8080/api/unregister \
  -H "Content-Type: application/json" \
  -d '{"device_id":"test-device-123","endpoint_url":"https://push.example.com/test"}' | jq . || echo "❌ Unregister failed"

# Test 6: Test notification
echo ""
echo "6️⃣ Testing /api/test endpoint (will fail - no endpoints registered)..."
curl -s -X POST http://localhost:8080/api/test | jq . || echo "❌ Test notification failed"

echo ""
echo "================================"
echo "✅ Testing complete!"
