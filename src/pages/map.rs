use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

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
        out.push_str(&format!(
            "<path style=\"fill: {}; cursor: pointer;\" ",
            color
        ));
        out.push_str(chunk);
    }
    out
}

/// Cyber palette ramp: red → orange → yellow → cyan → green.
pub fn value_to_color(val: f64, max: f64) -> String {
    if max <= 0.0 {
        return "#111".to_string();
    }
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
    format!(
        "rgb({:.0},{:.0},{:.0})",
        r.min(255.0),
        g.min(255.0),
        b.min(255.0)
    )
}

/// Client-side navigation from plain DOM handlers: push the URL, then
/// wake the router with a synthetic popstate — no full page reload, so
/// the wasm app, fonts and state all survive the hop. Wrapped in a view
/// transition where the browser has one: frozen chrome, body dissolves.
pub fn navigate_client(path: &str) {
    let path = path.to_string();
    let go = move || {
        if let Some(w) = web_sys::window() {
            if let Ok(h) = w.history() {
                let _ = h.push_state_with_url(&JsValue::NULL, "", Some(&path));
                if let Ok(ev) = web_sys::PopStateEvent::new("popstate") {
                    let _ = w.dispatch_event(&ev);
                }
            }
        }
    };

    // collapse open dropdowns BEFORE the VT old snapshot
    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        if let Ok(nodes) =
            doc.query_selector_all(".rating-menu, .brand-menu, .num-menu, .search-panel")
        {
            for i in 0..nodes.length() {
                if let Some(node) = nodes.item(i) {
                    let el: web_sys::HtmlElement = node.unchecked_into();
                    let _ = el.style().set_property("display", "none");
                }
            }
        }
    }

    // top-anchor both sides of the dissolve so sticky thead / scrolled
    // rows never ghost at mismatched Y. only jump when actually scrolled.
    if let Some(w) = web_sys::window() {
        let y = w.scroll_y().unwrap_or(0.0);
        if y > 1.0 {
            w.scroll_to_with_x_and_y(0.0, 0.0);
        }
    }

    let doc = web_sys::window().and_then(|w| w.document());
    let svt = doc.as_ref().and_then(|d| {
        js_sys::Reflect::get(d.as_ref(), &JsValue::from_str("startViewTransition"))
            .ok()
            .and_then(|v| v.dyn_into::<js_sys::Function>().ok())
    });
    match (svt, doc) {
        (Some(f), Some(d)) => {
            // mark the hop so CSS can freeze sticky geometry if needed
            if let Some(root) = d.document_element() {
                let _ = root.class_list().add_1("vt-active");
            }
            let cb = Closure::once_into_js(go);
            let _ = f.call1(d.as_ref(), &cb);
            // remeasure after the dissolve (CSS ~320ms) — mid-transition
            // --chrome-h reflow was a major source of header twitch
            if let Some(w) = web_sys::window() {
                let cleanup = Closure::once(Box::new(move |_: JsValue| {
                    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                        if let Some(root) = doc.document_element() {
                            let _ = root.class_list().remove_1("vt-active");
                        }
                    }
                    measure_chrome_height();
                }) as Box<dyn FnMut(JsValue)>);
                let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(
                    cleanup.as_ref().unchecked_ref(),
                    380,
                );
                cleanup.forget();
            }
        }
        _ => {
            go();
            measure_chrome_height();
        }
    }
}

/// Set --chrome-h without clobbering other inline styles on <html>.
pub fn measure_chrome_height() {
    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        if let (Ok(Some(chrome)), Some(root)) =
            (doc.query_selector(".site-chrome"), doc.document_element())
        {
            let h = chrome
                .unchecked_into::<web_sys::HtmlElement>()
                .offset_height();
            let _ = root
                .unchecked_into::<web_sys::HtmlElement>()
                .style()
                .set_property("--chrome-h", &format!("{}px", h));
        }
    }
}

/// Wire every state path to navigate to its /state/ page on click.
/// Idempotent: repaint effects call this often — each path is wired once.
pub fn setup_click_handlers() {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();

    let paths = document
        .query_selector_all("svg.world-map path[id]")
        .unwrap();
    for i in 0..paths.length() {
        if let Some(node) = paths.item(i) {
            let el: web_sys::Element = node.unchecked_into();
            if el.get_attribute("data-nav").is_some() {
                continue;
            }
            let _ = el.set_attribute("data-nav", "1");
            if let Some(id) = el.get_attribute("id") {
                // SVG path ids are ISO codes; navigate to the name slug.
                let href = crate::data::slug_for_code(&id)
                    .map(|s| format!("/state/{}", s))
                    .unwrap_or_else(|| format!("/state/{}", id.to_lowercase()));
                let closure = Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
                    navigate_client(&href);
                })
                    as Box<dyn FnMut(web_sys::MouseEvent)>);

                let _ =
                    el.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref());
                closure.forget();
            }
        }
    }
}
