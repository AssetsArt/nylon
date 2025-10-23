# Nylon Integration Tests

โฟลเดอร์นี้มี integration tests สำหรับ Nylon โดยเฉพาะ NATS plugin transport

## การรัน Tests

### วิธีที่ 1: ใช้ Test Script (แนะนำ)

Script จะ start NATS server อัตโนมัติและรัน tests:

```bash
./scripts/test-nats.sh
```

### วิธีที่ 2: Manual

1. Start NATS server:
```bash
./scripts/dev-nats.sh
```

2. ในอีก terminal, รัน tests:
```bash
cargo test --test integration --features messaging -- --nocapture
```

### รัน Test เฉพาะชุด

```bash
# Test เฉพาะ NATS plugin
cargo test --test nats_plugin_test --features messaging

# Test เฉพาะ retry logic
cargo test --test retry_logic_test --features messaging

# Test เฉพาะ response filter
cargo test --test response_filter_test --features messaging
```

## Test Suites

### 1. NATS Plugin Tests (`nats_plugin_test.rs`)

Tests พื้นฐานของ NATS plugin:
- ✅ การเชื่อมต่อ NATS
- ✅ Request-reply pattern
- ✅ Queue groups และ load balancing
- ✅ Timeout handling
- ✅ Request filter flow
- ✅ Error handling
- ✅ Multiple phases

### 2. Retry Logic Tests (`retry_logic_test.rs`)

Tests ของ retry mechanism:
- ✅ Retry on timeout
- ✅ Exponential backoff
- ✅ Max retries exceeded
- ✅ Success after failures
- ✅ On-error continue policy

### 3. Response Filter Tests (`response_filter_test.rs`)

Tests ของ response phases:
- ✅ Response filter basic
- ✅ Response body filter
- ✅ Logging phase
- ✅ End action
- ✅ Header modifications
- ✅ Full pipeline simulation
- ✅ Concurrent requests

## Requirements

- NATS server (ติดตั้งผ่าน `brew install nats-server` บน macOS)
- หรือ Docker (scripts จะใช้ Docker ถ้า nats-server ไม่พบ)

## ติดตั้ง NATS Server

### macOS
```bash
brew install nats-server
```

### Linux
```bash
curl -sf https://binaries.nats.dev/nats-io/nats-server/v2@latest | sh
```

### Docker
```bash
docker run -p 4222:4222 -p 8222:8222 nats:latest
```

## Troubleshooting

### NATS server ไม่ start
- ตรวจสอบว่า port 4222 และ 8222 ว่างอยู่: `lsof -i :4222`
- ดู logs: `cat tmp/nats-test.log`

### Tests timeout
- เพิ่มเวลา timeout ใน test cases
- ตรวจสอบว่า NATS server ทำงานปกติ: `curl http://localhost:8222/varz`

### Connection refused
- ตรวจสอบว่า NATS server กำลังทำงาน
- ตรวจสอบ firewall settings

## CI/CD Integration

ใน GitHub Actions หรือ CI อื่นๆ:

```yaml
- name: Start NATS
  run: |
    docker run -d -p 4222:4222 -p 8222:8222 nats:latest
    sleep 2

- name: Run tests
  run: cargo test --test integration --features messaging
```

## Contributing

เมื่อเพิ่ม features ใหม่:
1. เขียน integration tests สำหรับ feature นั้น
2. เพิ่ม test cases ใน test suite ที่เหมาะสม
3. อัปเดต README นี้ด้วยรายการ tests ใหม่
4. รันทุก tests เพื่อให้แน่ใจว่าไม่มี regression

