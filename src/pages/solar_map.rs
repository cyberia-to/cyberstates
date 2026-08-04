use leptos::prelude::*;
use std::collections::HashMap;
use crate::data::*;
use crate::pages::country::SiteHeader;
use crate::pages::map::value_to_color;

/// Heliocentric layout: (code, semi-major axis in AU, angle in degrees,
/// host code if the body is a moon). BOTH axes are real now: the radius
/// is the log-scaled semi-major axis, the angle is the heliocentric
/// ecliptic longitude as of 2026-08-04 (planets from J2000 mean
/// elements; distant dwarfs approximate from their current
/// constellations — they move degrees per decade). Moons cluster
/// around their host in orbital order: at this scale their true
/// positions are sub-pixel.
const LAYOUT: &[(&str, f64, f64, Option<&str>)] = &[
    ("SUN", 0.0, 0.0, None),
    ("MERC", 0.39, 35.0, None),
    ("VENU", 0.72, 261.1, None),
    ("ERTH", 1.0, 312.2, None),
    ("LUNA", 1.0, 312.2, Some("ERTH")),
    ("MARS", 1.52, 44.5, None),
    ("PHOB", 1.52, 44.5, Some("MARS")),
    ("DEIM", 1.52, 44.5, Some("MARS")),
    ("VEST", 2.36, 108.0, None),
    ("JUNO", 2.67, 210.0, None),
    ("CERE", 2.77, 70.0, None),
    ("PALL", 2.77, 125.0, None),
    ("HYGI", 3.14, 309.0, None),
    ("IO", 5.2, 121.2, Some("JUP")),
    ("EUPA", 5.2, 121.2, Some("JUP")),
    ("GANY", 5.2, 121.2, Some("JUP")),
    ("CALL", 5.2, 121.2, Some("JUP")),
    ("MIMA", 9.54, 14.9, Some("SAT")),
    ("ENCE", 9.54, 14.9, Some("SAT")),
    ("TETH", 9.54, 14.9, Some("SAT")),
    ("DION", 9.54, 14.9, Some("SAT")),
    ("RHEA", 9.54, 14.9, Some("SAT")),
    ("TITN", 9.54, 14.9, Some("SAT")),
    ("IAPE", 9.54, 14.9, Some("SAT")),
    ("MIRA", 19.2, 68.0, Some("URA")),
    ("ARIE", 19.2, 68.0, Some("URA")),
    ("UMBR", 19.2, 68.0, Some("URA")),
    ("TNIA", 19.2, 68.0, Some("URA")),
    ("OBER", 19.2, 68.0, Some("URA")),
    ("TRIT", 30.1, 2.4, Some("NEP")),
    ("PLUT", 39.5, 303.0, None),
    ("CHAR", 39.5, 303.0, Some("PLUT")),
    ("ORCU", 39.4, 185.0, None),
    ("IXIO", 39.6, 250.0, None),
    ("SALA", 42.0, 8.0, None),
    ("HAUM", 43.0, 212.0, None),
    ("QUAO", 43.7, 259.0, None),
    ("MAKE", 45.8, 192.0, None),
    ("VARD", 45.9, 225.0, None),
    ("GONG", 67.3, 332.0, None),
    ("ERIS", 67.7, 26.0, None),
    ("SEDN", 506.0, 57.0, None),
];

/// The unlisted hosts: gas and ice giants hold no solid surface, so they
/// hold no listing — they render as ghost rings their moons orbit.
const GHOSTS: &[(&str, f64, f64)] = &[
    ("Jupiter", 5.2, 121.2),
    ("Saturn", 9.54, 14.9),
    ("Uranus", 19.2, 68.0),
    ("Neptune", 30.1, 2.4),
];

const CX: f64 = 500.0;
const CY: f64 = 470.0;

/// Log-radial distance: 0.39 AU..506 AU maps to 90..420 units.
fn orbit_r(a: f64) -> f64 {
    if a <= 0.0 {
        return 0.0;
    }
    90.0 + 330.0 * ((a / 0.3).ln() / (506.0_f64 / 0.3).ln())
}

fn polar(a: f64, deg: f64) -> (f64, f64) {
    let r = orbit_r(a);
    let th = deg.to_radians();
    (CX + r * th.cos(), CY - r * th.sin())
}

/// Dot radius in viewBox units from the body's surface (log of radius).
fn dot_r(area_km2: u64) -> f64 {
    let r_km = ((area_km2 as f64) / (4.0 * std::f64::consts::PI)).sqrt().max(1.0);
    (4.0 + 5.2 * (r_km / 50.0).log10()).clamp(2.6, 30.0)
}

/// Rank-percentile colors with ties sharing one color (42 zero-capital
/// bodies must not fake a gradient).
fn colors_for(bodies: &[Country], field: SortField) -> HashMap<String, String> {
    let mut vals: Vec<f64> = bodies.iter().map(|c| c.metric(field)).collect();
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = vals.len().max(2);
    let first_idx = |v: f64| vals.iter().position(|&x| x == v).unwrap_or(0);
    bodies
        .iter()
        .map(|c| {
            let v = c.metric(field);
            let mut t = first_idx(v) as f64 / (n - 1) as f64;
            if field.lower_is_better() {
                t = 1.0 - t;
            }
            (c.code.clone(), value_to_color(0.02 + 0.98 * t, 1.0))
        })
        .collect()
}

/// /solar — the experiment: the solar system as an orbital map. Real
/// log-scaled orbits, bodies sized by their surfaces, colored by the
/// active rating exactly like the world map. Ghost rings mark the gas
/// giants: no solid ground, no listing — their moons shine without them.
#[component]
pub fn SolarMapPage() -> impl IntoView {
    Effect::new(move |_| {
        document().set_title("Solar map — the sovereignty terminal");
    });

    let bodies: Vec<Country> = load_countries()
        .into_iter()
        .filter(|c| c.region == "Solar System")
        .collect();
    let field = RwSignal::new(SortField::Territory);

    let by_code: HashMap<String, Country> =
        bodies.iter().map(|c| (c.code.clone(), c.clone())).collect();

    // moons gather around their host: index within the cluster sets the angle
    let mut cluster_seen: HashMap<&str, usize> = HashMap::new();

    let mut placed: Vec<(Country, f64, f64, f64)> = Vec::new(); // body, x, y, r
    for &(code, a, deg, host) in LAYOUT {
        let Some(c) = by_code.get(code) else { continue };
        let r = dot_r(c.land_area_km2);
        let (x, y) = match host {
            None => {
                if code == "SUN" {
                    (CX, CY)
                } else {
                    polar(a, deg)
                }
            }
            Some(h) => {
                let idx = cluster_seen.entry(h).or_insert(0);
                let (hx, hy) = polar(a, deg);
                // moons ring their host counterclockwise from the top
                let th = (90.0 - 62.0 * (*idx as f64)).to_radians();
                let orbit = 17.0 + 3.0 * (*idx as f64);
                *idx += 1;
                (hx + orbit * th.cos(), hy - orbit * th.sin())
            }
        };
        placed.push((c.clone(), x, y, r));
    }

    let bodies_for_colors = bodies.clone();
    let colors = Memo::new(move |_| colors_for(&bodies_for_colors, field.get()));

    // orbit rings for every distinct planetary distance
    let mut ring_as: Vec<f64> = LAYOUT
        .iter()
        .filter(|(_, _, _, host)| host.is_none())
        .map(|&(_, a, _, _)| a)
        .filter(|&a| a > 0.0)
        .collect();
    ring_as.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ring_as.dedup_by(|a, b| (*a - *b).abs() < 0.2);

    view! {
        <div class="page-frame">
            <SiteHeader />

            <div class="solar-map-head">
                <span class="by-label">"by"</span>
                <div class="region-pills">
                    {SortField::ALL.map(|f| {
                        view! {
                            {f.derived_break().then(|| view! { <span class="pill-dot">"·"</span> })}
                            <button
                                class=move || if field.get() == f { "region-pill active" } else { "region-pill" }
                                on:click=move |_| field.set(f)
                            >
                                {f.label()}
                            </button>
                        }
                    })}
                </div>
            </div>

            <svg class="solar-map" viewBox="0 0 1000 940">
                // orbits: faint truth under the listings
                {ring_as.into_iter().map(|a| view! {
                    <circle
                        cx=CX cy=CY r=orbit_r(a)
                        fill="none" stroke="#131313" stroke-width="1"
                    ></circle>
                }).collect_view()}

                // ghost giants: no solid surface, no listing
                {GHOSTS.iter().map(|&(name, a, deg)| {
                    let (x, y) = polar(a, deg);
                    view! {
                        <circle
                            cx=x cy=y r="9"
                            fill="#0a0a0a" stroke="#3a3a3a" stroke-width="1.2"
                            stroke-dasharray="3 3"
                        >
                            <title>{format!("{} — gas, no solid surface: not listed", name)}</title>
                        </circle>
                    }
                }).collect_view()}

                // the listed bodies
                {placed.into_iter().map(|(c, x, y, r)| {
                    let code = c.code.clone();
                    let href = format!("/state/{}", code.to_lowercase());
                    let title = format!("{} — {}", c.name, c.land_area_fmt());
                    let label = matches!(c.code.as_str(),
                        "SUN" | "MERC" | "VENU" | "ERTH" | "MARS" | "CERE" | "TITN" | "TRIT" | "PLUT" | "ERIS" | "SEDN" | "GANY")
                        .then(|| c.name.clone());
                    view! {
                        <g
                            class="solar-map-a"
                            on:click=move |_| crate::pages::map::navigate_client(&href)
                        >
                            <circle
                                cx=x cy=y r=r
                                fill=move || colors.get().get(&code).cloned().unwrap_or_else(|| "#1a1a1a".into())
                                class="solar-map-dot"
                            >
                                <title>{title}</title>
                            </circle>
                            {label.map(|name| view! {
                                <text
                                    x=x y=y + r + 13.0
                                    text-anchor="middle"
                                    class="solar-map-label"
                                >{name}</text>
                            })}
                        </g>
                    }
                }).collect_view()}
            </svg>
        </div>
    }
}
