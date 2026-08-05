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

/// Percentile-painted map values for a landing: rank coloring, log
/// min-max where polygon size already encodes the axis, inverted where
/// lower is better. Shared by the pre-painted mount and the live patch.
fn map_values(
    countries: &[Country],
    region: &str,
    field: SortField,
    query: &str,
    filters: &[NumFilter],
    cut: Option<f64>,
    classes: (bool, bool, bool),
) -> std::collections::HashMap<String, f64> {
    let mut ranked: Vec<(String, f64)> = countries
        .iter()
        .filter(|c| {
            (match c.class() {
                ListingClass::Planet => classes.0,
                ListingClass::Continent => classes.1,
                ListingClass::Country => classes.2,
            }) && (region == "All" || c.region == region)
                && !(region != "All" && is_aggregate(&c.code))
                && (query.is_empty()
                    || c.name.to_lowercase().contains(query)
                    || c.code.to_lowercase().contains(query)
                    || c.currency_code.to_lowercase().contains(query))
                && filters.iter().all(|f| passes(c, f))
        })
        .map(|c| (c.code.clone(), c.metric(field)))
        .collect();

    // Territory: Sun + gas giants dwarf every terrestrial body on a log
    // scale. Drop them from the domain and pin full green so the Earth
    // map keeps dynamic range. Codes match states/*.toml.
    const TERRITORY_PIN_GREEN: &[&str] = &["SUN", "JUPI", "SATN", "URAN", "NEPT"];
    let pin_green: Vec<String> = if field == SortField::Territory {
        ranked
            .iter()
            .filter(|(code, _)| TERRITORY_PIN_GREEN.contains(&code.as_str()))
            .map(|(code, _)| code.clone())
            .collect()
    } else {
        Vec::new()
    };
    if !pin_green.is_empty() {
        ranked.retain(|(code, _)| !TERRITORY_PIN_GREEN.contains(&code.as_str()));
    }

    ranked.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let n = ranked.len();
    let log_scale = matches!(field, SortField::Population | SortField::Territory);
    let (vmin, vmax) = if log_scale && n > 0 {
        let p10 = ranked[(n as f64 * 0.10) as usize].1.max(1.0).ln();
        let top = ranked.last().map(|x| x.1.max(1.0).ln()).unwrap_or(1.0);
        (p10, top)
    } else {
        (0.0, 1.0)
    };
    let mut values = std::collections::HashMap::new();
    for (i, (code, v)) in ranked.into_iter().enumerate() {
        let t = if log_scale {
            if vmax > vmin {
                ((v.max(1.0).ln() - vmin) / (vmax - vmin)).clamp(0.0, 1.0)
            } else {
                1.0
            }
        } else if n > 1 {
            i as f64 / (n - 1) as f64
        } else {
            1.0
        };
        let t = if field.lower_is_better() { 1.0 - t } else { t };
        // ranked ascending: flip the index so goodness_keep sees the
        // same descending order the table uses
        if let Some(g) = cut {
            if !goodness_keep(n - 1 - i, n, field.lower_is_better(), g) {
                continue;
            }
        }
        values.insert(code, 0.02 + 0.98 * t);
    }
    for code in pin_green {
        values.insert(code, 1.0);
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

/// Filters live in the URL: ?q= text, ?top=NN the legend cut (percent
/// kept), ?classes= the enabled class toggles. Parse them back out.
fn parse_filter_params(search: &str) -> (Option<f64>, (bool, bool, bool)) {
    let mut cut = None;
    let mut classes = (true, true, true);
    for kv in search.trim_start_matches('?').split('&') {
        if let Some(v) = kv.strip_prefix("top=") {
            if let Ok(p) = v.parse::<f64>() {
                if p > 0.0 && p < 100.0 {
                    cut = Some(1.0 - p / 100.0);
                }
            }
        }
        if let Some(v) = kv.strip_prefix("classes=") {
            let has = |s: &str| v.split(',').any(|x| x == s);
            classes = (has("planets"), has("continents"), has("countries"));
        }
    }
    (cut, classes)
}

fn build_filter_query(q: &str, cut: Option<f64>, classes: (bool, bool, bool)) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !q.is_empty() {
        parts.push(format!("q={}", js_sys::encode_uri_component(q)));
    }
    if let Some(g) = cut {
        parts.push(format!("top={:.0}", (1.0 - g) * 100.0));
    }
    if classes != (true, true, true) {
        let mut on: Vec<&str> = Vec::new();
        if classes.0 {
            on.push("planets");
        }
        if classes.1 {
            on.push("continents");
        }
        if classes.2 {
            on.push("countries");
        }
        parts.push(format!("classes={}", on.join(",")));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("?{}", parts.join("&"))
    }
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
            (true, true, true),
        ))
    };
    let (search, set_search) = signal(initial_q);
    let numeraire = use_context::<RwSignal<Numeraire>>().expect("numeraire context");

    // filters are URL, not ephemera: seed from the query string
    let (init_cut, init_classes) = parse_filter_params(
        &web_sys::window()
            .and_then(|w| w.location().search().ok())
            .unwrap_or_default(),
    );
    // color-bar filter: keep only states at least this good (0..1)
    let color_cut = RwSignal::new(init_cut);
    // class toggles: planets / continents / countries, each independent
    let show_planets = RwSignal::new(init_classes.0);
    let show_continents = RwSignal::new(init_classes.1);
    let show_countries = RwSignal::new(init_classes.2);
    let class_on = move |c: &Country| match c.class() {
        ListingClass::Planet => show_planets.get(),
        ListingClass::Continent => show_continents.get(),
        ListingClass::Country => show_countries.get(),
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

    // URL -> state: back/forward (and any navigation) re-reads the query,
    // so history steps through filter states faithfully
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
        if show_planets.get_untracked() != classes.0 {
            show_planets.set(classes.0);
        }
        if show_continents.get_untracked() != classes.1 {
            show_continents.set(classes.1);
        }
        if show_countries.get_untracked() != classes.2 {
            show_countries.set(classes.2);
        }
        if search.get_untracked() != q {
            set_search.set(q);
        }
    });

    // state -> URL: toggles and the cut push history entries; typing in
    // the search replaces in place (no entry per keystroke)
    let last_written = StoredValue::new((String::new(), None::<f64>, (true, true, true), true));
    Effect::new(move |_| {
        let q = search.get();
        let cut = color_cut.get();
        let classes = (
            show_planets.get(),
            show_continents.get(),
            show_countries.get(),
        );
        let desired = build_filter_query(&q, cut, classes);
        let Some(w) = web_sys::window() else { return };
        let current = w.location().search().unwrap_or_default();
        let (last_q, last_cut, last_classes, first_run) = last_written.get_value();
        last_written.set_value((q.clone(), cut, classes, false));
        if first_run || desired == current {
            return;
        }
        let path = w.location().pathname().unwrap_or_default();
        let url = format!("{}{}", path, desired);
        if let Ok(h) = w.history() {
            use wasm_bindgen::JsValue;
            // structural filters make history; text edits rewrite in place
            if cut != last_cut || classes != last_classes {
                let _ = h.push_state_with_url(&JsValue::NULL, "", Some(&url));
            } else if q != last_q {
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
            <div style="max-width: 1400px; margin: 0 auto;">
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
                    <button
                        class="region-pill active rating-btn"
                        on:click=move |_| set_rating_open.update(|o| *o = !*o)
                    >
                        {move || state.get().1.short()}
                        <span style="opacity: 0.7;">" ▾"</span>
                    </button>
                    <div class="rating-menu" style:display=move || if rating_open.get() { "flex" } else { "none" }>
                        {SortField::ALL.map(|f| {
                            view! {
                                {f.derived_break().then(|| view! { <div class="menu-divider"></div> })}
                                <a
                                    class=move || if state.get().1 == f { "region-pill active" } else { "region-pill" }
                                    href=move || landing_path(&state.get().0, f)
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
                                href=move || landing_path(&state.get().0, f)
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
                                <col class="c-metric" />
                            </colgroup>
                            <thead>
                                <tr>
                                    <th style="cursor: default; text-align: right;">"#"</th>
                                    <th class="th-static">"STATE"</th>
                                    <th class="th-static">"TOKEN"</th>
                                    <th class="th-static" style="text-align: right;">"PRICE"</th>
                                    <th class="th-static metric-th" style="text-align: right;">
                                        {move || sort_field.get().short()}
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
                                                nav_r(&landing_path(&r_click, field), Default::default());
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
