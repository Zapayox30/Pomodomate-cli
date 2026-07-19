# 🏷️ Etiquetas: en qué se fueron tus pomodoros

Un contador que dice "hiciste 12 pomodoros" no te dice gran cosa. Con etiquetas
sabes que fueron *8 en la tesis y 4 en Rust*.

## Etiquetar una sesión

Usa `--tag` (o `-t`) al arrancar. Todas las sesiones de esa ejecución quedan
marcadas:

```bash
pomodomate --tag tesis
pomodomate -t tesis -t rust        # varias etiquetas
pomodomate -t "tesis, rust"        # o separadas por comas
```

Las etiquetas se normalizan solas: se recortan espacios, se pasan a minúsculas
y se eliminan duplicados. `"Tesis, RUST"` y `-t rust` acaban guardados como
`tesis, rust`.

Si siempre trabajas en lo mismo, déjalas fijas en
`~/.config/pomodomate/config.toml`:

```toml
tags = ["tesis"]
```

La bandera `--tag` tiene prioridad sobre el archivo de configuración.

## Consultar por etiqueta

```bash
pomodomate stats --by-tag        # desglose de todas tus etiquetas
pomodomate stats --tag tesis     # solo las sesiones de "tesis"
```

Ejemplo de salida:

```
🍅 Pomodomate — pomodoros by tag (last 365 days)

  tesis     4
  rust      2
  ocio      1
```

Ambos comandos aceptan `--json` para scripts y barras de estado:

```bash
pomodomate stats --by-tag --json
# {"ocio":1,"rust":2,"tesis":4}

pomodomate stats --tag rust --json
# {"today":2,"week":2,"year":2,"active_days":1,"best_streak":1,"current_streak":1}
```

## Cómo se cuentan los días

Un pomodoro pertenece al día de **tu reloj**, no al del meridiano de
Greenwich. Si trabajas a las 21:00 en Lima, cuenta para hoy, no para mañana.
Las sesiones se guardan como instantes UTC —que es la forma correcta de
registrar un momento— y se agrupan por tu fecha local al mostrarlas.

Cada sesión guarda además `focus_seconds`, el tiempo que el reloj estuvo
realmente corriendo, sin contar las pausas. `duration_minutes` sigue siendo la
duración prevista de la fase: si estiras un bloque con `+` o lo saltas a mitad,
la diferencia entre ambos campos te dice qué pasó de verdad.

## Detalles que conviene saber

- Solo cuentan los **bloques de trabajo completados**. Un pomodoro saltado con
  `s` se guarda en el historial, pero no suma en las estadísticas.
- Las búsquedas por etiqueta **ignoran mayúsculas**: `--tag TESIS` encuentra
  `tesis`.
- Una sesión puede llevar varias etiquetas y suma en todas.
- Tu historial anterior sigue funcionando: las sesiones guardadas antes de que
  existieran las etiquetas se cargan con la lista vacía.
- Las etiquetas llegan también a tus [hooks](hooks.md) en la variable
  `POMODOMATE_TAGS`, por si quieres registrarlas en otro sistema.
