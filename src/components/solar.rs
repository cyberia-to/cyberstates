use leptos::prelude::*;
use std::collections::HashMap;
use crate::data::{load_countries, Country};
use crate::pages::map::navigate_client;
use crate::pages::solar_map::{orbit_r, placed_bodies, ELEMENTS};

/// The home page's solar catalog: the same live orbital disk as /solar
/// (today's real longitudes, log orbits, bodies sized by their surfaces),
/// but pure catalog logic — every dot is a LINK to its state page, no
/// selection. The home paint effect colors the dots by data-code with the
/// active rating, so the disk filters and morphs with the world map.
#[component]
pub fn SolarPanel() -> impl IntoView {
    let by_code: HashMap<String, Country> = load_countries()
        .into_iter()
        .filter(|c| c.region == "Solar System")
        .map(|c| (c.code.clone(), c))
        .collect();

    let placed = placed_bodies(&by_code);

    let mut ring_as: Vec<f64> = ELEMENTS.iter().map(|&(_, a, _, _)| a).collect();
    ring_as.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ring_as.dedup_by(|a, b| (*a - *b).abs() < 0.2);

    view! {
        <div class="solar-panel">
            <svg viewBox="48 18 904 904">
                <defs>
                    <filter id="pglow" x="-100%" y="-100%" width="300%" height="300%">
                        <feGaussianBlur stdDeviation="2.4" result="b"></feGaussianBlur>
                        <feMerge>
                            <feMergeNode in="b"></feMergeNode>
                            <feMergeNode in="SourceGraphic"></feMergeNode>
                        </feMerge>
                    </filter>
                    <radialGradient id="psun">
                        <stop offset="0%" stop-color="rgba(255,240,180,0.14)"></stop>
                        <stop offset="100%" stop-color="rgba(255,220,140,0)"></stop>
                    </radialGradient>
                </defs>

                <circle cx="500" cy="470" r="150" fill="url(#psun)"></circle>

                {ring_as.into_iter().map(|a| view! {
                    <circle
                        cx="500" cy="470" r=orbit_r(a)
                        fill="none" stroke="#161616" stroke-width="1.4"
                    ></circle>
                }).collect_view()}

                {placed.into_iter().map(|(c, x, y, r)| {
                    let href = format!("/state/{}", c.code.to_lowercase());
                    let title = format!("{} — {}", c.name, c.land_area_fmt());
                    // the panel is small: dots grow a size class to stay clickable
                    let r = (r * 1.35).max(5.0);
                    view! {
                        <g class="solar-body" on:click=move |_| navigate_client(&href)>
                            <circle
                                cx=x cy=y r=r
                                fill="#1a1a1a"
                                data-code=c.code.clone()
                                filter="url(#pglow)"
                            >
                                <title>{title}</title>
                            </circle>
                        </g>
                    }
                }).collect_view()}
            </svg>
        </div>
    }
}
