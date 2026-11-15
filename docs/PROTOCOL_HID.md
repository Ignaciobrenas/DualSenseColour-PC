# Especificación del Protocolo HID DualSense (`USB 0x02` & `Bluetooth 0x31`)

Este documento detalla los paquetes HID de salida (*Output Reports*) implementados por **SenseHue** en `src-tauri/src/hid/` para controlar la barra LED del panel táctil (*lightbar*), los 5 LEDs indicadores de jugador (*player LEDs*) y el LED de silencio de micrófono (*mic mute LED*).

---

## 1. Identificadores USB / Bluetooth soportados

| Modelo | Vendor ID (`VID`) | Product ID (`PID`) | Soporte |
| :--- | :---: | :---: | :--- |
| **Sony DualSense (PS5)** | `0x054C` | `0x0CE6` | USB-C y Bluetooth |
| **Sony DualSense Edge** | `0x054C` | `0x0DF2` | USB-C y Bluetooth |

---

## 2. Detección de Transporte (USB vs Bluetooth)

El backend evalúa cada interfaz enumerada mediante `hidapi`:
1. Comprueba `bus_type()`: `BusType::Bluetooth` identifica Bluetooth y `BusType::Usb` identifica USB.
2. Si `bus_type()` devuelve `Unknown`, aplica heurísticas portables:
   - En macOS/Linux/Windows, las conexiones Bluetooth suelen exponer `interface_number == -1` (frente a `>= 0`, típicamente interfaz `3`, en USB).
   - En Windows, las rutas del bus Bluetooth contienen `BTHENUM` o el UUID HID `00001124`.

---

## 3. Despertar / Habilitación en Modo Bluetooth (`Feature Report 0x05`)

Por defecto al emparejarse por Bluetooth sin software de Sony/Steam activo, el DualSense utiliza el descriptor reducido *legacy* (`Input Report 0x01`) e **ignora los paquetes `Output Report 0x31`**.

Para desbloquear los reportes extendidos:
1. Antes de enviar colores tras detectar una conexión Bluetooth, el monitor lee el **Feature Report `0x05`** (calibración IMU de 64 bytes) o en su defecto `0x20` (*firmware info*).
2. El firmware del mando conmuta inmediatamente al modo extendido completo y comienza a aceptar `Output Report 0x31` con CRC-32 válido.

---

## 4. Estructura del Reporte USB (`0x02`, 63 bytes)

> Nota: `hidapi::HidDevice::write` requiere el **Report ID en el byte 0**, seguido de los 62 bytes de carga útil (**total = 63 bytes**).

| Offset (Byte) | Tamaño | Campo | Valor / Descripción |
| :---: | :---: | :--- | :--- |
| `0` | 1 | `report_id` | `0x02` (USB Output Report) |
| `1` | 1 | `valid_flag0` | Flags de gatillos adaptativos (`0x04` = R2, `0x08` = L2, `0x0C` = ambos) |
| `2` | 1 | `valid_flag1` | Bitmask de funciones activas:<br>• `0x01` = Mic Mute LED<br>• `0x04` = Touchpad Lightbar RGB<br>• `0x10` = Player Indicator LEDs |
| `9` | 1 | `mic_mute_led` | `0x00` = Off · `0x01` = Solid On · `0x02` = Pulse |
| `11..21` | 11 | `right_trigger` | Descriptor de efecto adaptativo **R2** (modo + parámetros) |
| `22..32` | 11 | `left_trigger` | Descriptor de efecto adaptativo **L2** (modo + parámetros) |
| `39..40` | 2 | `valid_flag2` | `0x02` para habilitar control de arranque del Lightbar (`LIGHTBAR_SETUP_CONTROL_ENABLE`) |
| `42` | 1 | `lightbar_setup` | `0x02` (`LIGHTBAR_SETUP_LIGHT_OUT`) para desbloquear RGB personalizado |
| `44` | 1 | `player_leds` | Bitmask de los 5 LEDs inferiores + bit de brillo/fade |
| `45` | 1 | `lightbar_red` | Canal Rojo (`0x00` – `0xFF`, preescalado con brillo) |
| `46` | 1 | `lightbar_green` | Canal Verde (`0x00` – `0xFF`, preescalado con brillo) |
| `47` | 1 | `lightbar_blue` | Canal Azul (`0x00` – `0xFF`, preescalado con brillo) |
| Resto | — | Padding | `0x00` (mantiene háptica/audio sin modificar) |

---

## 5. Estructura del Reporte Bluetooth (`0x31`, 78 bytes)

Los reportes Bluetooth añaden una cabecera de 2 bytes (`seq_tag` y `tag`) al principio (desplazando los offsets comunes +2 bytes respecto a USB) y una firma **CRC-32 de 4 bytes** al final:

| Offset (Byte) | Tamaño | Campo | Valor / Descripción |
| :---: | :---: | :--- | :--- |
| `0` | 1 | `report_id` | `0x31` (BT Output Report) |
| `1` | 1 | `seq_tag` | `(seq & 0x0F) << 4` (nibble alto rotativo `0..15`, bit 0..3 = `0`) |
| `2` | 1 | `tag` | `0x10` (`DS_OUTPUT_TAG`, obligatorio en DualSense BT) |
| `3` | 1 | `valid_flag0` | Flags de gatillos adaptativos (`0x04` R2 \| `0x08` L2) |
| `4` | 1 | `valid_flag1` | Bitmask (`0x01` Mic LED \| `0x04` Lightbar \| `0x10` Player LEDs) |
| `11` | 1 | `mic_mute_led` | `0x00` = Off · `0x01` = On · `0x02` = Pulse |
| `13..23` | 11 | `right_trigger` | Descriptor adaptativo **R2** (offset `10 + 3`) |
| `24..34` | 11 | `left_trigger` | Descriptor adaptativo **L2** (offset `21 + 3`) |
| `41..42` | 2 | `valid_flag2` | `0x02` (`LIGHTBAR_SETUP_CONTROL_ENABLE`) |
| `44` | 1 | `lightbar_setup` | `0x02` (`LIGHTBAR_SETUP_LIGHT_OUT`) |
| `46` | 1 | `player_leds` | Bitmask Player LEDs (offset `44 + 2`) |
| `47` | 1 | `lightbar_red` | Rojo (`0x00` – `0xFF`) |
| `48` | 1 | `lightbar_green` | Verde (`0x00` – `0xFF`) |
| `49` | 1 | `lightbar_blue` | Azul (`0x00` – `0xFF`) |
| `74..77` | 4 | `crc32` | `u32` en formato **Little-Endian** |

---

## 6. Cálculo del CRC-32 Bluetooth (Seed `0xA2`)

Si el CRC-32 no es exacto, el DualSense **descarta silenciosamente** el paquete Bluetooth sin cambiar el color.

El algoritmo utilizado es **CRC-32 estándar IEEE 802.3** (`0xEDB88320`, equivalente a `crc32fast::Hasher::new()`), pero prepending un byte semilla fijo `0xA2` antes de los bytes `[0..74]` del reporte:

```rust
let mut hasher = crc32fast::Hasher::new();
hasher.update(&[0xA2]);          // Seed byte obligatorio del protocolo HID Bluetooth PS5
hasher.update(&report[0..74]);   // Incluye byte 0x31, seq_tag, tag 0x10 y payload hasta byte 73
let crc = hasher.finalize();
report[74..78].copy_from_slice(&crc.to_le_bytes());
```

---

## 7. Máscaras de LEDs de Jugador (*Player Indicators*)

Los 5 LEDs blancos bajo la barra del touchpad siguen la disposición simétrica oficial de Sony:

| Número de Jugadores | Máscara Hex (`byte 44 USB` / `46 BT`) | Patrón Visual (`○` apagado / `●` encendido) |
| :---: | :---: | :---: |
| `0` (Apagados) | `0x00` | `○ ○ ○ ○ ○` |
| `1` | `0x04` | `○ ○ ● ○ ○` |
| `2` | `0x0A` | `○ ● ○ ● ○` |
| `3` | `0x15` | `● ○ ● ○ ●` |
| `4` | `0x1B` | `● ● ○ ● ●` |
| `5` (Todos) | `0x1F` | `● ● ● ● ●` |

*(Si deseas desactivar la animación fade-in, puede combinarse con el bit `0x20`, aunque los presets base estándar preservan transiciones suaves con `0x00..=0x1F`.)*

---

## 8. Decodificación de Input Reports y Batería Real (`0x01` USB / `0x31` BT)

En `src-tauri/src/hid/input_report.rs`, SenseHue decodifica las lecturas analógicas y el nivel real de batería directamente desde el firmware:

- **USB (`Report ID 0x01`, >= 54 bytes)**: Byte de batería en **offset `53`** (`USB_BATTERY_BYTE_OFFSET`).
- **Bluetooth extendido (`Report ID 0x31`, >= 55 bytes)**: Byte de batería en **offset `54`** (`BT_BATTERY_BYTE_OFFSET`), disponible automáticamente tras despertar el mando con Feature Report `0x05` / `0x20`. Si existen tramas *legacy* `0x01` en cola sobre Bluetooth, el lector drena hasta 16 paquetes hasta obtener la lectura extendida `0x31`.
- **Nibble inferior (`byte & 0x0F`)**: Escala lineal de hardware (`0..9` → `(nibble * 10 + 5)%`, saturando al `100%` en `>= 0x0A` o estado `Full`).
- **Nibble superior (`(byte >> 4) & 0x0F`)**: `0x00` = `Discharging`, `0x01` = `Charging`, `0x02` = `Full`.
