use leptos::prelude::*;
use wasm_bindgen::JsCast;
use crate::data::*;
use crate::numeraires::{fmt_value, Numeraire};
use crate::components::nav::SiteNav;
use crate::components::brand::BrandChooser;

#[component]
pub fn CountryPage() -> impl IntoView {
    let params = leptos_router::hooks::use_params_map();
    let countries = load_countries();

    let country = move || {
        let p = params.get();
        let code = p.get("code").unwrap_or_default().to_uppercase();
        countries.iter().find(|c| c.code == code).cloned()
    };

    view! {
        <div class="page-frame">
            {move || match country() {
                None => view! {
                    <div>
                        <SiteHeader suffix="not found".to_string() />
                        <h1 style="color: var(--cyber-red); margin-top: 40px;">"STATE NOT FOUND"</h1>
                    </div>
                }.into_any(),
                Some(c) => {
                    let idx = c.index();
                    let sc = |v: f64| -> &'static str {
                        if v > 60.0 { "var(--cyber-green)" }
                        else if v > 40.0 { "var(--cyber-cyan)" }
                        else if v > 25.0 { "var(--cyber-yellow)" }
                        else if v > 10.0 { "var(--cyber-orange)" }
                        else { "var(--cyber-red)" }
                    };
                    let freedom_color = sc(idx.freedom);
                    let openness_color = sc(idx.openness);

                    let region_color = match c.region.as_str() {
                        "Africa" => "var(--cyber-yellow)",
                        "Asia" => "var(--cyber-magenta)",
                        "Europe" => "var(--cyber-blue)",
                        "Eurasia" => "var(--cyber-purple)",
                        "Latin America" => "var(--cyber-orange)",
                        "Middle East" => "var(--cyber-pink)",
                        "North America" => "var(--cyber-cyan)",
                        "Oceania" => "var(--cyber-green)",
                        _ => "#666",
                    };

                    let flag = c.flag.clone();
                    let name = c.name.clone();
                    let code_str = c.code.clone();
                    let region = c.region.clone();
                    let pop = c.population_fmt();
                    let area = c.land_area_fmt();
                    let token_code = c.currency_code.clone();
                    let token_slug = token_code.to_lowercase();
                    let token_name = c.currency_name.clone();
                    let price = c.price_fmt();
                    let cap = c.cap_fmt();
                    let supply = c.supply_fmt();
                    let human_value = fmt_value(c.metric(SortField::Human), Numeraire::Usd);
                    let land_value = fmt_value(c.metric(SortField::Land), Numeraire::Usd);

                    // rank in each rating = position in that landing's table
                    let all_states = load_countries();
                    let rank_of = |f: SortField| -> (String, &'static str, &'static str) {
                        let mine = c.metric(f);
                        let rank = all_states.iter().filter(|o| o.metric(f) > mine).count() + 1;
                        // podium badges wear their metal at full brightness
                        match rank_medal(rank) {
                            Some(m) => (format!("#{}", rank), m, "1"),
                            None => (format!("#{}", rank), "var(--cyber-green)", "0.75"),
                        }
                    };
                    let r_capital = rank_of(SortField::Capital);
                    let r_human = rank_of(SortField::Human);
                    let r_land = rank_of(SortField::Land);
                    let r_freedom = rank_of(SortField::Freedom);
                    let r_hosp = rank_of(SortField::Hospitality);
                    let r_pop = rank_of(SortField::Population);
                    let r_area = rank_of(SortField::Territory);
                    let r_density = rank_of(SortField::Density);
                    let freedom_str = format!("{:.1}", idx.freedom);
                    let freedom_bar = format!("{}%", idx.freedom.min(100.0));
                    let openness_str = format!("{:.1}", idx.openness);
                    let openness_bar = format!("{}%", idx.openness.min(100.0));
                    let density = if c.land_area_km2 > 0 {
                        format!("{:.1}/km²", c.population as f64 / c.land_area_km2 as f64)
                    } else {
                        "N/A".to_string()
                    };

                    // Index
                    let idx = c.index();
                    let eco_out = format!("{:.1}%", idx.eco_out_pct);
                    let eco_in = format!("{:.1}%", idx.eco_in_pct);
                    let pop_out = format!("{:.1}%", idx.pop_out_pct);
                    let pop_in = format!("{:.1}%", idx.pop_in_pct);

                    // Get visa data
                    let outgoing = get_visa_outgoing(&c.code);
                    let incoming = get_visa_incoming(&c.name);

                    let suffix = name.to_lowercase();
                    view! {
                        <div>
                            <SiteHeader suffix=suffix />

                            // Hero: identity left, the two scores right
                            <div class="state-hero">
                                <div class="state-identity">
                                    <div style="display: flex; align-items: center; gap: 20px;">
                                        <span style="font-size: clamp(48px, 8vw, 72px); line-height: 1;">{flag}</span>
                                        <div>
                                            <h1 style="font-size: clamp(28px, 5vw, 52px); font-weight: 700; color: #fff; margin: 0; line-height: 1.05;">
                                                {name}
                                            </h1>
                                            <div style="display: flex; gap: 12px; align-items: center; margin-top: 10px;">
                                                <span style="font-size: 13px; color: #555; letter-spacing: 3px;">{code_str}</span>
                                                <span style:color=region_color style="font-size: 11px; padding: 3px 10px; border-radius: 2px; border: 1px solid; letter-spacing: 1px;">{region}</span>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </div>

                            // Fundamental: measured, not computed
                            <div class="section-label" style="margin-top: 28px;">"FUNDAMENTAL"</div>
                            <div class="stat-grid">
                                <div class="stat-card">
                                    <div class="stat-label">"TOKEN"</div>
                                    <div class="stat-value" style="color: var(--cyber-yellow);">
                                        <a href=format!("/token/{}", token_slug) style="color: var(--cyber-yellow); text-decoration: none;">{token_code}</a>
                                    </div>
                                    <div style="font-size: 12px; color: #444; margin-top: 4px;">{token_name}</div>
                                </div>
                                <div class="stat-card">
                                    <div class="stat-label">"PRICE"</div>
                                    <div class="stat-value" style="color: var(--cyber-orange);">{price}</div>
                                </div>
                                <div class="stat-card">
                                    <div class="stat-label">"SUPPLY"</div>
                                    <div class="stat-value" style="color: #888;">{supply}</div>
                                    <div style="font-size: 10px; color: #333; margin-top: 4px;">"native tokens in circulation"</div>
                                </div>
                                <div class="stat-card">
                                    <div class="stat-label">"CAPITAL"<span class="rank-badge" style:color=r_capital.1 style:opacity=r_capital.2>{r_capital.0}</span></div>
                                    <div class="stat-value" style="color: var(--cyber-yellow);">{cap}</div>
                                    <div style="font-size: 10px; color: #333; margin-top: 4px;">"money supply in USD"</div>
                                </div>
                                <div class="stat-card">
                                    <div class="stat-label">"POPULATION"<span class="rank-badge" style:color=r_pop.1 style:opacity=r_pop.2>{r_pop.0}</span></div>
                                    <div class="stat-value">{pop}</div>
                                </div>
                                <div class="stat-card">
                                    <div class="stat-label">"TERRITORY"<span class="rank-badge" style:color=r_area.1 style:opacity=r_area.2>{r_area.0}</span></div>
                                    <div class="stat-value" style="color: var(--cyber-blue);">{area}</div>
                                </div>
                            </div>

                            // Derived: computed from the fundamentals
                            <div class="section-label" style="margin-top: 20px;">"DERIVED"</div>
                            <div class="stat-grid">
                                <div class="stat-card">
                                    <div class="stat-label">"HUMAN VALUE"<span class="rank-badge" style:color=r_human.1 style:opacity=r_human.2>{r_human.0}</span></div>
                                    <div class="stat-value" style="color: var(--cyber-green);">{human_value}</div>
                                    <div style="font-size: 10px; color: #333; margin-top: 4px;">"capital per person"</div>
                                </div>
                                <div class="stat-card">
                                    <div class="stat-label">"LAND VALUE"<span class="rank-badge" style:color=r_land.1 style:opacity=r_land.2>{r_land.0}</span></div>
                                    <div class="stat-value" style="color: var(--cyber-cyan);">{land_value}</div>
                                    <div style="font-size: 10px; color: #333; margin-top: 4px;">"capital per km²"</div>
                                </div>
                                <div class="stat-card">
                                    <div class="stat-label">"DENSITY"<span class="rank-badge" style:color=r_density.1 style:opacity=r_density.2>{r_density.0}</span></div>
                                    <div class="stat-value" style="color: var(--cyber-purple);">{density}</div>
                                    <div style="font-size: 10px; color: #333; margin-top: 4px;">"population per km²"</div>
                                </div>
                            </div>

                            // the two visa-matrix scores: value, bar, and the
                            // formula with its live components
                            <div class="score-grid" style="margin-top: 12px;">
                                <div class="stat-card">
                                    <div class="stat-label">"TRAVEL FREEDOM"<span class="rank-badge" style:color=r_freedom.1 style:opacity=r_freedom.2>{r_freedom.0}</span></div>
                                    <div class="stat-value tabular-nums" style:color=freedom_color>{freedom_str}</div>
                                    <div style="height: 5px; background: #111; border-radius: 3px; overflow: hidden; margin-top: 8px;">
                                        <div class="openness-bar" style:width=freedom_bar style:background=freedom_color></div>
                                    </div>
                                    <div style="font-size: 11px; color: #444; margin-top: 8px;">
                                        "√( eco out "<span style="color: var(--cyber-green);">{eco_out}</span>" × pop out "<span style="color: var(--cyber-cyan);">{pop_out}</span>" )  — of world reachable"
                                    </div>
                                </div>
                                <div class="stat-card">
                                    <div class="stat-label">"HOSPITALITY"<span class="rank-badge" style:color=r_hosp.1 style:opacity=r_hosp.2>{r_hosp.0}</span></div>
                                    <div class="stat-value tabular-nums" style:color=openness_color>{openness_str}</div>
                                    <div style="height: 5px; background: #111; border-radius: 3px; overflow: hidden; margin-top: 8px;">
                                        <div class="openness-bar" style:width=openness_bar style:background=openness_color></div>
                                    </div>
                                    <div style="font-size: 11px; color: #444; margin-top: 8px;">
                                        "√( eco in "<span style="color: var(--cyber-magenta);">{eco_in}</span>" × pop in "<span style="color: var(--cyber-pink);">{pop_in}</span>" )  — of world admitted"
                                    </div>
                                </div>
                            </div>


                            // Visa sections side by side
                            <div class="visa-grid" style="margin-top: 40px;">
                                <FilterableVisaSection
                                    title="OUTGOING — WHERE CITIZENS CAN TRAVEL"
                                    title_color="var(--cyber-green)"
                                    entries=outgoing
                                />
                                <FilterableVisaSection
                                    title="INCOMING — WHO CAN VISIT"
                                    title_color="var(--cyber-magenta)"
                                    entries=incoming
                                />
                            </div>
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}

/// Standard site shell header for drill-down pages: logo + suffix, search
/// that jumps to the states table on Enter, and the constant nav.
#[component]
pub fn SiteHeader(suffix: String) -> impl IntoView {
    view! {
        <div class="site-chrome">
        <div class="header-row1">
            <div class="logo-zone">
                <BrandChooser />
                <div class="logo-suffix">{suffix}</div>
            </div>
            <SiteNav active="" />
        </div>
        </div>
        <div class="search-dock">
            <input
                type="text"
                class="search-input"
                placeholder="Search country, code, or currency..."
                on:keydown=move |ev| {
                    if ev.key() == "Enter" {
                        let target = ev.target().unwrap();
                        let input: web_sys::HtmlInputElement = target.unchecked_into();
                        if let Some(w) = web_sys::window() {
                            let _ = w.location().set_href(&format!("/?q={}", input.value()));
                        }
                    }
                }
            />
        </div>
    }
}

#[component]
fn FilterableVisaSection(
    title: &'static str,
    title_color: &'static str,
    entries: Vec<VisaAccess>,
) -> impl IntoView {
    let (filter, set_filter) = signal(String::new()); // "" = all

    // Merged categories: eta + e-visa = "online"
    let categories: &[(&str, &[&str])] = &[
        ("visa-free", &["visa-free"]),
        ("visa-on-arrival", &["visa-on-arrival"]),
        ("online", &["eta", "e-visa"]),
        ("visa-required", &["visa-required"]),
        ("no-admission", &["no-admission"]),
    ];
    let counts: Vec<(String, usize)> = categories.iter().filter_map(|(key, types)| {
        let n = entries.iter().filter(|e| types.contains(&e.access_type.as_str())).count();
        if n > 0 { Some((key.to_string(), n)) } else { None }
    }).collect();

    let all_countries = load_countries();

    // Pre-compute hrefs
    let entries_with_href: Vec<(VisaAccess, Option<String>)> = entries.into_iter().map(|e| {
        let href = all_countries.iter()
            .find(|c| c.name == e.country)
            .map(|c| format!("/state/{}", c.code.to_lowercase()));
        (e, href)
    }).collect();

    view! {
        <div>
            <div style:color=title_color style="font-size: 11px; letter-spacing: 3px; margin-bottom: 16px;">
                {title}
            </div>

            // Filter badges
            <div style="display: flex; gap: 6px; flex-wrap: wrap; margin-bottom: 12px;">
                {counts.into_iter().map(|(t, n)| {
                    let t_for_click = t.clone();
                    let t_c1 = t.clone();
                    let t_c2 = t.clone();
                    let t_c3 = t.clone();
                    let color = access_type_color(if t == "online" { "e-visa" } else { &t });
                    let label = access_type_label(if t == "online" { "e-visa" } else { &t });
                    let text = format!("{} {}", n, label);
                    view! {
                        <button
                            style:color=move || if filter.get().is_empty() || filter.get() == t_c1 { color } else { "#333" }
                            style:border-color=move || if filter.get() == t_c2 { color } else { "currentColor" }
                            style:background=move || if filter.get() == t_c3 { "rgba(255,255,255,0.05)" } else { "transparent" }
                            style="font-size: 11px; padding: 3px 8px; border: 1px solid; border-radius: 2px; letter-spacing: 1px; cursor: pointer; font-family: 'Play', sans-serif;"
                            on:click=move |_| {
                                if filter.get() == t_for_click {
                                    set_filter.set(String::new()); // toggle off
                                } else {
                                    set_filter.set(t_for_click.clone());
                                }
                            }
                        >
                            {text}
                        </button>
                    }
                }).collect::<Vec<_>>()}
            </div>

            // Table
            <div style="border: 1px solid #111; border-radius: 4px; max-height: 600px; overflow-y: auto;">
                <table class="cyber-table" style="font-size: 12px;">
                    <thead>
                        <tr>
                            <th style="padding: 8px 12px;">"STATE"</th>
                            <th style="padding: 8px 12px;">"ACCESS"</th>
                            <th style="padding: 8px 12px; text-align: right;">"DAYS"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            let f = filter.get();
                            entries_with_href.iter()
                                .filter(|(e, _)| {
                                    if f.is_empty() { return true; }
                                    if f == "online" {
                                        e.access_type == "eta" || e.access_type == "e-visa"
                                    } else {
                                        e.access_type == f
                                    }
                                })
                                .map(|(e, href)| {
                                    let color = access_type_color(&e.access_type);
                                    let label = access_type_label(&e.access_type).to_string();
                                    let country = e.country.clone();
                                    let days_str = e.days.map(|d| format!("{}", d)).unwrap_or_else(|| "—".to_string());
                                    let href = href.clone();

                                    view! {
                                        <tr style="cursor: default;">
                                            <td style="padding: 6px 12px;">
                                                {match href {
                                                    Some(h) => view! {
                                                        <a href=h style="color: #ccc; text-decoration: none;">{country}</a>
                                                    }.into_any(),
                                                    None => view! {
                                                        <span style="color: #888;">{country}</span>
                                                    }.into_any(),
                                                }}
                                            </td>
                                            <td style="padding: 6px 12px;">
                                                <span style:color=color style="font-size: 10px; letter-spacing: 1px;">{label}</span>
                                            </td>
                                            <td class="tabular-nums" style="padding: 6px 12px; text-align: right; color: #666;">
                                                {days_str}
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
    }
}
