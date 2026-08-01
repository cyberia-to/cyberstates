use leptos::prelude::*;
use wasm_bindgen::JsCast;
use crate::data::*;
use crate::components::nav::SiteNav;
use crate::numeraires::{fmt_cap, fmt_price, Numeraire};

#[derive(Clone, Copy, PartialEq)]
enum TokenSort {
    Code,
    Name,
    States,
    Cap,
}

#[component]
pub fn TokensPage() -> impl IntoView {
    let tokens = get_tokens();
    let total = tokens.len();

    let (sort, set_sort) = signal(TokenSort::Cap);
    let (ascending, set_ascending) = signal(false);
    let (search, set_search) = signal(String::new());
    let (numeraire, set_numeraire) = signal(Numeraire::Usd);

    let on_sort = move |field: TokenSort| {
        if sort.get() == field {
            set_ascending.set(!ascending.get());
        } else {
            set_sort.set(field);
            set_ascending.set(false);
        }
    };

    let th_class = move |field: TokenSort| {
        let mut c = String::from("cyber-table-th");
        if sort.get() == field {
            c.push_str(" sorted");
            c.push_str(if ascending.get() { " sort-asc" } else { " sort-desc" });
        }
        c
    };

    Effect::new(move |_| {
        document().set_title("Cyberstates tokens — Global Visa Openness Analytics");
    });

    let tokens_for_count = tokens.clone();
    let filtered_sorted = move || {
        let mut list = tokens.clone();
        let query = search.get().to_lowercase();
        if !query.is_empty() {
            list.retain(|t| {
                t.code.to_lowercase().contains(&query) || t.name.to_lowercase().contains(&query)
            });
        }

        let field = sort.get();
        let asc = ascending.get();
        list.sort_by(|a, b| {
            let ord = match field {
                TokenSort::Code => a.code.cmp(&b.code),
                TokenSort::Name => a.name.cmp(&b.name),
                TokenSort::States => a.countries.len().cmp(&b.countries.len()),
                TokenSort::Cap => a.total_supply_b_usd.partial_cmp(&b.total_supply_b_usd).unwrap_or(std::cmp::Ordering::Equal),
            };
            if asc { ord } else { ord.reverse() }
        });
        list
    };

    view! {
        <div style="max-width: 1400px; margin: 0 auto; padding: 20px;">
            // Header row 1: logo — centered search — states/map flush right
            <div class="header-row1">
                <div class="logo-zone">
                    <h1 class="logo">
                        <span style="color: var(--cyber-green);">"CYBER"</span>
                        <span style="color: #fff;">"STATES"</span>
                    </h1>
                    <div class="logo-suffix" style="color: rgba(255, 215, 0, 0.55);">"tokens · by cap"</div>
                </div>
                <input
                    type="text"
                    class="search-input"
                    placeholder="Search token code or name..."
                    on:input=move |ev| {
                        let target = ev.target().unwrap();
                        let input: web_sys::HtmlInputElement = target.unchecked_into();
                        set_search.set(input.value());
                    }
                />
                <SiteNav active="TOKENS" />
            </div>

            // Count + numeraire
            <div class="header-row2">
                <div></div>
                <div class="count-zone">
                    <div class="numeraire-toggle">
                        {Numeraire::ALL.map(|n| {
                            view! {
                                <button
                                    class=move || if numeraire.get() == n { "region-pill active" } else { "region-pill" }
                                    on:click=move |_| set_numeraire.set(n)
                                >{n.label()}</button>
                            }
                        }).collect_view()}
                    </div>
                <p class="state-count">
                    {move || {
                        let query = search.get().to_lowercase();
                        let count = tokens_for_count.iter().filter(|t| {
                            query.is_empty() || t.code.to_lowercase().contains(&query) || t.name.to_lowercase().contains(&query)
                        }).count();
                        format!("{} of {} tokens", count, total)
                    }}
                </p>
                </div>
            </div>

            // Table
            <div style="overflow-x: auto; border: 1px solid #111; border-radius: 4px;">
                <table class="cyber-table">
                    <colgroup>
                        <col style="width: 5%;" />   // #
                        <col style="width: 9%;" />   // token
                        <col style="width: 24%;" />  // name
                        <col style="width: 30%;" />  // states
                        <col style="width: 11%;" />  // price
                        <col style="width: 11%;" />  // supply
                        <col style="width: 10%;" />  // cap
                    </colgroup>
                    <thead>
                        <tr>
                            <th style="cursor: default;">"#"</th>
                            <th class=move || th_class(TokenSort::Code) on:click=move |_| on_sort(TokenSort::Code)>"TOKEN"</th>
                            <th class=move || th_class(TokenSort::Name) on:click=move |_| on_sort(TokenSort::Name)>"NAME"</th>
                            <th class=move || th_class(TokenSort::States) on:click=move |_| on_sort(TokenSort::States)>"STATES"</th>
                            <th class="th-static">"PRICE"</th>
                            <th class="th-static">"SUPPLY"</th>
                            <th class=move || th_class(TokenSort::Cap) on:click=move |_| on_sort(TokenSort::Cap)>"CAP"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            filtered_sorted()
                                .into_iter()
                                .enumerate()
                                .map(|(i, t)| {
                                    let code = t.code.clone();
                                    let href = format!("/token/{}", code.to_lowercase());
                                    let name = t.name.clone();
                                    let n_states = t.countries.len();
                                    let flags: String = t.countries.iter().take(12).map(|(_, _, f)| f.as_str()).collect::<Vec<_>>().join(" ");
                                    let more = n_states.saturating_sub(12);
                                    let price_usd = t.price_usd;
                                    let cap_b_usd = t.total_supply_b_usd;
                                    let supply = t.supply_fmt();
                                    view! {
                                        <tr on:click=move |_| {
                                            if let Some(window) = web_sys::window() {
                                                let _ = window.location().set_href(&href);
                                            }
                                        }>
                                            <td style="color: #333; text-align: right;">{i + 1}</td>
                                            <td style="color: var(--cyber-yellow); font-weight: 700;">{code}</td>
                                            <td style="color: #ccc;">{name}</td>
                                            <td>
                                                <span style="color: #888; margin-right: 8px;">{n_states}</span>
                                                <span style="font-size: 14px;">{flags}</span>
                                                {(more > 0).then(|| view! { <span style="color: #444;">{format!(" +{}", more)}</span> })}
                                            </td>
                                            <td class="tabular-nums" style="text-align: right; color: var(--cyber-orange);">{move || fmt_price(price_usd, numeraire.get())}</td>
                                            <td class="tabular-nums" style="text-align: right; color: #888;">{supply}</td>
                                            <td class="tabular-nums" style="text-align: right;">{move || fmt_cap(cap_b_usd, numeraire.get())}</td>
                                        </tr>
                                    }
                                })
                                .collect::<Vec<_>>()
                        }}
                    </tbody>
                </table>
            </div>
        </div>
    }
}
