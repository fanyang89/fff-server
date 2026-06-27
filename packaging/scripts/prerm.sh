#!/bin/sh
# preremove (on removal, arg 0): stop and disable the units.
set -e

if [ "$1" = "0" ] && command -v systemctl >/dev/null 2>&1; then
    systemctl --no-reload disable --now plocate-server.service plocate-server-updatedb.timer ||:
fi

exit 0
