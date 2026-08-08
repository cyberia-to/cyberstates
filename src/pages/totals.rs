use crate::components::brand::BrandChooser;
use crate::components::nav::SiteNav;
use crate::data::{
    compact_count, format_area, is_aggregate, is_terrestrial, load_countries, Country, REGIONS,
};
use crate::numeraires::{fmt_cap, Numeraire};
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

struct Totals {
    capital_b: f64,
    capital_prev_b: f64,
    population: u64,
    territory_km2: u64,
    n: usize,
}

impl Totals {
    fn from(rows: &[Country]) -> Self {
        let mut t = Totals {
            capital_b: 0.0,
            capital_prev_b: 0.0,
            population: 0,
            territory_km2: 0,
            n: rows.len(),
        };
        for c in rows {
            t.capital_b += c.money_supply_b_usd;
            t.capital_prev_b += c.money_supply_b_usd_prev;
            t.population += c.population;
            t.territory_km2 += c.land_area_km2;
        }
        t
    }

    fn capital_delta(&self) -> Option<f64> {
        if self.capital_prev_b > 0.0 && self.capital_b > 0.0 {
            Some(self.capital_b / self.capital_prev_b - 1.0)
        } else {
            None
        }
    }
}

#[derive(Clone)]
struct RegionBar {
    name: String,
    capital_b: f64,
    population: u64,
    territory_km2: u64,
}

fn region_bars(rows: &[Country]) -> Vec<RegionBar> {
    let mut map: BTreeMap<String, RegionBar> = BTreeMap::new();
    for c in rows {
        let e = map.entry(c.region.clone()).or_insert_with(|| RegionBar {
            name: c.region.clone(),
            capital_b: 0.0,
            population: 0,
            territory_km2: 0,
        });
        e.capital_b += c.money_supply_b_usd;
        e.population += c.population;
        e.territory_km2 += c.land_area_km2;
    }
    // display order follows REGIONS when present
    let mut out: Vec<RegionBar> = REGIONS.iter().filter_map(|r| map.remove(*r)).collect();
    out.extend(map.into_values());
    out.retain(|r| r.population > 0 || r.capital_b > 0.0 || r.territory_km2 > 0);
    out
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

#[component]
pub fn TotalsPage() -> impl IntoView {
    let numeraire = use_context::<RwSignal<Numeraire>>().expect("numeraire context");
    let rows = country_rows();
    let totals = Totals::from(&rows);
    let regions = region_bars(&rows);

    let max_cap = regions
        .iter()
        .map(|r| r.capital_b)
        .fold(0.0_f64, f64::max)
        .max(1e-9);
    let max_pop = regions
        .iter()
        .map(|r| r.population)
        .max()
        .unwrap_or(1)
        .max(1) as f64;
    let max_land = regions
        .iter()
        .map(|r| r.territory_km2)
        .max()
        .unwrap_or(1)
        .max(1) as f64;

    let cap_delta = totals.capital_delta();
    let n_states = totals.n;
    let pop = totals.population;
    let land = totals.territory_km2;
    let cap_b = totals.capital_b;

    Effect::new(move |_| {
        document().set_title("World totals — capital · population · territory · cyberstates");
    });

    // share of world for the three macro stocks (normalized side-by-side)
    let share_items = {
        // pure relative visual: each stock scaled to 100% of its own total
        // shown as three equal full bars with labels — plus region breakdown
        ()
    };
    let _ = share_items;

    view! {
        <div class="page-frame">
            <div class="site-chrome">
            <div class="header-row1">
                <div class="logo-zone">
                    <BrandChooser active="TOTALS" />
                    <div class="logo-suffix">"capital · people · land"</div>
                </div>
                <SiteNav active="TOTALS" />
            </div>
            </div>

            <p class="totals-lede">
                "Sum of terrestrial countries only — planets and continent aggregates "
                "excluded so nothing is counted twice. "
                {format!("{n_states} states on the tape.")}
            </p>

            <div class="totals-kpis">
                <div class="totals-kpi">
                    <div class="totals-kpi-lab">"CAPITAL"</div>
                    <div class="totals-kpi-val cyan">
                        {move || fmt_cap(cap_b, numeraire.get())}
                    </div>
                    <div class="totals-kpi-hint">
                        {match cap_delta {
                            Some(d) if d > 0.0005 => format!("▲{:.2}% vs prior snapshot", d * 100.0),
                            Some(d) if d < -0.0005 => format!("▼{:.2}% vs prior snapshot", -d * 100.0),
                            Some(_) => "flat vs prior snapshot".into(),
                            None => "money stock · B USD".into(),
                        }}
                    </div>
                </div>
                <div class="totals-kpi">
                    <div class="totals-kpi-lab">"POPULATION"</div>
                    <div class="totals-kpi-val green">{compact_count(pop as f64)}</div>
                    <div class="totals-kpi-hint">
                        {if pop >= 1_000_000_000 {
                            format!("{:.2}B humans", pop as f64 / 1e9)
                        } else {
                            format!("{} humans", compact_count(pop as f64))
                        }}
                    </div>
                </div>
                <div class="totals-kpi">
                    <div class="totals-kpi-lab">"TERRITORY"</div>
                    <div class="totals-kpi-val yellow">{format!("{} km²", format_area(land))}</div>
                    <div class="totals-kpi-hint">"land area · terrestrial"</div>
                </div>
            </div>

            // three world stocks as a single comparative strip (each bar = 100% of that stock)
            <div class="totals-panel">
                <div class="totals-panel-title">"WORLD STOCKS"</div>
                <p class="totals-panel-hint">"each bar is the full terrestrial sum — different units, same stage"</p>
                <div class="totals-world-chart">
                    <div class="totals-world-col">
                        <div class="totals-world-bar cyan" style="height: 100%"></div>
                        <div class="totals-world-lab">"CAPITAL"</div>
                        <div class="totals-world-num">{move || fmt_cap(cap_b, numeraire.get())}</div>
                    </div>
                    <div class="totals-world-col">
                        <div class="totals-world-bar green" style="height: 100%"></div>
                        <div class="totals-world-lab">"POPULATION"</div>
                        <div class="totals-world-num">{compact_count(pop as f64)}</div>
                    </div>
                    <div class="totals-world-col">
                        <div class="totals-world-bar yellow" style="height: 100%"></div>
                        <div class="totals-world-lab">"TERRITORY"</div>
                        <div class="totals-world-num">{format_area(land)}</div>
                    </div>
                </div>
            </div>

            <div class="totals-panel">
                <div class="totals-panel-title">"CAPITAL BY REGION"</div>
                <p class="totals-panel-hint">"money stock share · bar width vs largest region"</p>
                <div class="totals-bars">
                    {regions.iter().map(|r| {
                        let pct = r.capital_b / max_cap * 100.0;
                        let val = fmt_cap(r.capital_b, Numeraire::Usd);
                        bar_row(&r.name, pct, &val, "var(--cyber-cyan)")
                    }).collect_view()}
                </div>
            </div>

            <div class="totals-panel">
                <div class="totals-panel-title">"POPULATION BY REGION"</div>
                <p class="totals-panel-hint">"humans · bar width vs largest region"</p>
                <div class="totals-bars">
                    {regions.iter().map(|r| {
                        let pct = r.population as f64 / max_pop * 100.0;
                        let val = compact_count(r.population as f64);
                        bar_row(&r.name, pct, &val, "var(--cyber-green)")
                    }).collect_view()}
                </div>
            </div>

            <div class="totals-panel">
                <div class="totals-panel-title">"TERRITORY BY REGION"</div>
                <p class="totals-panel-hint">"land km² · bar width vs largest region"</p>
                <div class="totals-bars">
                    {regions.iter().map(|r| {
                        let pct = r.territory_km2 as f64 / max_land * 100.0;
                        let val = format!("{} km²", format_area(r.territory_km2));
                        bar_row(&r.name, pct, &val, "var(--cyber-yellow)")
                    }).collect_view()}
                </div>
            </div>

            <div class="search-dock">
                <a href="/by/growth" class="dock-link">"growth today →"</a>
                <a href="/by/loss" class="dock-link">"loss today →"</a>
                <a href="https://x.com/cyberiacap" target="_blank" rel="noopener" class="dock-credit">
                    "🏴 a "<span style="color: var(--cyber-green);">"cyberia"</span>" project"
                </a>
            </div>
        </div>
    }
}
