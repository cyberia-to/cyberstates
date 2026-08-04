use leptos::prelude::*;
use leptos_router::hooks::use_location;
use wasm_bindgen::JsCast;
use std::collections::HashMap;
use crate::data::*;
use crate::components::nav::SiteNav;
use crate::components::brand::BrandChooser;
use crate::pages::map::{setup_click_handlers, value_to_color};
use crate::numeraires::{fmt_cap, fmt_value, price_parts, Numeraire};

const WORLD_SVG: &str = include_str!("../../assets/world.svg");

/// Canonical landing path for a token rating. Root = by capital.
fn landing_path(field: SortField) -> String {
    if field == SortField::Capital {
        "/tokens".to_string()
    } else {
        format!("/tokens/by/{}", field.slug())
    }
}

fn parse_path(path: &str) -> SortField {
    path.rsplit('/')
        .next()
        .and_then(SortField::from_slug)
        .unwrap_or(SortField::Capital)
}

/// A token is the aggregate of its zone: capital is the token's cap,
/// population and area sum over member states, the scores are the
/// population-weighted average holder's scores.
fn token_metric(t: &Token, f: SortField, scores: &HashMap<String, (f64, f64, f64)>) -> f64 {
    match f {
        SortField::Capital => t.total_supply_b_usd,
        SortField::Human => {
            if t.total_population > 0 { t.total_supply_b_usd * 1e9 / t.total_population as f64 } else { 0.0 }
        }
        SortField::Land => {
            if t.total_area_km2 > 0 { t.total_supply_b_usd * 1e9 / t.total_area_km2 as f64 } else { 0.0 }
        }
        SortField::Density => {
            if t.total_area_km2 > 0 { t.total_population as f64 / t.total_area_km2 as f64 } else { 0.0 }
        }
        SortField::Population => t.total_population as f64,
        SortField::Territory => t.total_area_km2 as f64,
        SortField::Freedom | SortField::Hospitality => {
            let (mut wsum, mut psum) = (0.0, 0.0);
            for (code, _, _) in &t.countries {
                if let Some((pop, fr, ho)) = scores.get(code) {
                    let v = if f == SortField::Freedom { *fr } else { *ho };
                    wsum += pop * v;
                    psum += pop;
                }
            }
            if psum > 0.0 { wsum / psum } else { 0.0 }
        }
    }
}

fn fmt_int(v: u64) -> String {
    let s = v.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 { out.push(','); }
        out.push(c);
    }
    out
}

fn score_color(v: f64) -> &'static str {
    if v > 60.0 { "var(--cyber-green)" }
    else if v > 40.0 { "var(--cyber-cyan)" }
    else if v > 25.0 { "var(--cyber-yellow)" }
    else if v > 10.0 { "var(--cyber-orange)" }
    else { "var(--cyber-red)" }
}

fn metric_cell(t: &Token, f: SortField, n: Numeraire, scores: &HashMap<String, (f64, f64, f64)>) -> (String, &'static str) {
    let v = token_metric(t, f, scores);
    match f {
        SortField::Capital => (fmt_cap(v, n), "#e0e0e0"),
        SortField::Human | SortField::Land => (fmt_value(v, n), "#e0e0e0"),
        SortField::Freedom | SortField::Hospitality => (format!("{:.1}", v), score_color(v)),
        SortField::Population => (fmt_int(v as u64), "#e0e0e0"),
        SortField::Territory => (format!("{} km²", fmt_int(v as u64)), "#e0e0e0"),
        SortField::Density => (format!("{:.1}/km²", v), "#e0e0e0"),
    }
}

#[component]
pub fn TokensPage() -> impl IntoView {
    let tokens = get_tokens();
    let total = tokens.len();

    // per-state (population, freedom, hospitality) for weighted zone scores
    let scores: HashMap<String, (f64, f64, f64)> = load_countries()
        .iter()
        .map(|c| {
            let idx = c.index();
            (c.code.clone(), (c.population as f64, idx.freedom, idx.openness))
        })
        .collect();
    let scores = std::sync::Arc::new(scores);

    let location = use_location();
    let field = Signal::derive(move || parse_path(&location.pathname.get()));

    let (search, set_search) = signal(String::new());
    let (rating_open, set_rating_open) = signal(false);
    let numeraire = use_context::<RwSignal<Numeraire>>().expect("numeraire context");

    Effect::new(move |_| {
        document().set_title(&format!(
            "Cyberstates tokens by {} — the sovereignty terminal",
            field.get().label().to_lowercase()
        ));
    });

    // Desktop side map: every state wears its token's rank — currency
    // zones surface as solid blocks of one color.
    let tokens_for_map = tokens.clone();
    let scores_for_map = scores.clone();
    Effect::new(move |_| {
        let f = field.get();
        let query = search.get().to_lowercase();
        let mut ranked: Vec<(Vec<String>, f64)> = tokens_for_map.iter()
            .filter(|t| {
                query.is_empty() || t.code.to_lowercase().contains(&query) || t.name.to_lowercase().contains(&query)
            })
            .map(|t| {
                let members = t.countries.iter().map(|(c, _, _)| c.clone()).collect();
                (members, token_metric(t, f, &scores_for_map))
            })
            .collect();
        ranked.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let n = ranked.len();
        // same palette rules as the states map: log min-max where polygon
        // size already encodes the axis, rank percentile everywhere else
        let log_scale = matches!(f, SortField::Population | SortField::Territory);
        let (vmin, vmax) = if log_scale && n > 0 {
            let p10 = ranked[(n as f64 * 0.10) as usize].1.max(1.0).ln();
            let top = ranked.last().map(|x| x.1.max(1.0).ln()).unwrap_or(1.0);
            (p10, top)
        } else {
            (0.0, 1.0)
        };
        let mut values: HashMap<String, f64> = HashMap::new();
        for (i, (members, v)) in ranked.into_iter().enumerate() {
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
            let t = if f.lower_is_better() { 1.0 - t } else { t };
            for code in members {
                values.insert(code, 0.02 + 0.98 * t);
            }
        }
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
                                        .map(|&v| value_to_color(v, 1.0))
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

    let tokens_for_count = tokens.clone();
    let scores_for_sort = scores.clone();
    let filtered_sorted = move || {
        let query = search.get().to_lowercase();
        let f = field.get();
        let mut list: Vec<(Token, f64)> = tokens
            .iter()
            .filter(|t| {
                query.is_empty() || t.code.to_lowercase().contains(&query) || t.name.to_lowercase().contains(&query)
            })
            .map(|t| {
                let m = token_metric(t, f, &scores_for_sort);
                (t.clone(), m)
            })
            .collect();
        list.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        list
    };

    let scores_for_cells = scores.clone();

    view! {
        <div class="page-shell">
            <div class="site-chrome">
            <div style="max-width: 1400px; margin: 0 auto;">
            // Header row 1: logo — nav
            <div class="header-row1">
                <div class="logo-zone">
                    <BrandChooser active="TOKENS" />
                    <div class="logo-suffix"></div>
                </div>
                <div class="rating-chooser">
                    <button
                        class="region-pill active rating-btn"
                        on:click=move |_| set_rating_open.update(|o| *o = !*o)
                    >
                        {move || field.get().short()}
                        <span style="opacity: 0.7;">" ▾"</span>
                    </button>
                    <div class="rating-menu" style:display=move || if rating_open.get() { "flex" } else { "none" }>
                        {SortField::ALL.map(|f| {
                            view! {
                                {f.derived_break().then(|| view! { <div class="menu-divider"></div> })}
                                <a
                                    class=move || if field.get() == f { "region-pill active" } else { "region-pill" }
                                    href=landing_path(f)
                                    on:click=move |_| set_rating_open.set(false)
                                >
                                    {f.label()}
                                </a>
                            }
                        }).collect_view()}
                    </div>
                </div>
                <SiteNav active="TOKENS" />
            </div>

            // Header row 2: the same eight ratings, for token zones
            <div class="header-row2">
                <span class="by-label">"by"</span>
                <div class="region-pills">
                    {SortField::ALL.map(|f| {
                        view! {
                            {f.derived_break().then(|| view! { <span class="pill-dot">"\u{b7}"</span> })}
                            <a
                                class=move || if field.get() == f { "region-pill active" } else { "region-pill" }
                                href=landing_path(f)
                            >
                                {f.label()}
                            </a>
                        }
                    }).collect_view()}
                </div>
            </div>

            </div>
            </div>

            // Table + desktop map, split — same stage as the states page
            <div class="home-split">
                <div class="table-pane">
                    <div class="table-shell">
                <table class="cyber-table slim">
                    // fixed layout: widths live here, not in row content
                    <colgroup>
                        <col class="c-rank" />
                        <col class="c-token" />
                        <col class="c-state" />
                        <col class="c-price" />
                        <col class="c-metric" />
                    </colgroup>
                    <thead>
                        <tr>
                            <th style="cursor: default; text-align: right;">"#"</th>
                            <th class="th-static">"TOKEN"</th>
                            <th class="th-static">"STATES"</th>
                            <th class="th-static" style="text-align: right;">"PRICE"</th>
                            <th class="th-static metric-th" style="text-align: right;">
                                {move || field.get().short()}
                            </th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            let scores = scores_for_cells.clone();
                            filtered_sorted()
                                .into_iter()
                                .enumerate()
                                .map(|(i, (t, _))| {
                                    let code = t.code.clone();
                                    let href = format!("/token/{}", code.to_lowercase());
                                    let n_states = t.countries.len();
                                    let flags: String = t.countries.iter().take(4).map(|(_, _, f)| f.as_str()).collect::<Vec<_>>().join(" ");
                                    let more = n_states.saturating_sub(4);
                                    let price_usd = t.price_usd;
                                    let t_for_metric = t.clone();
                                    let scores = scores.clone();
                                    view! {
                                        <tr on:click=move |_| {
                                            if let Some(window) = web_sys::window() {
                                                let _ = window.location().set_href(&href);
                                            }
                                        }>
                                            <td class="tabular-nums" style=format!("text-align: right; color: {}; font-weight: {};", rank_color(i + 1), rank_weight(i + 1))>{i + 1}</td>
                                            <td style="color: var(--cyber-yellow); font-weight: 700;">{code}</td>
                                            <td style="white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
                                                <span style="font-size: 16px;">{flags}</span>
                                                {(more > 0).then(|| view! { <span style="color: #444;">{format!(" +{}", more)}</span> })}
                                            </td>
                                            <td class="tabular-nums" style="text-align: right; color: var(--cyber-orange);">
                                                {move || {
                                                    let (head, frac, unit) = price_parts(price_usd, numeraire.get());
                                                    view! {
                                                        <span>{head}</span>
                                                        <span class="price-frac">{frac}</span>
                                                        <span class="price-unit">{unit}</span>
                                                    }
                                                }}
                                            </td>
                                            <td class="tabular-nums" style="text-align: right; font-weight: 700;">
                                                {move || {
                                                    let (text, color) = metric_cell(&t_for_metric, field.get(), numeraire.get(), &scores);
                                                    view! { <span style:color=color>{text}</span> }
                                                }}
                                            </td>
                                        </tr>
                                    }
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
                        <span style="font-size: 10px; color: #555; letter-spacing: 1px;">
                            {move || if field.get().lower_is_better() { "HIGH" } else { "LOW" }}
                        </span>
                        <div style="width: 240px; height: 8px; border-radius: 4px; background: linear-gradient(to right, #ff0040, #ff6600, #ffd700, #00e5ff, #00ff41);"></div>
                        <span style="font-size: 10px; color: #555; letter-spacing: 1px;">
                            {move || if field.get().lower_is_better() { "LOW" } else { "HIGH" }}
                        </span>
                        <span style="font-size: 10px; color: #444; letter-spacing: 2px; margin-left: 12px;">{move || format!("TOKEN {}", field.get().label())}</span>
                    </div>
                </div>
            </div>

            // Search dock
            <div class="search-dock">
                <span class="dock-count">
                    {move || {
                        let query = search.get().to_lowercase();
                        let count = tokens_for_count.iter().filter(|t| {
                            query.is_empty() || t.code.to_lowercase().contains(&query) || t.name.to_lowercase().contains(&query)
                        }).count();
                        if count == total { format!("{} tokens", total) } else { format!("{}/{} tokens", count, total) }
                    }}
                </span>
                <input
                    type="text"
                    class="search-input"
                    placeholder="Search token code or name..."
                    on:input=move |ev| {
                        let target = ev.target().unwrap();
                        let input: web_sys::HtmlInputElement = target.unchecked_into();
                        set_search.set(input.value());
                    }
                />
            </div>
        </div>
    }
}
