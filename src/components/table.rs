use leptos::prelude::*;
use crate::data::{Country, SortField};
use crate::numeraires::{fmt_cap, price_parts, Numeraire};

fn score_color(v: f64) -> &'static str {
    if v > 60.0 { "var(--cyber-green)" }
    else if v > 40.0 { "var(--cyber-cyan)" }
    else if v > 25.0 { "var(--cyber-yellow)" }
    else if v > 10.0 { "var(--cyber-orange)" }
    else { "var(--cyber-red)" }
}

#[component]
pub fn SortableHeader(
    field: SortField,
    #[prop(into)] current: Signal<SortField>,
    #[prop(into)] ascending: Signal<bool>,
    on_click: SignalSetter<SortField>,
) -> impl IntoView {
    let is_sorted = move || current.get() == field;
    let class = move || {
        let mut c = String::from("cyber-table-th");
        if is_sorted() {
            c.push_str(" sorted");
            if ascending.get() {
                c.push_str(" sort-asc");
            } else {
                c.push_str(" sort-desc");
            }
        }
        c
    };

    view! {
        <th class=class on:click=move |_| on_click.set(field)>
            {field.label()}
        </th>
    }
}

#[component]
pub fn CountryRow(country: Country, rank: usize, #[prop(into)] numeraire: Signal<Numeraire>) -> impl IntoView {
    let code = country.code.to_lowercase();
    let href = format!("/country/{}", code);
    let idx = country.index();

    let flag = country.flag.clone();
    let name = country.name.clone();
    let token_code = country.currency_code.clone();
    let token_slug = token_code.to_lowercase();
    let pop_fmt = country.population_fmt();
    let area_fmt = country.land_area_fmt();
    let price_usd = country.token_price_usd;
    let cap_b_usd = country.money_supply_b_usd;
    let supply_fmt = country.supply_fmt();
    let freedom_str = format!("{:.1}", idx.freedom);
    let openness_str = format!("{:.1}", idx.openness);

    let freedom_color = score_color(idx.freedom);
    let openness_color = score_color(idx.openness);

    view! {
        <tr on:click=move |_| {
            if let Some(window) = web_sys::window() {
                let _ = window.location().set_href(&href);
            }
        }>
            <td style="color: #333; width: 40px; text-align: right;">{rank}</td>
            <td>
                <span style="margin-right: 8px; font-size: 16px;">{flag.clone()}</span>
                <span style="color: #ccc;">{name.clone()}</span>
            </td>
            <td class="tabular-nums" style="text-align: right;">{pop_fmt.clone()}</td>
            <td class="tabular-nums" style="text-align: right;">{area_fmt.clone()}</td>
            <td>
                <a href=format!("/token/{}", token_slug) style="color: var(--cyber-yellow); text-decoration: none;"
                   on:click=move |ev| { ev.stop_propagation(); }>
                    {token_code.clone()}
                </a>
            </td>
            <td class="tabular-nums" style="text-align: right; color: var(--cyber-orange);">
                {move || {
                    let (head, tail) = price_parts(price_usd, numeraire.get());
                    view! { <span>{head}</span><span class="price-tail">{tail}</span> }
                }}
            </td>
            <td class="tabular-nums" style="text-align: right; color: #888;">{supply_fmt.clone()}</td>
            <td class="tabular-nums" style="text-align: right;">{move || fmt_cap(cap_b_usd, numeraire.get())}</td>
            <td class="tabular-nums" style:color=freedom_color style="text-align: right; font-weight: 700;">{freedom_str.clone()}</td>
            <td class="tabular-nums" style:color=openness_color style="text-align: right; font-weight: 700;">{openness_str.clone()}</td>
        </tr>
    }
}
