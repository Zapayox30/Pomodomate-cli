# 💤 Pausa automática al levantarte del escritorio

Si te levantas a mitad de un bloque de trabajo, ese tiempo no fue concentración.
Pomodomate lo detecta y pausa el temporizador solo.

## Cómo funciona

Tras unos minutos sin teclado ni ratón, el temporizador se pausa. **Al volver no
se reanuda solo**: te avisa de lo ocurrido y decides tú.

```
  ⏸ paused while you were away — press space to pick up where you left off
```

Esto es deliberado. Reanudar automáticamente contaría como concentración el rato
que tardas en volver a sentarte, y el objetivo de la función es justo lo
contrario: que tus estadísticas sean honestas.

## Configuración

En `~/.config/pomodomate/config.toml`:

```toml
idle_timeout = 5   # minutos sin actividad; 0 lo desactiva
```

El valor por defecto es **5 minutos**. Ponlo a `0` si prefieres que el
temporizador nunca se pause solo.

Funciona tanto en la TUI como en el [daemon](daemon.md).

## Requisitos

Necesitas Wayland con un compositor que implemente **`ext-idle-notify-v1`**.
Lo soportan Hyprland, Sway, river, niri, KDE Plasma y GNOME, entre otros.

En X11, o en un compositor sin ese protocolo, la función simplemente no se
activa: no verás errores ni cambios de comportamiento, el temporizador
funcionará como siempre.

Para comprobar si tu compositor lo expone:

```bash
wayland-info | grep ext_idle_notifier
# interface: 'ext_idle_notifier_v1', version: 2, name: 25
```

## Por qué no usamos la cámara

El PRD original planteaba detectar distracción con la cámara en la Fase 3. La
inactividad de teclado y ratón cubre el caso más común —levantarse del sitio—
con ventajas claras:

- **Sin permisos incómodos** ni acceso a la webcam.
- **Coste de CPU nulo**: el compositor nos avisa, no hacemos sondeo.
- **Privacidad por diseño**: nunca se procesa imagen alguna.

La detección por cámara sigue teniendo sentido como opción avanzada para el caso
distinto de "estoy sentado pero distraído".

## Detalles de implementación

- El protocolo se consulta **una sola vez al arrancar**. Si no está disponible,
  no se lanza ningún hilo de vigilancia.
- Si reinicias el compositor, la conexión se pierde y **se reintenta sola**,
  con esperas cada vez más largas. Antes se quedaba muerta en silencio hasta
  que reiniciabas Pomodomate.
- El vigilante corre en su propio hilo y se comunica por un canal, así que
  nunca bloquea el dibujado ni el reloj.
- Una pausa manual (`space`, `ctl pause`) **no** se marca como pausa por
  inactividad, y cualquier acción tuya borra el aviso.
- El campo `idle_paused` aparece en `pomodomate status --json`, por si quieres
  reflejarlo en tu barra.
