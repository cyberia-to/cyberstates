use crate::components::brand::BrandChooser;
use crate::components::legend::{goodness_keep, RatingLegend};
use crate::components::nav::SiteNav;
use crate::components::notyet::NotYet;
use crate::components::solar::SolarPanel;
use crate::data::*;
use crate::numeraires::{fmt_cap, fmt_value, price_parts, Numeraire};
use crate::pages::map::{painted_world, setup_click_handlers, value_to_color};
use leptos::prelude::*;
use leptos_router::hooks::use_location;
use std::collections::HashMap;
use wasm_bindgen::JsCast;

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
        SortField::Growth => t
            .price_delta()
            .filter(|d| *d > 0.0005)
            .map(|d| d * 100.0)
            .unwrap_or(f64::NEG_INFINITY),
        SortField::Loss => t
            .price_delta()
            .filter(|d| *d < -0.0005)
            .map(|d| -d * 100.0)
            .unwrap_or(f64::NEG_INFINITY),
        SortField::Human => {
            if t.total_population > 0 {
                t.total_supply_b_usd * 1e9 / t.total_population as f64
            } else {
                0.0
            }
        }
        SortField::Land => {
            if t.total_area_km2 > 0 {
                t.total_supply_b_usd * 1e9 / t.total_area_km2 as f64
            } else {
                0.0
            }
        }
        SortField::Density => {
            if t.total_area_km2 > 0 {
                t.total_population as f64 / t.total_area_km2 as f64
            } else {
                0.0
            }
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
            if psum > 0.0 {
                wsum / psum
            } else {
                0.0
            }
        }
    }
}

/// Map values for the token landing: every member state wears its
/// token's rank percentile — currency zones read as solid blocks.
/// Class toggles filter which members (and which tokens) participate —
/// same planets / continents / countries language as the states page.
fn token_map_values(
    tokens: &[Token],
    scores: &HashMap<String, (f64, f64, f64)>,
    by_code: &HashMap<String, Country>,
    field: SortField,
    query: &str,
    cut: Option<f64>,
    classes: (bool, bool, bool),
) -> HashMap<String, f64> {
    let (planets, continents, countries) = classes;
    let code_refs: HashMap<String, &Country> =
        by_code.iter().map(|(k, v)| (k.clone(), v)).collect();

    let mut ranked: Vec<(Vec<String>, f64)> = tokens
        .iter()
        .filter(|t| {
            (query.is_empty()
                || t.code.to_lowercase().contains(query)
                || t.name.to_lowercase().contains(query))
                && token_class_visible(t, &code_refs, planets, continents, countries)
        })
        .map(|t| {
            // only paint members that match the class toggles
            let members: Vec<String> = t
                .countries
                .iter()
                .filter(|(code, _, _)| {
                    by_code
                        .get(code)
                        .map(|c| c.class_visible(planets, continents, countries))
                        .unwrap_or(false)
                })
                .map(|(c, _, _)| c.clone())
                .collect();
            (members, token_metric(t, field, scores))
        })
        .filter(|(members, _)| !members.is_empty())
        .collect();
    ranked.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let n = ranked.len();
    let log_scale = matches!(field, SortField::Population | SortField::Territory);
    // first index of each distinct value — equal metrics share one color
    let first_rank = |band: &[(Vec<String>, f64)], v: f64| -> usize {
        band.iter().position(|(_, x)| *x == v).unwrap_or(0)
    };
    let (vmin, vmax) = if log_scale && n > 0 {
        let p10 = ranked[(n as f64 * 0.10) as usize].1.max(1.0).ln();
        let top = ranked.last().map(|x| x.1.max(1.0).ln()).unwrap_or(1.0);
        (p10, top)
    } else {
        (0.0, 1.0)
    };
    const FLOOR: f64 = 0.12;
    let mut values = HashMap::new();
    for (i, (members, v)) in ranked.iter().enumerate() {
        let t = if log_scale {
            if vmax > vmin {
                ((v.max(1.0).ln() - vmin) / (vmax - vmin)).clamp(0.0, 1.0)
            } else if *v > 0.0 {
                1.0
            } else {
                0.0
            }
        } else if n > 1 {
            first_rank(&ranked, *v) as f64 / (n - 1) as f64
        } else if *v > 0.0 {
            1.0
        } else {
            0.0
        };
        let t = if field.lower_is_better() { 1.0 - t } else { t };
        // ranked ascending: flip the index for the descending-order predicate
        if let Some(g) = cut {
            if !goodness_keep(n - 1 - i, n, field.lower_is_better(), g) {
                continue;
            }
        }
        let paint = FLOOR + (1.0 - FLOOR) * t.clamp(0.0, 1.0);
        for code in members {
            values.insert(code.clone(), paint);
        }
    }
    values
}

fn fmt_int(v: u64) -> String {
    compact_count(v as f64)
}

fn score_color(v: f64) -> &'static str {
    if v > 60.0 {
        "var(--cyber-green)"
    } else if v > 40.0 {
        "var(--cyber-cyan)"
    } else if v > 25.0 {
        "var(--cyber-yellow)"
    } else if v > 10.0 {
        "var(--cyber-orange)"
    } else {
        "var(--cyber-red)"
    }
}

fn metric_cell(
    t: &Token,
    f: SortField,
    n: Numeraire,
    scores: &HashMap<String, (f64, f64, f64)>,
) -> (String, &'static str) {
    match f {
        SortField::Growth | SortField::Loss => delta_cell(t.price_delta()),
        _ => {
            let v = token_metric(t, f, scores);
            match f {
                SortField::Capital => (fmt_cap(v, n), "#e0e0e0"),
                SortField::Human | SortField::Land => (fmt_value(v, n), "#e0e0e0"),
                SortField::Freedom | SortField::Hospitality => {
                    (format!("{:.1}", v), score_color(v))
                }
                SortField::Population => (fmt_int(v as u64), "#e0e0e0"),
                SortField::Territory => (format!("{} km²", format_area(v as u64)), "#e0e0e0"),
                SortField::Density => (format!("{:.1}/km²", v), "#e0e0e0"),
                SortField::Growth | SortField::Loss => unreachable!(),
            }
        }
    }
}

#[component]
pub fn TokensPage() -> impl IntoView {
    let tokens = get_tokens();
    let total = tokens.len();
    let all_countries = load_countries();
    let by_code: HashMap<String, Country> = all_countries
        .iter()
        .map(|c| (c.code.clone(), c.clone()))
        .collect();
    let by_code = std::sync::Arc::new(by_code);

    // per-state (population, freedom, hospitality) for weighted zone scores
    let scores: HashMap<String, (f64, f64, f64)> = all_countries
        .iter()
        .map(|c| {
            let idx = c.index();
            (
                c.code.clone(),
                (c.population as f64, idx.freedom, idx.openness),
            )
        })
        .collect();
    let scores = std::sync::Arc::new(scores);

    let location = use_location();
    let field = Signal::derive(move || parse_path(&location.pathname.get()));

    // same global class filter as states (localStorage)
    let init_classes = load_class_filter();
    store_class_filter(init_classes);
    let show_planets = RwSignal::new(init_classes.0);
    let show_continents = RwSignal::new(init_classes.1);
    let show_countries = RwSignal::new(init_classes.2);

    // the map mounts already painted — page switches never flash
    let initial_svg = painted_world(&token_map_values(
        &tokens,
        &scores,
        &by_code,
        parse_path(&location.pathname.get_untracked()),
        "",
        None,
        init_classes,
    ));
    // color-bar filter: keep only tokens at least this good (0..1)
    let color_cut = RwSignal::new(None::<f64>);
    let (search, set_search) = signal(String::new());
    let (rating_open, set_rating_open) = signal(false);
    let numeraire = use_context::<RwSignal<Numeraire>>().expect("numeraire context");

    Effect::new(move |_| {
        store_class_filter((
            show_planets.get(),
            show_continents.get(),
            show_countries.get(),
        ));
    });

    Effect::new(move |_| {
        document().set_title(&format!(
            "Cyberstates tokens by {} — the sovereignty terminal",
            field.get().label().to_lowercase()
        ));
    });

    // Desktop side map + solar panel: every visible member wears its
    // token's rank; class toggles dim planets / continents like states.
    let tokens_for_map = tokens.clone();
    let scores_for_map = scores.clone();
    let by_code_for_map = by_code.clone();
    Effect::new(move |_| {
        let f = field.get();
        let query = search.get().to_lowercase();
        let classes = (
            show_planets.get(),
            show_continents.get(),
            show_countries.get(),
        );
        let values = token_map_values(
            &tokens_for_map,
            &scores_for_map,
            &by_code_for_map,
            f,
            &query,
            color_cut.get(),
            classes,
        );
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
                                        .map(|&v| value_to_color(v, 1.0))
                                        .unwrap_or_else(|| "#1a1a1a".to_string());
                                    let _ = el.set_attribute(
                                        "style",
                                        &format!("fill: {}; cursor: pointer;", color),
                                    );
                                }
                            }
                        }
                    }
                    // solar dots share the values map — same as home
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

    let tokens_for_count = tokens.clone();
    let scores_for_sort = scores.clone();
    let by_code_for_filter = by_code.clone();
    let filtered_sorted = move || {
        let query = search.get().to_lowercase();
        let f = field.get();
        let classes = (
            show_planets.get(),
            show_continents.get(),
            show_countries.get(),
        );
        let code_refs: HashMap<String, &Country> = by_code_for_filter
            .iter()
            .map(|(k, v)| (k.clone(), v))
            .collect();
        let mut list: Vec<(Token, f64)> = tokens
            .iter()
            .filter(|t| {
                (query.is_empty()
                    || t.code.to_lowercase().contains(&query)
                    || t.name.to_lowercase().contains(&query))
                    && token_class_visible(t, &code_refs, classes.0, classes.1, classes.2)
            })
            .map(|t| {
                let m = token_metric(t, f, &scores_for_sort);
                (t.clone(), m)
            })
            .collect();
        list.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        if let Some(g) = color_cut.get() {
            let n = list.len();
            let mut i = 0;
            list.retain(|_| {
                let keep = goodness_keep(i, n, f.lower_is_better(), g);
                i += 1;
                keep
            });
        }
        list
    };

    let scores_for_cells = scores.clone();
    let by_code_for_count = by_code.clone();

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
                    <span class="by-label">"by"</span>
                    <button
                        class="region-pill active rating-btn"
                        on:click=move |_| set_rating_open.update(|o| *o = !*o)
                    >
                        {move || field.get().short()}
                        <span class="rating-caret" aria-hidden="true"></span>
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

            // Header row 2: ratings + class toggles (same as states)
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
                <RatingLegend field=field cut=color_cut />
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
                        <col class="c-delta" />
                        <col class="c-metric" />
                    </colgroup>
                    <thead>
                        <tr>
                            <th style="cursor: default; text-align: right;">"#"</th>
                            <th class="th-static">"TOKEN"</th>
                            <th class="th-static">"STATES"</th>
                            <th class="th-static" style="text-align: right;">"PRICE"</th>
                            <th class="th-static" style="text-align: right;">"24H"</th>
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
                                    let price_delta = t.price_delta();
                                    let t_for_metric = t.clone();
                                    let scores = scores.clone();
                                    view! {
                                        <tr on:click=move |_| crate::pages::map::navigate_client(&href)>
                                            <td class="tabular-nums" style=format!("text-align: right; color: {}; font-weight: {};", rank_color(i + 1), rank_weight(i + 1))>{i + 1}</td>
                                            <td style="color: var(--cyber-yellow); font-weight: 700;">{code}</td>
                                            <td style="white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
                                                <span style="font-size: 16px;">{flags}</span>
                                                {(more > 0).then(|| view! { <span style="color: #444;">{format!(" +{}", more)}</span> })}
                                            </td>
                                            <td class="tabular-nums" style="text-align: right; color: var(--cyber-orange);">
                                                {move || {
                                                    let (head, frac, unit) = price_parts(price_usd, numeraire.get());
                                                    if head == "N/A" {
                                                        view! { <NotYet /> }.into_any()
                                                    } else {
                                                        view! {
                                                            <span>{head}</span>
                                                            <span class="price-frac">{frac}</span>
                                                            <span class="price-unit">{unit}</span>
                                                        }.into_any()
                                                    }
                                                }}
                                            </td>
                                            <td class="tabular-nums delta-cell" style="text-align: right;">
                                                {
                                                    let (text, color) = delta_cell(price_delta);
                                                    view! { <span style:color=color>{text}</span> }
                                                }
                                            </td>
                                            <td class="tabular-nums" style="text-align: right; font-weight: 700;">
                                                {move || {
                                                    let (text, color) = metric_cell(&t_for_metric, field.get(), numeraire.get(), &scores);
                                                    if text == "N/A" {
                                                        return view! { <NotYet /> }.into_any();
                                                    }
                                                    let (num, unit) = match text.strip_suffix(" km\u{b2}") {
                                                        Some(n) => (n.to_string(), " km\u{b2}"),
                                                        None => (text, ""),
                                                    };
                                                    view! {
                                                        <span style:color=color>{num}</span>
                                                        {(!unit.is_empty()).then(|| view! { <span class="price-unit">{unit}</span> })}
                                                    }.into_any()
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
                    <div class="world-map-container" inner_html=initial_svg></div>
                    <SolarPanel />
                </div>
            </div>

            // Search dock
            <div class="search-dock">
                <span class="dock-count">
                    {move || {
                        let query = search.get().to_lowercase();
                        let classes = (
                            show_planets.get(),
                            show_continents.get(),
                            show_countries.get(),
                        );
                        let code_refs: HashMap<String, &Country> = by_code_for_count
                            .iter()
                            .map(|(k, v)| (k.clone(), v))
                            .collect();
                        let count = tokens_for_count.iter().filter(|t| {
                            (query.is_empty()
                                || t.code.to_lowercase().contains(&query)
                                || t.name.to_lowercase().contains(&query))
                                && token_class_visible(
                                    t,
                                    &code_refs,
                                    classes.0,
                                    classes.1,
                                    classes.2,
                                )
                        }).count();
                        let count = match color_cut.get() {
                            Some(g) => (0..count)
                                .filter(|&i| goodness_keep(i, count, field.get().lower_is_better(), g))
                                .count(),
                            None => count,
                        };
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
                            <a href="https://x.com/cyberiacap" target="_blank" rel="noopener" class="dock-credit">
                    "🏴 a "<span style="color: var(--cyber-green);">"cyberia"</span>" project"
                </a>
            </div>
        </div>
    }
}
