# Native Multiplayer Deployment

This deployment path is for the native dedicated multiplayer server only. Browser
multiplayer is not supported yet: browser clients cannot use the current raw
UDP/native socket transport. Future browser multiplayer should be implemented
behind the transport boundary in `crates/carcinisation_net/src/transport.rs`;
WebTransport is preferred eventually, with WebSocket acceptable as an earlier
bridge.

## Server Layout

- `/opt/carcinisation/releases/<git-sha>/` - immutable release directories.
- `/opt/carcinisation/current` - symlink to the active release.
- `/opt/carcinisation/configs/*.env` - persistent per-instance environment files (seeded from examples on first deploy, never overwritten).
- `/run/carcinisation/` - runtime directory for admin sockets (managed by systemd `RuntimeDirectory`).
- `/var/lib/carcinisation/`, `/var/cache/carcinisation/`, `/var/log/carcinisation/` - writable runtime paths.
- `/etc/systemd/system/carcinisation@.service` - systemd template unit.
- `/usr/local/sbin/carcinisation-deploy` - root-owned deploy helper used by CI.

## OS And ABI Baseline

The deploy workflow builds the native server on `ubuntu-latest` and expects an
`x86_64` Linux ELF binary. The server should run on a compatible x86_64 Linux
host with a glibc baseline compatible with the GitHub Actions Ubuntu image. If
the server is ARM or has an older libc baseline, switch the workflow to an
explicit target/container before enabling deploys.

## Fresh Server Bootstrap

Install required packages:

```bash
sudo apt-get update
sudo apt-get install -y file rsync openssh-server systemd sudo util-linux
```

Create the runtime user and base directories:

```bash
sudo groupadd --system carcinisation || true
sudo useradd --system --gid carcinisation --home-dir /var/lib/carcinisation --shell /usr/sbin/nologin carcinisation || true
sudo install -d -o root -g root -m 0755 /opt/carcinisation /opt/carcinisation/releases
sudo install -d -o carcinisation -g carcinisation -m 0750 /var/lib/carcinisation /var/cache/carcinisation /var/log/carcinisation
```

Install the stable systemd unit and root-owned deploy helper during bootstrap,
not during normal app deploys:

```bash
sudo install -o root -g root -m 0644 deploy/carcinisation@.service /etc/systemd/system/carcinisation@.service
sudo install -o root -g root -m 0755 deploy/carcinisation-deploy-helper.sh /usr/local/sbin/carcinisation-deploy
sudo systemctl daemon-reload
```

Create a deploy SSH user, add the GitHub deploy public key to its
`authorized_keys`, and allow only the deploy helper through sudo:

```sudoers
deploy ALL=(root) NOPASSWD: /usr/local/sbin/carcinisation-deploy
```

The deploy user can install new game binaries, assets, and per-release configs
through that helper. It should be treated as production deploy authority, but it
does not get a general root shell.

## GitHub Secrets

- `DEPLOY_HOST` - server hostname or IP.
- `DEPLOY_USER` - SSH deploy user.
- `DEPLOY_PORT` - SSH port, usually `22`.
- `DEPLOY_SSH_KEY` - private key for the deploy user.
- `DEPLOY_KNOWN_HOSTS` - pinned SSH known-hosts entry. For non-22 ports, use the `[host]:port keytype key` format.

Local deploys may use an SSH alias such as `sship` by running
`DEPLOY_REMOTE=sship bash deploy/deploy.sh`. CI must use the explicit secrets
above and must not rely on local shell aliases.

## Deploying

Deployment runs via `just deploy` which cross-compiles both binaries and
uploads them to the remote host. The build step produces:

```bash
cross build --release --target x86_64-unknown-linux-gnu \
  --bin carcinisation_server --package carcinisation_server \
  --bin carcinisationctl --package carcinisationctl
```

The deploy script verifies both binaries are Linux ELF artifacts for the
expected architecture, verifies the remote architecture, uploads both binaries,
assets, and configs to a staging directory, calls the root-owned deploy helper,
installs a new release, atomically switches `/opt/carcinisation/current`, and
restarts only currently active `carcinisation@*.service` units. Both binaries
live under `/opt/carcinisation/current/bin/` and roll back together via the
`current` symlink.

## Instances

Per-instance configs live in `deploy/configs/*.env.example` and are seeded to
`/opt/carcinisation/configs/` on first deploy. Existing server-managed configs
are never overwritten.

Start an instance:

```bash
sudo systemctl enable --now carcinisation@deathmatch.service
```

Stop or restart an instance:

```bash
sudo systemctl stop carcinisation@deathmatch.service
sudo systemctl restart carcinisation@deathmatch.service
```

Add a new instance by adding `deploy/configs/<name>.env.example` with a unique
`PORT`, `INSTANCE_NAME`, `ADMIN_SOCKET`, and absolute `MAP` path. Deploy it,
then enable `carcinisation@<name>.service`.

## Admin Commands

Each server instance exposes a local-only Unix domain socket for administration.
Sockets live at `/run/carcinisation/<instance>.admin.sock` (created by systemd's
`RuntimeDirectory`). The `carcinisationctl` CLI connects to this socket.

Admin commands are local-only. They are not exposed over the public multiplayer
port.

### Usage

```bash
# From the server host (SSH in first):
ssh sship

# Run as the carcinisation user:
sudo -u carcinisation /opt/carcinisation/current/bin/carcinisationctl deathmatch status
sudo -u carcinisation /opt/carcinisation/current/bin/carcinisationctl deathmatch players
sudo -u carcinisation /opt/carcinisation/current/bin/carcinisationctl deathmatch help
sudo -u carcinisation /opt/carcinisation/current/bin/carcinisationctl deathmatch restart
sudo -u carcinisation /opt/carcinisation/current/bin/carcinisationctl deathmatch reset-map
sudo -u carcinisation /opt/carcinisation/current/bin/carcinisationctl deathmatch shutdown
sudo -u carcinisation /opt/carcinisation/current/bin/carcinisationctl deathmatch say "Server restart in 2 minutes"
```

### Available Commands

| Command | Description |
|---------|-------------|
| `help` | Lists available commands |
| `status` | Instance name, port, map, uptime, player/enemy count |
| `players` | Lists connected players (ID, state, health, position) |
| `say <message>` | Not implemented yet (no in-game chat system) |
| `restart` | Exit with non-zero code so systemd `Restart=on-failure` brings it back |
| `reset-map` | Reset gameplay state in-place: despawn enemies/projectiles, respawn enemies, reset players to spawn points. Preserves connections. Uses cached map data from startup — does not re-read the map file from disk. |
| `shutdown` | Graceful server shutdown (clean exit code 0, no auto-restart) |

### Socket Override

```bash
/opt/carcinisation/current/bin/carcinisationctl deathmatch status --socket /tmp/test.admin.sock
```

## Logs And Health

```bash
systemctl status carcinisation@deathmatch.service
journalctl -u carcinisation@deathmatch.service -n 200 --no-pager
ss -lunp | grep 7142
```

Open the UDP ports used by enabled instances, for example:

```bash
sudo ufw allow 7142/udp
sudo ufw allow 7143/udp
sudo ufw allow 7144/udp
```

## Rollback

If a restart health check fails during deploy, `deploy/deploy.sh` switches
`/opt/carcinisation/current` back to the previous release and restarts the
previously active instances. Because configs are versioned inside each release,
rollback restores both binaries, assets, and configs together. The systemd
unit is bootstrap-managed and is not changed by normal app deploys.

Manual rollback:

```bash
ls -1 /opt/carcinisation/releases
sudo ln -sfn /opt/carcinisation/releases/<previous-sha> /opt/carcinisation/current.tmp
sudo mv -Tf /opt/carcinisation/current.tmp /opt/carcinisation/current
sudo systemctl restart 'carcinisation@*.service'
```
