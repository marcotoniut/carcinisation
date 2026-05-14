#!/usr/bin/env bash
set -euo pipefail

BASE="/opt/carcinisation"
LOCK_FILE="/run/lock/carcinisation-deploy.lock"

usage() {
	echo "usage: carcinisation-deploy install <release-id> <staging-dir>" >&2
}

validate_release_id() {
	case "$1" in
	"" | "." | ".." | */* | *\\*) return 1 ;;
	esac
	[[ "$1" =~ ^[A-Za-z0-9._-]+$ ]]
}

validate_staging_dir() {
	case "$1" in
	/tmp/carcinisation-deploy.*) ;;
	*) return 1 ;;
	esac
	[ -d "$1" ]
}

if [ "${EUID:-$(id -u)}" -ne 0 ]; then
	echo "carcinisation-deploy must run as root" >&2
	exit 1
fi

if [ "$#" -ne 3 ] || [ "$1" != "install" ]; then
	usage
	exit 2
fi

release_id="$2"
staging_dir="$3"

if ! validate_release_id "$release_id"; then
	echo "unsafe release id: $release_id" >&2
	exit 1
fi

if ! validate_staging_dir "$staging_dir"; then
	echo "unsafe or missing staging dir: $staging_dir" >&2
	exit 1
fi

for required in "$staging_dir/bin/carcinisation_server" "$staging_dir/bin/carcinisationctl" "$staging_dir/assets" "$staging_dir/configs"; do
	if [ ! -e "$required" ]; then
		echo "staging payload missing: $required" >&2
		exit 1
	fi
done

exec 9>"$LOCK_FILE"
flock -w 300 9

release_dir="${BASE}/releases/${release_id}"
current_link="${BASE}/current"
previous_release=""

if [ -L "$current_link" ]; then
	previous_release="$(readlink -f "$current_link" || true)"
fi

install -d -o root -g root -m 0755 "$BASE" "$BASE/releases" "$BASE/configs"
install -d -o carcinisation -g carcinisation -m 0750 \
	/var/lib/carcinisation /var/cache/carcinisation /var/log/carcinisation
rm -rf "$release_dir"
install -d -o root -g root -m 0755 "$release_dir" "$release_dir/bin" "$release_dir/assets"

rsync -a --delete "$staging_dir/bin/" "$release_dir/bin/"
rsync -a --delete "$staging_dir/assets/" "$release_dir/assets/"
chown -R root:root "$release_dir"
chmod 0755 "$release_dir" "$release_dir/bin" "$release_dir/assets"
chmod 0755 "$release_dir/bin/carcinisation_server" "$release_dir/bin/carcinisationctl"

# Seed config examples — never overwrite existing server-managed configs.
if [ -d "$staging_dir/configs" ]; then
	for example in "$staging_dir/configs"/*.env.example; do
		[ -f "$example" ] || continue
		name="$(basename "$example" .env.example).env"
		target="${BASE}/configs/${name}"
		if [ ! -f "$target" ]; then
			echo "Seeding new config: $name"
			install -o carcinisation -g carcinisation -m 0640 "$example" "$target"
		fi
	done
fi

ln -sfn "$release_dir" "${current_link}.tmp"
mv -Tf "${current_link}.tmp" "$current_link"

mapfile -t active_units < <(systemctl list-units 'carcinisation@*.service' --state=active --plain --no-legend | awk '{print $1}')

if [ "${#active_units[@]}" -eq 0 ]; then
	echo "No active carcinisation instances to restart."
	exit 0
fi

failed_units=()
for unit in "${active_units[@]}"; do
	echo "Restarting $unit"
	if ! systemctl restart "$unit"; then
		failed_units+=("$unit")
	fi
done

# Brief wait for crash-on-startup to manifest before checking health.
sleep 3

for unit in "${active_units[@]}"; do
	if ! systemctl is-active --quiet "$unit"; then
		failed_units+=("$unit")
	fi
done

if [ "${#failed_units[@]}" -eq 0 ]; then
	exit 0
fi

echo "Health check failed for: ${failed_units[*]}" >&2
if [ -n "$previous_release" ] && [ -d "$previous_release" ]; then
	echo "Rolling back current symlink to $previous_release" >&2
	ln -sfn "$previous_release" "${current_link}.tmp"
	mv -Tf "${current_link}.tmp" "$current_link"
	for unit in "${active_units[@]}"; do
		systemctl restart "$unit" || true
	done
fi

exit 1
