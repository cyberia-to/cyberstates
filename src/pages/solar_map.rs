use crate::components::table::metric_cell;
use crate::data::*;
use crate::numeraires::{price_parts, Numeraire};
use crate::pages::country::SiteHeader;
use crate::pages::map::{painted_world, value_to_color};
use leptos::prelude::*;
use std::collections::HashMap;

/// Orbital elements, not frozen angles: (code, semi-major axis in AU,
/// mean longitude at J2000 in degrees, mean motion in degrees/day).
/// The map computes today's heliocentric longitude at render time —
/// open it next year and the planets will have moved. Planets carry
/// real J2000 elements; the estimated dwarfs are anchored to their
/// 2026-08-04 longitudes and propagate on their true periods.
pub const ELEMENTS: &[(&str, f64, f64, f64)] = &[
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
pub const MOONS: &[(&str, &str)] = &[
    ("LUNA", "ERTH"),
    ("PHOB", "MARS"),
    ("DEIM", "MARS"),
    ("IO", "JUPI"),
    ("EUPA", "JUPI"),
    ("GANY", "JUPI"),
    ("CALL", "JUPI"),
    ("MIMA", "SATN"),
    ("ENCE", "SATN"),
    ("TETH", "SATN"),
    ("DION", "SATN"),
    ("RHEA", "SATN"),
    ("TITN", "SATN"),
    ("IAPE", "SATN"),
    ("MIRA", "URAN"),
    ("ARIE", "URAN"),
    ("UMBR", "URAN"),
    ("TNIA", "URAN"),
    ("OBER", "URAN"),
    ("TRIT", "NEPT"),
    ("CHAR", "PLUT"),
];

/// Days since J2000 (2000-01-01T12:00 UTC), from the wall clock.
pub fn days_since_j2000() -> f64 {
    js_sys::Date::now() / 86_400_000.0 - 10_957.5
}

/// Today's mean heliocentric longitude in degrees.
pub fn longitude(l0: f64, n: f64, d: f64) -> f64 {
    (l0 + n * d).rem_euclid(360.0)
}

pub const CX: f64 = 500.0;
pub const CY: f64 = 470.0;

/// Log-radial distance: 0.39 AU..506 AU maps to 90..420 units.
pub fn orbit_r(a: f64) -> f64 {
    if a <= 0.0 {
        return 0.0;
    }
    90.0 + 330.0 * ((a / 0.3).ln() / (506.0_f64 / 0.3).ln())
}

pub fn polar(a: f64, deg: f64) -> (f64, f64) {
    let r = orbit_r(a);
    let th = deg.to_radians();
    (CX + r * th.cos(), CY - r * th.sin())
}

/// Dot radius in viewBox units from the body's surface (log of radius).
pub fn dot_r(area_km2: u64) -> f64 {
    let r_km = ((area_km2 as f64) / (4.0 * std::f64::consts::PI))
        .sqrt()
        .max(1.0);
    (4.0 + 5.2 * (r_km / 50.0).log10()).clamp(2.6, 30.0)
}

/// Rank-percentile colors with ties sharing one color (42 zero-capital
/// bodies must not fake a gradient). Two spectra: planets among planets,
/// everything else among itself — same idea as the home map.
fn colors_for(bodies: &[Country], field: SortField) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for is_planet in [true, false] {
        let band: Vec<&Country> = bodies
            .iter()
            .filter(|c| matches!(c.class(), ListingClass::Planet) == is_planet)
            .collect();
        if band.is_empty() {
            continue;
        }
        let mut vals: Vec<f64> = band.iter().map(|c| c.metric(field)).collect();
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = vals.len().max(2);
        let first_idx = |v: f64| vals.iter().position(|&x| x == v).unwrap_or(0);
        for c in band {
            let v = c.metric(field);
            let mut t = first_idx(v) as f64 / (n - 1) as f64;
            if field.lower_is_better() {
                t = 1.0 - t;
            }
            out.insert(c.code.clone(), value_to_color(0.02 + 0.98 * t, 1.0));
        }
    }
    out
}

/// Rank-percentile values for the world map pane (terrestrial states,
/// no filters — the experiment keeps it simple).
fn world_values(field: SortField) -> HashMap<String, f64> {
    let mut ranked: Vec<(String, f64)> = load_countries()
        .iter()
        .filter(|c| is_terrestrial(&c.region) && !is_aggregate(&c.code))
        .map(|c| (c.code.clone(), c.metric(field)))
        .collect();
    ranked.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let n = ranked.len();
    let log_scale = matches!(field, SortField::Population | SortField::Territory);
    let (vmin, vmax) = if log_scale && n > 0 {
        let p10 = ranked[(n as f64 * 0.10) as usize].1.max(1.0).ln();
        let top = ranked.last().map(|x| x.1.max(1.0).ln()).unwrap_or(1.0);
        (p10, top)
    } else {
        (0.0, 1.0)
    };
    let mut values = HashMap::new();
    for (i, (code, v)) in ranked.into_iter().enumerate() {
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
        let t = if field.lower_is_better() { 1.0 - t } else { t };
        values.insert(code, 0.02 + 0.98 * t);
    }
    values
}

/// Positions for TODAY, shared by the /solar stage and the home panel:
/// hosts from their orbital elements, moons ringed in orbital order.
pub fn placed_bodies(by_code: &HashMap<String, Country>) -> Vec<(Country, f64, f64, f64)> {
    let d = days_since_j2000();
    let mut host_pos: HashMap<&str, (f64, f64)> = HashMap::new();
    host_pos.insert("SUN", (CX, CY));
    for &(code, a, l0, n) in ELEMENTS {
        host_pos.insert(code, polar(a, longitude(l0, n, d)));
    }
    let mut cluster_seen: HashMap<&str, usize> = HashMap::new();
    let mut placed: Vec<(Country, f64, f64, f64)> = Vec::new();
    for &(code, _, _, _) in std::iter::once(&("SUN", 0.0, 0.0, 0.0)).chain(ELEMENTS.iter()) {
        let Some(c) = by_code.get(code) else { continue };
        let (x, y) = host_pos[code];
        placed.push((c.clone(), x, y, dot_r(c.land_area_km2)));
    }
    for &(moon, host) in MOONS {
        let Some(c) = by_code.get(moon) else { continue };
        let Some(&(hx, hy)) = host_pos.get(host) else {
            continue;
        };
        let idx = cluster_seen.entry(host).or_insert(0);
        let th = (90.0 - 62.0 * (*idx as f64)).to_radians();
        let orbit = 17.0 + 3.0 * (*idx as f64);
        *idx += 1;
        placed.push((
            c.clone(),
            hx + orbit * th.cos(),
            hy - orbit * th.sin(),
            dot_r(c.land_area_km2),
        ));
    }
    placed
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

    let placed = placed_bodies(&by_code);

    let bodies_for_colors = bodies.clone();
    let colors = Memo::new(move |_| colors_for(&bodies_for_colors, field.get()));

    // the cockpit selection: a body chosen on either the table or the
    // system map; Earth by default — the only surveyed world
    let selected = RwSignal::new("ERTH".to_string());
    let numeraire = use_context::<RwSignal<Numeraire>>().expect("numeraire context");

    // orbit rings for every distinct heliocentric distance
    let mut ring_as: Vec<f64> = ELEMENTS.iter().map(|&(_, a, _, _)| a).collect();
    ring_as.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ring_as.dedup_by(|a, b| (*a - *b).abs() < 0.2);

    // table: the system ranked under the active rating
    let bodies_for_table = bodies.clone();
    let table_rows = move || {
        let f = field.get();
        let mut v = bodies_for_table.clone();
        v.sort_by(|a, b| {
            b.metric(f)
                .partial_cmp(&a.metric(f))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        v
    };

    // starfield: plastic-constant scatter — even, deterministic, no RNG
    let stars: Vec<(f64, f64, f64, f64)> = (0..170)
        .map(|i| {
            let f = |k: f64| (i as f64 * k).fract();
            (
                f(0.7548776662) * 1000.0,
                f(0.5698402909) * 940.0,
                0.4 + f(0.318) * 1.0,
                0.05 + f(0.61) * 0.22,
            )
        })
        .collect();

    let by_code_for_pane = by_code.clone();
    let sel_body = move || by_code_for_pane.get(&selected.get()).cloned();

    view! {
        <div class="page-shell">
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

            <div class="solar-cockpit">
                <div class="cockpit-stage">
                // the stage: the system fills the free band left of the world
                <svg class="solar-map" viewBox="48 18 904 904">
                    <defs>
                        <filter id="dotglow" x="-100%" y="-100%" width="300%" height="300%">
                            <feGaussianBlur stdDeviation="2.4" result="b"></feGaussianBlur>
                            <feMerge>
                                <feMergeNode in="b"></feMergeNode>
                                <feMergeNode in="SourceGraphic"></feMergeNode>
                            </feMerge>
                        </filter>
                        <radialGradient id="sunglow">
                            <stop offset="0%" stop-color="rgba(255,240,180,0.16)"></stop>
                            <stop offset="55%" stop-color="rgba(255,220,140,0.05)"></stop>
                            <stop offset="100%" stop-color="rgba(255,220,140,0)"></stop>
                        </radialGradient>
                    </defs>

                    {stars.into_iter().map(|(x, y, r, o)| view! {
                        <circle cx=x cy=y r=r fill="#fff" opacity=o></circle>
                    }).collect_view()}

                    <circle cx=CX cy=CY r="150" fill="url(#sunglow)"></circle>

                    {ring_as.clone().into_iter().map(|a| view! {
                        <circle
                            cx=CX cy=CY r=orbit_r(a)
                            fill="none" stroke="#191919" stroke-width="1"
                        ></circle>
                    }).collect_view()}

                    // distance labels on the rings, southwest
                    {[(1.0, "1 AU"), (9.54, "10 AU"), (39.5, "40 AU"), (506.0, "500 AU")].map(|(a, t)| {
                        let r = orbit_r(a);
                        let th = (225.0_f64).to_radians();
                        view! {
                            <text
                                x=CX + r * th.cos() y=CY - r * th.sin()
                                class="solar-au-label" text-anchor="middle"
                            >{t}</text>
                        }
                    })}

                    {placed.into_iter().map(|(c, x, y, r)| {
                        let code = c.code.clone();
                        let code_sel = c.code.clone();
                        let code_ring = c.code.clone();
                        let title = format!("{} — {}", c.name, c.land_area_fmt());
                        let label = matches!(c.code.as_str(),
                            "SUN" | "MERC" | "VENU" | "ERTH" | "MARS" | "CERE" | "JUPI" | "SATN" | "URAN" | "NEPT" | "PLUT" | "ERIS" | "SEDN")
                            .then(|| c.name.clone());
                        view! {
                            <g
                                class="solar-map-a"
                                on:click=move |_| selected.set(code_sel.clone())
                            >
                                <circle
                                    cx=x cy=y r=r + 4.0
                                    fill="none"
                                    stroke=move || if selected.get() == code_ring { "var(--cyber-green)" } else { "transparent" }
                                    stroke-width="1.5"
                                ></circle>
                                <circle
                                    cx=x cy=y r=r
                                    fill=move || colors.get().get(&code).cloned().unwrap_or_else(|| "#1a1a1a".into())
                                    class="solar-map-dot"
                                    filter="url(#dotglow)"
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

                // pane 1: the table of the system
                <div class="cockpit-table">
                    <table class="cyber-table slim">
                        <colgroup>
                            <col style="width: 30px;" />
                            <col />
                            <col style="width: 48px;" />
                            <col style="width: 82px;" />
                            <col style="width: 82px;" />
                        </colgroup>
                        <thead>
                            <tr>
                                <th style="cursor: default; text-align: right;">"#"</th>
                                <th class="th-static">"BODY"</th>
                                <th class="th-static">"TOKEN"</th>
                                <th class="th-static" style="text-align: right;">"PRICE"</th>
                                <th class="th-static metric-th" style="text-align: right;">
                                    {move || field.get().short()}
                                </th>
                            </tr>
                        </thead>
                        <tbody>
                            {move || table_rows().into_iter().enumerate().map(|(i, c)| {
                                let code = c.code.clone();
                                let code_cls = c.code.clone();
                                let c_metric = c.clone();
                                view! {
                                    <tr
                                        class=move || if selected.get() == code_cls { "cockpit-row sel" } else { "cockpit-row" }
                                        on:click=move |_| selected.set(code.clone())
                                    >
                                        <td class="tabular-nums" style=format!("text-align: right; color: {}; font-weight: {};", rank_color(i + 1), rank_weight(i + 1))>{i + 1}</td>
                                        <td>
                                            <span style="margin-right: 7px;">{c.flag.clone()}</span>
                                            <span style="color: #ccc;">{c.name.clone()}</span>
                                        </td>
                                        <td style="color: var(--cyber-yellow); font-weight: 700;">{c.currency_code.clone()}</td>
                                        <td class="tabular-nums" style="text-align: right; color: var(--cyber-orange);">
                                            {
                                                let price_usd = c.token_price_usd;
                                                move || {
                                                    let (head, frac, unit) = price_parts(price_usd, numeraire.get());
                                                    if head == "N/A" {
                                                        view! { <crate::components::notyet::NotYet /> }.into_any()
                                                    } else {
                                                        view! {
                                                            <span>{head}</span>
                                                            <span class="price-frac">{frac}</span>
                                                            <span class="price-unit">{unit}</span>
                                                        }.into_any()
                                                    }
                                                }
                                            }
                                        </td>
                                        <td class="tabular-nums" style="text-align: right; font-weight: 700;">
                                            {move || {
                                                let (text, color) = metric_cell(&c_metric, field.get(), numeraire.get());
                                                let (num, unit) = match text.strip_suffix(" km\u{b2}") {
                                                    Some(n) => (n.to_string(), " km\u{b2}"),
                                                    None => (text, ""),
                                                };
                                                view! {
                                                    <span style:color=color>{num}</span>
                                                    {(!unit.is_empty()).then(|| view! { <span class="price-unit">{unit}</span> })}
                                                }
                                            }}
                                        </td>
                                    </tr>
                                }
                            }).collect_view()}
                        </tbody>
                    </table>
                </div>

                <div class="stage-legend">
                    <span class="legend-end">"LOW"</span>
                    <div class="legend-bar" style="width: 140px; cursor: default;"></div>
                    <span class="legend-end">"HIGH"</span>
                </div>
                </div>

                // the world hero: the selected body at full height
                <div class="cockpit-world">
                    {move || sel_body().map(|c| {
                        let href = c.path();
                        let is_earth = c.code == "ERTH";
                        let color = colors.get().get(&c.code).cloned().unwrap_or_else(|| "#1a1a1a".into());
                        view! {
                            <div class="world-canvas">
                            {if is_earth {
                                view! {
                                    <div class="planet-world" inner_html=painted_world(&world_values(field.get()))></div>
                                }.into_any()
                            } else {
                                view! {
                                    <div class="planet-portrait">
                                        <svg viewBox="0 0 400 400">
                                            <defs>
                                                <radialGradient id="limb" cx="38%" cy="34%" r="75%">
                                                    <stop offset="0%" stop-color="rgba(255,255,255,0.32)"></stop>
                                                    <stop offset="45%" stop-color="rgba(255,255,255,0.06)"></stop>
                                                    <stop offset="80%" stop-color="rgba(0,0,0,0.25)"></stop>
                                                    <stop offset="100%" stop-color="rgba(0,0,0,0.55)"></stop>
                                                </radialGradient>
                                                <filter id="planetglow" x="-40%" y="-40%" width="180%" height="180%">
                                                    <feGaussianBlur stdDeviation="10" result="b"></feGaussianBlur>
                                                    <feMerge>
                                                        <feMergeNode in="b"></feMergeNode>
                                                        <feMergeNode in="SourceGraphic"></feMergeNode>
                                                    </feMerge>
                                                </filter>
                                            </defs>
                                            <g filter="url(#planetglow)">
                                                <circle cx="200" cy="200" r="150" fill=color.clone()></circle>
                                            </g>
                                            <circle cx="200" cy="200" r="150" fill="url(#limb)"></circle>
                                            <ellipse cx="200" cy="200" rx="150" ry="52" fill="none" stroke="rgba(0,0,0,0.30)" stroke-width="1"></ellipse>
                                            <ellipse cx="200" cy="200" rx="150" ry="102" fill="none" stroke="rgba(0,0,0,0.20)" stroke-width="1"></ellipse>
                                            <ellipse cx="200" cy="200" rx="52" ry="150" fill="none" stroke="rgba(0,0,0,0.30)" stroke-width="1"></ellipse>
                                            <ellipse cx="200" cy="200" rx="102" ry="150" fill="none" stroke="rgba(0,0,0,0.20)" stroke-width="1"></ellipse>
                                            <line x1="50" y1="200" x2="350" y2="200" stroke="rgba(0,0,0,0.30)" stroke-width="1"></line>
                                        </svg>
                                        <div class="planet-note">"unsurveyed world — no map yet"</div>
                                    </div>
                                }.into_any()
                            }}
                            </div>
                            <div class="world-derived">
                                <a href=href class="region-pill planet-open">
                                    {c.flag.clone()}" "{c.name.clone()}" →"
                                </a>
                                <div>
                                    <label>"CITIZEN VALUE"</label>
                                    <b style="color: var(--cyber-green);">{
                                        let v = c.metric(SortField::Human);
                                        move || crate::numeraires::fmt_value(v, numeraire.get())
                                    }</b>
                                </div>
                                <div>
                                    <label>"LAND VALUE"</label>
                                    <b style="color: var(--cyber-cyan);">{
                                        let v = c.metric(SortField::Land);
                                        move || crate::numeraires::fmt_value(v, numeraire.get())
                                    }</b>
                                </div>
                                <div>
                                    <label>"DENSITY"</label>
                                    <b style="color: var(--cyber-purple);">{
                                        if c.land_area_km2 > 0 {
                                            format!("{:.1}/km\u{b2}", c.population as f64 / c.land_area_km2 as f64)
                                        } else {
                                            "0/km\u{b2}".to_string()
                                        }
                                    }</b>
                                </div>
                            </div>
                        }
                    })}
                </div>
            </div>
        </div>
    }
}

/// cap_fmt says N/A for unmonetized worlds; the cockpit says soon.
fn value_or_soon(text: String) -> AnyView {
    if text == "N/A" {
        view! { <crate::components::notyet::NotYet /> }.into_any()
    } else {
        view! { <span>{text}</span> }.into_any()
    }
}
