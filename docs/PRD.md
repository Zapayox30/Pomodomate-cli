
Pomodomate-cli

Product Requirements Document
v0.1  -  2025  -  Pomodomate.com

Rust	Offline First	Open Source MIT

1. Vision y Objetivo
Pomodomate-cli es un temporizador Pomodoro de linea de comandos, nativo y completamente offline, disenado para desarrolladores y power users de Linux que usan gestores de ventanas modernos como Hyprland. Es parte del ecosistema Pomodomate.com junto a la web ya desplegada y la app mobile en camino.

Principio central: lanzar la CLI offline primero, conectarla al ecosistema despues. Un paso a la vez.

Propuesta de Valor
    • Sin internet. Sin telemetria. Sin servidores. Solo un binario.
    • Visual impactante: mascota Domate animada en Unicode + colores ANSI 24-bit.
    • Funciona en cualquier terminal moderna sin configuracion especial.
    • Historial local con heatmap estilo GitHub.
    • Base para conectarse al ecosistema Pomodomate cuando el usuario lo desee.

Problema que Resuelve
Los timers Pomodoro existentes son apps pesadas, requieren cuenta online, o son tan minimalistas que no generan ningun vinculo con el usuario. Pomodomate-cli apuesta por una experiencia que se siente viva, sin sacrificar la filosofia Unix de hacer una cosa y hacerla bien.

2. Ecosistema Pomodomate
La CLI es una pieza independiente de un ecosistema mas grande. Cada parte tiene su propio repositorio y ciclo de vida.

Producto	Estado	Repo	Notas
pomodomate.com	Desplegada	Privado	Web ya funcional
App Mobile	Planificacion	Privado (monorepo)	Monorepo junto a la web
pomodomate-cli	En desarrollo	Publico (GitHub)	Este documento

Estructura de Repositorios
El monorepo de web y mobile es privado. La CLI tiene su propio repo publico separado.

Repo publico (open source):
github.com/pomodomate/pomodomate-cli

Monorepo privado (web + mobile):
web/   →  pomodomate.com
mobile/  →  app iOS / Android

Como se conectan en el futuro
La CLI solo conoce una URL: api.pomodomate.com. El backend es completamente privado. La CLI es como un control remoto — su codigo es abierto pero el servidor es de Pomodomate.com.

Flujo de sincronizacion (Fase 2, no MVP):
CLI  →  api.pomodomate.com  →  backend privado  →  base de datos

3. Fases de Desarrollo
La CLI se lanza offline primero. La sincronizacion se agrega en Fase 2 cuando la API este lista.

Fase	Timeframe	Alcance
Fase 1
MVP offline	Mes 1 - 2	    • Timer Pomodoro completo (trabajo / descanso corto / descanso largo)
    • Mascota Domate animada en Unicode + ANSI 24-bit
    • Keybindings: space, r, s, q
    • Historial local en ~/.local/share/pomodomate/
    • Heatmap de sesiones estilo GitHub
    • Config en ~/.config/pomodomate/config.toml
    • Notificaciones nativas Wayland (Dunst, Mako, SwayNC)
    • Publicacion en AUR y cargo install
Fase 2
Sync	Mes 3 - 4	    • Login con cuenta Pomodomate.com desde la CLI
    • Sync del historial local con api.pomodomate.com
    • Ver pomodoros de la CLI en la web y mobile
    • Historial guardado en formato compatible con la API
Fase 3
Modo Domate	Mes 5+	    • Deteccion de distraccion por camara (100% local)
    • Deteccion de inactividad de teclado/mouse
    • Activacion con flag: pomodomate --domate
    • Codigo abierto, procesamiento local, sin datos a servidores

4. Diseno Visual y Mascota
Domate
Domate es la mascota oficial de Pomodomate.com. Un tomate redondo y expresivo con caracter propio: rojo vibrante, hoja verde, mejillas rosadas y un display en el pecho donde aparece el tiempo. Es la identidad visual central en todas las plataformas.

Estados de Animacion

Estado	Trigger	Comportamiento
Idle / Trabajando	Timer corriendo	Ojos abiertos, parpadeo lento, expresion concentrada
Descanso Corto	Break 5 min	Ojos cerrados felices, zzz flotando arriba
Descanso Largo	Break 15 min	Animacion de estiramiento, cara relajada
Amanecer	Primera sesion del dia	Animacion especial de bienvenida
Ultimo Minuto	Menos de 60 seg	Sudando, ojos grandes, expresion urgente
Completado	Pomodoro terminado	Saltando, estrellitas alrededor, expresion euforica

Implementacion Tecnica
    • Renderizado con Unicode block characters + colores ANSI 24-bit.
    • Compatible con cualquier terminal moderna sin protocolos especiales.
    • Frames generados desde la imagen original de Domate y hardcodeados en src/ui/mascot.rs.
    • 2 a 4 frames por estado, intercambiados en cada tick del timer.

Paleta de Colores
Rojo Tomate
#C0392B	Verde Naturaleza
#27AE60	Rosa Mejillas
#FADBD8	Oscuro Base
#1C2833

5. Stack Tecnico
Categoria	Tecnologia	Razon
Lenguaje	Rust	Binario unico, sin runtime, rendimiento nativo
TUI Framework	ratatui + crossterm	Estandar actual para TUI en Rust, layouts flexibles
Configuracion	serde + toml	Config legible en ~/.config/pomodomate/config.toml
Historial	chrono + JSON	Timestamps de sesiones, formato compatible con API futura
Notificaciones	notify-rust	Nativas Wayland: Dunst, Mako, SwayNC
Distribucion	AUR + cargo install	Nativo Arch Linux + universal con Cargo

Estructura del Proyecto
pomodomate-cli/
src/main.rs          - Entry point y argumentos CLI
src/app.rs           - Estado global de la aplicacion
src/timer.rs         - Logica del ciclo Pomodoro
src/config.rs        - Configuracion por usuario
src/storage.rs       - Historial local (compatible con API futura)
src/ui/mascot.rs     - Frames de animacion de Domate
src/ui/heatmap.rs    - Vista progreso estilo GitHub
Cargo.toml  /  README.md  /  PRIVACY.md

6. Licencia y Propiedad Intelectual
Que es open source y que no
Open Source (MIT) - Publico	Cerrado - Propiedad de Pomodomate.com
CLI completa (timer, UI, Domate)	Backend y API de sincronizacion
Modo Domate (deteccion local)	Base de datos de usuarios
Toda la logica que corre en tu maquina	pomodomate.com (web)
Codigo auditado por la comunidad	App mobile iOS / Android

Proteccion de la Marca
El codigo es libre pero el nombre y la mascota son de Pomodomate.com. Nadie puede distribuir algo llamado Pomodomate o usar a Domate sin permiso.

    • Nombre Pomodomate y mascota Domate: registrar como marca en INDECOPI (Peru).
    • El diseno de Domate tiene derechos de autor automaticos desde su creacion.
    • Cualquier fork debe mantener el copyright original visible en el codigo.
    • La expansion internacional se planifica via OMPI / WIPO cuando el proyecto crezca.

Copyright
MIT License - Copyright 2025 Pomodomate.com
pomodomate.com

Transparencia de Datos (PRIVACY.md)
El repositorio incluira un archivo PRIVACY.md con las siguientes garantias verificables por cualquiera en el codigo fuente:

    • Todo se procesa localmente en la maquina del usuario.
    • Ningun frame de video de la camara se guarda ni se transmite.
    • Sin sync activo, ningun dato sale de tu maquina jamas.
    • El codigo que lo garantiza esta en src/ y es auditable publicamente.

7. Distribucion
Linux - Plataforma Principal
    • AUR (Arch User Repository): paquete pomodomate-cli para Arch Linux y derivados.
    • cargo install pomodomate-cli: instalacion universal para cualquier usuario con Rust.
    • Releases en GitHub con binarios precompilados para x86_64 y aarch64.

Compatibilidad
    • Hyprland + Waybar: integracion nativa como widget de estado.
    • Terminales: Alacritty, WezTerm, Kitty, GNOME Terminal, Konsole.
    • Notificaciones Wayland: Dunst, Mako, SwayNC.

Windows (Futuro)
Posible en v2.0+ dado que Rust es cross-platform. Requiere adaptaciones en notificaciones (WinRT) y rutas de configuracion. No forma parte del MVP.

Pomodomate.com  -  pomodomate-cli PRD v0.1  -  2025