#!/bin/sh
# preremove: stop and disable the units on actual removal, but keep the
# service running across an RPM upgrade.
#
# Argument semantics differ across package formats:
#   RPM   $1 = 0 on remove, 1 on upgrade.
#   pacman $1 = <version> on remove (never a bare "1").
# So: skip only when $1 = "1" (RPM upgrade); otherwise stop+disable. This
# makes the scriptlet correct for both RPM and pacman.
set -e

if [ "$1" = "1" ]; then
    # RPM upgrade — leave the running service untouched; the new package's
    # postinst will daemon-reload and restart as needed.
    exit 0
fi

if command -v systemctl >/dev/null 2>&1; then
    systemctl --no-reload disable --now plocate-server.service plocate-server-updatedb.timer ||:
fi

exit 0
