use leptos::prelude::*;
use crate::data::{load_countries, Country};

/// Distance from Earth in AU (semi-major axes; moons keep their orbital
/// order as tie-breakers). Sorting by this puts the inner system in the
/// first row and the outer system in the second — and the Sun, honestly,
/// at 1 AU between Mercury and the belt.
fn earth_distance(code: &str) -> f64 {
    match code {
        "ERTH" => 0.0,
        "LUNA" => 0.003,
        "VENU" => 0.28,
        "MARS" => 0.52,
        "PHOB" => 0.521,
        "DEIM" => 0.522,
        "MERC" => 0.61,
        "SUN" => 1.0,
        "VEST" => 1.36,
        "JUNO" => 1.67,
        "CERE" => 1.77,
        "PALL" => 1.78,
        "HYGI" => 2.14,
        "IO" => 4.20,
        "EUPA" => 4.21,
        "GANY" => 4.22,
        "CALL" => 4.23,
        "MIMA" => 8.54,
        "ENCE" => 8.55,
        "TETH" => 8.56,
        "DION" => 8.57,
        "RHEA" => 8.58,
        "TITN" => 8.59,
        "IAPE" => 8.60,
        "MIRA" => 18.20,
        "ARIE" => 18.21,
        "UMBR" => 18.22,
        "TNIA" => 18.23,
        "OBER" => 18.24,
        "TRIT" => 29.1,
        "ORCU" => 38.3,
        "PLUT" => 38.5,
        "CHAR" => 38.51,
        "IXIO" => 38.6,
        "SALA" => 41.0,
        "HAUM" => 42.2,
        "QUAO" => 42.5,
        "MAKE" => 44.6,
        "VARD" => 44.8,
        "GONG" => 66.1,
        "ERIS" => 66.9,
        "SEDN" => 505.0,
        _ => 9999.0,
    }
}

/// Symbolic size: dot diameter from the body's radius on a log scale —
/// the Sun reads huge, Deimos reads as a grain, both stay visible.
fn dot_diameter(area_km2: u64) -> f64 {
    let r_km = ((area_km2 as f64) / (4.0 * std::f64::consts::PI)).sqrt().max(1.0);
    (6.0 + 7.0 * (r_km / 50.0).log10()).clamp(4.0, 40.0)
}

/// The solar system as an instrument, not an icon strip: every body is a
/// circle sized by its real radius, painted by the SAME rank colors as
/// the world map (the home paint effect fills them by data-code), sorted
/// by distance from Earth — row one is the inner system, row two the
/// outer. Every dot links to its state page.
#[component]
pub fn SolarPanel() -> impl IntoView {
    let mut bodies: Vec<Country> = load_countries()
        .into_iter()
        .filter(|c| c.region == "Solar System")
        .collect();
    bodies.sort_by(|a, b| {
        earth_distance(&a.code)
            .partial_cmp(&earth_distance(&b.code))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let half = bodies.len().div_ceil(2);
    let outer = bodies.split_off(half);

    let row = |bodies: Vec<Country>| {
        view! {
            <div class="solar-row">
                {bodies.into_iter().map(|c| {
                    let href = format!("/state/{}", c.code.to_lowercase());
                    let d = dot_diameter(c.land_area_km2);
                    let title = format!("{} — {}", c.name, c.land_area_fmt());
                    view! {
                        <a href=href class="solar-body" title=title>
                            <svg
                                class="solar-dot"
                                width=format!("{:.0}", d)
                                height=format!("{:.0}", d)
                                viewBox="0 0 10 10"
                            >
                                <circle cx="5" cy="5" r="5" fill="#1a1a1a" data-code=c.code.clone()></circle>
                            </svg>
                        </a>
                    }
                }).collect_view()}
            </div>
        }
    };

    view! {
        <div class="solar-panel">
            {row(bodies)}
            {row(outer)}
        </div>
    }
}
