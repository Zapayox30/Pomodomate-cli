use std::sync::mpsc::Sender;
use std::time::Duration;

use anyhow::{Context, Result};
use wayland_client::protocol::{wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::ext::idle_notify::v1::client::{
    ext_idle_notification_v1, ext_idle_notifier_v1,
};

/// What the compositor told us about the user's presence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleEvent {
    /// No keyboard or pointer input for the configured timeout.
    Idled,
    /// The user came back.
    Resumed,
}

/// Wayland state for the idle watcher.
struct Watcher {
    seat: Option<wl_seat::WlSeat>,
    notifier: Option<ext_idle_notifier_v1::ExtIdleNotifierV1>,
    tx: Sender<IdleEvent>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for Watcher {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        else {
            return;
        };

        match interface.as_str() {
            "wl_seat" => {
                state.seat = Some(registry.bind(name, version.min(7), qh, ()));
            }
            "ext_idle_notifier_v1" => {
                state.notifier = Some(registry.bind(name, 1, qh, ()));
            }
            _ => {}
        }
    }
}

impl Dispatch<ext_idle_notification_v1::ExtIdleNotificationV1, ()> for Watcher {
    fn event(
        state: &mut Self,
        _: &ext_idle_notification_v1::ExtIdleNotificationV1,
        event: ext_idle_notification_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let signal = match event {
            ext_idle_notification_v1::Event::Idled => IdleEvent::Idled,
            ext_idle_notification_v1::Event::Resumed => IdleEvent::Resumed,
            _ => return,
        };
        // The receiver going away just means the app is shutting down.
        let _ = state.tx.send(signal);
    }
}

// Neither of these ever sends us events; the impls exist to satisfy the
// dispatch requirements of the objects we bind.
impl Dispatch<wl_seat::WlSeat, ()> for Watcher {
    fn event(
        _: &mut Self,
        _: &wl_seat::WlSeat,
        _: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ext_idle_notifier_v1::ExtIdleNotifierV1, ()> for Watcher {
    fn event(
        _: &mut Self,
        _: &ext_idle_notifier_v1::ExtIdleNotifierV1,
        _: ext_idle_notifier_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

/// Check whether idle detection can work in this session.
///
/// Connects, binds the globals and reports precisely what is missing. Useful
/// for diagnosing "why doesn't my timer pause" without guessing.
pub fn probe() -> Result<()> {
    let conn = Connection::connect_to_env().context("not a Wayland session")?;
    let display = conn.display();

    let mut queue = conn.new_event_queue();
    let qh = queue.handle();
    display.get_registry(&qh, ());

    let (tx, _rx) = std::sync::mpsc::channel();
    let mut watcher = Watcher {
        seat: None,
        notifier: None,
        tx,
    };

    queue
        .roundtrip(&mut watcher)
        .context("failed to talk to the Wayland compositor")?;

    if watcher.notifier.is_none() {
        anyhow::bail!("compositor does not advertise ext-idle-notify-v1");
    }
    if watcher.seat.is_none() {
        anyhow::bail!("no Wayland seat available");
    }
    Ok(())
}

/// Watch for user inactivity, sending events down `tx` until the channel is
/// dropped.
///
/// Blocks, so callers should run it on its own thread. Returns an error when
/// the session is not Wayland or the compositor lacks `ext-idle-notify-v1`,
/// which callers are expected to treat as "this feature is unavailable"
/// rather than as a failure.
fn run(timeout: Duration, tx: Sender<IdleEvent>) -> Result<()> {
    let conn = Connection::connect_to_env().context("not a Wayland session")?;
    let display = conn.display();

    let mut queue = conn.new_event_queue();
    let qh = queue.handle();
    display.get_registry(&qh, ());

    let mut watcher = Watcher {
        seat: None,
        notifier: None,
        tx,
    };

    // First roundtrip populates the globals advertised by the compositor.
    queue
        .roundtrip(&mut watcher)
        .context("failed to talk to the Wayland compositor")?;

    let notifier = watcher
        .notifier
        .clone()
        .context("compositor does not support ext-idle-notify-v1")?;
    let seat = watcher.seat.clone().context("no Wayland seat available")?;

    // The protocol takes milliseconds.
    let _notification = notifier.get_idle_notification(timeout.as_millis() as u32, &seat, &qh, ());

    loop {
        queue
            .blocking_dispatch(&mut watcher)
            .context("Wayland connection closed")?;
    }
}

/// Start watching for inactivity on a background thread.
///
/// Returns the receiving end of the event channel, or `None` when idle
/// detection is not available in this session. Callers should degrade
/// gracefully: an X11 session or a compositor without the protocol simply
/// gets no idle events.
pub fn watch(timeout: Duration) -> Option<std::sync::mpsc::Receiver<IdleEvent>> {
    if timeout.is_zero() {
        return None;
    }

    // Decide availability synchronously rather than racing a background
    // thread: either the protocol is there now or the feature is off.
    probe().ok()?;

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = run(timeout, tx);
    });

    Some(rx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_zero_timeout_disables_watching() {
        assert!(
            watch(Duration::ZERO).is_none(),
            "idle detection must stay off when the timeout is zero"
        );
    }

    #[test]
    fn idle_events_compare_by_value() {
        assert_eq!(IdleEvent::Idled, IdleEvent::Idled);
        assert_ne!(IdleEvent::Idled, IdleEvent::Resumed);
    }

    /// Confirms the protocol binds here. Ignored by default: it needs a live
    /// Wayland session, so it cannot run in CI.
    ///
    /// Run it with: `cargo test -- --ignored --nocapture idle_probe`
    #[test]
    #[ignore]
    fn idle_probe_finds_the_protocol() {
        match probe() {
            Ok(()) => eprintln!("idle detection is available in this session"),
            Err(e) => panic!("idle detection unavailable: {e:#}"),
        }
    }

    /// Live check against the running compositor. Ignored by default because
    /// it needs a Wayland session with `ext-idle-notify-v1` and, by its very
    /// nature, a few seconds of not touching the keyboard.
    ///
    /// Run it with: `cargo test -- --ignored --nocapture idle_watcher`
    #[test]
    #[ignore]
    fn idle_watcher_receives_events_from_the_compositor() {
        let events = watch(Duration::from_secs(3)).expect("idle watching unavailable here");
        eprintln!("waiting up to 20s for an idle event — stop typing now…");

        let event = events
            .recv_timeout(Duration::from_secs(20))
            .expect("compositor never reported inactivity");
        assert_eq!(event, IdleEvent::Idled);
        eprintln!("got {event:?} from the compositor");
    }
}
