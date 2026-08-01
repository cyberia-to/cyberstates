use leptos::prelude::*;
use leptos_router::hooks::{use_location, use_navigate};
use wasm_bindgen::JsCast;
use crate::data::*;
use crate::components::table::*;

fn region_slug(r: &str) -> String {
    r.to_lowercase().replace(' ', "-")
}

fn region_from_slug(s: &str) -> Option<String> {
    REGIONS.iter().find(|r| region_slug(r) == s).map(|r| r.to_string())
}

/// Canonical landing path for a view state. Root = All regions, cap descending.
fn landing_path(region: &str, field: SortField, asc: bool) -> String {
    let mut p = String::new();
    if region != "All" {
        p.push_str(&format!("/in/{}", region_slug(region)));
    }
    if field != SortField::Cap || asc {
        p.push_str(&format!("/by/{}", field.slug()));
        if asc {
            p.push_str("/asc");
        }
    }
    if p.is_empty() {
        p.push('/');
    }
    p
}

/// Parse a landing path back into (region, field, ascending).
fn parse_path(path: &str) -> (String, SortField, bool) {
    let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let mut region = "All".to_string();
    let mut field = SortField::Cap;
    let mut asc = false;
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
                if segs.get(i) == Some(&"asc") {
                    asc = true;
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    (region, field, asc)
}

#[component]
pub fn HomePage() -> impl IntoView {
    let countries = load_countries();
    let total = countries.len();

    // The URL is the single source of truth for region + sort landings.
    let location = use_location();
    let state = Memo::new(move |_| parse_path(&location.pathname.get()));
    let sort_field = Signal::derive(move || state.get().1);
    let ascending = Signal::derive(move || state.get().2);

    let (search, set_search) = signal(String::new());

    let nav = use_navigate();
    let nav_sort = nav.clone();
    let on_sort = SignalSetter::map(move |field: SortField| {
        let (region, cur, asc) = state.get();
        let next_asc = if cur == field { !asc } else { false };
        nav_sort(&landing_path(&region, field, next_asc), Default::default());
    });

    // Landing title: "Cyberstates in Europe by freedom"
    Effect::new(move |_| {
        let (region, field, asc) = state.get();
        let mut t = String::from("Cyberstates");
        if region != "All" {
            t.push_str(&format!(" in {}", region));
        }
        t.push_str(&format!(" by {}", field.label().to_lowercase()));
        if asc {
            t.push_str(" ascending");
        }
        t.push_str(" — Global Visa Openness Analytics");
        document().set_title(&t);
    });

    let countries_for_count = countries.clone();
    let filtered_sorted = move || {
        let mut list = countries.clone();
        let (region, field, asc) = state.get();
        let query = search.get().to_lowercase();

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

        list.sort_by(|a, b| {
            let ord = match field {
                SortField::Name => a.name.cmp(&b.name),
                SortField::Population => a.population.cmp(&b.population),
                SortField::LandArea => a.land_area_km2.cmp(&b.land_area_km2),
                SortField::Token => a.currency_code.cmp(&b.currency_code),
                SortField::Cap => a.money_supply_b_usd.partial_cmp(&b.money_supply_b_usd).unwrap_or(std::cmp::Ordering::Equal),
                SortField::Freedom => a.index().freedom.partial_cmp(&b.index().freedom).unwrap_or(std::cmp::Ordering::Equal),
                SortField::Openness => a.index().openness.partial_cmp(&b.index().openness).unwrap_or(std::cmp::Ordering::Equal),
            };
            if asc { ord } else { ord.reverse() }
        });

        list
    };

    view! {
        <div style="max-width: 1400px; margin: 0 auto; padding: 20px;">
            // Header row 1: logo — centered search — map flush right
            <div class="header-row1">
                <div class="logo-zone">
                    <h1 class="logo">
                        <span style="color: var(--cyber-green);">"CYBER"</span>
                        <span style="color: #fff;">"STATES"</span>
                    </h1>
                    <div class="logo-suffix">
                        {move || {
                            let (region, field, asc) = state.get();
                            let mut s = String::new();
                            if region != "All" {
                                s.push_str(&format!("in {} · ", region.to_lowercase()));
                            }
                            s.push_str(&format!("by {}", field.label().to_lowercase()));
                            if asc {
                                s.push_str(" ↑");
                            }
                            s
                        }}
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
                <div class="map-zone">
                    <a href="/tokens" class="tokens-btn">"TOKENS"</a>
                    <a href="/map" class="map-btn">"MAP"</a>
                </div>
            </div>

            // Header row 2: region filters | count
            <div class="header-row2">
                <div class="region-pills">
                    {REGIONS.iter().map(|&r| {
                        let r_owned = r.to_string();
                        let r_for_class = r.to_string();
                        let r_for_click = r.to_string();
                        let nav_pill = nav.clone();
                        view! {
                            <button
                                class=move || {
                                    if state.get().0 == r_for_class {
                                        "region-pill active"
                                    } else {
                                        "region-pill"
                                    }
                                }
                                on:click=move |_| {
                                    let (_, field, asc) = state.get();
                                    nav_pill(&landing_path(&r_for_click, field, asc), Default::default());
                                }
                            >
                                {r_owned}
                            </button>
                        }
                    }).collect::<Vec<_>>()}
                </div>
                <p class="state-count">
                    {move || {
                        let region = state.get().0;
                        let query = search.get().to_lowercase();
                        let count = countries_for_count.iter().filter(|c| {
                            (region == "All" || c.region == region)
                            && (query.is_empty() || c.name.to_lowercase().contains(&query) || c.code.to_lowercase().contains(&query))
                        }).count();
                        format!("{} of {} states", count, total)
                    }}
                </p>
            </div>

            // Table
            <div style="overflow-x: auto; border: 1px solid #111; border-radius: 4px;">
                <table class="cyber-table">
                    // fixed layout: widths live here, not in row content — filtering must not move columns
                    <colgroup>
                        <col style="width: 5%;" />   // #
                        <col style="width: 18%;" />  // country
                        <col style="width: 11%;" />  // population
                        <col style="width: 12%;" />  // land area
                        <col style="width: 7%;" />   // token
                        <col style="width: 10%;" />  // price
                        <col style="width: 9%;" />   // supply
                        <col style="width: 9%;" />   // cap
                        <col style="width: 9.5%;" /> // freedom
                        <col style="width: 9.5%;" /> // openness
                    </colgroup>
                    <thead>
                        <tr>
                            <th style="width: 40px; cursor: default;">"#"</th>
                            <SortableHeader field=SortField::Name current=sort_field ascending=ascending on_click=on_sort />
                            <SortableHeader field=SortField::Population current=sort_field ascending=ascending on_click=on_sort />
                            <SortableHeader field=SortField::LandArea current=sort_field ascending=ascending on_click=on_sort />
                            <SortableHeader field=SortField::Token current=sort_field ascending=ascending on_click=on_sort />
                            <th class="th-static">"PRICE"</th>
                            <th class="th-static">"SUPPLY"</th>
                            <SortableHeader field=SortField::Cap current=sort_field ascending=ascending on_click=on_sort />
                            <SortableHeader field=SortField::Freedom current=sort_field ascending=ascending on_click=on_sort />
                            <SortableHeader field=SortField::Openness current=sort_field ascending=ascending on_click=on_sort />
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            filtered_sorted()
                                .into_iter()
                                .enumerate()
                                .map(|(i, country)| {
                                    view! { <CountryRow country=country rank={i + 1} /> }
                                })
                                .collect::<Vec<_>>()
                        }}
                    </tbody>
                </table>
            </div>

            // Footer
            <div style="margin-top: 24px; padding: 16px; background: #050505; border: 1px solid #111; border-radius: 4px; font-size: 10px; color: #333; line-height: 2;">
                <div><span class="text-cyber-green">"FREEDOM"</span>" — √(eco_out × pop_out) weighted moving freedom"</div>
                <div><span class="text-cyber-magenta">"OPENNESS"</span>" — √(eco_in × pop_in) weighted border openness"</div>
                <div style="color: #444; margin-top: 4px;">"Weights: visa-free=1.0, VoA=0.8, eTA/eVisa=0.5, visa-required=0.1, no-admission=0.0"" · "<a href="/methodology" style="color: var(--cyber-green); text-decoration: none;">"METHODOLOGY →"</a></div>
            </div>
        </div>
    }
}
