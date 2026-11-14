#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RULES_SRC="${SCRIPT_DIR}/99-sensehue.rules"
RULES_DEST="/etc/udev/rules.d/99-sensehue.rules"

if [[ "${EUID}" -ne 0 ]]; then
  echo "Error: Please run with sudo (e.g. sudo ./packaging/linux/install-udev.sh)." >&2
  exit 1
fi

if [[ ! -f "${RULES_SRC}" ]]; then
  echo "Error: Missing udev rules file at ${RULES_SRC}." >&2
  exit 1
fi

install -Dm644 "${RULES_SRC}" "${RULES_DEST}"
udevadm control --reload-rules
udevadm trigger

echo "SenseHue udev rules installed to ${RULES_DEST}."
echo "If your DualSense controller is already connected, unplug/replug USB or power-cycle Bluetooth."
