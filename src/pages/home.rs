use leptos::prelude::*;
use leptos_router::hooks::{use_location, use_navigate};
use wasm_bindgen::JsCast;
use crate::data::*;
use crate::components::table::*;
use crate::pages::map::{setup_click_handlers, value_to_color};
use std::collections::HashMap;
use crate::components::nav::SiteNav;
use crate::numeraires::Numeraire;
use crate::components::numeraire_chooser::NumeraireChooser;

fn region_slug(r: &str) -> String {
    r.to_lowercase().replace(' ', "-")
}

fn region_from_slug(s: &str) -> Option<String> {
    REGIONS.iter().find(|r| region_slug(r) == s).map(|r| r.to_string())
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


const WORLD_SVG: &str = include_str!("../../assets/world.svg");

#[derive(Clone, Copy, PartialEq)]
enum NumField { Pop, Cap, Land, Freedom, Openness }

#[derive(Clone, Copy)]
struct NumFilter { field: NumField, gt: bool, val: f64 }

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
    let (idx, gt) = t.find('>').map(|i| (i, true)).or_else(|| t.find('<').map(|i| (i, false)))?;
    let field = match &t[..idx] {
        "pop" | "population" => NumField::Pop,
        "cap" => NumField::Cap,
        "land" | "area" => NumField::Land,
        "freedom" | "free" => NumField::Freedom,
        "openness" | "open" => NumField::Openness,
        _ => return None,
    };
    let rest = &t[idx + 1..];
    if rest.is_empty() { return None; }
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
    Some(NumFilter { field, gt, val: base * mult })
}

fn passes(c: &Country, f: &NumFilter) -> bool {
    let v = match f.field {
        NumField::Pop => c.population as f64,
        NumField::Cap => c.money_supply_b_usd,
        NumField::Land => c.land_area_km2 as f64,
        NumField::Freedom => c.index().freedom,
        NumField::Openness => c.index().openness,
    };
    if f.gt { v > f.val } else { v < f.val }
}

const FILTER_EXAMPLES: [&str; 5] = ["freedom>60", "openness>50", "pop>100m", "cap>1t", "area>1m"];

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
    let (search, set_search) = signal(initial_q);
    let (numeraire, set_numeraire) = signal(Numeraire::Usd);

    let (panel_open, set_panel_open) = signal(false);
    let input_ref = NodeRef::<leptos::html::Input>::new();
    let wrap_ref = NodeRef::<leptos::html::Div>::new();

    // "/" focuses search from anywhere on the page
    Effect::new(move |_| {
        use wasm_bindgen::prelude::*;
        let closure = Closure::wrap(Box::new(move |ev: web_sys::KeyboardEvent| {
            if ev.key() == "/" {
                let tag = ev.target()
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

    // Landing title: "Cyberstates in Europe by movement freedom"
    Effect::new(move |_| {
        let (region, field) = state.get();
        let mut t = String::from("Cyberstates");
        if region != "All" {
            t.push_str(&format!(" in {}", region));
        }
        t.push_str(&format!(" by {}", field.label().to_lowercase()));
        t.push_str(" — Global Visa Openness Analytics");
        document().set_title(&t);
    });

    // Desktop side map: paint the world by the active rating object;
    // region/search/numeric filters dim non-matching states live.
    let countries_for_map = countries.clone();
    Effect::new(move |_| {
        let (region, field) = state.get();
        let (query, filters) = parse_query(&search.get().to_lowercase());
        let mut values: HashMap<String, f64> = HashMap::new();
        for c in &countries_for_map {
            let visible = (region == "All" || c.region == region)
                && (query.is_empty()
                    || c.name.to_lowercase().contains(&query)
                    || c.code.to_lowercase().contains(&query)
                    || c.currency_code.to_lowercase().contains(&query))
                && filters.iter().all(|f| passes(c, f));
            if visible {
                let v = c.metric(field);
                // log-compress heavy-tailed axes so the palette spreads
                let v = match field {
                    SortField::Freedom | SortField::Hospitality => v,
                    _ => (v + 1.0).ln().max(0.0),
                };
                values.insert(c.code.clone(), v);
            }
        }
        let max_val = values.values().cloned().fold(0.0_f64, f64::max);
        if let Some(w) = web_sys::window() {
            use wasm_bindgen::prelude::*;
            let cb = Closure::wrap(Box::new(move || {
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    if let Ok(paths) = doc.query_selector_all("svg.world-map path[id]") {
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
                }
                setup_click_handlers();
            }) as Box<dyn FnMut()>);
            let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), 200);
            cb.forget();
        }
    });

    let countries_for_count = countries.clone();
    let filtered_sorted = move || {
        let mut list = countries.clone();
        let (region, field) = state.get();
        let (query, filters) = parse_query(&search.get().to_lowercase());

        if region != "All" {
            list.retain(|c| c.region == region);
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
            b.metric(field).partial_cmp(&a.metric(field)).unwrap_or(std::cmp::Ordering::Equal)
        });

        list
    };

    view! {
        <div style="padding: 20px;">
            <div style="max-width: 1400px; margin: 0 auto;">
            // Header row 1: logo — centered search — map flush right
            <div class="header-row1">
                <div class="logo-zone">
                    <h1 class="logo">
                        <span style="color: var(--cyber-green);">"CYBER"</span>
                        <span style="color: #fff;">"STATES"</span>
                    </h1>
                    <div class="logo-suffix">
                        {move || {
                            let (region, field) = state.get();
                            let mut s = String::new();
                            if region != "All" {
                                s.push_str(&format!("in {} · ", region.to_lowercase()));
                            }
                            s.push_str(&format!("by {}", field.label().to_lowercase()));
                            s
                        }}
                    </div>
                </div>
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
                <SiteNav active="STATES">
                    <NumeraireChooser numeraire=numeraire set_numeraire=set_numeraire />
                </SiteNav>
            </div>

            // Header row 2: rating landings — real links, every one a page
            <div class="header-row2">
                <div class="region-pills">
                    {SortField::ALL.map(|f| {
                        view! {
                            <a
                                class=move || if state.get().1 == f { "region-pill active" } else { "region-pill" }
                                href=move || landing_path(&state.get().0, f)
                            >
                                {f.label()}
                            </a>
                        }
                    }).collect_view()}
                </div>
                <p class="state-count">
                    {move || {
                        let region = state.get().0;
                        let (query, filters) = parse_query(&search.get().to_lowercase());
                        let count = countries_for_count.iter().filter(|c| {
                            (region == "All" || c.region == region)
                            && (query.is_empty() || c.name.to_lowercase().contains(&query) || c.code.to_lowercase().contains(&query))
                            && filters.iter().all(|f| passes(c, f))
                        }).count();
                        format!("{} of {} states", count, total)
                    }}
                </p>
            </div>

            </div>

            // Table + desktop map, split — full viewport width
            <div class="home-split">
                <div class="table-pane">
                    <div style="overflow-x: auto; border: 1px solid #111; border-radius: 4px;">
                        <table class="cyber-table slim">
                            // fixed layout: widths live here, not in row content
                            <colgroup>
                                <col style="width: 9%;" />   // #
                                <col style="width: 34%;" />  // state
                                <col style="width: 13%;" />  // token
                                <col style="width: 22%;" />  // price
                                <col style="width: 22%;" />  // rating object
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
                    <div class="world-map-container" inner_html=WORLD_SVG></div>
                    <div style="display: flex; align-items: center; gap: 12px; margin-top: 12px; justify-content: center;">
                        <span style="font-size: 10px; color: #555; letter-spacing: 1px;">"LOW"</span>
                        <div style="width: 240px; height: 8px; border-radius: 4px; background: linear-gradient(to right, #ff0040, #ff6600, #ffd700, #00e5ff, #00ff41);"></div>
                        <span style="font-size: 10px; color: #555; letter-spacing: 1px;">"HIGH"</span>
                        <span style="font-size: 10px; color: #444; letter-spacing: 2px; margin-left: 12px;">{move || sort_field.get().label()}</span>
                    </div>
                </div>
            </div>

            // Footer
            <div style="max-width: 1400px; margin: 24px auto 0; padding: 16px; background: #050505; border: 1px solid #111; border-radius: 4px; font-size: 10px; color: #333; line-height: 2;">
                <div><span class="text-cyber-green">"FREEDOM"</span>" — √(eco_out × pop_out) weighted moving freedom"</div>
                <div><span class="text-cyber-magenta">"OPENNESS"</span>" — √(eco_in × pop_in) weighted border openness"</div>
                <div style="color: #444; margin-top: 4px;">"Weights: visa-free=1.0, VoA=0.8, eTA/eVisa=0.5, visa-required=0.1, no-admission=0.0"" · "<a href="/methodology" style="color: var(--cyber-green); text-decoration: none;">"METHODOLOGY →"</a></div>
            </div>
        </div>
    }
}
