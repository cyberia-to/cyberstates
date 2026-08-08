use crate::components::brand::BrandChooser;
use crate::components::legend::{goodness_keep, RatingLegend};
use crate::components::nav::SiteNav;
use crate::components::solar::SolarPanel;
use crate::components::table::*;
use crate::data::*;
use crate::numeraires::Numeraire;
use crate::pages::map::{painted_world, setup_click_handlers, value_to_color};
use leptos::prelude::*;
use leptos_router::hooks::{use_location, use_navigate};
use std::collections::HashMap;
use wasm_bindgen::JsCast;

fn region_slug(r: &str) -> String {
    r.to_lowercase().replace(' ', "-")
}

fn region_from_slug(s: &str) -> Option<String> {
    REGIONS
        .iter()
        .find(|r| region_slug(r) == s)
        .map(|r| r.to_string())
}

/// Canonical landing path for a view state. Root = All regions by capital.
/// Every rating reads top-down — there is no ascending direction.
fn landing_path(region: &str, field: SortField) -> String {
    let mut p = String::new();
    if region != "All" {
        p.push_str(&format!("/in/{}", region_slug(region)));
    }
    if field != SortField::Capital {
        p.push_str(&format!("/by/{}", field.slug()));
    }
    if p.is_empty() {
        p.push('/');
    }
    p
}

/// Parse a landing path back into (region, field).
fn parse_path(path: &str) -> (String, SortField) {
    let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let mut region = "All".to_string();
    let mut field = SortField::Capital;
    let mut i = 0;
    while i < segs.len() {
        match segs[i] {
            "in" if i + 1 < segs.len() => {
                if let Some(r) = region_from_slug(segs[i + 1]) {
                    region = r;
                }
                i += 2;
            }
            "by" if i + 1 < segs.len() => {
                if let Some(f) = SortField::from_slug(segs[i + 1]) {
                    field = f;
                }
                i += 2;
            }
            _ => i += 1,
        }
    }
    (region, field)
}

/// Two color spectra: planets scale among planets; continents+countries
/// among themselves. Same ramp, independent domains — works for every
/// rating (territory log no longer crushed by the Sun).
fn map_values(
    countries: &[Country],
    region: &str,
    field: SortField,
    query: &str,
    filters: &[NumFilter],
    cut: Option<f64>,
    classes: (bool, bool, bool),
) -> std::collections::HashMap<String, f64> {
    // (code, paint_value, rank_value, is_planet)
    let mut items: Vec<(String, f64, f64, bool)> = countries
        .iter()
        .filter(|c| {
            c.class_visible(classes.0, classes.1, classes.2)
                && (region == "All" || c.region == region)
                && !(region != "All" && is_aggregate(&c.code))
                && (query.is_empty()
                    || c.name.to_lowercase().contains(query)
                    || c.code.to_lowercase().contains(query)
                    || c.currency_code.to_lowercase().contains(query))
                && filters.iter().all(|f| passes(c, f))
        })
        .map(|c| {
            (
                c.code.clone(),
                c.paint_metric(field),
                c.metric(field), // rank metric for cut alignment with table
                c.belongs_to(ListingClass::Planet),
            )
        })
        .collect();

    // Global rank for the goodness cut (table-aligned), then paint each band alone.
    // cut uses the *rank* metric (gainers-only / losers-only), not signed paint.
    items.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
    let n_all = items.len();
    let pass_cut: std::collections::HashSet<String> = items
        .iter()
        .enumerate()
        .filter(|(i, _)| match cut {
            Some(g) => goodness_keep(n_all - 1 - i, n_all, field.lower_is_better(), g),
            None => true,
        })
        .map(|(_, item)| item.0.clone())
        .collect();

    let log_scale = matches!(field, SortField::Population | SortField::Territory);
    let day_tape = field.is_day_change();
    let mut values = std::collections::HashMap::new();
    for is_planet in [true, false] {
        let mut band: Vec<(String, f64, f64)> = items
            .iter()
            .filter(|item| item.3 == is_planet)
            .map(|item| (item.0.clone(), item.1, item.2))
            .collect();
        // day tape: diverging color on signed %; else rank-paint on rank metric
        if day_tape {
            band.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        } else {
            band.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
        }
        let n = band.len();
        if n == 0 {
            continue;
        }
        // diverging: red ← flat → green from signed price %
        let max_abs = if day_tape {
            band.iter()
                .filter(|(_, p, _)| p.is_finite())
                .map(|(_, p, _)| p.abs())
                .fold(0.0_f64, f64::max)
                .max(1e-9)
        } else {
            0.0
        };
        // first index of each distinct value — equal metrics share one color
        // (all zero-capital solar bodies must not rainbow by sort order)
        let (vmin, vmax) = if log_scale {
            let p10 = band[(n as f64 * 0.10) as usize].2.max(1.0).ln();
            let top = band.last().map(|x| x.2.max(1.0).ln()).unwrap_or(1.0);
            (p10, top)
        } else {
            (0.0, 1.0)
        };
        for (code, paint, rank_v) in &band {
            if !pass_cut.contains(code) {
                continue;
            }
            let t = if day_tape {
                if !paint.is_finite() {
                    0.0 // no price tape → dark grey
                } else {
                    // 0.0 = worst red, 0.5 = flat, 1.0 = best green
                    (0.5 + 0.5 * (*paint / max_abs)).clamp(0.0_f64, 1.0)
                }
            } else if log_scale {
                if vmax > vmin {
                    ((rank_v.max(1.0).ln() - vmin) / (vmax - vmin)).clamp(0.0, 1.0)
                } else if *rank_v > 0.0 {
                    1.0
                } else {
                    0.0
                }
            } else if n > 1 {
                let first = band.iter().position(|(_, _, x)| x == rank_v).unwrap_or(0);
                first as f64 / (n - 1) as f64
            } else if *rank_v > 0.0 {
                1.0
            } else {
                0.0
            };
            let t = if !day_tape && field.lower_is_better() {
                1.0 - t
            } else {
                t
            };
            // Always paint entities that are in the listing. Floor keeps the
            // bottom rank (AQ capital=0, low-density GL, log-scale p10 clamp)
            // on the red tip of the ramp — never the missing-data grey
            // (#1a1a1a). Ties still share one t, so zero-capital solar bodies
            // stay a single color instead of a fake rainbow.
            // Day tape keeps mid-grey for flat (t≈0.5); floor only for missing.
            const FLOOR: f64 = 0.12;
            let paint_v = if day_tape {
                if t < 0.01 {
                    0.0
                } else {
                    t
                }
            } else {
                FLOOR + (1.0 - FLOOR) * t.clamp(0.0, 1.0)
            };
            values.insert(code.clone(), paint_v);
        }
    }
    values
}

#[derive(Clone, Copy, PartialEq)]
enum NumField {
    Pop,
    Cap,
    Land,
    Freedom,
    Openness,
}

#[derive(Clone, Copy)]
struct NumFilter {
    field: NumField,
    gt: bool,
    val: f64,
}

/// Split the raw search string into free text and typed numeric filters:
/// `europe cap>1t freedom>60` -> ("europe", [cap>1000, freedom>60])
fn parse_query(raw: &str) -> (String, Vec<NumFilter>) {
    let mut text = Vec::new();
    let mut filters = Vec::new();
    for tok in raw.split_whitespace() {
        match parse_filter_token(tok) {
            Some(f) => filters.push(f),
            None => text.push(tok),
        }
    }
    (text.join(" "), filters)
}

fn parse_filter_token(tok: &str) -> Option<NumFilter> {
    let t = tok.to_lowercase();
    let (idx, gt) = t
        .find('>')
        .map(|i| (i, true))
        .or_else(|| t.find('<').map(|i| (i, false)))?;
    let field = match &t[..idx] {
        "pop" | "population" => NumField::Pop,
        "cap" => NumField::Cap,
        "land" | "area" | "territory" => NumField::Land,
        "freedom" | "free" => NumField::Freedom,
        "openness" | "open" => NumField::Openness,
        _ => return None,
    };
    let rest = &t[idx + 1..];
    if rest.is_empty() {
        return None;
    }
    let (num_part, suffix) = match rest.chars().last() {
        Some(c @ ('k' | 'm' | 'b' | 't')) => (&rest[..rest.len() - 1], Some(c)),
        _ => (rest, None),
    };
    let base: f64 = num_part.parse().ok()?;
    // cap lives in billions USD; pop and land in raw units
    let mult = match (field, suffix) {
        (NumField::Cap, Some('k')) => 1e-6,
        (NumField::Cap, Some('m')) => 1e-3,
        (NumField::Cap, Some('b')) | (NumField::Cap, None) => 1.0,
        (NumField::Cap, Some('t')) => 1e3,
        (_, Some('k')) => 1e3,
        (_, Some('m')) => 1e6,
        (_, Some('b')) => 1e9,
        (_, Some('t')) => 1e12,
        (_, None) => 1.0,
        _ => 1.0,
    };
    Some(NumFilter {
        field,
        gt,
        val: base * mult,
    })
}

fn passes(c: &Country, f: &NumFilter) -> bool {
    let v = match f.field {
        NumField::Pop => c.population as f64,
        NumField::Cap => c.money_supply_b_usd,
        NumField::Land => c.land_area_km2 as f64,
        NumField::Freedom => c.index().freedom,
        NumField::Openness => c.index().openness,
    };
    if f.gt {
        v > f.val
    } else {
        v < f.val
    }
}

const FILTER_EXAMPLES: [&str; 5] = [
    "freedom>60",
    "openness>50",
    "pop>100m",
    "cap>1t",
    "territory>1m",
];

/// Filters in the URL: ?q= text, ?top=NN legend cut, ?classes= toggles.
/// `classes` is `None` when the param is absent — caller must not treat
/// that as "all on" (would wipe the global setting on ranking hops).
fn parse_filter_params(search: &str) -> (Option<f64>, Option<(bool, bool, bool)>) {
    let mut cut = None;
    let mut classes = None;
    for kv in search.trim_start_matches('?').split('&') {
        if let Some(v) = kv.strip_prefix("top=") {
            if let Ok(p) = v.parse::<f64>() {
                if p > 0.0 && p < 100.0 {
                    cut = Some(1.0 - p / 100.0);
                }
            }
        }
        if let Some(v) = kv.strip_prefix("classes=") {
            let has = |s: &str| v.split(',').filter(|x| !x.is_empty()).any(|x| x == s);
            classes = Some((has("planets"), has("continents"), has("countries")));
        }
    }
    (cut, classes)
}

fn encode_classes_for_url(c: (bool, bool, bool)) -> String {
    let mut on: Vec<&str> = Vec::new();
    if c.0 {
        on.push("planets");
    }
    if c.1 {
        on.push("continents");
    }
    if c.2 {
        on.push("countries");
    }
    on.join(",")
}

fn build_filter_query(q: &str, cut: Option<f64>, classes: (bool, bool, bool)) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !q.is_empty() {
        parts.push(format!("q={}", js_sys::encode_uri_component(q)));
    }
    if let Some(g) = cut {
        parts.push(format!("top={:.0}", (1.0 - g) * 100.0));
    }
    // stamp classes when not default so ranking links carry the global setting
    if classes != DEFAULT_CLASSES {
        parts.push(format!("classes={}", encode_classes_for_url(classes)));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("?{}", parts.join("&"))
    }
}

/// Path + current filter query — ranking / region hops keep the global
/// class filter (and search / top cut) instead of dropping the query.
fn landing_href(
    region: &str,
    field: SortField,
    q: &str,
    cut: Option<f64>,
    classes: (bool, bool, bool),
) -> String {
    format!(
        "{}{}",
        landing_path(region, field),
        build_filter_query(q, cut, classes)
    )
}

#[component]
pub fn HomePage() -> impl IntoView {
    let countries = load_countries();
    let total = countries.len();

    // The URL is the single source of truth for region + sort landings.
    let location = use_location();
    let state = Memo::new(move |_| parse_path(&location.pathname.get()));
    let sort_field = Signal::derive(move || state.get().1);

    let initial_q = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .and_then(|s| s.strip_prefix('?').map(str::to_string))
        .and_then(|s| {
            s.split('&')
                .find_map(|kv| kv.strip_prefix("q=").map(str::to_string))
        })
        .and_then(|v| js_sys::decode_uri_component(&v.replace('+', " ")).ok())
        .map(String::from)
        .unwrap_or_default();
    // filters: URL classes= wins (shareable link), else localStorage global,
    // else countries-only default. Cut still seeds from the query only.
    let (init_cut, url_classes) = parse_filter_params(
        &web_sys::window()
            .and_then(|w| w.location().search().ok())
            .unwrap_or_default(),
    );
    let init_classes = url_classes.unwrap_or_else(load_class_filter);
    // persist URL override so the next ranking hop without ?classes= keeps it
    store_class_filter(init_classes);

    // the map mounts already painted — page switches never flash
    let initial_svg = {
        let (region, field) = parse_path(&location.pathname.get_untracked());
        let (q, filters) = parse_query(&initial_q.to_lowercase());
        painted_world(&map_values(
            &countries,
            &region,
            field,
            &q,
            &filters,
            None,
            init_classes,
        ))
    };
    let (search, set_search) = signal(initial_q);
    let numeraire = use_context::<RwSignal<Numeraire>>().expect("numeraire context");

    // color-bar filter: keep only states at least this good (0..1)
    let color_cut = RwSignal::new(init_cut);
    // class toggles: global site setting (planets / continents / countries)
    let show_planets = RwSignal::new(init_classes.0);
    let show_continents = RwSignal::new(init_classes.1);
    let show_countries = RwSignal::new(init_classes.2);
    // any enabled class membership matches (AQ = continent ∪ country)
    let class_on = move |c: &Country| {
        c.class_visible(
            show_planets.get(),
            show_continents.get(),
            show_countries.get(),
        )
    };
    let (panel_open, set_panel_open) = signal(false);
    let (rating_open, set_rating_open) = signal(false);
    let input_ref = NodeRef::<leptos::html::Input>::new();
    let wrap_ref = NodeRef::<leptos::html::Div>::new();

    // "/" focuses search from anywhere on the page
    Effect::new(move |_| {
        use wasm_bindgen::prelude::*;
        let closure = Closure::wrap(Box::new(move |ev: web_sys::KeyboardEvent| {
            if ev.key() == "/" {
                let tag = ev
                    .target()
                    .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                    .map(|e| e.tag_name())
                    .unwrap_or_default();
                if tag != "INPUT" && tag != "TEXTAREA" {
                    ev.prevent_default();
                    if let Some(inp) = input_ref.get_untracked() {
                        let _ = inp.focus();
                    }
                }
            }
        }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref());
        }
        closure.forget();
    });

    let nav = use_navigate();

    // URL -> state: back/forward re-reads the query. Missing ?classes=
    // must NOT reset toggles — ranking hops drop the query; the global
    // localStorage setting (and current signals) stay put.
    Effect::new(move |_| {
        let raw = location.search.get();
        let (cut, classes) = parse_filter_params(&raw);
        let q = raw
            .trim_start_matches('?')
            .split('&')
            .find_map(|kv| kv.strip_prefix("q=").map(str::to_string))
            .and_then(|v| js_sys::decode_uri_component(&v.replace('+', " ")).ok())
            .map(String::from)
            .unwrap_or_default();
        if color_cut.get_untracked() != cut {
            color_cut.set(cut);
        }
        if let Some(c) = classes {
            if show_planets.get_untracked() != c.0 {
                show_planets.set(c.0);
            }
            if show_continents.get_untracked() != c.1 {
                show_continents.set(c.1);
            }
            if show_countries.get_untracked() != c.2 {
                show_countries.set(c.2);
            }
            store_class_filter(c);
        }
        if search.get_untracked() != q {
            set_search.set(q);
        }
    });

    // class toggles → localStorage (global site setting)
    Effect::new(move |_| {
        store_class_filter((
            show_planets.get(),
            show_continents.get(),
            show_countries.get(),
        ));
    });

    // state -> URL: toggles and the cut push history entries; typing in
    // the search replaces in place (no entry per keystroke). Also re-stamps
    // the query when the pathname changes (rating hop via plain path).
    let last_written = StoredValue::new((
        String::new(),
        None::<f64>,
        DEFAULT_CLASSES,
        String::new(),
        true,
    ));
    Effect::new(move |_| {
        let q = search.get();
        let cut = color_cut.get();
        let classes = (
            show_planets.get(),
            show_continents.get(),
            show_countries.get(),
        );
        let path_now = location.pathname.get();
        let desired = build_filter_query(&q, cut, classes);
        let Some(w) = web_sys::window() else { return };
        let current = w.location().search().unwrap_or_default();
        let (last_q, last_cut, last_classes, last_path, first_run) = last_written.get_value();
        last_written.set_value((q.clone(), cut, classes, path_now.clone(), false));
        if first_run {
            return;
        }
        let path_changed = path_now != last_path;
        if desired == current && !path_changed {
            return;
        }
        // prefer live pathname (router may have just navigated)
        let path = w.location().pathname().unwrap_or(path_now);
        let url = format!("{}{}", path, desired);
        if let Ok(h) = w.history() {
            use wasm_bindgen::JsValue;
            // structural filters make history; text edits / path hops rewrite
            if cut != last_cut || classes != last_classes {
                let _ = h.push_state_with_url(&JsValue::NULL, "", Some(&url));
            } else if q != last_q || path_changed {
                let _ = h.replace_state_with_url(&JsValue::NULL, "", Some(&url));
            }
        }
    });

    // Landing title: "Cyberstates in Europe by movement freedom"
    Effect::new(move |_| {
        let (region, field) = state.get();
        let mut t = String::from("Cyberstates");
        if region != "All" {
            t.push_str(&format!(" in {}", region));
        }
        t.push_str(&format!(" by {}", field.label().to_lowercase()));
        t.push_str(" — the sovereignty terminal");
        document().set_title(&t);
    });

    // Desktop side map: paint the world by the active rating object;
    // region/search/numeric filters dim non-matching states live.
    let countries_for_map = countries.clone();
    Effect::new(move |_| {
        let (region, field) = state.get();
        let (query, filters) = parse_query(&search.get().to_lowercase());
        let values = map_values(
            &countries_for_map,
            &region,
            field,
            &query,
            &filters,
            color_cut.get(),
            (
                show_planets.get(),
                show_continents.get(),
                show_countries.get(),
            ),
        );
        let max_val = 1.0;
        if let Some(w) = web_sys::window() {
            use wasm_bindgen::prelude::*;
            let cb = Closure::wrap(Box::new(move || {
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    if let Ok(paths) = doc.query_selector_all("svg.world-map path[id]") {
                        for i in 0..paths.length() {
                            if let Some(node) = paths.item(i) {
                                let el: web_sys::Element = node.unchecked_into();
                                if let Some(id) = el.get_attribute("id") {
                                    let color = values
                                        .get(&id)
                                        .map(|&v| value_to_color(v, max_val))
                                        .unwrap_or_else(|| "#1a1a1a".to_string());
                                    let _ = el.set_attribute(
                                        "style",
                                        &format!("fill: {}; cursor: pointer;", color),
                                    );
                                }
                            }
                        }
                    }
                }
                // the solar dots are painted from the SAME values map, so
                // every filter that darkens the world darkens the system
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    if let Ok(dots) = doc.query_selector_all(".solar-panel circle[data-code]") {
                        for i in 0..dots.length() {
                            if let Some(node) = dots.item(i) {
                                let el: web_sys::Element = node.unchecked_into();
                                if let Some(code) = el.get_attribute("data-code") {
                                    let color = values
                                        .get(&code)
                                        .map(|&v| value_to_color(v, 1.0))
                                        .unwrap_or_else(|| "#1a1a1a".to_string());
                                    let _ = el.set_attribute("fill", &color);
                                }
                            }
                        }
                    }
                }
                setup_click_handlers();
            }) as Box<dyn FnMut()>);
            let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                200,
            );
            cb.forget();
        }
    });

    let countries_for_count = countries.clone();
    let filtered_sorted = move || {
        let mut list = countries.clone();
        let (region, field) = state.get();
        let (query, filters) = parse_query(&search.get().to_lowercase());

        list.retain(|c| class_on(c));
        if region != "All" {
            // aggregates rank globally, never among their own members
            list.retain(|c| c.region == region && !is_aggregate(&c.code));
        }
        if !query.is_empty() {
            list.retain(|c| {
                c.name.to_lowercase().contains(&query)
                    || c.code.to_lowercase().contains(&query)
                    || c.currency_code.to_lowercase().contains(&query)
            });
        }
        for f in &filters {
            list.retain(|c| passes(c, f));
        }

        list.sort_by(|a, b| {
            b.metric(field)
                .partial_cmp(&a.metric(field))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if let Some(g) = color_cut.get() {
            let n = list.len();
            let mut i = 0;
            list.retain(|_| {
                let keep = goodness_keep(i, n, field.lower_is_better(), g);
                i += 1;
                keep
            });
        }

        list
    };

    view! {
        <div class="page-shell">
            <div class="site-chrome">
            // full width of the shell — same stage as table + map (no 1400px cap)
            <div class="chrome-inner">
            // Header row 1: logo — centered search — map flush right
            <div class="header-row1">
                <div class="logo-zone">
                    <BrandChooser active="STATES" />
                    <div class="logo-suffix">
                        {move || {
                            let (region, _) = state.get();
                            if region != "All" { format!("in {}", region.to_lowercase()) } else { String::new() }
                        }}
                    </div>
                </div>
                <div class="rating-chooser">
                    <span class="by-label">"by"</span>
                    <button
                        class="region-pill active rating-btn"
                        on:click=move |_| set_rating_open.update(|o| *o = !*o)
                    >
                        {move || state.get().1.label()}
                        <span class="rating-caret" aria-hidden="true"></span>
                    </button>
                    <div class="rating-menu" style:display=move || if rating_open.get() { "flex" } else { "none" }>
                        {SortField::ALL.map(|f| {
                            view! {
                                {f.derived_break().then(|| view! { <div class="menu-divider"></div> })}
                                <a
                                    class=move || if state.get().1 == f { "region-pill active" } else { "region-pill" }
                                    href=move || landing_href(
                                        &state.get().0,
                                        f,
                                        &search.get(),
                                        color_cut.get(),
                                        (show_planets.get(), show_continents.get(), show_countries.get()),
                                    )
                                    on:click=move |_| set_rating_open.set(false)
                                >
                                    {f.label()}
                                </a>
                            }
                        }).collect_view()}
                    </div>
                </div>
                <SiteNav active="STATES" />
            </div>

            // Header row 2: rating landings — pills on desktop, a dropdown
            // at thumb scale
            <div class="header-row2">
                <span class="by-label">"by"</span>
                <div class="region-pills">
                    {SortField::ALL.map(|f| {
                        view! {
                            {f.derived_break().then(|| view! { <span class="pill-dot">"\u{b7}"</span> })}
                            <a
                                class=move || if state.get().1 == f { "region-pill active" } else { "region-pill" }
                                href=move || landing_href(
                                    &state.get().0,
                                    f,
                                    &search.get(),
                                    color_cut.get(),
                                    (show_planets.get(), show_continents.get(), show_countries.get()),
                                )
                            >
                                {f.label()}
                            </a>
                        }
                    }).collect_view()}
                </div>
                <div class="class-filter">
                    {[
                        ("\u{1FA90}", "planets", show_planets),
                        ("\u{1F30D}", "continents", show_continents),
                        ("\u{1F1FA}\u{1F1F3}", "countries", show_countries),
                    ].map(|(icon, name, sig)| view! {
                        <button
                            class=move || if sig.get() { "class-toggle on" } else { "class-toggle" }
                            title=name
                            on:click=move |_| sig.update(|v| *v = !*v)
                        >{icon}</button>
                    })}
                </div>
                <RatingLegend field=sort_field cut=color_cut />
            </div>

            </div>
            </div>

            // Table + desktop map, split — full viewport width
            <div class="home-split">
                <div class="table-pane">
                    <div class="table-shell">
                        <table class="cyber-table slim">
                            // fixed layout: widths live here, not in row content
                            <colgroup>
                                <col class="c-rank" />
                                <col class="c-state" />
                                <col class="c-token" />
                                <col class="c-price" />
                                <col class="c-delta" />
                                <col class="c-metric" />
                            </colgroup>
                            <thead>
                                <tr>
                                    <th style="cursor: default; text-align: right;">"#"</th>
                                    <th class="th-static">"STATE"</th>
                                    <th class="th-static">"TOKEN"</th>
                                    <th class="th-static" style="text-align: right;">"PRICE"</th>
                                    <th class="th-static" style="text-align: right;">"24H"</th>
                                    <th class="th-static metric-th" style="text-align: right;">
                                        // growth/loss only reorders rows — values stay capital
                                        {move || {
                                            if sort_field.get().is_day_change() {
                                                "CAPITAL"
                                            } else {
                                                sort_field.get().short()
                                            }
                                        }}
                                    </th>
                                </tr>
                            </thead>
                            <tbody>
                                {move || {
                                    filtered_sorted()
                                        .into_iter()
                                        .enumerate()
                                        .map(|(i, country)| {
                                            view! { <CountryRow country=country rank={i + 1} numeraire=numeraire field=sort_field /> }
                                        })
                                        .collect::<Vec<_>>()
                                }}
                            </tbody>
                        </table>
                    </div>
                </div>
                <div class="map-pane">
                    <div class="world-map-container" inner_html=initial_svg></div>
                    <SolarPanel />
                </div>
            </div>

            // Search dock — the command line lives at the thumb; the count
            // lives beside the filters that change it
            <div class="search-dock">
                <span class="dock-count">
                    {move || {
                        let region = state.get().0;
                        let (query, filters) = parse_query(&search.get().to_lowercase());
                        let count = countries_for_count.iter().filter(|c| {
                            class_on(c)
                            && (region == "All" || c.region == region)
                            && !(region != "All" && is_aggregate(&c.code))
                            && (query.is_empty() || c.name.to_lowercase().contains(&query) || c.code.to_lowercase().contains(&query))
                            && filters.iter().all(|f| passes(c, f))
                        }).count();
                        let count = match color_cut.get() {
                            Some(g) => (0..count)
                                .filter(|&i| goodness_keep(i, count, state.get().1.lower_is_better(), g))
                                .count(),
                            None => count,
                        };
                        if count == total { format!("{} cyberstates", total) } else { format!("{}/{} cyberstates", count, total) }
                    }}
                </span>
                <div
                    class="search-wrap"
                    node_ref=wrap_ref
                    on:focusout=move |ev| {
                        // close only when focus leaves the wrapper entirely
                        let inside = ev.related_target()
                            .and_then(|t| t.dyn_into::<web_sys::Node>().ok())
                            .map(|n| wrap_ref.get_untracked().map(|w| w.contains(Some(&n))).unwrap_or(false))
                            .unwrap_or(false);
                        if !inside {
                            set_panel_open.set(false);
                        }
                    }
                    on:keydown=move |ev| {
                        if ev.key() == "Escape" {
                            set_panel_open.set(false);
                            if let Some(inp) = input_ref.get_untracked() {
                                let _ = inp.blur();
                            }
                        }
                    }
                >
                    <input
                        type="text"
                        class="search-input"
                        placeholder="Search — or filter: freedom>60, cap>1t..."
                        node_ref=input_ref
                        prop:value=move || search.get()
                        on:focus=move |_| set_panel_open.set(true)
                        on:input=move |ev| {
                            let target = ev.target().unwrap();
                            let input: web_sys::HtmlInputElement = target.unchecked_into();
                            set_search.set(input.value());
                        }
                    />
                    <div class="search-panel" style:display=move || if panel_open.get() { "block" } else { "none" }>
                        <div class="panel-row">
                            <span class="panel-label">"REGION"</span>
                            <div class="panel-chips">
                                {REGIONS.iter().map(|&r| {
                                    let r_click = r.to_string();
                                    let r_class = r.to_string();
                                    let nav_r = nav.clone();
                                    view! {
                                        <button
                                            class=move || if state.get().0 == r_class { "region-pill active" } else { "region-pill" }
                                            on:click=move |_| {
                                                let (_, field) = state.get();
                                                let href = landing_href(
                                                    &r_click,
                                                    field,
                                                    &search.get_untracked(),
                                                    color_cut.get_untracked(),
                                                    (
                                                        show_planets.get_untracked(),
                                                        show_continents.get_untracked(),
                                                        show_countries.get_untracked(),
                                                    ),
                                                );
                                                nav_r(&href, Default::default());
                                            }
                                        >{r}</button>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>
                        <div class="panel-row">
                            <span class="panel-label">"FILTER"</span>
                            <div class="panel-chips">
                                {FILTER_EXAMPLES.map(|ex| {
                                    view! {
                                        <button
                                            class="region-pill filter-chip"
                                            on:click=move |_| {
                                                let cur = search.get_untracked();
                                                let sep = if cur.is_empty() || cur.ends_with(' ') { "" } else { " " };
                                                set_search.set(format!("{}{}{}", cur, sep, ex));
                                                if let Some(inp) = input_ref.get_untracked() {
                                                    let _ = inp.focus();
                                                }
                                            }
                                        >{ex}</button>
                                    }
                                }).collect_view()}
                            </div>
                        </div>
                        <div class="panel-hint">"mix text and filters: europe cap>1t · / focuses · esc closes"</div>
                    </div>
                </div>
                <a href="https://x.com/cyberiacap" target="_blank" rel="noopener" class="dock-credit">
                    "🏴 a "<span style="color: var(--cyber-green);">"cyberia"</span>" project"
                </a>
            </div>
        </div>
    }
}
