use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use std::collections::HashMap;

pub const WORLD_SVG: &str = include_str!("../../assets/world.svg");

/// The world with its fills already inlined: mounting this string shows
/// the map in final colors on its very first frame — switching pages
/// never flashes an unpainted world. Later changes patch the live DOM.
pub fn painted_world(values: &HashMap<String, f64>) -> String {
    let mut parts = WORLD_SVG.split("<path ");
    let mut out = String::with_capacity(WORLD_SVG.len() + values.len() * 48);
    out.push_str(parts.next().unwrap_or(""));
    for chunk in parts {
        let color = chunk
            .split("id=\"")
            .nth(1)
            .and_then(|r| r.split('"').next())
            .and_then(|id| values.get(id))
            .map(|&v| value_to_color(v, 1.0))
            .unwrap_or_else(|| "#1a1a1a".to_string());
        out.push_str(&format!("<path style=\"fill: {}; cursor: pointer;\" ", color));
        out.push_str(chunk);
    }
    out
}

/// Cyber palette ramp: red → orange → yellow → cyan → green.
pub fn value_to_color(val: f64, max: f64) -> String {
    if max <= 0.0 { return "#111".to_string(); }
    let t = (val / max).min(1.0).max(0.0);
    if t < 0.01 {
        return "#1a1a1a".to_string();
    }
    let (r, g, b) = if t < 0.25 {
        let s = t / 0.25;
        (255.0, s * 102.0, 64.0 * (1.0 - s))
    } else if t < 0.5 {
        let s = (t - 0.25) / 0.25;
        (255.0, 102.0 + s * 113.0, 0.0)
    } else if t < 0.75 {
        let s = (t - 0.5) / 0.25;
        (255.0 * (1.0 - s), 215.0 + s * 14.0, s * 255.0)
    } else {
        let s = (t - 0.75) / 0.25;
        (0.0, 229.0 + s * 26.0, 255.0 - s * 190.0)
    };
    format!("rgb({:.0},{:.0},{:.0})", r.min(255.0), g.min(255.0), b.min(255.0))
}

/// Wire every state path to navigate to its /state/ page on click.
pub fn setup_click_handlers() {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();

    let paths = document.query_selector_all("svg.world-map path[id]").unwrap();
    for i in 0..paths.length() {
        if let Some(node) = paths.item(i) {
            let el: web_sys::Element = node.unchecked_into();
            if let Some(id) = el.get_attribute("id") {
                let code = id.to_lowercase();
                let closure = Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
                    if let Some(w) = web_sys::window() {
                        let _ = w.location().set_href(&format!("/state/{}", code));
                    }
                }) as Box<dyn FnMut(web_sys::MouseEvent)>);

                let _ = el.add_event_listener_with_callback(
                    "click",
                    closure.as_ref().unchecked_ref(),
                );
                closure.forget();
            }
        }
    }
}
