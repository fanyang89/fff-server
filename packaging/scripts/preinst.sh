#!/bin/sh
# preinstall: create the dedicated user/group and the database directory.
set -e

if ! getent group plocate-server >/dev/null; then
    groupadd --system plocate-server
fi

if ! getent passwd plocate-server >/dev/null; then
    useradd --system --no-create-home \
        --gid plocate-server \
        --home-dir /var/lib/plocate-server \
        --shell /usr/sbin/nologin \
        plocate-server
fi

install -d -o plocate-server -g plocate-server -m 0755 /var/lib/plocate-server

exit 0
