#!/bin/sh
# preremove: stop and disable the units on actual removal, but keep the
# service running across an upgrade.
#
# Argument semantics differ across package formats:
#   RPM     $1 = 0 on remove, 1 on upgrade.
#   deb     $1 = "remove" on remove, "upgrade" on upgrade (and "failed-upgrade"
#           / "deconfigure" in edge cases).
#   pacman  $1 = <version> on remove (never "1" or "upgrade").
# So: skip when $1 indicates an upgrade (RPM "1" or deb "upgrade"); otherwise
# stop+disable. Correct for RPM, deb, and pacman.
set -e

if [ "$1" = "1" ] || [ "$1" = "upgrade" ] || [ "$1" = "failed-upgrade" ]; then
    # Upgrade — leave the running service untouched; the new package's
    # postinst will daemon-reload and restart as needed.
    exit 0
fi

if command -v systemctl >/dev/null 2>&1; then
    systemctl --no-reload disable --now plocate-server.service plocate-server-updatedb.timer ||:
fi

exit 0
