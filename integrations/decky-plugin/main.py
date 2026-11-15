"""SenseHue Decky Loader backend.

Habla con el daemon `sensehue` (systemd --user) por la API HTTP local 127.0.0.1:17420.
No inyecta nada en juegos: solo cliente HTTP local.
"""
import json
import urllib.request

import decky  # type: ignore  # proporcionado por Decky Loader

API = "http://127.0.0.1:17420"


def _call(method: str, path: str):
    req = urllib.request.Request(API + path, method=method)
    with urllib.request.urlopen(req, timeout=2) as resp:
        return json.loads(resp.read().decode("utf-8") or "{}")


class Plugin:
    async def status(self):
        try:
            return _call("GET", "/status")
        except Exception as exc:  # noqa: BLE001
            decky.logger.error(f"SenseHue daemon no disponible: {exc}")
            return {"error": "El servicio sensehue no está en ejecución (systemctl --user start sensehue)."}

    async def set_preset(self, preset_id: str):
        # Solo ids alfanuméricos con guiones; el daemon valida de nuevo.
        if not preset_id.replace("-", "").isalnum():
            return {"error": "preset inválido"}
        return _call("POST", f"/preset/{preset_id}")

    async def night_mode(self, enabled: bool):
        return _call("POST", "/off" if enabled else "/preset/playstation-blue")

    async def _unload(self):
        decky.logger.info("SenseHue plugin descargado")
