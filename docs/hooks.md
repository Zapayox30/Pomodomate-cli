# 🪝 Hooks: automatiza tu entorno con cada pomodoro

Pomodomate puede ejecutar un comando de shell cada vez que el temporizador cruza
una frontera de fase. Con eso puedes silenciar notificaciones mientras te
concentras, pausar la música, encender luces o registrar tu tiempo donde quieras
— sin que Pomodomate tenga que integrarse con cada herramienta.

## Los cuatro hooks

Se configuran en la sección `[hooks]` de `~/.config/pomodomate/config.toml`:

```toml
[hooks]
work_start  = "swaync-client -d"        # empieza un bloque de enfoque
work_end    = "swaync-client -d"        # termina (completado o saltado)
break_start = "notify-send 'Descanso'"  # empieza un descanso
break_end   = "notify-send 'A trabajar'"# termina un descanso
```

Todos son opcionales: si omites uno, no pasa nada.

## Cómo se ejecutan

- Se lanzan con `sh -c`, así que **puedes usar tuberías, `&&` y variables**.
- Son **fire-and-forget**: Pomodomate no espera a que terminen ni revisa su
  código de salida. Un hook lento o roto nunca congela tu temporizador.
- Su salida va a `/dev/null`. Es intencional: la TUI es dueña de la terminal y
  un `echo` dentro de un hook corrompería el dibujado.
- Se ejecutan también cuando **saltas** una fase con `s`, no solo al
  completarla. Así un hook que activa el "no molestar" siempre tiene su pareja
  que lo desactiva.

## Variables disponibles

Cada hook recibe el contexto por entorno:

| Variable | Contenido |
| :--- | :--- |
| `POMODOMATE_PHASE` | `work`, `short_break` o `long_break` |
| `POMODOMATE_POMODOROS` | Pomodoros completados en esta ejecución |
| `POMODOMATE_DURATION` | Duración de la fase, en minutos |
| `POMODOMATE_TAGS` | Etiquetas de la sesión, separadas por comas |
| `POMODOMATE_COMPLETED` | `true` si la fase se completó, `false` si se saltó |

## Recetas

### No molestar automático (swaync)

```toml
[hooks]
work_start = "swaync-client -d"
work_end   = "swaync-client -d"
```

### No molestar automático (mako / dunst)

```toml
[hooks]
work_start = "makoctl mode -a do-not-disturb"
work_end   = "makoctl mode -r do-not-disturb"
```

```toml
[hooks]
work_start = "dunstctl set-paused true"
work_end   = "dunstctl set-paused false"
```

### Pausar la música mientras trabajas

```toml
[hooks]
work_start = "playerctl pause"
break_start = "playerctl play"
```

### Registrar cada bloque en un archivo

```toml
[hooks]
work_end = "echo \"$(date -Is) $POMODOMATE_PHASE completed=$POMODOMATE_COMPLETED\" >> ~/pomodoros.log"
```

### Avisar solo cuando de verdad terminaste el bloque

```toml
[hooks]
work_end = "[ \"$POMODOMATE_COMPLETED\" = true ] && notify-send '🍅 Bloque #'$POMODOMATE_POMODOROS' terminado'"
```

## Entrecomilla tus variables

Los valores llegan como **datos**, nunca como código: Pomodomate los pasa por
el entorno en vez de pegarlos dentro de la línea de shell, así que una etiqueta
como `$(rm -rf ~)` se queda en texto literal y no se ejecuta.

Aun así, entrecomilla al usarlas, como en cualquier script:

```toml
[hooks]
work_end = "echo \"$POMODOMATE_TAGS\" >> ~/log.txt"   # ✅
# work_end = "eval $POMODOMATE_TAGS"                    # ❌ nunca hagas esto
```

## Al cerrar también se ejecutan

Si cierras el temporizador (con `q`) o detienes el daemon (con `ctl quit`,
Ctrl+C o `systemctl stop`) mientras hay una fase en marcha, su hook de fin
**se ejecuta igualmente**. Así, un `work_start` que activa el "no molestar"
siempre encuentra su pareja y no te deja el escritorio silenciado sin saber
por qué.

## Un aviso de seguridad

Los hooks ejecutan comandos arbitrarios con **tus permisos**. Trata tu
`config.toml` como cualquier script tuyo: no pegues en él líneas que no
entiendas, igual que no las pegarías en tu terminal.
