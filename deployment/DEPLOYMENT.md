# Deployment Guide

This guide covers deploying SpacetimeDB on a Linux server for Sunaba multiplayer.

## Server Requirements

- Linux server with public IPv4/IPv6 address
- SpacetimeDB CLI installed (latest version)
- Open firewall port 3000 (or your chosen port)
- Optionally: Nginx/Caddy for reverse proxy with TLS

## Quick Setup (systemd user service)

### 1. Create data directory

```bash
mkdir -p ~/spacetimedb
```

### 2. Install systemd service

```bash
mkdir -p ~/.config/systemd/user
cp deployment/spacetimedb.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable spacetimedb.service
```

### 3. Start the service

```bash
systemctl --user start spacetimedb.service
```

### 4. Check status

```bash
systemctl --user status spacetimedb.service
journalctl --user -u spacetimedb.service -f
```

### 5. Enable linger (keep service running after logout)

```bash
sudo loginctl enable-linger $USER
```

## Firewall Configuration

### Open port 3000 for SpacetimeDB

**UFW (Ubuntu/Debian):**
```bash
sudo ufw allow 3000/tcp
sudo ufw status
```

**firewalld (RHEL/CentOS/Fedora):**
```bash
sudo firewall-cmd --permanent --add-port=3000/tcp
sudo firewall-cmd --reload
sudo firewall-cmd --list-ports
```

**iptables:**
```bash
sudo iptables -A INPUT -p tcp --dport 3000 -j ACCEPT
sudo iptables-save | sudo tee /etc/iptables/rules.v4
```

## Publishing Your Module

### 1. Build the server module

On your development machine:
```bash
just spacetime-build
# or manually:
# spacetime build -p crates/sunaba-server
```

### 2. Publish to your production server

```bash
cd crates/sunaba-server
spacetime publish -s http://YOUR_SERVER_IP:3000 sunaba-server
```

### 3. Verify deployment

```bash
# Check server logs
journalctl --user -u spacetimedb.service -f

# Query the database
spacetime sql sunaba-server --server http://YOUR_SERVER_IP:3000 "SELECT * FROM world_config"

# Test spawning a creature
spacetime call sunaba-server spawn_creature --server http://YOUR_SERVER_IP:3000 -- spider 0.0 100.0
```

## Network Configuration

### Listen Address

The systemd service is configured to bind to `0.0.0.0:3000`, which listens on all network interfaces (IPv4 and IPv6).

**Default ports:**
- `3000` - HTTP API + WebSocket connections (default)

**Custom port:**
Edit `deployment/spacetimedb.service` and change:
```ini
ExecStart=/usr/local/bin/spacetime start --listen-addr=0.0.0.0:YOUR_PORT
```

Then reload systemd:
```bash
systemctl --user daemon-reload
systemctl --user restart spacetimedb.service
```

## TLS/HTTPS Setup (Production Recommended)

For production, use a reverse proxy with TLS:

### Option 1: Nginx + Let's Encrypt

```nginx
# /etc/nginx/sites-available/spacetimedb
server {
    listen 80;
    listen [::]:80;
    server_name your-domain.com;

    # Redirect to HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    listen [::]:443 ssl http2;
    server_name your-domain.com;

    # SSL certificates (Let's Encrypt)
    ssl_certificate /etc/letsencrypt/live/your-domain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/your-domain.com/privkey.pem;

    # Proxy to SpacetimeDB
    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;

        # WebSocket support
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Timeouts for long-lived connections
        proxy_read_timeout 86400;
        proxy_send_timeout 86400;
    }
}
```

Enable and get certificate:
```bash
sudo ln -s /etc/nginx/sites-available/spacetimedb /etc/nginx/sites-enabled/
sudo certbot --nginx -d your-domain.com
sudo systemctl reload nginx
```

**Update systemd service to bind to localhost only:**
```ini
ExecStart=/usr/local/bin/spacetime start --listen-addr=127.0.0.1:3000
```

### Option 2: Caddy (Automatic TLS)

Caddy automatically handles HTTPS with Let's Encrypt:

```caddyfile
# /etc/caddy/Caddyfile
your-domain.com {
    reverse_proxy localhost:3000
}
```

```bash
sudo systemctl restart caddy
```

## Authentication & Security

### SpacetimeDB Identity System

SpacetimeDB uses **cryptographic identity-based authentication**:

1. **Client Identity**: Each client generates a public/private key pair
2. **No shared secrets**: No server-side passwords or API keys required
3. **Reducer authorization**: Use `ctx.sender` in reducers to check caller identity

### Example: Restrict admin actions

```rust
#[spacetimedb::reducer]
pub fn admin_reset_world(ctx: &ReducerContext) -> Result<(), String> {
    // Hardcode admin identity (get from ctx.sender when first connecting)
    let admin_identity = Identity::from_hex("YOUR_ADMIN_IDENTITY_HEX").unwrap();

    if ctx.sender != admin_identity {
        return Err("Unauthorized: admin only".to_string());
    }

    // Reset world logic
    Ok(())
}
```

### Default Access Control

**By default, SpacetimeDB is open:**
- Anyone can publish modules
- Anyone can call reducers
- Anyone can subscribe to tables

**To restrict access in production:**

1. **Use Nginx to block publishing** (allow only specific IPs):
```nginx
# Allow publishing only from your dev machine
location ~ ^/database/.*/publish$ {
    allow YOUR_DEV_IP;
    deny all;
    proxy_pass http://127.0.0.1:3000;
}

# Allow everyone to subscribe and call reducers
location / {
    proxy_pass http://127.0.0.1:3000;
}
```

2. **Implement authorization in reducers** (check `ctx.sender`)

3. **Use environment-based secrets** for sensitive operations

### Getting Your Identity

When the client connects, log the identity:
```rust
log::info!("Connected with identity: {}", client.identity());
```

Or query via CLI:
```bash
spacetime identity list
```

## Monitoring

### View logs
```bash
journalctl --user -u spacetimedb.service -f
```

### Check service status
```bash
systemctl --user status spacetimedb.service
```

### Restart service
```bash
systemctl --user restart spacetimedb.service
```

### Stop service
```bash
systemctl --user stop spacetimedb.service
```

### Disable auto-start
```bash
systemctl --user disable spacetimedb.service
```

## Backup & Data Location

SpacetimeDB stores data in `~/spacetimedb/` by default.

**Backup:**
```bash
# Stop the service
systemctl --user stop spacetimedb.service

# Backup data directory
tar -czf spacetimedb-backup-$(date +%Y%m%d).tar.gz ~/spacetimedb/

# Restart service
systemctl --user start spacetimedb.service
```

**Custom data directory:**
Edit the service file and change `WorkingDirectory` and add `--root-dir` flag:
```ini
WorkingDirectory=/path/to/custom/data
ExecStart=/usr/local/bin/spacetime start --root-dir=/path/to/custom/data --listen-addr=0.0.0.0:3000
```

## Updating SpacetimeDB

```bash
# Stop service
systemctl --user stop spacetimedb.service

# Update CLI (if installed via cargo)
cargo install spacetimedb-cli

# Or download latest binary
# https://github.com/clockworklabs/SpacetimeDB/releases

# Restart service
systemctl --user start spacetimedb.service
```

## Troubleshooting

### Port already in use
```bash
sudo lsof -i :3000
# Kill the process or change port in systemd service
```

### Service won't start
```bash
journalctl --user -u spacetimedb.service -n 50
# Check for errors in the logs
```

### Can't connect from web client
- Verify firewall allows port 3000
- Check server is listening: `sudo netstat -tulpn | grep 3000`
- Test connection: `curl http://YOUR_SERVER_IP:3000/`
- Check CORS if using reverse proxy

### Permission denied
```bash
# Ensure data directory exists and is writable
mkdir -p ~/spacetimedb
chmod 755 ~/spacetimedb
```

## Next Steps

After the server is running:
1. Test connection from your dev machine: `spacetime sql sunaba-server --server http://YOUR_SERVER_IP:3000 "SELECT 1"`
2. Update web client connection URL (see `crates/sunaba/src/app.rs:230`)
3. Deploy web client to GitHub Pages
4. Test multiplayer gameplay

## Resources

- [SpacetimeDB Docs](https://spacetimedb.com/docs/)
- [Self-Hosting Guide](https://spacetimedb.com/docs/deploying/spacetimedb-standalone/)
- [Configuration Reference](https://spacetimedb.com/docs/cli-reference/standalone-config/)

Sources:
- [Self-hosting | SpacetimeDB docs](https://spacetimedb.com/docs/deploying/spacetimedb-standalone/)
- [Standalone Configuration | SpacetimeDB docs](https://spacetimedb.com/docs/cli-reference/standalone-config/)
- [GitHub - clockworklabs/SpacetimeDB](https://github.com/clockworklabs/SpacetimeDB)
