# 🔊 Sonido ambiental

Pomodomate puede reproducir una pista en bucle mientras trabajas, y detenerla
sola cuando empieza el descanso.

## Pomodomate no incluye ningún audio

Esto es deliberado. Cada pista de sonido tiene su propia licencia, y meter
archivos ajenos en el repositorio significaría arrastrar sus condiciones. En
vez de eso, **tú pones tus archivos** y no hay ninguna duda sobre qué se está
reproduciendo ni bajo qué permiso.

Si buscas pistas libres, [Freesound](https://freesound.org) permite filtrar por
licencia CC0 (dominio público).

## Uso

Coloca tus archivos en:

```
~/.local/share/pomodomate/sounds/
```

Y elige uno en `~/.config/pomodomate/config.toml`:

```toml
ambient_sound = "rain"      # busca rain.mp3, rain.ogg, rain.wav o rain.flac
```

También aceptamos una ruta explícita, por si prefieres tenerlos en otro sitio:

```toml
ambient_sound = "/home/tu-usuario/Musica/lluvia.mp3"
```

Formatos soportados: **mp3, ogg, wav y flac**.

Deja el valor vacío (`ambient_sound = ""`) para desactivarlo.

## Comportamiento

- Suena **solo durante los bloques de trabajo en marcha**. Se detiene al pausar,
  al saltar, al reiniciar y durante los descansos.
- Se reanuda al continuar, sin volver a empezar la pista desde cero si ya
  estaba sonando.
- Si el archivo no existe o el dispositivo de audio está ocupado, no pasa nada:
  el temporizador sigue su curso sin avisos ni interrupciones.

## Compilar con soporte de audio

El audio es una **característica opcional**. Se compila así:

```bash
cargo build --release --features audio
```

### Por qué no viene activado por defecto

Reproducir audio enlaza con ALSA (a través de `cpal`), lo que exige tener
`libasound2-dev` (`alsa-lib` en Arch) al compilar. Como los binarios de
`aarch64` se compilan de forma cruzada en un runner que no dispone de esa
biblioteca para dicha arquitectura, activarlo por defecto rompería la
publicación de esas versiones.

En la práctica:

| Binario | Sonido ambiental |
| :--- | :--- |
| `x86_64` de las releases | ✅ incluido |
| `aarch64` de las releases | ❌ no incluido |
| Compilado por ti con `--features audio` | ✅ |
| Compilado por ti sin la bandera | ❌ |

Sin la característica activada, `ambient_sound` simplemente se ignora.

## Alternativa sin recompilar

Los [hooks](hooks.md) consiguen lo mismo con cualquier reproductor que ya
tengas instalado, sin tocar el binario:

```toml
[hooks]
work_start = "mpv --loop --volume=40 ~/Musica/lluvia.mp3 &"
work_end   = "pkill -f 'mpv.*lluvia'"
```
