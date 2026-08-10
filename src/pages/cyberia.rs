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

/// Existing Gesing hard-force workers (ops roster) — 15 people on site.
/// Source: cve/ops/hard force.md crews (repair · cube · base · pruning · delivery).
const WORKERS: &[FleetUnit] = &[
    // repair
    FleetUnit {
        id: "w-sutar",
        name: "SUTAR",
        kind: "worker",
        role: "repair lead · energy/water",
        status: "idle",
        phase: 0,
    },
    FleetUnit {
        id: "w-witaya",
        name: "WITAYA",
        kind: "worker",
        role: "repair · electronics",
        status: "idle",
        phase: 0,
    },
    FleetUnit {
        id: "w-lupus",
        name: "LUPUS",
        kind: "worker",
        role: "repair · mechanical",
        status: "idle",
        phase: 0,
    },
    // cube
    FleetUnit {
        id: "w-sudi",
        name: "SUDI",
        kind: "worker",
        role: "cube lead · build",
        status: "idle",
        phase: 0,
    },
    FleetUnit {
        id: "w-budi",
        name: "BUDI",
        kind: "worker",
        role: "cube · build",
        status: "idle",
        phase: 0,
    },
    FleetUnit {
        id: "w-tika",
        name: "TIKA",
        kind: "worker",
        role: "cube · stove",
        status: "idle",
        phase: 0,
    },
    FleetUnit {
        id: "w-sastra",
        name: "SASTRA",
        kind: "worker",
        role: "cube · build",
        status: "idle",
        phase: 0,
    },
    // base
    FleetUnit {
        id: "w-angga",
        name: "ANGGA",
        kind: "worker",
        role: "base lead · road/trail",
        status: "idle",
        phase: 0,
    },
    FleetUnit {
        id: "w-darma",
        name: "DARMA",
        kind: "worker",
        role: "base · mason",
        status: "idle",
        phase: 0,
    },
    FleetUnit {
        id: "w-darsana",
        name: "DARSANA",
        kind: "worker",
        role: "base · terrace",
        status: "idle",
        phase: 0,
    },
    // pruning
    FleetUnit {
        id: "w-arima",
        name: "ARIMA",
        kind: "worker",
        role: "pruning lead · land",
        status: "idle",
        phase: 0,
    },
    FleetUnit {
        id: "w-doplang",
        name: "DOPLANG",
        kind: "worker",
        role: "pruning · firewood",
        status: "idle",
        phase: 0,
    },
    FleetUnit {
        id: "w-surya",
        name: "SURYA",
        kind: "worker",
        role: "pruning · fodder",
        status: "idle",
        phase: 0,
    },
    FleetUnit {
        id: "w-suardita",
        name: "SUARDITA",
        kind: "worker",
        role: "pruning · compost",
        status: "idle",
        phase: 0,
    },
    // delivery
    FleetUnit {
        id: "w-pande",
        name: "PANDE",
        kind: "worker",
        role: "delivery lead · haul",
        status: "idle",
        phase: 0,
    },
];

/// Phase-0 machines (hardware fleet).
const MACHINES: &[FleetUnit] = &[
    FleetUnit {
        id: "f-eye",
        name: "EYE-01",
        kind: "machine",
        role: "survey drone",
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

fn all_stock_fleets() -> impl Iterator<Item = &'static FleetUnit> {
    WORKERS.iter().chain(MACHINES.iter())
}

fn stock_fleet(id: &str) -> Option<&'static FleetUnit> {
    all_stock_fleets().find(|f| f.id == id)
}

/// Split a polygon by median longitude into west / east halves.
fn split_flat(flat: &Flat) -> (Flat, Flat) {
    let mut coords = flat.coords.clone();
    if coords.len() > 1 && coords.first() == coords.last() {
        coords.pop(); // drop closing point for split
    }
    let mut lons: Vec<f64> = coords.iter().map(|c| c[0]).collect();
    lons.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = lons[lons.len() / 2];
    let mut west: Vec<[f64; 2]> = coords.iter().copied().filter(|c| c[0] <= mid).collect();
    let mut east: Vec<[f64; 2]> = coords.iter().copied().filter(|c| c[0] >= mid).collect();
    // ensure polygons close
    if west.len() >= 2 && west.first() != west.last() {
        west.push(west[0]);
    }
    if east.len() >= 2 && east.first() != east.last() {
        east.push(east[0]);
    }
    // fallback if a half collapsed
    if west.len() < 3 {
        west = flat.coords.clone();
    }
    if east.len() < 3 {
        east = flat.coords.clone();
    }
    let a = Flat {
        id: format!("{}-a", flat.id),
        name: format!("{}-a", flat.name),
        kind: flat.kind.clone(),
        phase: flat.phase,
        geom: "polygon".into(),
        coords: west,
    };
    let b = Flat {
        id: format!("{}-b", flat.id),
        name: format!("{}-b", flat.name),
        kind: flat.kind.clone(),
        phase: flat.phase,
        geom: "polygon".into(),
        coords: east,
    };
    (a, b)
}

fn merge_flats(a: &Flat, b: &Flat) -> Flat {
    let mut coords = a.coords.clone();
    if coords.len() > 1 && coords.first() == coords.last() {
        coords.pop();
    }
    for c in &b.coords {
        if coords.last() != Some(c) {
            coords.push(*c);
        }
    }
    if coords.len() >= 2 && coords.first() != coords.last() {
        coords.push(coords[0]);
    }
    Flat {
        id: format!("{}+{}", a.id, b.id),
        name: format!("{}+{}", a.name, b.name),
        kind: a.kind.clone(),
        phase: a.phase.min(b.phase),
        geom: "polygon".into(),
        coords,
    }
}

fn flat_fill(id: &str, selected: bool) -> &'static str {
    if selected {
        if id.contains("avalon") {
            "rgba(255,102,0,0.45)"
        } else if id.contains("sinwood") {
            "rgba(0,255,65,0.45)"
        } else {
            "rgba(0,229,255,0.40)"
        }
    } else if id.contains("avalon") {
        "rgba(255,102,0,0.18)"
    } else if id.contains("sinwood") {
        "rgba(0,255,65,0.18)"
    } else {
        "rgba(0,229,255,0.15)"
    }
}

fn flat_stroke_col(id: &str, selected: bool) -> &'static str {
    if selected {
        if id.contains("avalon") {
            "var(--cyber-orange)"
        } else if id.contains("sinwood") {
            "var(--cyber-green)"
        } else {
            "var(--cyber-cyan)"
        }
    } else if id.contains("avalon") {
        "rgba(255,102,0,0.55)"
    } else if id.contains("sinwood") {
        "rgba(0,255,65,0.55)"
    } else {
        "rgba(0,229,255,0.45)"
    }
}

fn render_fleet_card(
    f: &'static FleetUnit,
    selected_fleet: RwSignal<Option<String>>,
) -> impl IntoView {
    let id = f.id.to_string();
    let id2 = id.clone();
    let offline = f.status == "offline" || f.phase > 0;
    let kind_worker = f.kind == "worker";
    let name = f.name;
    let role = f.role;
    let kind = f.kind;
    let status = f.status;
    let phase = f.phase;
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
                <span class="fleet-name">{name}</span>
                <span class="fleet-status" style:color=status_color(status)>
                    {status.to_uppercase()}
                </span>
            </div>
            <div class="fleet-role">{role}</div>
            <div class="fleet-meta">
                <span>{kind.to_uppercase()}</span>
                <span>{format!("P{phase}")}</span>
            </div>
        </button>
    }
}

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
    SplitLand,
    MergeLand,
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

#[component]
pub fn CyberiaPage() -> impl IntoView {
    let map = load_map();
    let map = std::sync::Arc::new(map);

    let selected_flat = RwSignal::new(Some("sinwood".to_string()));
    let selected_fleet = RwSignal::new(Some("w-sutar".to_string()));
    let selected_action = RwSignal::new("survey".to_string());
    let intents = RwSignal::new(Vec::<Intent>::new());
    let next_id = RwSignal::new(1u64);
    let owned = RwSignal::new(Vec::<OwnedRobot>::new());
    let leased = RwSignal::new(Vec::<String>::new()); // flat ids
    let flats = RwSignal::new(map.phase0.clone()); // live land geometry (split/merge)
    let sheet = RwSignal::new(Sheet::None);
    let buy_pick = RwSignal::new("cat-eye".to_string());
    let robot_serial = RwSignal::new(1u32);
    let merge_pick = RwSignal::new(None::<String>); // second flat for merge

    Effect::new(move |_| {
        document().set_title("Cyberia — fleets & flats · Gesing, Bali");
    });

    let map_for_svg = map.clone();
    let map_for_meta = map.clone();

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
                        <span class="panel-sub">{format!("{} workers · {} machines", WORKERS.len(), MACHINES.len())}</span>
                    </div>
                    <div class="fleet-list">
                        <div class="fleet-section">"WORKERS · HARD FORCE"</div>
                        {WORKERS.iter().map(|f| render_fleet_card(f, selected_fleet)).collect_view()}
                        <div class="fleet-section">"MACHINES"</div>
                        {MACHINES.iter().map(|f| render_fleet_card(f, selected_fleet)).collect_view()}
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
                    </div>
                    <div class="flat-map-wrap">
                        {
                            let m = map_for_svg.clone();
                            let m_places = map_for_svg.clone();
                            let m_cap = map_for_svg.clone();
                            const W: f64 = 640.0;
                            const H: f64 = 480.0;
                            const PAD: f64 = 28.0;
                            view! {
                                <svg class="flat-map" viewBox=format!("0 0 {W} {H}") preserveAspectRatio="xMidYMid meet">
                                    {(0..8).map(|i| {
                                        let x = PAD + i as f64 * (W - 2.0*PAD) / 7.0;
                                        let y2 = H - PAD;
                                        view! {
                                            <line x1=x y1=PAD x2=x y2=y2 stroke="#1a1a1a" stroke-width="1" />
                                        }
                                    }).collect_view()}
                                    {(0..6).map(|i| {
                                        let y = PAD + i as f64 * (H - 2.0*PAD) / 5.0;
                                        let x2 = W - PAD;
                                        view! {
                                            <line x1=PAD y1=y x2=x2 y2=y stroke="#1a1a1a" stroke-width="1" />
                                        }
                                    }).collect_view()}

                                    {move || {
                                        let list = flats.get();
                                        list.into_iter().map(|flat| {
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
                                                            flat_fill(&id_fill, sel).to_string()
                                                        }
                                                        stroke=move || {
                                                            let sel = selected_flat.get().as_deref() == Some(id_stroke.as_str());
                                                            flat_stroke_col(&id_stroke, sel).to_string()
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
                                        }).collect_view()
                                    }}

                                    {m_places.places.iter().map(|p| {
                                        if p.coords.is_empty() { return view! { <g></g> }.into_any(); }
                                        let (x, y) = project(p.coords[0][0], p.coords[0][1], &m_places.bbox, W, H, PAD);
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
                                        {format!("N ↑  ·  {}  ·  open KML", m_cap.source)}
                                    </text>
                                </svg>
                            }
                        }
                    </div>
                    <div class="flat-legend">
                        <span class="leg sin">"SINWOOD"</span>
                        <span class="leg avalon">"AVALON"</span>
                        <span class="leg dim">"split/merge live"</span>
                    </div>
                </section>

                // RIGHT — 3D + INTENT
                <section class="cyberia-panel cyberia-right">
                    <div class="cyberia-panel-h">
                        <span class="panel-kicker">"RENDER"</span>
                        <span class="panel-sub">"flat · robot · 3d"</span>
                    </div>
                    <div class="render-3d">
                        {move || {
                            // —— flat ——
                            let fid = selected_flat.get().unwrap_or_else(|| "sinwood".into());
                            let list = flats.get();
                            let flat = list.iter().find(|f| f.id == fid);
                            let flat_name = flat
                                .map(|f| f.name.to_uppercase())
                                .unwrap_or_else(|| fid.to_uppercase());
                            let flat_kind = flat.map(|f| f.kind.clone()).unwrap_or_default();
                            let n = flat.map(|f| f.coords.len()).unwrap_or(0);
                            let land_hue = if fid.contains("avalon") {
                                "var(--cyber-orange)"
                            } else if fid.contains("sinwood") {
                                "var(--cyber-green)"
                            } else {
                                "var(--cyber-cyan)"
                            };
                            let leased_tag = if leased.get().iter().any(|x| x == &fid) {
                                " · LEASED"
                            } else {
                                ""
                            };

                            // —— robot / worker ——
                            let rid = selected_fleet.get();
                            let (bot_name, bot_kind, bot_role, bot_status, bot_owned) = rid
                                .as_ref()
                                .and_then(|id| {
                                    stock_fleet(id).map(|f| {
                                        (
                                            f.name.to_string(),
                                            f.kind.to_string(),
                                            f.role.to_string(),
                                            f.status.to_string(),
                                            false,
                                        )
                                    }).or_else(|| {
                                        owned.get().into_iter().find(|r| &r.id == id).map(|r| {
                                            (r.name, r.kind, r.role, "idle".into(), true)
                                        })
                                    })
                                })
                                .unwrap_or_else(|| {
                                    ("—".into(), "none".into(), "no unit selected".into(), "—".into(), false)
                                });
                            let is_worker = bot_kind == "worker";
                            let bot_hue = if bot_kind == "none" {
                                "#444"
                            } else if is_worker {
                                "var(--cyber-cyan)"
                            } else {
                                "var(--cyber-orange)"
                            };
                            let bot_cls = if bot_kind == "none" {
                                "bot-figure empty"
                            } else if is_worker {
                                "bot-figure worker"
                            } else {
                                "bot-figure machine"
                            };

                            view! {
                                <div class="render-pair">
                                    // LAND
                                    <div class="prism-scene">
                                        <div class="render-tag">"FLAT"</div>
                                        <div class="prism" style:--prism-color=land_hue>
                                            <div class="prism-face prism-top"></div>
                                            <div class="prism-face prism-front"></div>
                                            <div class="prism-face prism-side"></div>
                                        </div>
                                        <div class="prism-meta">
                                            <div class="prism-name">{format!("{flat_name}{leased_tag}")}</div>
                                            <div class="prism-sub">{format!("{flat_kind} · {n} verts")}</div>
                                            <div class="prism-sub">"Gesing hold"</div>
                                        </div>
                                    </div>
                                    // ROBOT
                                    <div class="prism-scene bot-scene">
                                        <div class="render-tag">"ROBOT"</div>
                                        <div class=bot_cls style:--bot-color=bot_hue>
                                            <div class="bot-head"></div>
                                            <div class="bot-body">
                                                <div class="bot-chest"></div>
                                                <div class="bot-arm bot-arm-l"></div>
                                                <div class="bot-arm bot-arm-r"></div>
                                            </div>
                                            <div class="bot-legs">
                                                <div class="bot-leg"></div>
                                                <div class="bot-leg"></div>
                                            </div>
                                            <div class="bot-glow"></div>
                                        </div>
                                        <div class="prism-meta">
                                            <div class="prism-name" style:color=bot_hue>{bot_name}</div>
                                            <div class="prism-sub">{bot_role}</div>
                                            <div class="prism-sub">
                                                {format!(
                                                    "{} · {}",
                                                    bot_kind.to_uppercase(),
                                                    if bot_owned { "OWNED" } else { &bot_status.to_uppercase() }
                                                )}
                                            </div>
                                        </div>
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
                                    .and_then(|id| {
                                        stock_fleet(&id)
                                            .map(|f| f.name.to_string())
                                            .or_else(|| {
                                                owned
                                                    .get_untracked()
                                                    .into_iter()
                                                    .find(|r| r.id == id)
                                                    .map(|r| r.name)
                                            })
                                    })
                                    .unwrap_or_else(|| "—".into())}
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
                                if stock_fleet(&fleet)
                                    .is_some_and(|f| f.status == "offline" || f.phase > 0)
                                {
                                    return;
                                }
                                let fleet_name = stock_fleet(&fleet)
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
                                                "split" => "ii-act split",
                                                "merge" => "ii-act merge",
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
                Sheet::LeaseLand => view! {
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
                                {move || flats.get().into_iter().map(|f| {
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
                }.into_any(),
                Sheet::SplitLand => view! {
                    <div class="cyberia-sheet-backdrop" on:click=move |_| sheet.set(Sheet::None)>
                        <div class="cyberia-sheet" on:click=move |ev| ev.stop_propagation()>
                            <div class="sheet-h">
                                <span class="panel-kicker">"SPLIT LAND"</span>
                                <button class="sheet-x" on:click=move |_| sheet.set(Sheet::None)>"✕"</button>
                            </div>
                            <p class="sheet-note">
                                "Split the selected flat into west / east halves (local geometry). Soft3 intent — no closed cadastral backend."
                            </p>
                            <div class="sheet-catalog">
                                {move || flats.get().into_iter().map(|f| {
                                    let id = f.id.clone();
                                    let id2 = id.clone();
                                    let name = f.name.to_uppercase();
                                    view! {
                                        <button
                                            class=move || {
                                                let sel = selected_flat.get().as_deref() == Some(id.as_str());
                                                format!("fleet-card catalog flat-pick{}", if sel { " sel" } else { "" })
                                            }
                                            on:click=move |_| selected_flat.set(Some(id2.clone()))
                                        >
                                            <div class="fleet-top">
                                                <span class="fleet-name">{name}</span>
                                                <span class="fleet-status" style:color="var(--cyber-cyan)">"SPLIT?"</span>
                                            </div>
                                            <div class="fleet-role">"click to select · confirm below"</div>
                                        </button>
                                    }
                                }).collect_view()}
                            </div>
                            <button
                                class="intent-commit split"
                                on:click=move |_| {
                                    let Some(fid) = selected_flat.get() else { return };
                                    let list = flats.get_untracked();
                                    let Some(src) = list.iter().find(|f| f.id == fid).cloned() else { return };
                                    if src.coords.len() < 3 { return; }
                                    let (a, b) = split_flat(&src);
                                    let a_id = a.id.clone();
                                    flats.update(|v| {
                                        v.retain(|f| f.id != fid);
                                        v.push(a);
                                        v.push(b);
                                    });
                                    // leases on parent drop — sub-flats are open
                                    leased.update(|v| v.retain(|x| x != &fid));
                                    selected_flat.set(Some(a_id));
                                    let id = next_id.get();
                                    next_id.set(id + 1);
                                    intents.update(|q| {
                                        q.insert(0, Intent {
                                            id,
                                            fleet: "YOU".into(),
                                            action: "split".into(),
                                            flat: fid,
                                        });
                                    });
                                    sheet.set(Sheet::None);
                                }
                            >
                                "CONFIRM SPLIT"
                            </button>
                        </div>
                    </div>
                }.into_any(),
                Sheet::MergeLand => view! {
                    <div class="cyberia-sheet-backdrop" on:click=move |_| {
                        merge_pick.set(None);
                        sheet.set(Sheet::None);
                    }>
                        <div class="cyberia-sheet" on:click=move |ev| ev.stop_propagation()>
                            <div class="sheet-h">
                                <span class="panel-kicker">"MERGE LAND"</span>
                                <button class="sheet-x" on:click=move |_| {
                                    merge_pick.set(None);
                                    sheet.set(Sheet::None);
                                }>"✕"</button>
                            </div>
                            <p class="sheet-note">
                                "Pick two flats: first is primary (A), second is B. Merge into one hold. Soft3 local intent."
                            </p>
                            <div class="intent-row" style="margin-bottom:8px;">
                                <span class="intent-k">"A"</span>
                                <span class="intent-v">
                                    {move || selected_flat.get().unwrap_or_else(|| "—".into()).to_uppercase()}
                                </span>
                            </div>
                            <div class="intent-row" style="margin-bottom:10px;">
                                <span class="intent-k">"B"</span>
                                <span class="intent-v">
                                    {move || merge_pick.get().unwrap_or_else(|| "—".into()).to_uppercase()}
                                </span>
                            </div>
                            <div class="sheet-catalog">
                                {move || flats.get().into_iter().map(|f| {
                                    let id = f.id.clone();
                                    let id2 = id.clone();
                                    let name = f.name.to_uppercase();
                                    view! {
                                        <button
                                            class=move || {
                                                let a = selected_flat.get().as_deref() == Some(id.as_str());
                                                let b = merge_pick.get().as_deref() == Some(id.as_str());
                                                format!(
                                                    "fleet-card catalog flat-pick{}{}",
                                                    if a { " sel" } else { "" },
                                                    if b { " merge-b" } else { "" },
                                                )
                                            }
                                            on:click=move |_| {
                                                let cur_a = selected_flat.get_untracked();
                                                if cur_a.as_deref() == Some(id2.as_str()) {
                                                    return;
                                                }
                                                if cur_a.is_none() {
                                                    selected_flat.set(Some(id2.clone()));
                                                } else if merge_pick.get_untracked().as_deref() == Some(id2.as_str()) {
                                                    merge_pick.set(None);
                                                } else {
                                                    merge_pick.set(Some(id2.clone()));
                                                }
                                            }
                                        >
                                            <div class="fleet-top">
                                                <span class="fleet-name">{name}</span>
                                                <span class="fleet-status" style:color="var(--cyber-magenta)">"MERGE"</span>
                                            </div>
                                            <div class="fleet-role">"1st click = A · 2nd = B"</div>
                                        </button>
                                    }
                                }).collect_view()}
                            </div>
                            <button
                                class="intent-commit merge"
                                on:click=move |_| {
                                    let Some(a_id) = selected_flat.get() else { return };
                                    let Some(b_id) = merge_pick.get() else { return };
                                    if a_id == b_id { return; }
                                    let list = flats.get_untracked();
                                    let Some(a) = list.iter().find(|f| f.id == a_id).cloned() else { return };
                                    let Some(b) = list.iter().find(|f| f.id == b_id).cloned() else { return };
                                    let merged = merge_flats(&a, &b);
                                    let mid = merged.id.clone();
                                    flats.update(|v| {
                                        v.retain(|f| f.id != a_id && f.id != b_id);
                                        v.push(merged);
                                    });
                                    leased.update(|v| {
                                        v.retain(|x| x != &a_id && x != &b_id);
                                    });
                                    selected_flat.set(Some(mid.clone()));
                                    merge_pick.set(None);
                                    let id = next_id.get();
                                    next_id.set(id + 1);
                                    intents.update(|q| {
                                        q.insert(0, Intent {
                                            id,
                                            fleet: "YOU".into(),
                                            action: "merge".into(),
                                            flat: format!("{a_id}+{b_id}"),
                                        });
                                    });
                                    sheet.set(Sheet::None);
                                }
                            >
                                "CONFIRM MERGE"
                            </button>
                        </div>
                    </div>
                }.into_any(),
            }}

            <div class="search-dock cyberia-dock">
                <span class="dock-count">
                    {move || {
                        let n = intents.get().len();
                        let m = flats.get().len();
                        let o = owned.get().len();
                        let l = leased.get().len();
                        format!(
                            "{n} intents · {m} flats · {}w+{}m+{o} fleets · {l} leases",
                            WORKERS.len(),
                            MACHINES.len(),
                        )
                    }}
                </span>
                <div class="cyberia-cta-bar dock-ctas">
                    <button class="cta-btn cta-buy cta-lg" on:click=move |_| sheet.set(Sheet::BuyRobot)>
                        <span class="cta-ico">"🤖"</span>
                        <span class="cta-copy">
                            <span class="cta-title">"BUY ROBOT"</span>
                            <span class="cta-sub">"add a unit to your fleet"</span>
                        </span>
                    </button>
                    <button class="cta-btn cta-lease cta-lg" on:click=move |_| sheet.set(Sheet::LeaseLand)>
                        <span class="cta-ico">"🗺"</span>
                        <span class="cta-copy">
                            <span class="cta-title">"LEASE LAND"</span>
                            <span class="cta-sub">"hold a phase-0 flat"</span>
                        </span>
                    </button>
                    <button class="cta-btn cta-split cta-lg cta-bold" on:click=move |_| sheet.set(Sheet::SplitLand)>
                        <span class="cta-ico">"✂"</span>
                        <span class="cta-copy">
                            <span class="cta-title">"SPLIT LAND"</span>
                            <span class="cta-sub">"cut flat west / east"</span>
                        </span>
                    </button>
                    <button class="cta-btn cta-merge cta-lg cta-bold" on:click=move |_| {
                        merge_pick.set(None);
                        sheet.set(Sheet::MergeLand);
                    }>
                        <span class="cta-ico">"⧉"</span>
                        <span class="cta-copy">
                            <span class="cta-title">"MERGE LAND"</span>
                            <span class="cta-sub">"join two flats"</span>
                        </span>
                    </button>
                </div>
                <a href="https://x.com/cyberiacap" target="_blank" rel="noopener" class="dock-credit">
                    "🏴 a "<span style="color: var(--cyber-green);">"cyberia"</span>" project"
                </a>
            </div>
        </div>
    }
}
