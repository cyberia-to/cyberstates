use leptos::prelude::*;
use crate::data::*;
use crate::pages::country::SiteHeader;

#[component]
pub fn TokenPage() -> impl IntoView {
    let params = leptos_router::hooks::use_params_map();

    let token = move || {
        let p = params.get();
        let code = p.get("code").unwrap_or_default().to_uppercase();
        get_token(&code)
    };

    view! {
        <div class="page-frame">
            {move || match token() {
                None => view! {
                    <div>
                        <SiteHeader />
                        <h1 style="color: var(--cyber-red); margin-top: 40px;">"TOKEN NOT FOUND"</h1>
                    </div>
                }.into_any(),
                Some(t) => {
                    let code = t.code.clone();
                    let name = t.name.clone();
                    let price = t.price_fmt();
                    let cap = t.cap_fmt();
                    let supply = t.supply_fmt();
                    let num_countries = t.countries.len();
                    let countries = t.countries.clone();
                    let total_supply = t.total_supply_b_usd;

                    view! {
                        <div>
                            <SiteHeader />

                            // Hero: identity, mirroring the state page
                            <div class="state-hero">
                                <div class="state-identity">
                                    <div style="display: flex; align-items: center; gap: 20px;">
                                        <div>
                                            <h1 style="font-size: clamp(28px, 5vw, 52px); font-weight: 700; color: var(--cyber-yellow); margin: 0; line-height: 1.05;">
                                                {code.clone()}
                                            </h1>
                                            <div style="display: flex; gap: 12px; align-items: center; margin-top: 10px;">
                                                <span style="font-size: 13px; color: #888; letter-spacing: 1px;">{name}</span>
                                                <span style="color: var(--cyber-yellow); font-size: 11px; padding: 3px 10px; border-radius: 2px; border: 1px solid; letter-spacing: 1px;">"TOKEN"</span>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </div>

                            // Fundamental, in site vocabulary
                            <div class="section-label" style="margin-top: 28px;">"FUNDAMENTAL"</div>
                            <div class="stat-grid">
                                <div class="stat-card">
                                    <div class="stat-label">"PRICE"</div>
                                    <div class="stat-value" style="color: var(--cyber-orange);">{price}</div>
                                    <div style="font-size: 10px; color: #333; margin-top: 4px;">"USD per token"</div>
                                </div>
                                <div class="stat-card">
                                    <div class="stat-label">"SUPPLY"</div>
                                    <div class="stat-value" style="color: #888;">{supply}</div>
                                    <div style="font-size: 10px; color: #333; margin-top: 4px;">"native tokens in circulation"</div>
                                </div>
                                <div class="stat-card">
                                    <div class="stat-label">"CAPITAL"</div>
                                    <div class="stat-value" style="color: var(--cyber-yellow);">{cap}</div>
                                    <div style="font-size: 10px; color: #333; margin-top: 4px;">"money supply in USD"</div>
                                </div>
                                <div class="stat-card">
                                    <div class="stat-label">"STATES"</div>
                                    <div class="stat-value" style="color: var(--cyber-cyan);">{num_countries}</div>
                                    <div style="font-size: 10px; color: #333; margin-top: 4px;">"states running on this token"</div>
                                </div>
                            </div>

                            // Countries using this token
                            <div style="margin-top: 32px;">
                                <div class="section-label">"STATES ON THIS TOKEN"</div>
                                <div style="border: 1px solid #111; border-radius: 4px;">
                                    <table class="cyber-table" style="font-size: 13px;">
                                        <thead>
                                            <tr>
                                                <th style="padding: 8px 16px;">"STATE"</th>
                                                <th style="padding: 8px 16px; text-align: right;">"SUPPLY SHARE"</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {countries.into_iter().map(|(ccode, cname, cflag)| {
                                                let href = format!("/state/{}", ccode.to_lowercase());
                                                let all_countries = load_countries();
                                                let supply = all_countries.iter()
                                                    .find(|c| c.code == ccode)
                                                    .map(|c| c.money_supply_b_usd)
                                                    .unwrap_or(0.0);
                                                let share = if total_supply > 0.0 {
                                                    format!("{:.1}%", supply / total_supply * 100.0)
                                                } else {
                                                    "—".to_string()
                                                };
                                                let supply_str = if supply >= 1000.0 {
                                                    format!("${:.1}T", supply / 1000.0)
                                                } else if supply > 0.0 {
                                                    format!("${:.0}B", supply)
                                                } else {
                                                    "N/A".to_string()
                                                };

                                                view! {
                                                    <tr>
                                                        <td style="padding: 8px 16px;">
                                                            <a href=href style="color: #ccc; text-decoration: none;">
                                                                <span style="margin-right: 8px;">{cflag}</span>
                                                                {cname}
                                                            </a>
                                                        </td>
                                                        <td class="tabular-nums" style="padding: 8px 16px; text-align: right; color: var(--cyber-yellow);">
                                                            {supply_str}" "{share}
                                                        </td>
                                                    </tr>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </tbody>
                                    </table>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}
