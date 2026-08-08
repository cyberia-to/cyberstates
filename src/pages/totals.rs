use crate::components::brand::BrandChooser;
use crate::components::nav::SiteNav;
use crate::data::{
    compact_count, format_area, is_aggregate, is_terrestrial, load_countries, Country, REGIONS,
};
use crate::numeraires::{fmt_cap, fmt_value, Numeraire};
use leptos::prelude::*;
use std::collections::BTreeMap;

/// Terrestrial country-class rows only — no planets, no continent
/// aggregates (those would double-count).
fn country_rows() -> Vec<Country> {
    load_countries()
        .into_iter()
        .filter(|c| is_terrestrial(&c.region) && !is_aggregate(&c.code))
        .collect()
}

/// One region's (or the world's) full rating tape.
#[derive(Clone, Default)]
struct Bundle {
    name: String,
    capital_b: f64,
    capital_prev_b: f64,
    population: u64,
    territory_km2: u64,
    /// pop-weighted freedom / hospitality (Σ score·pop)
    freedom_w: f64,
    hospitality_w: f64,
    n: usize,
}

impl Bundle {
    fn add(&mut self, c: &Country) {
        self.capital_b += c.money_supply_b_usd;
        self.capital_prev_b += c.money_supply_b_usd_prev;
        self.population += c.population;
        self.territory_km2 += c.land_area_km2;
        let idx = c.index();
        let p = c.population as f64;
        self.freedom_w += idx.freedom * p;
        self.hospitality_w += idx.openness * p;
        self.n += 1;
    }

    fn citizen_usd(&self) -> f64 {
        if self.population > 0 {
            self.capital_b * 1e9 / self.population as f64
        } else {
            0.0
        }
    }

    fn land_usd(&self) -> f64 {
        if self.territory_km2 > 0 {
            self.capital_b * 1e9 / self.territory_km2 as f64
        } else {
            0.0
        }
    }

    fn density(&self) -> f64 {
        if self.territory_km2 > 0 {
            self.population as f64 / self.territory_km2 as f64
        } else {
            0.0
        }
    }

    fn freedom(&self) -> f64 {
        if self.population > 0 {
            self.freedom_w / self.population as f64
        } else {
            0.0
        }
    }

    fn hospitality(&self) -> f64 {
        if self.population > 0 {
            self.hospitality_w / self.population as f64
        } else {
            0.0
        }
    }

    fn capital_delta(&self) -> Option<f64> {
        if self.capital_prev_b > 0.0 && self.capital_b > 0.0 {
            Some(self.capital_b / self.capital_prev_b - 1.0)
        } else {
            None
        }
    }
}

fn world_and_regions(rows: &[Country]) -> (Bundle, Vec<Bundle>) {
    let mut world = Bundle {
        name: "World".into(),
        ..Default::default()
    };
    let mut map: BTreeMap<String, Bundle> = BTreeMap::new();
    for c in rows {
        world.add(c);
        let e = map.entry(c.region.clone()).or_insert_with(|| Bundle {
            name: c.region.clone(),
            ..Default::default()
        });
        e.add(c);
    }
    let mut regions: Vec<Bundle> = REGIONS.iter().filter_map(|r| map.remove(*r)).collect();
    regions.extend(map.into_values());
    regions.retain(|r| r.n > 0);
    (world, regions)
}

fn bar_row(label: &str, pct: f64, value: &str, color: &str) -> AnyView {
    let w = pct.clamp(0.0, 100.0);
    view! {
        <div class="totals-bar-row">
            <div class="totals-bar-lab">{label.to_string()}</div>
            <div class="totals-bar-track">
                <div class="totals-bar-fill" style=format!("width:{w:.2}%;background:{color}")></div>
            </div>
            <div class="totals-bar-val">{value.to_string()}</div>
        </div>
    }
    .into_any()
}

fn region_panel(
    title: &'static str,
    hint: &'static str,
    regions: &[Bundle],
    color: &'static str,
    value_of: impl Fn(&Bundle) -> f64,
    fmt: impl Fn(f64) -> String,
) -> AnyView {
    let max = regions
        .iter()
        .map(|r| value_of(r))
        .fold(0.0_f64, f64::max)
        .max(1e-12);
    let rows: Vec<AnyView> = regions
        .iter()
        .map(|r| {
            let v = value_of(r);
            bar_row(&r.name, v / max * 100.0, &fmt(v), color)
        })
        .collect();
    view! {
        <div class="totals-panel">
            <div class="totals-panel-title">{title}</div>
            <p class="totals-panel-hint">{hint}</p>
            <div class="totals-bars">{rows}</div>
        </div>
    }
    .into_any()
}

fn kpi_card(lab: &'static str, val: String, hint: String, klass: &'static str) -> AnyView {
    view! {
        <div class="totals-kpi">
            <div class="totals-kpi-lab">{lab}</div>
            <div class=format!("totals-kpi-val {klass}")>{val}</div>
            <div class="totals-kpi-hint">{hint}</div>
        </div>
    }
    .into_any()
}

#[component]
pub fn TotalsPage() -> impl IntoView {
    let numeraire = use_context::<RwSignal<Numeraire>>().expect("numeraire context");
    let rows = country_rows();
    let (world, regions) = world_and_regions(&rows);

    let n_states = world.n;
    let pop = world.population;
    let land = world.territory_km2;
    let cap_b = world.capital_b;
    let cap_delta = world.capital_delta();
    let citizen = world.citizen_usd();
    let land_v = world.land_usd();
    let dens = world.density();
    let free = world.freedom();
    let hosp = world.hospitality();

    Effect::new(move |_| {
        document().set_title("World totals — all ratings by region · cyberstates");
    });

    view! {
        <div class="page-frame">
            <div class="site-chrome">
            <div class="header-row1">
                <div class="logo-zone">
                    <BrandChooser active="TOTALS" />
                    <div class="logo-suffix">"all ratings · by region"</div>
                </div>
                <SiteNav active="TOTALS" />
            </div>
            </div>

            <p class="totals-lede">
                "Sum of terrestrial countries only — planets and continent aggregates "
                "excluded so nothing is counted twice. "
                "Citizen / land value and density are derived from regional stocks; "
                "freedom / hospitality are population-weighted averages. "
                {format!("{n_states} states on the tape.")}
            </p>

            // every primary + derived rating as world KPIs
            <div class="totals-kpis totals-kpis-wide">
                {move || {
                    let n = numeraire.get();
                    view! {
                        {kpi_card(
                            "CAPITAL",
                            fmt_cap(cap_b, n),
                            match cap_delta {
                                Some(d) if d > 0.0005 => format!("▲{:.2}% vs prior", d * 100.0),
                                Some(d) if d < -0.0005 => format!("▼{:.2}% vs prior", -d * 100.0),
                                Some(_) => "flat vs prior".into(),
                                None => "money stock".into(),
                            },
                            "cyan",
                        )}
                        {kpi_card(
                            "POPULATION",
                            compact_count(pop as f64),
                            if pop >= 1_000_000_000 {
                                format!("{:.2}B humans", pop as f64 / 1e9)
                            } else {
                                format!("{} humans", compact_count(pop as f64))
                            },
                            "green",
                        )}
                        {kpi_card(
                            "TERRITORY",
                            format!("{} km²", format_area(land)),
                            "land area · terrestrial".into(),
                            "yellow",
                        )}
                        {kpi_card(
                            "CITIZEN VALUE",
                            fmt_value(citizen, n),
                            "capital / human".into(),
                            "magenta",
                        )}
                        {kpi_card(
                            "LAND VALUE",
                            fmt_value(land_v, n),
                            "capital / km²".into(),
                            "orange",
                        )}
                        {kpi_card(
                            "DENSITY",
                            format!("{:.1}/km²", dens),
                            "humans / km²".into(),
                            "green",
                        )}
                        {kpi_card(
                            "TRAVEL FREEDOM",
                            format!("{:.1}", free),
                            "pop-weighted mean".into(),
                            "cyan",
                        )}
                        {kpi_card(
                            "HOSPITALITY",
                            format!("{:.1}", hosp),
                            "pop-weighted mean".into(),
                            "yellow",
                        )}
                    }
                }}
            </div>

            // stocks by region
            {region_panel(
                "CAPITAL BY REGION",
                "money stock · bar vs largest region",
                &regions,
                "var(--cyber-cyan)",
                |r| r.capital_b,
                |v| fmt_cap(v, Numeraire::Usd),
            )}
            {region_panel(
                "POPULATION BY REGION",
                "humans · bar vs largest region",
                &regions,
                "var(--cyber-green)",
                |r| r.population as f64,
                |v| compact_count(v),
            )}
            {region_panel(
                "TERRITORY BY REGION",
                "land km² · bar vs largest region",
                &regions,
                "var(--cyber-yellow)",
                |r| r.territory_km2 as f64,
                |v| format!("{} km²", format_area(v as u64)),
            )}

            // derived prices by region
            {region_panel(
                "CITIZEN VALUE BY REGION",
                "capital ÷ population · $ per human",
                &regions,
                "var(--cyber-magenta)",
                |r| r.citizen_usd(),
                |v| fmt_value(v, Numeraire::Usd),
            )}
            {region_panel(
                "LAND VALUE BY REGION",
                "capital ÷ territory · $ per km²",
                &regions,
                "var(--cyber-orange)",
                |r| r.land_usd(),
                |v| fmt_value(v, Numeraire::Usd),
            )}
            {region_panel(
                "DENSITY BY REGION",
                "population ÷ territory · humans / km²",
                &regions,
                "var(--cyber-green)",
                |r| r.density(),
                |v| format!("{:.1}/km²", v),
            )}

            // freedom scores by region
            {region_panel(
                "TRAVEL FREEDOM BY REGION",
                "population-weighted mean score",
                &regions,
                "var(--cyber-cyan)",
                |r| r.freedom(),
                |v| format!("{:.1}", v),
            )}
            {region_panel(
                "HOSPITALITY BY REGION",
                "population-weighted mean score",
                &regions,
                "var(--cyber-yellow)",
                |r| r.hospitality(),
                |v| format!("{:.1}", v),
            )}

            <div class="search-dock">
                <a href="/by/capital" class="dock-link">"by capital →"</a>
                <a href="/by/citizen-value" class="dock-link">"by citizen →"</a>
                <a href="/by/land-value" class="dock-link">"by land →"</a>
                <a href="https://x.com/cyberiacap" target="_blank" rel="noopener" class="dock-credit">
                    "🏴 a "<span style="color: var(--cyber-green);">"cyberia"</span>" project"
                </a>
            </div>
        </div>
    }
}
