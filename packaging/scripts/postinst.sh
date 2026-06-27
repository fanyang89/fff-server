#!/bin/sh
# postinstall: reload systemd, enable per presets. Does not start the service.
set -e

if command -v systemctl >/dev/null 2>&1; then
    systemctl daemon-reload ||:
    # Respect the distro preset policy (enable, but do not start).
    systemctl preset plocate-server.service plocate-server-updatedb.timer ||:
fi

exit 0
