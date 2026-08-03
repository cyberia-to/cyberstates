use leptos::prelude::*;
use crate::data::{Country, SortField};
use crate::numeraires::{fmt_cap, fmt_value, price_parts, Numeraire};

fn score_color(v: f64) -> &'static str {
    if v > 60.0 { "var(--cyber-green)" }
    else if v > 40.0 { "var(--cyber-cyan)" }
    else if v > 25.0 { "var(--cyber-yellow)" }
    else if v > 10.0 { "var(--cyber-orange)" }
    else { "var(--cyber-red)" }
}

/// Render the rating-object cell: value formatted per field, scores colored.
pub fn metric_cell(c: &Country, f: SortField, n: Numeraire) -> (String, &'static str) {
    match f {
        SortField::Capital => (fmt_cap(c.money_supply_b_usd, n), "#e0e0e0"),
        SortField::Human | SortField::Land => (fmt_value(c.metric(f), n), "#e0e0e0"),
        SortField::Freedom | SortField::Hospitality => {
            let v = c.metric(f);
            (format!("{:.1}", v), score_color(v))
        }
        SortField::Population => (c.population_fmt(), "#e0e0e0"),
        SortField::Area => (c.land_area_fmt(), "#e0e0e0"),
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
    let code = country.code.to_lowercase();
    let href = format!("/state/{}", code);

    let flag = country.flag.clone();
    let name = country.name.clone();
    let token_code = country.currency_code.clone();
    let token_slug = token_code.to_lowercase();
    let price_usd = country.token_price_usd;
    let c_for_metric = country.clone();

    view! {
        <tr on:click=move |_| {
            if let Some(window) = web_sys::window() {
                let _ = window.location().set_href(&href);
            }
        }>
            <td style="color: #333; text-align: right;">{rank}</td>
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
                    view! {
                        <span>{head}</span>
                        <span class="price-frac">{frac}</span>
                        <span class="price-unit">{unit}</span>
                    }
                }}
            </td>
            <td class="tabular-nums" style="text-align: right; font-weight: 700;">
                {move || {
                    let (text, color) = metric_cell(&c_for_metric, field.get(), numeraire.get());
                    view! { <span style:color=color>{text}</span> }
                }}
            </td>
        </tr>
    }
}
