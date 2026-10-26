#!/bin/sh
# Lumaglass's optional cold-boot hook. Installed only when explicitly requested.
# At boot $0 is the hook symlink; resolve it before finding the owning package.
# This also works for cryptofs and system installations, without guessing one.
SCRIPT=$(readlink -f "$0") || exit 1
APP_DIR=$(CDPATH='' cd -- "$(dirname -- "$SCRIPT")" && pwd) || exit 1
HOOK=/var/lib/webosbrew/init.d/65-lumaglass
case "${1:-}" in
  enable)
    [ "$(id -u)" = 0 ] || { printf '%s\n' 'Run as root.' >&2; exit 1; }
    [ -f "$APP_DIR/appinfo.json" ] || { printf '%s\n' 'Install Lumaglass first.' >&2; exit 1; }
    [ -x "$APP_DIR/lumaglass" ] || { printf '%s\n' 'The native executable is missing.' >&2; exit 1; }
    if [ -e /var/lib/webosbrew/init.d/64-lumaglass-home ] || [ -L /var/lib/webosbrew/init.d/64-lumaglass-home ]; then
      printf '%s\n' 'Disable Lumaglass Home takeover before enabling standalone autostart.' >&2
      exit 1
    fi
    mkdir -p /var/lib/webosbrew/init.d
    if [ -e "$HOOK" ] || [ -L "$HOOK" ]; then
      [ "$(readlink "$HOOK")" = "$APP_DIR/autostart.sh" ] || { printf '%s\n' '65-lumaglass is already used by another file.' >&2; exit 1; }
    fi
    chmod 755 "$APP_DIR/autostart.sh"
    ln -sf "$APP_DIR/autostart.sh" "$HOOK"
    printf '%s\n' 'Lumaglass will open after a full boot.'
    exit 0 ;;
  disable)
    [ "$(id -u)" = 0 ] || exit 1
    if [ "$(readlink "$HOOK")" = "$APP_DIR/autostart.sh" ]; then rm -f "$HOOK"; fi
    printf '%s\n' 'Lumaglass boot launch disabled.'
    exit 0 ;;
esac
# A lock lives for this boot only; do not launch twice when the hook is invoked again.
(
    exec 9>/tmp/lumaglass-boot.lock
    flock -n 9 || exit 0
    [ -f /tmp/lumaglass-boot-done ] && exit 0
    [ -f "$APP_DIR/appinfo.json" ] || exit 0
    sleep 25
    # -n prints the reply. -t is timing mode, not a firmware-specific flag.
    RESULT="$(/usr/bin/luna-send -n 1 -w 5000 luna://com.webos.service.applicationmanager/launch '{"id":"org.lumaglass.app"}' 2>/dev/null)"
    printf '%s\n' "$RESULT"
    case "$RESULT" in *'"returnValue":true'*|*'"returnValue": true'*) touch /tmp/lumaglass-boot-done ;; esac
) </dev/null >/tmp/lumaglass-boot.log 2>&1 &
