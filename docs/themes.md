# 🎨 Guía de Temas y Personalización en Pomodomate CLI

Pomodomate CLI incluye un motor de temas dinámico y flexible. Esta guía te muestra cómo aplicar temas integrados y cómo personalizar cada color de la interfaz si eres un usuario avanzado (*power user*).

---

## 🚀 Temas Integrados

Puedes cambiar el estilo visual de tu temporizador usando cualquiera de los siguientes temas predefinidos:

| Tema | Descripción | Comando Rápido |
| :--- | :--- | :--- |
| `default` | El clásico rojo tomate de Pomodomate, con tonos verdes y morados. | `./pomodomate --theme default` |
| `nord` | Tonos árticos fríos, verdes pálidos y azules relajantes estilo Nord. | `./pomodomate --theme nord` |
| `dracula` | Estética clásica de hacker con fondo morado oscuro, rosa vibrante y verde neón. | `./pomodomate --theme dracula` |
| `gruvbox` | Colores cálidos y retro estilo máquina de escribir clásica (tonos arena, ocre y café). | `./pomodomate --theme gruvbox` |
| `monochrome` | Escala de grises pura (blanco y negro), ideal para pantallas e-ink o minimalistas extremos. | `./pomodomate --theme monochrome` |

---

## 🔢 Estilos del Reloj

Además del color, puedes cambiar la tipografía de los dígitos grandes. Pulsa
`d` dentro del temporizador para ir rotando entre los tres estilos; el pie de
pantalla te indica cuál está activo.

| Estilo | Aspecto |
| :--- | :--- |
| `line` | `╶─╮ ╭─╴ ● ╭─╮ ╭─╮` — trazo fino redondeado (por defecto) |
| `heavy` | `╺━┓ ┏━╸ ◆ ┏━┓ ┏━┓` — trazo grueso |
| `double` | `══╗ ╔══ ◉ ╔═╗ ╔═╗` — línea doble |

Para fijar tu preferido, añade a `config.toml`:

```toml
digit_style = "double"   # "line", "heavy" o "double"
```

---

## ⚙️ Cómo Activar un Tema

### Temporalmente (una sola ejecución)
Puedes arrancar Pomodomate con un tema específico usando la bandera `--theme`:
```bash
pomodomate --theme nord
```

### Permanentemente (Configuración)
Para dejar tu tema favorito por defecto, edita tu archivo de configuración en `~/.config/pomodomate/config.toml` y añade la clave `theme`:
```toml
# ~/.config/pomodomate/config.toml
work_duration = 25
short_break = 5
theme = "nord"  # Opciones: "default", "nord", "dracula", "gruvbox", "monochrome"
```

---

## 🛠️ Personalización Avanzada (Para Expertos)

Si quieres ir más allá de los temas integrados, puedes definir tus propios colores hexadecimales en la sección `[custom_colors]` de tu archivo `config.toml`. 

El CLI interpretará los colores en formato `#RRGGBB` o usando nombres de colores estándar de terminal (como `red`, `blue`, `green`, `white`, `black`, `yellow`, `magenta`, `cyan`, `gray`, `darkgray`).

### Variables de Color Disponibles

| Variable | Rol en la Interfaz |
| :--- | :--- |
| `tomato_red` | Color de la fase de enfoque (Trabajo) y título principal. |
| `nature_green` | Color de los descansos cortos y estado "Running". |
| `accent_purple` | Color de los descansos largos. |
| `dark_bg` | Fondo principal de la ventana del temporizador. |
| `dark_base` | Fondo de las tarjetas (cabecera, contador de pomodoros y pie de página). |
| `border_dim` | Bordes sutiles y separadores inactivos. |
| `border_glow` | Bordes encendidos y resaltado de teclas activas. |
| `muted_text` | Textos secundarios, etiquetas inactivas e indicaciones. |
| `progress_bg` | Fondo de la barra de progreso (porción no completada). |
| `soft_white` | Color del texto principal y del reloj. |
| `warm_yellow` | Alertas, indicaciones de reanudación y cuenta regresiva del último minuto. |

---

### Ejemplo: Creando un tema "One Dark" personalizado

Edita tu archivo `~/.config/pomodomate/config.toml` y pega la siguiente configuración experta:

```toml
theme = "default" # Usamos base default pero sobreescribimos colores

[custom_colors]
tomato_red = "#E06C75"      # Rojo suave
nature_green = "#98C379"    # Verde One Dark
accent_purple = "#C678DD"   # Morado
dark_bg = "#282C34"         # Fondo oscuro grisáceo
dark_base = "#21252B"       # Base más oscura
border_dim = "#3E4452"      # Bordes discretos
border_glow = "#61AFEF"     # Azul brillante de enfoque
muted_text = "#5C6370"      # Gris para textos secundarios
progress_bg = "#3E4452"     # Fondo de la barra
soft_white = "#ABB2BF"      # Texto blanco suave
warm_yellow = "#D19A66"     # Naranja/amarillo
```

Prueba guardando esto y arrancando `pomodomate`. ¡El temporizador se vestirá con los colores exactos de tu editor favorito!
