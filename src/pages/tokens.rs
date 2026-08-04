use leptos::prelude::*;
use leptos_router::hooks::use_location;
use wasm_bindgen::JsCast;
use std::collections::HashMap;
use crate::data::*;
use crate::components::nav::SiteNav;
use crate::components::brand::BrandChooser;
use crate::numeraires::{fmt_cap, fmt_value, price_parts, Numeraire};

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
        <div class="page-frame">
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

            // Table
            <div class="table-shell">
                <table class="cyber-table">
                    <colgroup>
                        <col style="width: 5%;" />   // #
                        <col style="width: 9%;" />   // token
                        <col style="width: 22%;" />  // name
                        <col style="width: 27%;" />  // states
                        <col style="width: 15%;" />  // price
                        <col style="width: 22%;" />  // rating object
                    </colgroup>
                    <thead>
                        <tr>
                            <th style="cursor: default;">"#"</th>
                            <th class="th-static">"TOKEN"</th>
                            <th class="th-static">"NAME"</th>
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
                                    let name = t.name.clone();
                                    let n_states = t.countries.len();
                                    let flags: String = t.countries.iter().take(10).map(|(_, _, f)| f.as_str()).collect::<Vec<_>>().join(" ");
                                    let more = n_states.saturating_sub(10);
                                    let price_usd = t.price_usd;
                                    let t_for_metric = t.clone();
                                    let scores = scores.clone();
                                    view! {
                                        <tr on:click=move |_| {
                                            if let Some(window) = web_sys::window() {
                                                let _ = window.location().set_href(&href);
                                            }
                                        }>
                                            <td style="color: #333; text-align: right;">{i + 1}</td>
                                            <td style="color: var(--cyber-yellow); font-weight: 700;">{code}</td>
                                            <td style="color: #ccc;">{name}</td>
                                            <td>
                                                <span style="color: #888; margin-right: 8px;">{n_states}</span>
                                                <span style="font-size: 14px;">{flags}</span>
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
