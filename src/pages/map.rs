use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use std::collections::HashMap;
use crate::data::*;
use crate::components::nav::SiteNav;

const WORLD_SVG: &str = include_str!("../../assets/world.svg");

#[derive(Clone, Copy, PartialEq)]
enum MapMetric {
    Freedom,
    Openness,
    Population,
    Cap,
}

impl MapMetric {
    fn label(&self) -> &'static str {
        match self {
            Self::Freedom => "FREEDOM",
            Self::Openness => "OPENNESS",
            Self::Population => "POPULATION",
            Self::Cap => "CAP",
        }
    }
}

fn value_to_color(val: f64, max: f64) -> String {
    if max <= 0.0 { return "#111".to_string(); }
    let t = (val / max).min(1.0).max(0.0);

    // Cyber palette: red → orange → yellow → cyan → green
    // Matching --cyber-red #ff0040, --cyber-orange #ff6600,
    // --cyber-yellow #ffd700, --cyber-cyan #00e5ff, --cyber-green #00ff41
    if t < 0.01 {
        return "#1a1a1a".to_string(); // near-zero = dark
    }

    let (r, g, b) = if t < 0.25 {
        // red → orange
        let s = t / 0.25;
        (255.0, s * 102.0, 64.0 * (1.0 - s))
    } else if t < 0.5 {
        // orange → yellow
        let s = (t - 0.25) / 0.25;
        (255.0, 102.0 + s * 113.0, 0.0)
    } else if t < 0.75 {
        // yellow → cyan
        let s = (t - 0.5) / 0.25;
        (255.0 * (1.0 - s), 215.0 + s * 14.0, s * 255.0)
    } else {
        // cyan → green
        let s = (t - 0.75) / 0.25;
        (0.0, 229.0 + s * 26.0, 255.0 - s * 190.0)
    };

    format!("rgb({:.0},{:.0},{:.0})", r.min(255.0), g.min(255.0), b.min(255.0))
}

fn colorize_map(metric: MapMetric, query: &str) {
    let countries = load_countries();
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();

    // Pre-compute indices and values; a search query keeps only matching
    // states lit — the rest of the world goes dark
    let query = query.to_lowercase();
    let mut values: HashMap<String, f64> = HashMap::new();
    let mut max_val: f64 = 0.0;

    for c in &countries {
        let val = match metric {
            MapMetric::Freedom => c.index().freedom,
            MapMetric::Openness => c.index().openness,
            MapMetric::Population => (c.population as f64).ln().max(0.0),
            MapMetric::Cap => (c.money_supply_b_usd + 1.0).ln().max(0.0),
        };
        if val > max_val { max_val = val; }
        let visible = query.is_empty()
            || c.name.to_lowercase().contains(&query)
            || c.code.to_lowercase().contains(&query)
            || c.currency_code.to_lowercase().contains(&query);
        if visible {
            values.insert(c.code.clone(), val);
        }
    }

    // Find all path elements in the SVG
    let paths = document.query_selector_all("svg.world-map path[id]").unwrap();
    for i in 0..paths.length() {
        if let Some(node) = paths.item(i) {
            let el: web_sys::Element = node.unchecked_into();
            if let Some(id) = el.get_attribute("id") {
                let color = values.get(&id)
                    .map(|&v| value_to_color(v, max_val))
                    .unwrap_or_else(|| "#1a1a1a".to_string());
                let _ = el.set_attribute("style", &format!("fill: {}; cursor: pointer;", color));
            }
        }
    }
}

fn setup_click_handlers() {
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
                        let _ = w.location().set_href(&format!("/country/{}", code));
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

#[component]
pub fn MapPage() -> impl IntoView {
    let (metric, set_metric) = signal(MapMetric::Freedom);
    let (search, set_search) = signal(String::new());

    // Colorize + click handlers after DOM is ready
    Effect::new(move |_| {
        let m = metric.get();
        let q = search.get();
        let window = web_sys::window().unwrap();
        // Use setTimeout to ensure inner_html SVG is in the DOM
        let cb = Closure::wrap(Box::new(move || {
            colorize_map(m, &q);
            setup_click_handlers();
        }) as Box<dyn FnMut()>);
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
            cb.as_ref().unchecked_ref(), 300
        );
        cb.forget();
    });

    Effect::new(move |_| {
        document().set_title(&format!(
            "Cyberstates map by {} — Global Visa Openness Analytics",
            metric.get().label().to_lowercase()
        ));
    });

    let metrics = [MapMetric::Freedom, MapMetric::Openness, MapMetric::Population, MapMetric::Cap];

    view! {
        <div style="max-width: 1400px; margin: 0 auto; padding: 20px;">
            // Header row 1: logo — search — nav, same panel as the table pages
            <div class="header-row1">
                <div class="logo-zone">
                    <a href="/" style="text-decoration: none;">
                        <h1 class="logo">
                            <span style="color: var(--cyber-green);">"CYBER"</span>
                            <span style="color: #fff;">"STATES"</span>
                        </h1>
                    </a>
                    <div class="logo-suffix">
                        {move || format!("map · by {}", metric.get().label().to_lowercase())}
                    </div>
                </div>
                <input
                    type="text"
                    class="search-input"
                    placeholder="Search country, code, or currency..."
                    on:input=move |ev| {
                        let target = ev.target().unwrap();
                        let input: web_sys::HtmlInputElement = target.unchecked_into();
                        set_search.set(input.value());
                    }
                />
                <SiteNav active="MAP" />
            </div>

            // Header row 2: metric pills
            <div class="header-row2">
                <div class="region-pills">
                    {metrics.into_iter().map(|m| {
                        let label = m.label();
                        view! {
                            <button
                                class=move || if metric.get() == m { "region-pill active" } else { "region-pill" }
                                on:click=move |_| set_metric.set(m)
                            >
                                {label}
                            </button>
                        }
                    }).collect::<Vec<_>>()}
                </div>
            </div>

            // Map
            <div
                class="world-map-container"
                inner_html=WORLD_SVG
            />

            // Legend
            <div style="display: flex; align-items: center; gap: 12px; margin-top: 16px; justify-content: center;">
                <span style="font-size: 10px; color: #555; letter-spacing: 1px;">"LOW"</span>
                <div style="width: 300px; height: 8px; border-radius: 4px; background: linear-gradient(to right, #ff0040, #ff6600, #ffd700, #00e5ff, #00ff41);"></div>
                <span style="font-size: 10px; color: #555; letter-spacing: 1px;">"HIGH"</span>
            </div>
        </div>
    }
}
