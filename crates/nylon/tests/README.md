# Nylon Integration Tests

โฟลเดอร์นี้มี integration tests สำหรับ Nylon โดยเฉพาะ NATS plugin transport

## การรัน Tests

### วิธีที่ 1: Manual

1. Start NATS server:
```bash
# Using Homebrew (macOS)
brew install nats-server
nats-server --jetstream

# หรือใช้ Docker
docker run -p 4222:4222 -p 8222:8222 nats:latest
```

2. ในอีก terminal, รัน tests:
```bash
cargo test --package nylon --test integration --features messaging
```

### รัน Test เฉพาะชุด

```bash
# Test เฉพาะ basic NATS plugin
cargo test --package nylon --test integration --features messaging nats_basic_test

# Test เฉพาะ read methods
cargo test --package nylon --test integration --features messaging read_methods_test

# รัน test เฉพาะ function
cargo test --package nylon --test integration --features messaging test_nats_request_reply -- --nocapture
```

## Test Suites

### 1. Basic NATS Plugin Tests (`nats_basic_test.rs`)

Tests พื้นฐานของ NATS plugin - **9 tests**:
- ✅ `test_nats_connection` - การเชื่อมต่อ NATS
- ✅ `test_nats_request_reply` - Request-reply pattern
- ✅ `test_nats_queue_groups` - Queue groups และ load balancing
- ✅ `test_nats_timeout_handling` - Timeout handling
- ✅ `test_plugin_request_filter_flow` - Plugin request filter flow
- ✅ `test_plugin_error_handling` - Error handling
- ✅ `test_plugin_multiple_phases` - Multiple phases (request_filter, response_filter, response_body_filter, logging)
- ✅ `test_retry_on_slow_worker` - Retry mechanism
- ✅ `test_concurrent_requests` - Concurrent requests

### 2. Read Methods Tests (`read_methods_test.rs`)

Tests ของ read methods - **9 tests**:
- ✅ `test_read_request_methods` - อ่านข้อมูล request (URL, path, query, host, method, client_ip)
- ✅ `test_read_response_methods` - อ่านข้อมูล response (status, headers, bytes)
- ✅ `test_get_payload_method` - GET_PAYLOAD method
- ✅ `test_read_request_headers` - อ่าน request headers
- ✅ `test_read_request_body` - อ่าน request body
- ✅ `test_read_request_params` - อ่าน route parameters
- ✅ `test_read_response_status` - อ่าน response status
- ✅ `test_read_response_duration` - อ่าน response duration
- ✅ `test_read_methods_concurrent` - Concurrent read methods

**Total: 18 tests passing**

## Requirements

- NATS server running on `localhost:4222`
- Rust with `messaging` feature enabled

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
docker run -d -p 4222:4222 -p 8222:8222 --name nats nats:latest
```

## Test Implementation Details

### Helper Functions (`test_helpers.rs`)

- `create_test_client(prefix)` - สร้าง NATS client พร้อม subject prefix
- `test_request(client, subject, payload)` - ส่ง request และรอ response
- `subscribe_and_respond(client, subject, queue_group, handler)` - สร้าง mock worker
- `subscribe_with_delay(...)` - สร้าง mock worker ที่มี delay
- `wait_for_workers()` - รอให้ workers พร้อม (500ms)

### Key Implementation Notes

1. **Reply Subject Handling**: ใช้ raw NATS client (`client.client()`) สำหรับส่ง reply เพื่อหลีกเลี่ยง subject prefix expansion ของ `_INBOX.*` subjects

2. **Worker Pattern**: Workers ใช้ queue groups สำหรับ load balancing และรับข้อความผ่าน `subscribe_queue()`

3. **Async Pattern**: ทุก tests เป็น `#[tokio::test]` และใช้ `async/await`

## Troubleshooting

### NATS server ไม่ start
- ตรวจสอบว่า port 4222 และ 8222 ว่างอยู่: `lsof -i :4222`
- Kill process ที่ใช้ port: `kill -9 $(lsof -t -i:4222)`

### Tests timeout
- เพิ่ม delay ใน `wait_for_workers()` ใน `test_helpers.rs`
- ตรวจสอบว่า NATS server ทำงานปกติ: `curl http://localhost:8222/varz`

### Connection refused
- ตรวจสอบว่า NATS server กำลังทำงาน: `ps aux | grep nats-server`
- ตรวจสอบ firewall settings

### Worker ไม่ได้รับข้อความ
- เพิ่ม debug output ด้วย `-- --nocapture`
- ตรวจสอบ subject names และ queue groups

## CI/CD Integration

ใน GitHub Actions หรือ CI อื่นๆ:

```yaml
- name: Start NATS
  run: |
    docker run -d -p 4222:4222 -p 8222:8222 nats:latest
    sleep 2

- name: Run integration tests
  run: cargo test --package nylon --test integration --features messaging
```

## Performance

Tests ปัจจุบันรันเสร็จภายใน ~1.2 วินาที (18 tests) โดย:
- Connection tests: ~100ms
- Request-reply tests: ~200-300ms
- Concurrent tests: ~500ms

## Contributing

เมื่อเพิ่ม features ใหม่:
1. เขียน integration tests สำหรับ feature นั้น
2. เพิ่ม test cases ใน test suite ที่เหมาะสม
3. อัปเดต README นี้ด้วยรายการ tests ใหม่
4. รันทุก tests เพื่อให้แน่ใจว่าไม่มี regression

## Test Coverage

Current coverage:
- ✅ Basic NATS functionality (connection, pub/sub, queue groups)
- ✅ Plugin lifecycle (initialize, shutdown)
- ✅ All plugin phases (request_filter, response_filter, response_body_filter, logging)
- ✅ Control methods (NEXT, END)
- ✅ Write methods (SET_RESPONSE_*)
- ✅ Read methods (GET_PAYLOAD, READ_REQUEST_*, READ_RESPONSE_*)
- ✅ Error handling and retry logic
- ✅ Concurrent request handling
- ⏳ Metrics and observability (pending)
- ⏳ Load testing and benchmarks (pending)
- ⏳ Failure simulations (pending)
- ❌ WebSocket methods (not yet implemented)
