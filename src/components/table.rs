use crate::components::notyet::NotYet;
use crate::data::{delta_cell, Country, SortField};
use crate::numeraires::{fmt_cap, fmt_value, price_parts, Numeraire};
use leptos::prelude::*;

fn score_color(v: f64) -> &'static str {
    if v > 60.0 {
        "var(--cyber-green)"
    } else if v > 40.0 {
        "var(--cyber-cyan)"
    } else if v > 25.0 {
        "var(--cyber-yellow)"
    } else if v > 10.0 {
        "var(--cyber-orange)"
    } else {
        "var(--cyber-red)"
    }
}

/// Render the rating-object cell: value formatted per field, scores colored.
pub fn metric_cell(c: &Country, f: SortField, n: Numeraire) -> (String, &'static str) {
    match f {
        SortField::Capital => (fmt_cap(c.money_supply_b_usd, n), "#e0e0e0"),
        SortField::Growth | SortField::Loss => delta_cell(c.price_delta()),
        SortField::Human | SortField::Land => (fmt_value(c.metric(f), n), "#e0e0e0"),
        SortField::Freedom | SortField::Hospitality => {
            let v = c.metric(f);
            (format!("{:.1}", v), score_color(v))
        }
        SortField::Population => (c.population_fmt(), "#e0e0e0"),
        SortField::Territory => (c.land_area_fmt(), "#e0e0e0"),
        SortField::Density => (format!("{:.1}/km²", c.metric(f)), "#e0e0e0"),
    }
}

#[component]
pub fn CountryRow(
    country: Country,
    rank: usize,
    #[prop(into)] numeraire: Signal<Numeraire>,
    #[prop(into)] field: Signal<SortField>,
) -> impl IntoView {
    let href = country.path();

    let flag = country.flag.clone();
    let name = country.name.clone();
    let state_code = country.code.clone();
    let token_code = country.currency_code.clone();
    let token_slug = token_code.to_lowercase();
    let price_usd = country.token_price_usd;
    let price_delta = country.price_delta();
    let c_for_metric = country.clone();
    let code_enter = state_code.clone();
    let code_leave = state_code.clone();

    view! {
        <tr
            data-code=state_code
            on:click=move |_| crate::pages::map::navigate_client(&href)
            on:mouseenter=move |_| crate::pages::map::set_hover_sync(&code_enter, false)
            on:mouseleave=move |_| crate::pages::map::clear_hover_sync()
        >
            <td class="tabular-nums" style=format!("text-align: right; color: {}; font-weight: {};", crate::data::rank_color(rank), crate::data::rank_weight(rank))>{rank}</td>
            <td>
                <span style="margin-right: 8px; font-size: 16px;">{flag.clone()}</span>
                <span style="color: #ccc;">{name.clone()}</span>
            </td>
            <td>
                <a href=format!("/token/{}", token_slug) style="color: var(--cyber-yellow); text-decoration: none;"
                   on:click=move |ev| { ev.stop_propagation(); }>
                    {token_code.clone()}
                </a>
            </td>
            <td class="tabular-nums" style="text-align: right; color: var(--cyber-orange);">
                {move || {
                    let (head, frac, unit) = price_parts(price_usd, numeraire.get());
                    if head == "N/A" {
                        view! { <NotYet /> }.into_any()
                    } else {
                        view! {
                            <span>{head}</span>
                            <span class="price-frac">{frac}</span>
                            <span class="price-unit">{unit}</span>
                        }.into_any()
                    }
                }}
            </td>
            // PRICE Δ only when the rating is something else — under
            // GROWTH TODAY / LOSS TODAY the metric column *is* that %.
            {move || {
                if field.get().is_day_change() {
                    view! { }.into_any()
                } else {
                    let (text, color) = delta_cell(price_delta);
                    view! {
                        <td class="tabular-nums delta-cell" style="text-align: right;">
                            <span style:color=color>{text}</span>
                        </td>
                    }
                    .into_any()
                }
            }}
            <td class="tabular-nums" style="text-align: right; font-weight: 700;">
                {move || {
                    let (text, color) = metric_cell(&c_for_metric, field.get(), numeraire.get());
                    if text == "N/A" {
                        return view! { <NotYet /> }.into_any();
                    }
                    // big units ride small and dim, like the price's sat
                    let (num, unit) = match text.strip_suffix(" km\u{b2}") {
                        Some(n) => (n.to_string(), " km\u{b2}"),
                        None => (text, ""),
                    };
                    view! {
                        <span style:color=color>{num}</span>
                        {(!unit.is_empty()).then(|| view! { <span class="price-unit">{unit}</span> })}
                    }.into_any()
                }}
            </td>
        </tr>
    }
}
