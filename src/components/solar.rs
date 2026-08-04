use leptos::prelude::*;
use crate::data::{load_countries, Country};

/// The solar bodies have no polygon on the world map, so they'd be
/// invisible there. This panel gives them a home in the map's empty
/// space: Sun, Earth, Moon and Mars featured large, the rest in two
/// even rows — icon-only, the name lives in the hover title, so the
/// strip never steals height from the map. Every body a link.
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

    // the rest, largest first, split into two even rows — exactly two,
    // whatever the width, each spread evenly across the strip
    bodies.retain(|c| !FEATURED.contains(&c.code.as_str()));
    bodies.sort_by(|a, b| b.land_area_km2.cmp(&a.land_area_km2));
    let half = bodies.len().div_ceil(2);
    let row2 = bodies.split_off(half);

    let icon_row = |bodies: Vec<Country>, big: bool| {
        let cls = if big { "solar-body solar-big" } else { "solar-body" };
        view! {
            <div class="solar-rest">
                {bodies.into_iter().map(|c| {
                    let href = format!("/state/{}", c.code.to_lowercase());
                    view! {
                        <a href=href class=cls title=c.name.clone()>
                            <span class="solar-icon">{c.flag.clone()}</span>
                        </a>
                    }
                }).collect_view()}
            </div>
        }
    };

    view! {
        <div class="solar-panel">
            {icon_row(featured, true)}
            {icon_row(bodies, false)}
            {icon_row(row2, false)}
        </div>
    }
}
