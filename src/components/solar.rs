use leptos::prelude::*;
use crate::data::{load_countries, Country};

/// The solar bodies have no polygon on the world map, so they'd be
/// invisible there. This panel gives them a home in the map's empty
/// space: Sun, Earth, Moon and Mars featured large, the rest in two
/// compact rows — every body a link to its state page.
#[component]
pub fn SolarPanel() -> impl IntoView {
    const FEATURED: [&str; 4] = ["SUN", "ERTH", "LUNA", "MARS"];

    let mut bodies: Vec<Country> = load_countries()
        .into_iter()
        .filter(|c| c.region == "Solar System")
        .collect();

    let featured: Vec<Country> = FEATURED
        .iter()
        .filter_map(|code| bodies.iter().find(|c| c.code == *code).cloned())
        .collect();

    // the rest, largest first
    bodies.retain(|c| !FEATURED.contains(&c.code.as_str()));
    bodies.sort_by(|a, b| b.land_area_km2.cmp(&a.land_area_km2));

    view! {
        <div class="solar-panel">
            <div class="solar-featured">
                {featured.into_iter().map(|c| {
                    let href = format!("/state/{}", c.code.to_lowercase());
                    view! {
                        <a href=href class="solar-body solar-big" title=c.name.clone()>
                            <span class="solar-icon">{c.flag.clone()}</span>
                            <span class="solar-name">{c.name.clone()}</span>
                        </a>
                    }
                }).collect_view()}
            </div>
            <div class="solar-rest">
                {bodies.into_iter().map(|c| {
                    let href = format!("/state/{}", c.code.to_lowercase());
                    view! {
                        <a href=href class="solar-body" title=c.name.clone()>
                            <span class="solar-icon">{c.flag.clone()}</span>
                            <span class="solar-name">{c.name.clone()}</span>
                        </a>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}
