# ⚙️ Daemon: el temporizador fuera de la terminal

La TUI es genial mientras la miras, pero muere cuando cierras la terminal. El
daemon corre el mismo temporizador en segundo plano y lo expone por un socket
UNIX, así puedes verlo en Waybar y controlarlo con atajos de teclado.

El daemon es **opcional**: `pomodomate` a secas sigue funcionando exactamente
igual que siempre, por su cuenta.

## Arrancarlo

```bash
pomodomate daemon
```

Respeta las mismas banderas y el mismo `config.toml` que la TUI, hooks
incluidos:

```bash
pomodomate daemon -w 50 -b 10 --tag tesis
```

Escucha en `$XDG_RUNTIME_DIR/pomodomate.sock`. Si tu sistema no define esa
variable, usa `/tmp/pomodomate-<uid>/pomodomate.sock`, dentro de un directorio
propio con permisos `0700`. Puedes elegir la ruta con `POMODOMATE_SOCKET`:

```bash
POMODOMATE_SOCKET=/tmp/pomodomate.sock pomodomate daemon
```

> La ruta del socket no puede pasar de 103 bytes — es un límite del kernel, no
> nuestro. Si te pasas, Pomodomate te lo dice claramente en vez de fallar con
> un error críptico.

## Controlarlo

```bash
pomodomate ctl toggle    # arrancar o pausar
pomodomate ctl start     # arrancar (idempotente)
pomodomate ctl pause     # pausar (idempotente)
pomodomate ctl resume    # reanudar (idempotente)
pomodomate ctl skip      # saltar a la siguiente fase
pomodomate ctl reset     # reiniciar la fase actual
pomodomate ctl quit      # detener el daemon
```

`pause` y `resume` son idempotentes a propósito: puedes llamarlos desde un
script sin comprobar antes en qué estado estabas.

## Consultarlo

```bash
$ pomodomate status
🍅 24:57

$ pomodomate status --format "{icon} {time} · {percent}%"
🍅 24:57 · 0%

$ pomodomate status --json
{"phase":"work","status":"running","remaining_seconds":1497,"total_seconds":1500,"percent":0,"pomodoros":0,"time":"24:57","idle_paused":false,"error":null}
```

### Marcadores de plantilla

| Marcador | Ejemplo |
| :--- | :--- |
| `{icon}` | `🍅` trabajo · `☕` descanso corto · `🌴` descanso largo · `⏸` pausado |
| `{time}` | `24:57` |
| `{percent}` | `35` |
| `{phase}` | `work`, `short_break`, `long_break` |
| `{status}` | `running`, `paused`, `idle`, `completed` |
| `{pomodoros}` | `3` |
| `{remaining}` / `{total}` | segundos en crudo |

El JSON incluye además `idle_paused` (si la pausa fue por ausencia) y `error`
(el último fallo no fatal, normalmente una escritura del historial que no pudo
completarse).

Un marcador que no exista se deja tal cual, para que una errata se vea en la
barra en lugar de desaparecer en silencio.

## Waybar: temporizador en vivo con clics

A diferencia del módulo de estadísticas, este muestra la cuenta atrás y
responde al ratón:

```jsonc
"custom/pomodomate": {
    "exec": "pomodomate status --format '{icon} {time}' 2>/dev/null || echo '🍅 —'",
    "interval": 1,
    "on-click": "pomodomate ctl toggle",
    "on-click-right": "pomodomate ctl skip",
    "on-click-middle": "pomodomate ctl reset",
    "tooltip": false
}
```

El `|| echo '🍅 —'` hace que la barra muestre un guion cuando no hay daemon,
en vez de quedarse vacía.

## Hyprland: atajos de teclado

En `~/.config/hypr/hyprland.conf`:

```conf
bind = SUPER, P, exec, pomodomate ctl toggle
bind = SUPER SHIFT, P, exec, pomodomate ctl skip
exec-once = pomodomate daemon
```

## Arranque automático con systemd

Crea `~/.config/systemd/user/pomodomate.service`:

```ini
[Unit]
Description=Pomodomate timer daemon

[Service]
ExecStart=%h/.cargo/bin/pomodomate daemon
Restart=on-failure

[Install]
WantedBy=default.target
```

```bash
systemctl --user enable --now pomodomate
```

## Detenerlo de forma segura

`ctl quit`, Ctrl+C y `systemctl stop` hacen lo mismo: registran la fase que
estuviera en marcha, ejecutan su hook de fin y borran el socket. Esto último
importa más de lo que parece: si tienes un `work_start` que activa el "no
molestar", detener el daemon sin ejecutar su pareja te dejaría el escritorio
silenciado sin ninguna pista de por qué.

## Detalles del diseño

- **Un daemon por usuario.** Si intentas arrancar un segundo, te avisa en vez
  de pisar el socket del primero.
- **Los sockets huérfanos se limpian solos.** Si el daemon muere de golpe
  (`kill -9`, corte de luz), el siguiente arranque detecta que nadie escucha
  en ese archivo y lo reemplaza.
- **El reloj corre en su propio hilo**, así que un cliente lento nunca retrasa
  un tic, y **cada conexión se atiende en el suyo**, de modo que un cliente
  atascado no deja a los demás esperando.
- **El socket es tuyo y solo tuyo**: se crea con permisos `0600`, sin heredar
  tu `umask`. Si no hay `XDG_RUNTIME_DIR`, va dentro de un directorio propio
  con permisos `0700` en lugar de quedar suelto en `/tmp`, donde otro usuario
  del sistema podría adelantarse a crearlo.
- **Las lecturas están acotadas** a 1 KiB: el comando más largo ocupa seis
  bytes, así que un cliente que nunca envíe un fin de línea no puede hacer
  crecer la memoria del daemon.
- **Un fichero que no sea un socket nunca se borra.** Si apuntas
  `POMODOMATE_SOCKET` a un archivo normal por error, Pomodomate se niega a
  arrancar en vez de destruirlo.
- **El daemon guarda sesiones y dispara [hooks](hooks.md)** igual que la TUI:
  ambos comparten el mismo motor, así que no hay dos comportamientos distintos
  que mantener sincronizados.
- **El protocolo es texto plano**, un comando por línea. Puedes depurarlo a
  mano:

  ```bash
  echo status | socat - UNIX-CONNECT:$XDG_RUNTIME_DIR/pomodomate.sock
  ```

## Limitación actual

La TUI y el daemon son **independientes**: si corres los dos a la vez, tendrás
dos temporizadores distintos. Usa uno u otro según prefieras mirar la cuenta
atrás o tenerla en la barra.
