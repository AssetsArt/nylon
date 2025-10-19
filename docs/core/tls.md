# TLS/HTTPS

Nylon provides comprehensive TLS support with automatic certificate management through ACME (Let's Encrypt).

## Quick HTTPS Setup

### Automatic Certificates (Let's Encrypt)

```yaml
# Runtime config
https:
  - 0.0.0.0:443

config_dir: "./config"
acme: "./acme"  # Certificate storage directory
```

```yaml
# Proxy config
tls:
  - domains:
      - example.com
      - www.example.com
    acme:
      email: admin@example.com
      directory_url: https://acme-v02.api.letsencrypt.org/directory

routes:
  - route:
      type: host
      value: example.com
    name: main
    tls:
      enabled: true
    paths:
      - path: /*
        service:
          name: backend
```

That's it! Nylon will:
- Automatically request certificates
- Handle ACME HTTP-01 challenge
- Renew certificates before expiry
- Serve HTTPS traffic

## Manual Certificates

### Using Your Own Certificates

```yaml
tls:
  - domains:
      - example.com
      - www.example.com
    cert: /path/to/cert.pem
    key: /path/to/key.pem
```

### Certificate Formats

Nylon accepts standard PEM-encoded certificates:

```bash
# Certificate file (cert.pem)
-----BEGIN CERTIFICATE-----
...
-----END CERTIFICATE-----

# Private key file (key.pem)
-----BEGIN PRIVATE KEY-----
...
-----END PRIVATE KEY-----
```

## ACME Configuration

### Let's Encrypt Production

```yaml
tls:
  - domains:
      - example.com
    acme:
      email: admin@example.com
      directory_url: https://acme-v02.api.letsencrypt.org/directory
```

### Let's Encrypt Staging (Testing)

```yaml
tls:
  - domains:
      - example.com
    acme:
      email: admin@example.com
      directory_url: https://acme-staging-v02.api.letsencrypt.org/directory
```

**Use staging for testing** to avoid rate limits!

### ACME HTTP-01 Challenge

Nylon automatically handles HTTP-01 challenge:

1. ACME server requests validation at: `http://example.com/.well-known/acme-challenge/{token}`
2. Nylon responds with challenge response
3. Certificate issued after validation

**Requirements:**
- Domain must point to your Nylon server
- Port 80 must be accessible from internet
- HTTP listener must be configured

## Multi-Domain Certificates

### Separate Certificates

```yaml
tls:
  # Certificate for api.example.com
  - domains:
      - api.example.com
    acme:
      email: admin@example.com
      directory_url: https://acme-v02.api.letsencrypt.org/directory

  # Certificate for admin.example.com
  - domains:
      - admin.example.com
    acme:
      email: admin@example.com
      directory_url: https://acme-v02.api.letsencrypt.org/directory
```

### Wildcard Certificates

Wildcard certificates require DNS-01 challenge (not yet supported). Use separate certificates or SAN certificates instead.

```yaml
# NOT YET SUPPORTED
tls:
  - domains:
      - "*.example.com"  # Wildcard not supported yet
```

**Workaround:** List all subdomains:

```yaml
tls:
  - domains:
      - api.example.com
      - admin.example.com
      - app.example.com
    acme:
      email: admin@example.com
      directory_url: https://acme-v02.api.letsencrypt.org/directory
```

## HTTP to HTTPS Redirect

Automatically redirect HTTP requests to HTTPS:

```yaml
routes:
  - route:
      type: host
      value: example.com
    name: main
    tls:
      enabled: true
      redirect: https://example.com  # Redirect HTTP to HTTPS
    paths:
      - path: /*
        service:
          name: backend
```

Requests to `http://example.com/*` → `https://example.com/*`

## Certificate Renewal

### Automatic Renewal

Nylon automatically renews certificates:
- Checks expiry daily
- Renews certificates 30 days before expiry
- No downtime during renewal

### Manual Renewal

Force certificate renewal:

```bash
# Reload configuration (triggers renewal check)
sudo nylon service reload

# Or send SIGHUP
kill -HUP $(cat /var/run/nylon.pid)
```

## Certificate Storage

Certificates are stored in the ACME directory:

```
acme/
├── example.com.cert
├── example.com.key
├── api.example.com.cert
├── api.example.com.key
└── account.json
```

**Keep these files safe!**
- Back up regularly
- Set appropriate permissions: `chmod 600 *.key`
- Don't commit to version control

## SNI (Server Name Indication)

Nylon supports SNI for serving multiple domains on one IP:

```yaml
tls:
  - domains:
      - example.com
    cert: /path/to/example.com.pem
    key: /path/to/example.com.key

  - domains:
      - another.com
    cert: /path/to/another.com.pem
    key: /path/to/another.com.key
```

Nylon automatically selects the correct certificate based on the requested hostname.

## TLS Versions and Ciphers

Nylon uses secure defaults from Pingora:
- **TLS 1.2** and **TLS 1.3** enabled
- Modern cipher suites
- Forward secrecy
- No support for insecure protocols (SSLv3, TLS 1.0, TLS 1.1)

## Examples

### Production Setup

```yaml
# config.yaml
http:
  - 0.0.0.0:80
https:
  - 0.0.0.0:443

config_dir: "/etc/nylon/config"
acme: "/etc/nylon/acme"

pingora:
  threads: 4
```

```yaml
# config/proxy.yaml
tls:
  - domains:
      - example.com
      - www.example.com
    acme:
      email: admin@example.com
      directory_url: https://acme-v02.api.letsencrypt.org/directory

services:
  - name: backend
    service_type: http
    endpoints:
      - ip: 10.0.0.1
        port: 3000

routes:
  - route:
      type: host
      value: example.com
    name: main
    tls:
      enabled: true
      redirect: https://example.com
    paths:
      - path: /*
        service:
          name: backend
```

### Multiple Domains

```yaml
tls:
  - domains:
      - api.example.com
    acme:
      email: api@example.com
      directory_url: https://acme-v02.api.letsencrypt.org/directory

  - domains:
      - admin.example.com
    acme:
      email: admin@example.com
      directory_url: https://acme-v02.api.letsencrypt.org/directory

routes:
  - route:
      type: host
      value: api.example.com
    name: api
    tls:
      enabled: true
    paths:
      - path: /*
        service:
          name: api-service

  - route:
      type: host
      value: admin.example.com
    name: admin
    tls:
      enabled: true
    paths:
      - path: /*
        service:
          name: admin-service
```

### Mixed HTTP/HTTPS

```yaml
routes:
  # HTTPS only
  - route:
      type: host
      value: secure.example.com
    name: secure
    tls:
      enabled: true
    paths:
      - path: /*
        service:
          name: secure-service

  # HTTP only (internal service)
  - route:
      type: host
      value: internal.example.com
    name: internal
    paths:
      - path: /*
        service:
          name: internal-service
```

## Troubleshooting

### Certificate Not Issued

**Check domain DNS:**
```bash
dig example.com
nslookup example.com
```

**Check port 80 accessibility:**
```bash
curl -I http://example.com/.well-known/acme-challenge/test
```

**Check logs:**
```bash
sudo journalctl -u nylon -f
```

### Rate Limiting

Let's Encrypt has rate limits:
- 50 certificates per domain per week
- 5 duplicate certificates per week

**Solution:** Use staging for testing:
```yaml
directory_url: https://acme-staging-v02.api.letsencrypt.org/directory
```

### Certificate Expired

If auto-renewal fails:

```bash
# Remove old certificate
rm acme/example.com.cert acme/example.com.key

# Reload to trigger new request
sudo nylon service reload
```

### SNI Not Working

Ensure:
- Client supports SNI (all modern browsers do)
- Hostname in request matches configured domain
- Certificate includes the requested domain

## Security Best Practices

### 1. Use Production ACME URL

```yaml
# ✅ Good
directory_url: https://acme-v02.api.letsencrypt.org/directory

# ❌ Bad (staging certificates not trusted)
directory_url: https://acme-staging-v02.api.letsencrypt.org/directory
```

### 2. Protect Private Keys

```bash
chmod 600 acme/*.key
chown root:root acme/*.key
```

### 3. Enable HTTPS Redirect

```yaml
tls:
  enabled: true
  redirect: https://example.com  # Force HTTPS
```

### 4. Use HSTS Header

```yaml
middleware:
  - plugin: ResponseHeaderModifier
    payload:
      set:
        - name: strict-transport-security
          value: "max-age=31536000; includeSubDomains"
```

### 5. Monitor Certificate Expiry

Set up monitoring for certificates expiring soon:

```bash
# Check certificate expiry
openssl x509 -in acme/example.com.cert -noout -dates
```

## See Also

- [Configuration](/core/configuration) - TLS configuration reference
- [Routing](/core/routing) - Configure routes with TLS
- [Examples](/examples/basic-proxy) - TLS examples

