use leptos::prelude::*;
use std::collections::HashMap;
use crate::data::*;
use crate::pages::country::SiteHeader;
use crate::pages::map::value_to_color;

/// Orbital elements, not frozen angles: (code, semi-major axis in AU,
/// mean longitude at J2000 in degrees, mean motion in degrees/day).
/// The map computes today's heliocentric longitude at render time —
/// open it next year and the planets will have moved. Planets carry
/// real J2000 elements; the estimated dwarfs are anchored to their
/// 2026-08-04 longitudes and propagate on their true periods.
const ELEMENTS: &[(&str, f64, f64, f64)] = &[
    ("MERC", 0.39, 252.2503, 4.09233445),
    ("VENU", 0.72, 181.9791, 1.60213034),
    ("ERTH", 1.0, 100.4664, 0.98560912),
    ("MARS", 1.52, 355.4533, 0.52402068),
    ("JUPI", 5.2, 34.3515, 0.08308529),
    ("SATN", 9.54, 50.0774, 0.03344414),
    ("URAN", 19.2, 314.0550, 0.01172834),
    ("NEPT", 30.1, 304.3487, 0.00598103),
    ("VEST", 2.36, 350.8965, 0.27154441),
    ("JUNO", 2.67, 176.6876, 0.22584693),
    ("CERE", 2.77, 152.8589, 0.21388468),
    ("PALL", 2.77, 211.3701, 0.21352313),
    ("HYGI", 3.14, 26.7635, 0.17733990),
    ("PLUT", 39.5, 264.4036, 0.00397430),
    ("ORCU", 39.4, 145.9628, 0.00401968),
    ("IXIO", 39.6, 211.7124, 0.00394251),
    ("SALA", 42.0, 333.0635, 0.00359744),
    ("HAUM", 43.0, 178.2723, 0.00347296),
    ("QUAO", 43.7, 225.8563, 0.00341283),
    ("MAKE", 45.8, 160.7397, 0.00321890),
    ("VARD", 45.9, 194.4188, 0.00314897),
    ("GONG", 67.3, 314.7347, 0.00177783),
    ("ERIS", 67.7, 8.8767, 0.00176320),
    ("SEDN", 506.0, 56.1604, 0.00008646),
];

/// Who orbits whom: moons take their host's computed position and ring
/// around it in orbital order (their true offsets are sub-pixel here).
const MOONS: &[(&str, &str)] = &[
    ("LUNA", "ERTH"),
    ("PHOB", "MARS"), ("DEIM", "MARS"),
    ("IO", "JUPI"), ("EUPA", "JUPI"), ("GANY", "JUPI"), ("CALL", "JUPI"),
    ("MIMA", "SATN"), ("ENCE", "SATN"), ("TETH", "SATN"), ("DION", "SATN"),
    ("RHEA", "SATN"), ("TITN", "SATN"), ("IAPE", "SATN"),
    ("MIRA", "URAN"), ("ARIE", "URAN"), ("UMBR", "URAN"), ("TNIA", "URAN"), ("OBER", "URAN"),
    ("TRIT", "NEPT"),
    ("CHAR", "PLUT"),
];

/// Days since J2000 (2000-01-01T12:00 UTC), from the wall clock.
fn days_since_j2000() -> f64 {
    js_sys::Date::now() / 86_400_000.0 - 10_957.5
}

/// Today's mean heliocentric longitude in degrees.
fn longitude(l0: f64, n: f64, d: f64) -> f64 {
    (l0 + n * d).rem_euclid(360.0)
}

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

    // positions computed for TODAY: hosts from their elements, moons
    // ringed around the host in orbital order
    let d = days_since_j2000();
    let mut host_pos: HashMap<&str, (f64, f64)> = HashMap::new();
    host_pos.insert("SUN", (CX, CY));
    for &(code, a, l0, n) in ELEMENTS {
        host_pos.insert(code, polar(a, longitude(l0, n, d)));
    }

    let mut cluster_seen: HashMap<&str, usize> = HashMap::new();
    let mut placed: Vec<(Country, f64, f64, f64)> = Vec::new(); // body, x, y, r
    let host_of = |code: &str| MOONS.iter().find(|(m, _)| *m == code).map(|(_, h)| *h);
    for &(code, _, _, _) in std::iter::once(&("SUN", 0.0, 0.0, 0.0)).chain(ELEMENTS.iter()) {
        let Some(c) = by_code.get(code) else { continue };
        let (x, y) = host_pos[code];
        placed.push((c.clone(), x, y, dot_r(c.land_area_km2)));
    }
    for &(moon, host) in MOONS {
        let Some(c) = by_code.get(moon) else { continue };
        let Some(&(hx, hy)) = host_pos.get(host) else { continue };
        let idx = cluster_seen.entry(host).or_insert(0);
        let th = (90.0 - 62.0 * (*idx as f64)).to_radians();
        let orbit = 17.0 + 3.0 * (*idx as f64);
        *idx += 1;
        placed.push((c.clone(), hx + orbit * th.cos(), hy - orbit * th.sin(), dot_r(c.land_area_km2)));
    }
    let _ = host_of;

    let bodies_for_colors = bodies.clone();
    let colors = Memo::new(move |_| colors_for(&bodies_for_colors, field.get()));

    // orbit rings for every distinct heliocentric distance
    let mut ring_as: Vec<f64> = ELEMENTS.iter().map(|&(_, a, _, _)| a).collect();
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

                // the listed bodies
                {placed.into_iter().map(|(c, x, y, r)| {
                    let code = c.code.clone();
                    let href = format!("/state/{}", code.to_lowercase());
                    let title = format!("{} — {}", c.name, c.land_area_fmt());
                    let label = matches!(c.code.as_str(),
                        "SUN" | "MERC" | "VENU" | "ERTH" | "MARS" | "CERE" | "JUPI" | "SATN" | "URAN" | "NEPT" | "PLUT" | "ERIS" | "SEDN")
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
