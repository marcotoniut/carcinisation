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
- `/opt/carcinisation/current/configs/*.env` - per-release instance environment files.
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

Deployment runs on pushes to the `release` branch and can also be started with
`workflow_dispatch`. The workflow builds:

```bash
cargo build --release --locked --bin carcinisation_server --package carcinisation_server
```

The deploy script verifies the local binary is a Linux ELF for the expected
architecture, verifies the remote architecture, uploads the server binary,
assets, and configs to a staging directory, calls the root-owned deploy helper,
installs a new release, atomically switches `/opt/carcinisation/current`, and
restarts only currently active `carcinisation@*.service` units.

## Instances

Per-instance configs live in `deploy/configs/*.env` and are deployed to
`/opt/carcinisation/current/configs/` as part of each release.

Start an instance:

```bash
sudo systemctl enable --now carcinisation@deathmatch.service
```

Stop or restart an instance:

```bash
sudo systemctl stop carcinisation@deathmatch.service
sudo systemctl restart carcinisation@deathmatch.service
```

Add a new instance by adding `deploy/configs/<name>.env` with a unique `PORT`
and absolute `MAP` path, deploying it, then enabling `carcinisation@<name>.service`.

## Logs And Health

```bash
systemctl status carcinisation@deathmatch.service
journalctl -u carcinisation@deathmatch.service -n 200 --no-pager
ss -lunp | grep 7001
```

Open the UDP ports used by enabled instances, for example:

```bash
sudo ufw allow 7001/udp
sudo ufw allow 7002/udp
sudo ufw allow 7003/udp
```

## Rollback

If a restart health check fails during deploy, `deploy/deploy.sh` switches
`/opt/carcinisation/current` back to the previous release and restarts the
previously active instances. Because configs are versioned inside each release,
rollback restores the previous binary, assets, and configs together. The systemd
unit is bootstrap-managed and is not changed by normal app deploys.

Manual rollback:

```bash
ls -1 /opt/carcinisation/releases
sudo ln -sfn /opt/carcinisation/releases/<previous-sha> /opt/carcinisation/current.tmp
sudo mv -Tf /opt/carcinisation/current.tmp /opt/carcinisation/current
sudo systemctl restart 'carcinisation@*.service'
```
