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
use wasm_bindgen::JsCast;

const MAP_JSON: &str = include_str!("cyberia_map.json");

#[derive(Clone, Debug, Deserialize)]
struct MapData {
    site: String,
    center: [f64; 2],
    bbox: BBox,
    #[serde(default)]
    stats: MapStats,
    /// All interactive land plots (from KML `plots` folder).
    phase0: Vec<Flat>,
    #[serde(default)]
    districts: Vec<Flat>,
    places: Vec<Flat>,
    source: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct MapStats {
    #[serde(default)]
    plot_count: u32,
    #[serde(default)]
    plot_ha: f64,
    #[serde(default)]
    district_ha: f64,
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
    #[serde(default)]
    zone: String,
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

/// Open ring (no duplicate close point).
fn open_ring(coords: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut c = coords.to_vec();
    if c.len() > 1 && c.first() == c.last() {
        c.pop();
    }
    c
}

fn close_ring(mut c: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    if c.len() >= 2 && c.first() != c.last() {
        let first = c[0];
        c.push(first);
    }
    c
}

fn poly_bbox(coords: &[[f64; 2]]) -> (f64, f64, f64, f64) {
    let ring = open_ring(coords);
    let mut min_lon = f64::INFINITY;
    let mut max_lon = f64::NEG_INFINITY;
    let mut min_lat = f64::INFINITY;
    let mut max_lat = f64::NEG_INFINITY;
    for p in &ring {
        min_lon = min_lon.min(p[0]);
        max_lon = max_lon.max(p[0]);
        min_lat = min_lat.min(p[1]);
        max_lat = max_lat.max(p[1]);
    }
    (min_lon, max_lon, min_lat, max_lat)
}

/// Planar area in m² via equirectangular projection around centroid (WGS84).
fn area_m2(coords: &[[f64; 2]]) -> f64 {
    let ring = open_ring(coords);
    if ring.len() < 3 {
        return 0.0;
    }
    let lat0 = ring.iter().map(|c| c[1]).sum::<f64>() / ring.len() as f64;
    let lon0 = ring.iter().map(|c| c[0]).sum::<f64>() / ring.len() as f64;
    const R: f64 = 6_378_137.0;
    let cos_lat = lat0.to_radians().cos();
    let to_xy = |lon: f64, lat: f64| -> (f64, f64) {
        let x = (lon - lon0).to_radians() * R * cos_lat;
        let y = (lat - lat0).to_radians() * R;
        (x, y)
    };
    let mut a = 0.0;
    for i in 0..ring.len() {
        let (x1, y1) = to_xy(ring[i][0], ring[i][1]);
        let j = (i + 1) % ring.len();
        let (x2, y2) = to_xy(ring[j][0], ring[j][1]);
        a += x1 * y2 - x2 * y1;
    }
    (a.abs()) * 0.5
}

fn fmt_area_m2(m2: f64) -> String {
    if m2 >= 10_000.0 {
        format!("{:.2} ha ({:.0} m²)", m2 / 10_000.0, m2)
    } else if m2 >= 100.0 {
        format!("{:.0} m²", m2)
    } else {
        format!("{:.1} m²", m2)
    }
}

/// Bbox span along axis in meters (axis 0 = lon width, 1 = lat height).
fn bbox_span_m(coords: &[[f64; 2]], axis: usize) -> f64 {
    let (min_lon, max_lon, min_lat, max_lat) = poly_bbox(coords);
    const R: f64 = 6_378_137.0;
    let mid_lat = (min_lat + max_lat) * 0.5;
    if axis == 0 {
        (max_lon - min_lon).to_radians() * R * mid_lat.to_radians().cos()
    } else {
        (max_lat - min_lat).to_radians() * R
    }
}

/// Sutherland–Hodgman clip against half-plane on lon (axis=0) or lat (axis=1).
/// `keep_le`: keep points with value ≤ t (west / south side).
fn clip_half_plane(poly: &[[f64; 2]], axis: usize, t: f64, keep_le: bool) -> Vec<[f64; 2]> {
    let ring = open_ring(poly);
    if ring.is_empty() {
        return vec![];
    }
    let inside = |p: [f64; 2]| -> bool {
        let v = p[axis];
        if keep_le {
            v <= t + 1e-12
        } else {
            v >= t - 1e-12
        }
    };
    let intersect = |a: [f64; 2], b: [f64; 2]| -> [f64; 2] {
        let da = a[axis] - t;
        let db = b[axis] - t;
        let denom = da - db;
        let u = if denom.abs() < 1e-15 { 0.5 } else { da / denom };
        [a[0] + u * (b[0] - a[0]), a[1] + u * (b[1] - a[1])]
    };
    let mut out: Vec<[f64; 2]> = Vec::new();
    let n = ring.len();
    for i in 0..n {
        let cur = ring[i];
        let prev = ring[(i + n - 1) % n];
        let cur_in = inside(cur);
        let prev_in = inside(prev);
        if cur_in {
            if !prev_in {
                out.push(intersect(prev, cur));
            }
            out.push(cur);
        } else if prev_in {
            out.push(intersect(prev, cur));
        }
    }
    // dedupe consecutive near-equal points
    let mut cleaned: Vec<[f64; 2]> = Vec::new();
    for p in out {
        if cleaned
            .last()
            .map(|q| (q[0] - p[0]).abs() > 1e-10 || (q[1] - p[1]).abs() > 1e-10)
            .unwrap_or(true)
        {
            cleaned.push(p);
        }
    }
    close_ring(cleaned)
}

/// User-controlled split: axis 0 = vertical cut (E/W), 1 = horizontal (N/S).
/// `ratio` in (0,1) is cut position across the polygon bbox (0 = west/south).
fn split_flat_at(flat: &Flat, axis: usize, ratio: f64) -> Option<(Flat, Flat, f64)> {
    let r = ratio.clamp(0.05, 0.95);
    let (min_lon, max_lon, min_lat, max_lat) = poly_bbox(&flat.coords);
    let t = if axis == 0 {
        min_lon + r * (max_lon - min_lon)
    } else {
        min_lat + r * (max_lat - min_lat)
    };
    let a_coords = clip_half_plane(&flat.coords, axis, t, true);
    let b_coords = clip_half_plane(&flat.coords, axis, t, false);
    // closed ring needs ≥4 points (triangle + close)
    if a_coords.len() < 4 || b_coords.len() < 4 {
        return None;
    }
    if area_m2(&a_coords) < 1.0 || area_m2(&b_coords) < 1.0 {
        return None;
    }
    let a = Flat {
        id: format!("{}-a", flat.id),
        name: format!("{}-a", flat.name),
        kind: flat.kind.clone(),
        phase: flat.phase,
        geom: "polygon".into(),
        coords: a_coords,
        zone: flat.zone.clone(),
    };
    let b = Flat {
        id: format!("{}-b", flat.id),
        name: format!("{}-b", flat.name),
        kind: flat.kind.clone(),
        phase: flat.phase,
        geom: "polygon".into(),
        coords: b_coords,
        zone: flat.zone.clone(),
    };
    Some((a, b, t))
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
        zone: if a.zone == b.zone {
            a.zone.clone()
        } else {
            "mixed".into()
        },
    }
}

fn zone_key(flat: &Flat) -> &str {
    if !flat.zone.is_empty() {
        return flat.zone.as_str();
    }
    // fallback: prefix of id/name
    let s = if flat.id.is_empty() {
        flat.name.as_str()
    } else {
        flat.id.as_str()
    };
    s
}

fn flat_fill(zone_or_id: &str, selected: bool) -> &'static str {
    let z = zone_or_id.to_lowercase();
    let (hi, lo) = if z.contains("avalon") {
        ("rgba(255,102,0,0.48)", "rgba(255,102,0,0.16)")
    } else if z.contains("sinwood") {
        ("rgba(0,255,65,0.45)", "rgba(0,255,65,0.14)")
    } else if z.contains("bridge") {
        ("rgba(0,229,255,0.42)", "rgba(0,229,255,0.12)")
    } else if z.contains("core") {
        ("rgba(255,215,0,0.42)", "rgba(255,215,0,0.12)")
    } else if z.contains("ether") {
        ("rgba(153,69,255,0.42)", "rgba(153,69,255,0.12)")
    } else if z.contains("asgard") {
        ("rgba(255,0,64,0.38)", "rgba(255,0,64,0.10)")
    } else if z.contains("edem") || z.contains("canyon") {
        ("rgba(0,255,200,0.38)", "rgba(0,255,200,0.10)")
    } else {
        ("rgba(160,160,160,0.35)", "rgba(160,160,160,0.10)")
    };
    if selected {
        hi
    } else {
        lo
    }
}

fn flat_stroke_col(zone_or_id: &str, selected: bool) -> &'static str {
    let z = zone_or_id.to_lowercase();
    if selected {
        return "#ffffff";
    }
    if z.contains("avalon") {
        "rgba(255,102,0,0.65)"
    } else if z.contains("sinwood") {
        "rgba(0,255,65,0.55)"
    } else if z.contains("bridge") {
        "rgba(0,229,255,0.55)"
    } else if z.contains("core") {
        "rgba(255,215,0,0.55)"
    } else if z.contains("ether") {
        "rgba(153,69,255,0.55)"
    } else if z.contains("asgard") {
        "rgba(255,0,64,0.5)"
    } else {
        "rgba(140,140,140,0.45)"
    }
}

fn zone_hover_fill(zone_or_id: &str) -> &'static str {
    let z = zone_or_id.to_lowercase();
    if z.contains("avalon") {
        "rgba(255,102,0,0.40)"
    } else if z.contains("sinwood") {
        "rgba(0,255,65,0.40)"
    } else if z.contains("bridge") {
        "rgba(0,229,255,0.38)"
    } else if z.contains("core") {
        "rgba(255,215,0,0.38)"
    } else if z.contains("ether") {
        "rgba(153,69,255,0.38)"
    } else {
        "rgba(200,200,200,0.28)"
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

    let selected_flat = RwSignal::new(
        map.phase0
            .first()
            .map(|f| f.id.clone())
            .or_else(|| Some("sinwood".into())),
    );
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
                                                    // split controls: axis 0 = E/W (vertical line), 1 = N/S (horizontal line)
    let split_axis = RwSignal::new(0usize);
    let split_ratio = RwSignal::new(0.50_f64); // 0..1 across bbox

    // game map camera — static viewport, zoom/pan never scroll the panel
    let map_zoom = RwSignal::new(1.0_f64);
    let map_pan = RwSignal::new((0.0_f64, 0.0_f64)); // px in wrap space
    let map_hover = RwSignal::new(None::<(String, String, f64)>); // id, label, m2
    let map_dragging = RwSignal::new(false);
    let map_drag_last = RwSignal::new((0.0_f64, 0.0_f64));
    let map_wrap_ref = NodeRef::<leptos::html::Div>::new();

    let zoom_by = move |factor: f64| {
        map_zoom.update(|z| *z = (*z * factor).clamp(0.6, 8.0));
    };
    let zoom_at = move |factor: f64, cx: f64, cy: f64| {
        // keep world point under (cx,cy) stable while scaling
        let z0 = map_zoom.get_untracked();
        let z1 = (z0 * factor).clamp(0.6, 8.0);
        if (z1 - z0).abs() < 1e-9 {
            return;
        }
        let (px, py) = map_pan.get_untracked();
        // point in world = (screen - pan) / z
        let wx = (cx - px) / z0;
        let wy = (cy - py) / z0;
        let npx = cx - wx * z1;
        let npy = cy - wy * z1;
        map_zoom.set(z1);
        map_pan.set((npx, npy));
    };
    let pan_by = move |dx: f64, dy: f64| {
        map_pan.update(|(x, y)| {
            *x += dx;
            *y += dy;
        });
    };
    let reset_cam = move || {
        map_zoom.set(1.0);
        map_pan.set((0.0, 0.0));
    };

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
                    {move || {
                        let n = flats.get().len();
                        format!("PHASE 0 · {n} PLOTS · GESING")
                    }}
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

                // CENTER — FLATS MAP (game camera: static viewport, wheel/keys zoom, drag pan)
                <section class="cyberia-panel cyberia-flats">
                    <div class="cyberia-panel-h">
                        <span class="panel-kicker">"FLATS"</span>
                        <span class="panel-sub">
                            {format!(
                                "{} plots · {:.1} ha plots · {:.0} ha site · scroll zoom",
                                map.stats.plot_count.max(map.phase0.len() as u32),
                                map.stats.plot_ha,
                                map.stats.district_ha,
                            )}
                        </span>
                        <span class="map-zoom-readout">
                            {move || format!("×{:.1}", map_zoom.get())}
                        </span>
                    </div>
                    <div
                        class="flat-map-wrap game-map"
                        node_ref=map_wrap_ref
                        tabindex="0"
                        on:wheel=move |ev| {
                            ev.prevent_default();
                            ev.stop_propagation();
                            let dy = ev.delta_y();
                            if dy == 0.0 { return; }
                            let factor = if dy < 0.0 { 1.12 } else { 1.0 / 1.12 };
                            if let Some(el) = map_wrap_ref.get_untracked() {
                                let rect = el.get_bounding_client_rect();
                                let cx = ev.client_x() as f64 - rect.left();
                                let cy = ev.client_y() as f64 - rect.top();
                                zoom_at(factor, cx, cy);
                            } else {
                                zoom_by(factor);
                            }
                        }
                        on:keydown=move |ev| {
                            let key = ev.key();
                            let step = 36.0;
                            match key.as_str() {
                                "ArrowLeft" | "a" | "A" => { ev.prevent_default(); pan_by(step, 0.0); }
                                "ArrowRight" | "d" | "D" => { ev.prevent_default(); pan_by(-step, 0.0); }
                                "ArrowUp" | "w" | "W" => { ev.prevent_default(); pan_by(0.0, step); }
                                "ArrowDown" | "s" | "S" => { ev.prevent_default(); pan_by(0.0, -step); }
                                "+" | "=" => { ev.prevent_default(); zoom_by(1.15); }
                                "-" | "_" => { ev.prevent_default(); zoom_by(1.0 / 1.15); }
                                "0" => { ev.prevent_default(); reset_cam(); }
                                _ => {}
                            }
                        }
                        on:pointerdown=move |ev| {
                            // only pan with primary button on empty space / background
                            if ev.button() != 0 { return; }
                            let target = ev.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok());
                            let on_zone = target
                                .as_ref()
                                .and_then(|el| el.closest(".flat-poly, .flat-path, .place-dot").ok().flatten())
                                .is_some();
                            if on_zone {
                                return; // zone click selects, no drag
                            }
                            map_dragging.set(true);
                            map_drag_last.set((ev.client_x() as f64, ev.client_y() as f64));
                            if let Some(el) = map_wrap_ref.get_untracked() {
                                let _ = el.set_pointer_capture(ev.pointer_id());
                            }
                        }
                        on:pointermove=move |ev| {
                            if !map_dragging.get_untracked() { return; }
                            let (lx, ly) = map_drag_last.get_untracked();
                            let cx = ev.client_x() as f64;
                            let cy = ev.client_y() as f64;
                            pan_by(cx - lx, cy - ly);
                            map_drag_last.set((cx, cy));
                        }
                        on:pointerup=move |_| map_dragging.set(false)
                        on:pointercancel=move |_| map_dragging.set(false)
                        on:pointerleave=move |_| {
                            if !map_dragging.get_untracked() {
                                map_hover.set(None);
                            }
                        }
                    >
                        // left zoom rail (game-style)
                        <div class="map-zoom-rail" aria-label="map zoom">
                            <button class="map-cam-btn" type="button" title="zoom in"
                                on:click=move |ev| { ev.stop_propagation(); zoom_by(1.2); }
                            >"+"</button>
                            <button class="map-cam-btn" type="button" title="zoom out"
                                on:click=move |ev| { ev.stop_propagation(); zoom_by(1.0 / 1.2); }
                            >"−"</button>
                            <button class="map-cam-btn" type="button" title="reset view"
                                on:click=move |ev| { ev.stop_propagation(); reset_cam(); }
                            >"⌂"</button>
                            <div class="map-cam-sep"></div>
                            <button class="map-cam-btn" type="button" title="pan left"
                                on:click=move |ev| { ev.stop_propagation(); pan_by(48.0, 0.0); }
                            >"←"</button>
                            <button class="map-cam-btn" type="button" title="pan right"
                                on:click=move |ev| { ev.stop_propagation(); pan_by(-48.0, 0.0); }
                            >"→"</button>
                            <button class="map-cam-btn" type="button" title="pan up"
                                on:click=move |ev| { ev.stop_propagation(); pan_by(0.0, 48.0); }
                            >"↑"</button>
                            <button class="map-cam-btn" type="button" title="pan down"
                                on:click=move |ev| { ev.stop_propagation(); pan_by(0.0, -48.0); }
                            >"↓"</button>
                        </div>

                        {
                            let m = map_for_svg.clone();
                            let m_places = map_for_svg.clone();
                            let m_cap = map_for_svg.clone();
                            let m_districts = map_for_svg.clone();
                            const W: f64 = 960.0;
                            const H: f64 = 720.0;
                            const PAD: f64 = 20.0;
                            view! {
                                <div
                                    class="map-world"
                                    style:transform=move || {
                                        let z = map_zoom.get();
                                        let (x, y) = map_pan.get();
                                        format!("translate({x}px, {y}px) scale({z})")
                                    }
                                >
                                <svg class="flat-map" viewBox=format!("0 0 {W} {H}") preserveAspectRatio="xMidYMid meet">
                                    <rect x="0" y="0" width=W height=H fill="#070707" />
                                    {(0..12).map(|i| {
                                        let x = PAD + i as f64 * (W - 2.0*PAD) / 11.0;
                                        let y2 = H - PAD;
                                        view! {
                                            <line x1=x y1=PAD x2=x y2=y2 stroke="#121212" stroke-width="1" />
                                        }
                                    }).collect_view()}
                                    {(0..9).map(|i| {
                                        let y = PAD + i as f64 * (H - 2.0*PAD) / 8.0;
                                        let x2 = W - PAD;
                                        view! {
                                            <line x1=PAD y1=y x2=x2 y2=y stroke="#121212" stroke-width="1" />
                                        }
                                    }).collect_view()}

                                    // district outlines — full site ~37 ha envelope
                                    {m_districts.districts.iter().map(|d| {
                                        let path = poly_path(&d.coords, &m_districts.bbox, W, H, PAD);
                                        let (clon, clat) = centroid(&d.coords);
                                        let (lx, ly) = project(clon, clat, &m_districts.bbox, W, H, PAD);
                                        let label = d.name.to_uppercase();
                                        view! {
                                            <g class="district-poly" pointer-events="none">
                                                <path
                                                    d=path
                                                    fill="rgba(255,255,255,0.015)"
                                                    stroke="rgba(255,255,255,0.12)"
                                                    stroke-width="1"
                                                    stroke-dasharray="4 3"
                                                />
                                                <text x=lx y=ly text-anchor="middle" class="district-label">{label}</text>
                                            </g>
                                        }
                                    }).collect_view()}

                                    // ALL plots (interactive)
                                    {move || {
                                        let list = flats.get();
                                        list.into_iter().map(|flat| {
                                            let id_click = flat.id.clone();
                                            let id_fill = flat.id.clone();
                                            let id_stroke = flat.id.clone();
                                            let id_sw = flat.id.clone();
                                            let id_cls = flat.id.clone();
                                            let id_lab = flat.id.clone();
                                            let id_hov = flat.id.clone();
                                            let id_hov2 = flat.id.clone();
                                            let zkey = zone_key(&flat).to_string();
                                            let z_fill = zkey.clone();
                                            let z_stroke = zkey.clone();
                                            let z_hov = zkey.clone();
                                            let label_hov = flat.name.to_uppercase();
                                            let area = area_m2(&flat.coords);
                                            let d = poly_path(&flat.coords, &m.bbox, W, H, PAD);
                                            let (clon, clat) = centroid(&flat.coords);
                                            let (lx, ly) = project(clon, clat, &m.bbox, W, H, PAD);
                                            // labels only when zoomed or selected/hovered — keep default clean
                                            let base_label = flat.name.to_uppercase();
                                            let show_label_always = false;
                                            view! {
                                                <g class="flat-poly"
                                                    on:click=move |ev| {
                                                        ev.stop_propagation();
                                                        selected_flat.set(Some(id_click.clone()));
                                                    }
                                                    on:pointerenter=move |_| {
                                                        map_hover.set(Some((id_hov.clone(), label_hov.clone(), area)));
                                                    }
                                                    on:pointerleave=move |_| {
                                                        map_hover.update(|h| {
                                                            if h.as_ref().map(|t| t.0.as_str()) == Some(id_hov2.as_str()) {
                                                                *h = None;
                                                            }
                                                        });
                                                    }
                                                >
                                                    <path
                                                        d=d
                                                        fill=move || {
                                                            let sel = selected_flat.get().as_deref() == Some(id_fill.as_str());
                                                            let hov = map_hover.get().as_ref().map(|t| t.0.as_str()) == Some(id_fill.as_str());
                                                            if hov && !sel {
                                                                zone_hover_fill(&z_hov).to_string()
                                                            } else {
                                                                flat_fill(&z_fill, sel).to_string()
                                                            }
                                                        }
                                                        stroke=move || {
                                                            let sel = selected_flat.get().as_deref() == Some(id_stroke.as_str());
                                                            let hov = map_hover.get().as_ref().map(|t| t.0.as_str()) == Some(id_stroke.as_str());
                                                            if hov || sel {
                                                                "#ffffff".into()
                                                            } else {
                                                                flat_stroke_col(&z_stroke, sel).to_string()
                                                            }
                                                        }
                                                        stroke-width=move || {
                                                            let sel = selected_flat.get().as_deref() == Some(id_sw.as_str());
                                                            let hov = map_hover.get().as_ref().map(|t| t.0.as_str()) == Some(id_sw.as_str());
                                                            if sel { "2.2" } else if hov { "1.8" } else { "0.9" }
                                                        }
                                                        class=move || {
                                                            let sel = selected_flat.get().as_deref() == Some(id_cls.as_str());
                                                            let hov = map_hover.get().as_ref().map(|t| t.0.as_str()) == Some(id_cls.as_str());
                                                            format!(
                                                                "flat-path{}{}",
                                                                if sel { " is-sel" } else { "" },
                                                                if hov { " is-hov" } else { "" },
                                                            )
                                                        }
                                                    />
                                                    {move || {
                                                        let sel = selected_flat.get().as_deref() == Some(id_lab.as_str());
                                                        let hov = map_hover.get().as_ref().map(|t| t.0.as_str()) == Some(id_lab.as_str());
                                                        let zoomed = map_zoom.get() >= 1.8;
                                                        if !(show_label_always || sel || hov || zoomed) {
                                                            return view! { <g></g> }.into_any();
                                                        }
                                                        let text = if leased.get().iter().any(|x| x == &id_lab) {
                                                            format!("{base_label} · LEASED")
                                                        } else {
                                                            base_label.clone()
                                                        };
                                                        view! {
                                                            <text x=lx y=ly text-anchor="middle" class="flat-label">{text}</text>
                                                        }.into_any()
                                                    }}
                                                </g>
                                            }
                                        }).collect_view()
                                    }}

                                    {m_places.places.iter().map(|p| {
                                        if p.coords.is_empty() { return view! { <g></g> }.into_any(); }
                                        let (x, y) = project(p.coords[0][0], p.coords[0][1], &m_places.bbox, W, H, PAD);
                                        let name = p.name.clone();
                                        let ty = y - 6.0;
                                        view! {
                                            <g class="place-dot">
                                                <circle cx=x cy=y r="2.5" fill="#3a3a3a" stroke="#666" stroke-width="0.8" />
                                                <text x=x y=ty text-anchor="middle" class="place-label">{name}</text>
                                            </g>
                                        }.into_any()
                                    }).collect_view()}

                                    <text x=PAD y={H - 8.0} class="map-caption">
                                        {format!(
                                            "N ↑  ·  {} plots  ·  {:.1} ha plots / {:.0} ha site  ·  {}",
                                            m_cap.stats.plot_count,
                                            m_cap.stats.plot_ha,
                                            m_cap.stats.district_ha,
                                            m_cap.source
                                        )}
                                    </text>
                                </svg>
                                </div>
                            }
                        }

                        // hover tooltip (game HUD)
                        {move || map_hover.get().map(|(id, label, m2)| {
                            let leased_tag = if leased.get().iter().any(|x| x == &id) { " · LEASED" } else { "" };
                            let sel = selected_flat.get().as_deref() == Some(id.as_str());
                            view! {
                                <div class="map-tooltip">
                                    <div class="mt-name">{format!("{label}{leased_tag}")}</div>
                                    <div class="mt-area">{fmt_area_m2(m2)}</div>
                                    <div class="mt-hint">{if sel { "SELECTED" } else { "click to select" }}</div>
                                </div>
                            }
                        })}

                        <div class="map-hud-hint">
                            "scroll zoom · drag pan · ←→↑↓ pan · +/− zoom · 0 reset"
                        </div>
                    </div>
                    <div class="flat-legend">
                        <span class="leg sin">"SINWOOD"</span>
                        <span class="leg avalon">"AVALON"</span>
                        <span class="leg bridge">"BRIDGE"</span>
                        <span class="leg core">"CORE"</span>
                        <span class="leg ether">"ETHERLAND"</span>
                        <span class="leg dim">"dashed = district · hover plot"</span>
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
                                                {
                                                    let status_lbl = if bot_owned {
                                                        "OWNED".to_string()
                                                    } else {
                                                        bot_status.to_uppercase()
                                                    };
                                                    format!("{} · {}", bot_kind.to_uppercase(), status_lbl)
                                                }
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
                        <div class="cyberia-sheet sheet-wide" on:click=move |ev| ev.stop_propagation()>
                            <div class="sheet-h">
                                <span class="panel-kicker">"SPLIT LAND"</span>
                                <button class="sheet-x" on:click=move |_| sheet.set(Sheet::None)>"✕"</button>
                            </div>
                            <p class="sheet-note">
                                "Pick a flat, set cut direction and position. Areas are geodesic-approx m² (equirectangular). Soft3 local intent."
                            </p>

                            <div class="split-pick-label">"1 · SELECT FLAT"</div>
                            <div class="sheet-catalog sheet-catalog-compact">
                                {move || flats.get().into_iter().map(|f| {
                                    let id = f.id.clone();
                                    let id2 = id.clone();
                                    let name = f.name.to_uppercase();
                                    let a = area_m2(&f.coords);
                                    view! {
                                        <button
                                            class=move || {
                                                let sel = selected_flat.get().as_deref() == Some(id.as_str());
                                                format!("fleet-card catalog flat-pick{}", if sel { " sel" } else { "" })
                                            }
                                            on:click=move |_| {
                                                selected_flat.set(Some(id2.clone()));
                                                split_ratio.set(0.50);
                                            }
                                        >
                                            <div class="fleet-top">
                                                <span class="fleet-name">{name}</span>
                                                <span class="fleet-status" style:color="var(--cyber-cyan)">{fmt_area_m2(a)}</span>
                                            </div>
                                        </button>
                                    }
                                }).collect_view()}
                            </div>

                            <div class="split-pick-label">"2 · CUT DIRECTION"</div>
                            <div class="split-dir-row">
                                <button
                                    class=move || if split_axis.get() == 0 { "act-pill sel" } else { "act-pill" }
                                    on:click=move |_| split_axis.set(0)
                                >"↕ E / W  ·  vertical cut"</button>
                                <button
                                    class=move || if split_axis.get() == 1 { "act-pill sel" } else { "act-pill" }
                                    on:click=move |_| split_axis.set(1)
                                >"↔ N / S  ·  horizontal cut"</button>
                            </div>

                            <div class="split-pick-label">
                                {move || {
                                    if split_axis.get() == 0 {
                                        "3 · POSITION  (west ← → east)"
                                    } else {
                                        "3 · POSITION  (south ← → north)"
                                    }
                                }}
                            </div>
                            <div class="split-slider-row">
                                <input
                                    type="range"
                                    min="5"
                                    max="95"
                                    step="1"
                                    class="split-range"
                                    prop:value=move || format!("{:.0}", split_ratio.get() * 100.0)
                                    on:input=move |ev| {
                                        if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                            split_ratio.set((v / 100.0).clamp(0.05, 0.95));
                                        }
                                    }
                                />
                                <span class="split-ratio-lbl">
                                    {move || format!("{:.0}%", split_ratio.get() * 100.0)}
                                </span>
                            </div>

                            // live preview metrics + mini map
                            {move || {
                                let fid = selected_flat.get();
                                let axis = split_axis.get();
                                let ratio = split_ratio.get();
                                let list = flats.get();
                                let Some(src) = fid.as_ref().and_then(|id| list.iter().find(|f| &f.id == id).cloned()) else {
                                    return view! {
                                        <div class="split-metrics empty">"select a flat to preview the cut"</div>
                                    }.into_any();
                                };
                                let total = area_m2(&src.coords);
                                let span = bbox_span_m(&src.coords, axis);
                                let offset = span * ratio;
                                let preview = split_flat_at(&src, axis, ratio);
                                let (ok, a_m2, b_m2, a_coords, b_coords) = match &preview {
                                    Some((a, b, _)) => (
                                        true,
                                        area_m2(&a.coords),
                                        area_m2(&b.coords),
                                        a.coords.clone(),
                                        b.coords.clone(),
                                    ),
                                    None => (false, 0.0, 0.0, vec![], vec![]),
                                };
                                let pct_a = if total > 0.0 { a_m2 / total * 100.0 } else { 0.0 };
                                let pct_b = if total > 0.0 { b_m2 / total * 100.0 } else { 0.0 };
                                let side_a = if axis == 0 { "WEST (A)" } else { "SOUTH (A)" };
                                let side_b = if axis == 0 { "EAST (B)" } else { "NORTH (B)" };
                                let (min_lon, max_lon, min_lat, max_lat) = poly_bbox(&src.coords);
                                // mini viewBox
                                const MW: f64 = 320.0;
                                const MH: f64 = 180.0;
                                const MP: f64 = 12.0;
                                let bbox = BBox { min_lon, max_lon, min_lat, max_lat };
                                let path_src = poly_path(&src.coords, &bbox, MW, MH, MP);
                                let path_a = poly_path(&a_coords, &bbox, MW, MH, MP);
                                let path_b = poly_path(&b_coords, &bbox, MW, MH, MP);
                                let (cut_x1, cut_y1, cut_x2, cut_y2) = if axis == 0 {
                                    let t = min_lon + ratio * (max_lon - min_lon);
                                    let (x, y1) = project(t, max_lat, &bbox, MW, MH, MP);
                                    let (_, y2) = project(t, min_lat, &bbox, MW, MH, MP);
                                    (x, y1, x, y2)
                                } else {
                                    let t = min_lat + ratio * (max_lat - min_lat);
                                    let (x1, y) = project(min_lon, t, &bbox, MW, MH, MP);
                                    let (x2, _) = project(max_lon, t, &bbox, MW, MH, MP);
                                    (x1, y, x2, y)
                                };
                                view! {
                                    <div class="split-preview">
                                        <svg class="split-mini" viewBox=format!("0 0 {MW} {MH}")>
                                            <path d=path_src fill="rgba(255,255,255,0.03)" stroke="#333" stroke-width="1" />
                                            {ok.then(|| view! {
                                                <path d=path_a fill="rgba(0,229,255,0.28)" stroke="var(--cyber-cyan)" stroke-width="1.5" />
                                                <path d=path_b fill="rgba(255,0,255,0.22)" stroke="var(--cyber-magenta)" stroke-width="1.5" />
                                                <line x1=cut_x1 y1=cut_y1 x2=cut_x2 y2=cut_y2
                                                    stroke="var(--cyber-yellow)" stroke-width="2"
                                                    stroke-dasharray="4 3" />
                                            })}
                                        </svg>
                                        <div class="split-metrics">
                                            <div class="sm-row total">
                                                <span>"TOTAL"</span>
                                                <span>{fmt_area_m2(total)}</span>
                                            </div>
                                            <div class="sm-row a">
                                                <span>{side_a}</span>
                                                <span>{if ok { format!("{} · {:.1}%", fmt_area_m2(a_m2), pct_a) } else { "—".into() }}</span>
                                            </div>
                                            <div class="sm-row b">
                                                <span>{side_b}</span>
                                                <span>{if ok { format!("{} · {:.1}%", fmt_area_m2(b_m2), pct_b) } else { "—".into() }}</span>
                                            </div>
                                            <div class="sm-row dim">
                                                <span>"cut offset"</span>
                                                <span>{format!("{:.1} m from {}", offset, if axis == 0 { "west" } else { "south" })}</span>
                                            </div>
                                            <div class="sm-row dim">
                                                <span>"bbox span"</span>
                                                <span>{format!("{:.1} m", span)}</span>
                                            </div>
                                            {(!ok).then(|| view! {
                                                <div class="sm-row warn">"cut invalid — move slider (sliver or empty half)"</div>
                                            })}
                                        </div>
                                    </div>
                                }.into_any()
                            }}

                            <button
                                class="intent-commit split"
                                on:click=move |_| {
                                    let Some(fid) = selected_flat.get() else { return };
                                    let list = flats.get_untracked();
                                    let Some(src) = list.iter().find(|f| f.id == fid).cloned() else { return };
                                    let axis = split_axis.get_untracked();
                                    let ratio = split_ratio.get_untracked();
                                    let Some((a, b, _)) = split_flat_at(&src, axis, ratio) else { return };
                                    let a_m2 = area_m2(&a.coords);
                                    let b_m2 = area_m2(&b.coords);
                                    let a_id = a.id.clone();
                                    flats.update(|v| {
                                        v.retain(|f| f.id != fid);
                                        v.push(a);
                                        v.push(b);
                                    });
                                    leased.update(|v| v.retain(|x| x != &fid));
                                    selected_flat.set(Some(a_id));
                                    let id = next_id.get();
                                    next_id.set(id + 1);
                                    intents.update(|q| {
                                        q.insert(0, Intent {
                                            id,
                                            fleet: "YOU".into(),
                                            action: "split".into(),
                                            flat: format!(
                                                "{fid} → {:.0}+{:.0} m²",
                                                a_m2, b_m2
                                            ),
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
                    <button class="cta-btn cta-split cta-lg cta-bold" on:click=move |_| {
                        split_ratio.set(0.50);
                        sheet.set(Sheet::SplitLand);
                    }>
                        <span class="cta-ico">"✂"</span>
                        <span class="cta-copy">
                            <span class="cta-title">"SPLIT LAND"</span>
                            <span class="cta-sub">"precise cut · m² preview"</span>
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
