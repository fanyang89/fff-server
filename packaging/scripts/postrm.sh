#!/bin/sh
# postremove: drop the units from systemd's view.
set -e

if command -v systemctl >/dev/null 2>&1; then
    systemctl daemon-reload ||:
fi

exit 0
