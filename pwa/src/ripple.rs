//! Material ripple feedback for `.btn` clicks (theme-android.css's
//! `.ripple-dot`/`md-ripple`). Called unconditionally from every button's
//! `on:click`, regardless of which theme is active — under any theme but
//! Android's, `.ripple-dot` has no animation defined, so the spawned span
//! is inert (briefly present in the DOM, invisible, cleaned up the same
//! way) rather than needing its own theme check here.

use std::time::Duration;

use gloo_timers::future::sleep;
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, MouseEvent};

const RIPPLE_DURATION: Duration = Duration::from_millis(500);

/// Positions a ripple at the click point — `MouseEvent::offset_x/y` are
/// already relative to `current_target`, so no `getBoundingClientRect`
/// subtraction is needed — sized to comfortably cover the button from any
/// corner, and removes itself once the CSS animation would have finished.
pub fn spawn(ev: &MouseEvent) {
    let Some(button) = ev
        .current_target()
        .and_then(|target| target.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let Ok(dot) = document.create_element("span") else {
        return;
    };
    dot.set_class_name("ripple-dot");

    let size = button.offset_width().max(button.offset_height()) as f64 * 2.0;
    let x = ev.offset_x() as f64 - size / 2.0;
    let y = ev.offset_y() as f64 - size / 2.0;
    if let Some(dot_el) = dot.dyn_ref::<HtmlElement>() {
        let style = dot_el.style();
        let _ = style.set_property("left", &format!("{x}px"));
        let _ = style.set_property("top", &format!("{y}px"));
        let _ = style.set_property("width", &format!("{size}px"));
        let _ = style.set_property("height", &format!("{size}px"));
    }

    if button.append_child(&dot).is_err() {
        return;
    }

    wasm_bindgen_futures::spawn_local(async move {
        sleep(RIPPLE_DURATION).await;
        dot.remove();
    });
}
