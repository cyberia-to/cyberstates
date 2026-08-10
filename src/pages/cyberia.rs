//! Experimental cyberia console: fleets & flats.
//!
//! soft3 compliance (experimental surface):
//! - intent is first-class (assign fleet × action × flat)
//! - open map data (public Gesing KML), no closed gatekeeper
//! - client-local queue only — no proprietary backend API
//! - vocabulary: fleet (work), flat (hold/space), intent (write)

use crate::components::brand::BrandChooser;
use crate::components::nav::SiteNav;
use leptos::prelude::*;
use serde::Deserialize;

const MAP_JSON: &str = include_str!("cyberia_map.json");

#[derive(Clone, Debug, Deserialize)]
struct MapData {
    site: String,
    center: [f64; 2],
    bbox: BBox,
    phase0: Vec<Flat>,
    places: Vec<Flat>,
    source: String,
}

#[derive(Clone, Debug, Deserialize)]
struct BBox {
    min_lon: f64,
    max_lon: f64,
    min_lat: f64,
    max_lat: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct Flat {
    id: String,
    name: String,
    kind: String,
    phase: u32,
    geom: String,
    coords: Vec<[f64; 2]>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct FleetUnit {
    id: &'static str,
    name: &'static str,
    kind: &'static str, // worker | machine
    role: &'static str,
    status: &'static str, // idle | busy | offline
    phase: u32,
}

/// Phase-0 fleets: the units you can actually assign today.
const FLEETS: &[FleetUnit] = &[
    FleetUnit {
        id: "f-eye",
        name: "EYE-01",
        kind: "machine",
        role: "survey drone",
        status: "idle",
        phase: 0,
    },
    FleetUnit {
        id: "f-hand",
        name: "HAND-01",
        kind: "worker",
        role: "field hand",
        status: "idle",
        phase: 0,
    },
    FleetUnit {
        id: "f-haul",
        name: "HAUL-01",
        kind: "machine",
        role: "ground rover",
        status: "idle",
        phase: 0,
    },
    FleetUnit {
        id: "f-cut",
        name: "CUT-01",
        kind: "machine",
        role: "clearing arm",
        status: "offline",
        phase: 0,
    },
    FleetUnit {
        id: "f-build",
        name: "CUBE-01",
        kind: "machine",
        role: "build stack",
        status: "offline",
        phase: 1,
    },
];

const ACTIONS: &[(&str, &str)] = &[
    ("survey", "map + sense the flat"),
    ("clear", "remove undergrowth / debris"),
    ("plant", "set regeneratives"),
    ("haul", "move material on-site"),
    ("watch", "hold a perimeter watch"),
    ("build", "raise a cyberCube (phase 1+)"),
];

/// Robots available for purchase (catalog, not yet owned).
const ROBOT_CATALOG: &[FleetUnit] = &[
    FleetUnit {
        id: "cat-eye",
        name: "EYE",
        kind: "machine",
        role: "survey drone",
        status: "idle",
        phase: 0,
    },
    FleetUnit {
        id: "cat-hand",
        name: "HAND",
        kind: "worker",
        role: "field hand",
        status: "idle",
        phase: 0,
    },
    FleetUnit {
        id: "cat-haul",
        name: "HAUL",
        kind: "machine",
        role: "ground rover",
        status: "idle",
        phase: 0,
    },
    FleetUnit {
        id: "cat-cut",
        name: "CUT",
        kind: "machine",
        role: "clearing arm",
        status: "idle",
        phase: 0,
    },
];

#[derive(Clone, Copy, Debug, PartialEq)]
enum Sheet {
    None,
    BuyRobot,
    LeaseLand,
}

#[derive(Clone, Debug, PartialEq)]
struct Intent {
    id: u64,
    /// work unit / buyer label / "LEASE"
    fleet: String,
    action: String,
    flat: String,
}

#[derive(Clone, Debug, PartialEq)]
struct OwnedRobot {
    id: String,
    name: String,
    kind: String,
    role: String,
}

fn load_map() -> MapData {
    serde_json::from_str(MAP_JSON).expect("cyberia_map.json")
}

/// Project lon/lat into a viewBox-local SVG plane (y flipped).
fn project(lon: f64, lat: f64, bbox: &BBox, w: f64, h: f64, pad: f64) -> (f64, f64) {
    let dx = (bbox.max_lon - bbox.min_lon).max(1e-9);
    let dy = (bbox.max_lat - bbox.min_lat).max(1e-9);
    // square-ish fit with padding
    let x = pad + (lon - bbox.min_lon) / dx * (w - 2.0 * pad);
    let y = pad + (1.0 - (lat - bbox.min_lat) / dy) * (h - 2.0 * pad);
    (x, y)
}

fn poly_path(coords: &[[f64; 2]], bbox: &BBox, w: f64, h: f64, pad: f64) -> String {
    if coords.is_empty() {
        return String::new();
    }
    let mut s = String::new();
    for (i, c) in coords.iter().enumerate() {
        let (x, y) = project(c[0], c[1], bbox, w, h, pad);
        if i == 0 {
            s.push_str(&format!("M{x:.2},{y:.2}"));
        } else {
            s.push_str(&format!(" L{x:.2},{y:.2}"));
        }
    }
    s.push('Z');
    s
}

fn centroid(coords: &[[f64; 2]]) -> (f64, f64) {
    if coords.is_empty() {
        return (0.0, 0.0);
    }
    let n = coords.len() as f64;
    let lon = coords.iter().map(|c| c[0]).sum::<f64>() / n;
    let lat = coords.iter().map(|c| c[1]).sum::<f64>() / n;
    (lon, lat)
}

fn status_color(s: &str) -> &'static str {
    match s {
        "idle" => "var(--cyber-green)",
        "busy" => "var(--cyber-yellow)",
        _ => "#555",
    }
}

fn flat_color(id: &str, selected: bool) -> &'static str {
    match (id, selected) {
        ("sinwood", true) => "rgba(0,255,65,0.45)",
        ("sinwood", false) => "rgba(0,255,65,0.18)",
        ("avalon", true) => "rgba(255,102,0,0.45)",
        ("avalon", false) => "rgba(255,102,0,0.18)",
        _ => "rgba(0,229,255,0.15)",
    }
}

fn flat_stroke(id: &str, selected: bool) -> &'static str {
    match (id, selected) {
        ("sinwood", true) => "var(--cyber-green)",
        ("sinwood", false) => "rgba(0,255,65,0.55)",
        ("avalon", true) => "var(--cyber-orange)",
        ("avalon", false) => "rgba(255,102,0,0.55)",
        _ => "var(--cyber-cyan)",
    }
}

#[component]
pub fn CyberiaPage() -> impl IntoView {
    let map = load_map();
    let map = std::sync::Arc::new(map);

    let selected_flat = RwSignal::new(Some("sinwood".to_string()));
    let selected_fleet = RwSignal::new(Some("f-eye".to_string()));
    let selected_action = RwSignal::new("survey".to_string());
    let intents = RwSignal::new(Vec::<Intent>::new());
    let next_id = RwSignal::new(1u64);
    let owned = RwSignal::new(Vec::<OwnedRobot>::new());
    let leased = RwSignal::new(Vec::<String>::new()); // flat ids
    let sheet = RwSignal::new(Sheet::None);
    let buy_pick = RwSignal::new("cat-eye".to_string());
    let robot_serial = RwSignal::new(1u32);

    Effect::new(move |_| {
        document().set_title("Cyberia — fleets & flats · Gesing, Bali");
    });

    let map_for_svg = map.clone();
    let map_for_3d = map.clone();
    let map_for_meta = map.clone();
    let map_for_intent = map.clone();
    let map_for_lease = map.clone();

    view! {
        <div class="page-shell cyberia-shell">
            <div class="site-chrome">
            <div class="chrome-inner">
            <div class="header-row1">
                <div class="logo-zone">
                    <BrandChooser active="CYBERIA" />
                    <div class="logo-suffix"></div>
                </div>
                <div class="cyberia-phase-pill">
                    <span class="phase-dot"></span>
                    "PHASE 0 · SINWOOD + AVALON"
                </div>
                <SiteNav active="CYBERIA" />
            </div>
            <div class="header-row2 cyberia-subhead">
                <span class="by-label">"fleets & flats"</span>
                <span class="cyberia-site">{map_for_meta.site.clone()}</span>
                <div class="cyberia-ctas">
                    <button class="cta-btn cta-buy" on:click=move |_| sheet.set(Sheet::BuyRobot)>
                        "BUY ROBOT"
                    </button>
                    <button class="cta-btn cta-lease" on:click=move |_| sheet.set(Sheet::LeaseLand)>
                        "LEASE LAND"
                    </button>
                </div>
                <span class="cyberia-soft3" title="soft3: intent first-class · open map · no closed API">
                    "soft3 · intent → fleet × flat × work"
                </span>
            </div>
            </div>
            </div>

            <div class="cyberia-stage">
                // LEFT — FLEETS
                <section class="cyberia-panel cyberia-fleets">
                    <div class="cyberia-panel-h">
                        <span class="panel-kicker">"FLEETS"</span>
                        <span class="panel-sub">"workers · machines"</span>
                        <button class="panel-cta" on:click=move |_| sheet.set(Sheet::BuyRobot)>"+ BUY ROBOT"</button>
                    </div>
                    <div class="fleet-list">
                        {FLEETS.iter().map(|f| {
                            let id = f.id.to_string();
                            let id2 = id.clone();
                            let offline = f.status == "offline" || f.phase > 0;
                            let kind_worker = f.kind == "worker";
                            view! {
                                <button
                                    class=move || {
                                        let sel = selected_fleet.get().as_deref() == Some(id.as_str());
                                        format!(
                                            "fleet-card{}{}{}",
                                            if sel { " sel" } else { "" },
                                            if offline { " offline" } else { "" },
                                            if kind_worker { " worker" } else { " machine" },
                                        )
                                    }
                                    disabled=offline
                                    on:click=move |_| {
                                        if !offline {
                                            selected_fleet.set(Some(id2.clone()));
                                        }
                                    }
                                >
                                    <div class="fleet-top">
                                        <span class="fleet-name">{f.name}</span>
                                        <span class="fleet-status" style:color=status_color(f.status)>
                                            {f.status.to_uppercase()}
                                        </span>
                                    </div>
                                    <div class="fleet-role">{f.role}</div>
                                    <div class="fleet-meta">
                                        <span>{f.kind.to_uppercase()}</span>
                                        <span>{format!("P{}", f.phase)}</span>
                                    </div>
                                </button>
                            }
                        }).collect_view()}
                        // robots you bought this session
                        {move || owned.get().into_iter().map(|r| {
                            let id = r.id.clone();
                            let id2 = id.clone();
                            let kind_worker = r.kind == "worker";
                            let name = r.name.clone();
                            let role = r.role.clone();
                            let kind = r.kind.clone();
                            view! {
                                <button
                                    class=move || {
                                        let sel = selected_fleet.get().as_deref() == Some(id.as_str());
                                        format!(
                                            "fleet-card owned{}{}",
                                            if sel { " sel" } else { "" },
                                            if kind_worker { " worker" } else { " machine" },
                                        )
                                    }
                                    on:click=move |_| selected_fleet.set(Some(id2.clone()))
                                >
                                    <div class="fleet-top">
                                        <span class="fleet-name">{name}</span>
                                        <span class="fleet-status" style:color=status_color("idle")>"OWNED"</span>
                                    </div>
                                    <div class="fleet-role">{role}</div>
                                    <div class="fleet-meta">
                                        <span>{kind.to_uppercase()}</span>
                                        <span>"BOUGHT"</span>
                                    </div>
                                </button>
                            }
                        }).collect_view()}
                    </div>
                    <div class="cyberia-hint">
                        "Pick a fleet unit, a flat on the map, an action — then commit intent. Or buy a robot / lease land."
                    </div>
                </section>

                // CENTER — FLATS MAP
                <section class="cyberia-panel cyberia-flats">
                    <div class="cyberia-panel-h">
                        <span class="panel-kicker">"FLATS"</span>
                        <span class="panel-sub">"Gesing local · phase 0"</span>
                        <button class="panel-cta lease" on:click=move |_| sheet.set(Sheet::LeaseLand)>"LEASE LAND"</button>
                    </div>
                    <div class="flat-map-wrap">
                        {
                            let m = map_for_svg.clone();
                            const W: f64 = 640.0;
                            const H: f64 = 480.0;
                            const PAD: f64 = 28.0;
                            view! {
                                <svg class="flat-map" viewBox=format!("0 0 {W} {H}") preserveAspectRatio="xMidYMid meet">
                                    // grid
                                    {(0..8).map(|i| {
                                        let x = PAD + i as f64 * (W - 2.0*PAD) / 7.0;
                                        let y2 = H - PAD;
                                        view! {
                                            <line x1=x y1=PAD x2=x y2=y2
                                                stroke="#1a1a1a" stroke-width="1" />
                                        }
                                    }).collect_view()}
                                    {(0..6).map(|i| {
                                        let y = PAD + i as f64 * (H - 2.0*PAD) / 5.0;
                                        let x2 = W - PAD;
                                        view! {
                                            <line x1=PAD y1=y x2=x2 y2=y
                                                stroke="#1a1a1a" stroke-width="1" />
                                        }
                                    }).collect_view()}

                                    // phase 0 polygons
                                    {m.phase0.iter().map(|flat| {
                                        let id_click = flat.id.clone();
                                        let id_fill = flat.id.clone();
                                        let id_stroke = flat.id.clone();
                                        let id_w = flat.id.clone();
                                        let id_lab = flat.id.clone();
                                        let d = poly_path(&flat.coords, &m.bbox, W, H, PAD);
                                        let (clon, clat) = centroid(&flat.coords);
                                        let (lx, ly) = project(clon, clat, &m.bbox, W, H, PAD);
                                        let base_label = flat.name.to_uppercase();
                                        view! {
                                            <g class="flat-poly"
                                                on:click=move |_| selected_flat.set(Some(id_click.clone()))
                                            >
                                                <path
                                                    d=d
                                                    fill=move || {
                                                        let sel = selected_flat.get().as_deref() == Some(id_fill.as_str());
                                                        flat_color(&id_fill, sel).to_string()
                                                    }
                                                    stroke=move || {
                                                        let sel = selected_flat.get().as_deref() == Some(id_stroke.as_str());
                                                        flat_stroke(&id_stroke, sel).to_string()
                                                    }
                                                    stroke-width=move || {
                                                        if selected_flat.get().as_deref() == Some(id_w.as_str()) { "2.5" } else { "1.5" }
                                                    }
                                                    class="flat-path"
                                                />
                                                <text x=lx y=ly text-anchor="middle" class="flat-label">
                                                    {move || {
                                                        if leased.get().iter().any(|x| x == &id_lab) {
                                                            format!("{base_label} · LEASED")
                                                        } else {
                                                            base_label.clone()
                                                        }
                                                    }}
                                                </text>
                                            </g>
                                        }
                                    }).collect_view()}

                                    // context places
                                    {m.places.iter().map(|p| {
                                        if p.coords.is_empty() { return view! { <g></g> }.into_any(); }
                                        let (x, y) = project(p.coords[0][0], p.coords[0][1], &m.bbox, W, H, PAD);
                                        let name = p.name.clone();
                                        let ty = y - 7.0;
                                        view! {
                                            <g class="place-dot">
                                                <circle cx=x cy=y r="3.5" fill="#444" stroke="#777" stroke-width="1" />
                                                <text x=x y=ty text-anchor="middle" class="place-label">{name}</text>
                                            </g>
                                        }.into_any()
                                    }).collect_view()}

                                    <text x=PAD y={H - 8.0} class="map-caption">
                                        {format!("N ↑  ·  {}  ·  open KML", m.source)}
                                    </text>
                                </svg>
                            }
                        }
                    </div>
                    <div class="flat-legend">
                        <span class="leg sin">"SINWOOD"</span>
                        <span class="leg avalon">"AVALON"</span>
                        <span class="leg dim">"places P1+"</span>
                    </div>
                </section>

                // RIGHT — 3D + INTENT
                <section class="cyberia-panel cyberia-right">
                    <div class="cyberia-panel-h">
                        <span class="panel-kicker">"RENDER"</span>
                        <span class="panel-sub">"selected flat · 3d"</span>
                    </div>
                    <div class="render-3d">
                        {move || {
                            let m = map_for_3d.clone();
                            let fid = selected_flat.get().unwrap_or_else(|| "sinwood".into());
                            let flat = m.phase0.iter().find(|f| f.id == fid);
                            let name = flat.map(|f| f.name.to_uppercase()).unwrap_or_else(|| fid.to_uppercase());
                            let kind = flat.map(|f| f.kind.clone()).unwrap_or_default();
                            let n = flat.map(|f| f.coords.len()).unwrap_or(0);
                            // simple extruded prism via CSS 3D
                            let hue = if fid == "sinwood" { "var(--cyber-green)" } else { "var(--cyber-orange)" };
                            view! {
                                <div class="prism-scene">
                                    <div class="prism" style:--prism-color=hue>
                                        <div class="prism-face prism-top"></div>
                                        <div class="prism-face prism-front"></div>
                                        <div class="prism-face prism-side"></div>
                                    </div>
                                    <div class="prism-meta">
                                        <div class="prism-name">{name}</div>
                                        <div class="prism-sub">{format!("{kind} · {n} vertices · phase 0")}</div>
                                        <div class="prism-sub">"Gesing · local hold"</div>
                                    </div>
                                </div>
                            }
                        }}
                    </div>

                    <div class="cyberia-panel-h" style="margin-top: 12px;">
                        <span class="panel-kicker">"INTENT"</span>
                        <span class="panel-sub">"soft3 write"</span>
                    </div>
                    <div class="intent-form">
                        <div class="intent-row">
                            <span class="intent-k">"fleet"</span>
                            <span class="intent-v">
                                {move || selected_fleet.get()
                                    .and_then(|id| FLEETS.iter().find(|f| f.id == id).map(|f| f.name))
                                    .unwrap_or("—")}
                            </span>
                        </div>
                        <div class="intent-row">
                            <span class="intent-k">"flat"</span>
                            <span class="intent-v">
                                {move || selected_flat.get().unwrap_or_else(|| "—".into()).to_uppercase()}
                            </span>
                        </div>
                        <div class="intent-actions">
                            {ACTIONS.iter().map(|(a, _)| {
                                let act = (*a).to_string();
                                let act2 = act.clone();
                                let locked = *a == "build";
                                view! {
                                    <button
                                        class=move || {
                                            let sel = selected_action.get() == act;
                                            format!("act-pill{}{}", if sel { " sel" } else { "" }, if locked { " locked" } else { "" })
                                        }
                                        disabled=locked
                                        on:click=move |_| {
                                            if !locked {
                                                selected_action.set(act2.clone());
                                            }
                                        }
                                    >{a.to_uppercase()}</button>
                                }
                            }).collect_view()}
                        </div>
                        <button
                            class="intent-commit"
                            on:click=move |_| {
                                let Some(fleet) = selected_fleet.get() else { return };
                                let Some(flat) = selected_flat.get() else { return };
                                let action = selected_action.get();
                                // soft3: offline stock fleets cannot take intent
                                if FLEETS.iter().any(|f| f.id == fleet && (f.status == "offline" || f.phase > 0)) {
                                    return;
                                }
                                let fleet_name = FLEETS
                                    .iter()
                                    .find(|f| f.id == fleet)
                                    .map(|f| f.name.to_string())
                                    .or_else(|| {
                                        owned
                                            .get_untracked()
                                            .into_iter()
                                            .find(|r| r.id == fleet)
                                            .map(|r| r.name)
                                    })
                                    .unwrap_or(fleet);
                                let id = next_id.get();
                                next_id.set(id + 1);
                                intents.update(|q| {
                                    q.insert(0, Intent {
                                        id,
                                        fleet: fleet_name,
                                        action,
                                        flat,
                                    });
                                });
                            }
                        >
                            "COMMIT INTENT"
                        </button>
                        <div class="intent-queue">
                            {move || {
                                let q = intents.get();
                                if q.is_empty() {
                                    return view! {
                                        <div class="intent-empty">"no intents yet — buy, lease, or assign a fleet"</div>
                                    }.into_any();
                                }
                                view! {
                                    <div class="intent-list">
                                        {q.into_iter().take(10).map(|it| {
                                            let act_cls = match it.action.as_str() {
                                                "buy" => "ii-act buy",
                                                "lease" => "ii-act lease",
                                                _ => "ii-act",
                                            };
                                            view! {
                                                <div class="intent-item">
                                                    <span class="ii-id">{format!("#{:03}", it.id)}</span>
                                                    <span class="ii-fleet">{it.fleet}</span>
                                                    <span class=act_cls>{it.action.to_uppercase()}</span>
                                                    <span class="ii-flat">{it.flat.to_uppercase()}</span>
                                                </div>
                                            }
                                        }).collect_view()}
                                    </div>
                                }.into_any()
                            }}
                        </div>
                    </div>
                </section>
            </div>

            // ── sheets: BUY ROBOT / LEASE LAND ──
            {move || match sheet.get() {
                Sheet::None => view! { <div></div> }.into_any(),
                Sheet::BuyRobot => view! {
                    <div class="cyberia-sheet-backdrop" on:click=move |_| sheet.set(Sheet::None)>
                        <div class="cyberia-sheet" on:click=move |ev| ev.stop_propagation()>
                            <div class="sheet-h">
                                <span class="panel-kicker">"BUY ROBOT"</span>
                                <button class="sheet-x" on:click=move |_| sheet.set(Sheet::None)>"✕"</button>
                            </div>
                            <p class="sheet-note">
                                "Phase 0 catalog — soft3 local intent. No payment rail yet; purchase queues an intent and adds the unit to your fleet."
                            </p>
                            <div class="sheet-catalog">
                                {ROBOT_CATALOG.iter().map(|r| {
                                    let id = r.id.to_string();
                                    let id2 = id.clone();
                                    let kind_worker = r.kind == "worker";
                                    view! {
                                        <button
                                            class=move || {
                                                let sel = buy_pick.get() == id;
                                                format!(
                                                    "fleet-card catalog{}{}",
                                                    if sel { " sel" } else { "" },
                                                    if kind_worker { " worker" } else { " machine" },
                                                )
                                            }
                                            on:click=move |_| buy_pick.set(id2.clone())
                                        >
                                            <div class="fleet-top">
                                                <span class="fleet-name">{r.name}</span>
                                                <span class="fleet-status" style:color="var(--cyber-yellow)">"FOR SALE"</span>
                                            </div>
                                            <div class="fleet-role">{r.role}</div>
                                            <div class="fleet-meta">
                                                <span>{r.kind.to_uppercase()}</span>
                                                <span>"P0"</span>
                                            </div>
                                        </button>
                                    }
                                }).collect_view()}
                            </div>
                            <button
                                class="intent-commit"
                                on:click=move |_| {
                                    let pick = buy_pick.get();
                                    let Some(cat) = ROBOT_CATALOG.iter().find(|r| r.id == pick) else { return };
                                    let n = robot_serial.get();
                                    robot_serial.set(n + 1);
                                    let unit_id = format!("own-{}-{}", cat.name.to_lowercase(), n);
                                    let unit_name = format!("{}-{:02}", cat.name, n);
                                    owned.update(|v| {
                                        v.push(OwnedRobot {
                                            id: unit_id.clone(),
                                            name: unit_name.clone(),
                                            kind: cat.kind.to_string(),
                                            role: cat.role.to_string(),
                                        });
                                    });
                                    selected_fleet.set(Some(unit_id));
                                    let id = next_id.get();
                                    next_id.set(id + 1);
                                    intents.update(|q| {
                                        q.insert(0, Intent {
                                            id,
                                            fleet: unit_name,
                                            action: "buy".into(),
                                            flat: "market".into(),
                                        });
                                    });
                                    sheet.set(Sheet::None);
                                }
                            >
                                "CONFIRM BUY"
                            </button>
                        </div>
                    </div>
                }.into_any(),
                Sheet::LeaseLand => {
                    let flats = map_for_lease.phase0.clone();
                    view! {
                        <div class="cyberia-sheet-backdrop" on:click=move |_| sheet.set(Sheet::None)>
                            <div class="cyberia-sheet" on:click=move |ev| ev.stop_propagation()>
                                <div class="sheet-h">
                                    <span class="panel-kicker">"LEASE LAND"</span>
                                    <button class="sheet-x" on:click=move |_| sheet.set(Sheet::None)>"✕"</button>
                                </div>
                                <p class="sheet-note">
                                    "Phase 0 flats — century-index denominated later. Soft3 local intent only; no closed payment backend."
                                </p>
                                <div class="sheet-catalog">
                                    {flats.into_iter().map(|f| {
                                        let id_cls = f.id.clone();
                                        let id_dis = f.id.clone();
                                        let id_click = f.id.clone();
                                        let id_col = f.id.clone();
                                        let id_lab = f.id.clone();
                                        let name = f.name.to_uppercase();
                                        let kind = f.kind.clone();
                                        view! {
                                            <button
                                                class=move || {
                                                    let sel = selected_flat.get().as_deref() == Some(id_cls.as_str());
                                                    let taken = leased.get().iter().any(|x| x == &id_cls);
                                                    format!(
                                                        "fleet-card catalog flat-pick{}{}",
                                                        if sel { " sel" } else { "" },
                                                        if taken { " offline" } else { "" },
                                                    )
                                                }
                                                disabled=move || leased.get().iter().any(|x| x == &id_dis)
                                                on:click=move |_| selected_flat.set(Some(id_click.clone()))
                                            >
                                                <div class="fleet-top">
                                                    <span class="fleet-name">{name.clone()}</span>
                                                    <span class="fleet-status" style:color=move || {
                                                        if leased.get().iter().any(|x| x == &id_col) {
                                                            "var(--cyber-yellow)"
                                                        } else {
                                                            "var(--cyber-green)"
                                                        }
                                                    }>
                                                        {move || {
                                                            if leased.get().iter().any(|x| x == &id_lab) {
                                                                "LEASED"
                                                            } else {
                                                                "OPEN"
                                                            }
                                                        }}
                                                    </span>
                                                </div>
                                                <div class="fleet-role">{format!("{kind} · Gesing · phase 0")}</div>
                                                <div class="fleet-meta">
                                                    <span>"FLAT"</span>
                                                    <span>"CX later"</span>
                                                </div>
                                            </button>
                                        }
                                    }).collect_view()}
                                </div>
                                <button
                                    class="intent-commit lease"
                                    on:click=move |_| {
                                        let Some(flat) = selected_flat.get() else { return };
                                        if leased.get_untracked().iter().any(|x| x == &flat) {
                                            return;
                                        }
                                        leased.update(|v| v.push(flat.clone()));
                                        let id = next_id.get();
                                        next_id.set(id + 1);
                                        intents.update(|q| {
                                            q.insert(0, Intent {
                                                id,
                                                fleet: "YOU".into(),
                                                action: "lease".into(),
                                                flat: flat.clone(),
                                            });
                                        });
                                        sheet.set(Sheet::None);
                                    }
                                >
                                    "CONFIRM LEASE"
                                </button>
                            </div>
                        </div>
                    }.into_any()
                }
            }}

            <div class="search-dock cyberia-dock">
                <span class="dock-count">
                    {move || {
                        let n = intents.get().len();
                        let m = map_for_intent.phase0.len();
                        let o = owned.get().len();
                        let l = leased.get().len();
                        format!(
                            "{n} intents · {m} flats · {}+{o} fleets · {l} leases",
                            FLEETS.len()
                        )
                    }}
                </span>
                <span class="dock-credit cyberia-soft3-dock">
                    "soft3-compliant · buy/lease = local intent · no closed backend"
                </span>
                <a href="https://x.com/cyberiacap" target="_blank" rel="noopener" class="dock-credit">
                    "🏴 a "<span style="color: var(--cyber-green);">"cyberia"</span>" project"
                </a>
            </div>
        </div>
    }
}
