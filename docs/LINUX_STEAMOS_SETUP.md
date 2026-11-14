# Guía de Instalación y Soporte en Linux (Ubuntu 26.04 LTS Wayland) y SteamOS

Por seguridad del kernel Linux, los nodos de dispositivo HID en bruto (`/dev/hidraw*`) pertenecen por defecto a `root` o al grupo `plugdev` con permisos de solo lectura/escritura restringidos. Para que **DualSenseColour PC** cambie el color de tu DualSense sin requerir `sudo`, es necesario instalar las reglas `udev` del proyecto.

---

## 1. Reglas `udev` para DualSense y DualSense Edge

El archivo [`packaging/linux/99-dualsensecolour.rules`](../packaging/linux/99-dualsensecolour.rules) otorga acceso `hidraw` y USB (tanto por cable USB-C como por Bluetooth HID) para:
- `054c:0ce6` (Sony DualSense)
- `054c:0df2` (Sony DualSense Edge)

### Instalación automática en Ubuntu / Debian / Arch / Fedora
Desde la raíz del repositorio o paquete descargado:

```bash
chmod +x packaging/linux/install-udev.sh
sudo ./packaging/linux/install-udev.sh
```

El script realiza tres pasos:
1. Copia `99-dualsensecolour.rules` a `/etc/udev/rules.d/99-dualsensecolour.rules`.
2. Recarga las reglas con `udevadm control --reload-rules`.
3. Dispara el reescaneo en caliente (`udevadm trigger`).

> **Importante:** si tenías el mando conectado antes de instalar las reglas, desconecta y vuelve a conectar el cable USB o apaga y vuelve a encender el mando por Bluetooth (manteniendo pulsado el botón PS 10 segundos) para que se aplique el nuevo permiso.

---

## 2. Ubuntu 26.04 LTS (Wayland)

### Dependencias de ejecución (AppImage / `.deb`)
```bash
sudo apt update
sudo apt install -y \
  libwebkit2gtk-4.1-0 \
  libayatana-appindicator3-1 \
  libhidapi-hidraw0
```

### Notas sobre Wayland, GPU NVIDIA y WebKitGTK
1. En sesiones Wayland puras con GPU NVIDIA o controladores Mesa recientes, WebKitGTK 2.40+ puede presentar pantalla transparente o en blanco al usar renderizado DMA-BUF. Si ocurre, lanza la aplicación con:
   ```bash
   WEBKIT_DISABLE_DMABUF_RENDERER=1 dualsensecolour-pc
   ```
2. Para mostrar el **System Tray** en GNOME (Ubuntu por defecto), asegúrate de tener habilitada la extensión *Ubuntu AppIndicators* (`gnome-shell-extension-appindicator`, activa de serie en Ubuntu Desktop).

---

## 3. SteamOS / Steam Deck

El sistema raíz de SteamOS es inmutable (*read-only*), **excepto** carpetas persistentes como `/etc/udev/rules.d/` y el directorio `$HOME` del usuario `deck`.

1. Cambia al **Desktop Mode**.
2. Instala las reglas `udev` (no sobrevive a ciertas actualizaciones mayores del SO sin reinstalarlas, aunque `/etc/udev/rules.d/` suele conservarse):
   ```bash
   sudo cp packaging/linux/99-dualsensecolour.rules /etc/udev/rules.d/
   sudo udevadm control --reload-rules && sudo udevadm trigger
   ```
3. Descarga la **AppImage** desde Releases, márcala como ejecutable (`chmod +x DualSenseColour-PC*.AppImage`) y, opcionalmente, añádela como **juego que no es de Steam** si quieres lanzarla desde Game Mode.

### Convivencia con Steam Input y modo Keep-Alive
Cuando **Steam** está en ejecución con *PlayStation Controller Support* activo, Steam Input puede enviar su propio reporte HID periodicamente restableciendo el color por defecto (azul estándar) tras iniciar/cerrar un juego.

Para evitarlo tienes dos opciones:
- Activar el interruptor **"Keep-Alive SteamOS"** dentro de DualSenseColour PC (reafirma tu color seleccionado periódicamente sin saturar la batería Bluetooth).
- O en **Steam → Parámetros → Mando → Compatibilidad con mando de PlayStation**, seleccionar *Activado solo en juegos sin compatibilidad integrada* si solo deseas conservar tu color en escritorio.
