use leptos::prelude::*;
use crate::data::*;

#[component]
pub fn TokenPage() -> impl IntoView {
    let params = leptos_router::hooks::use_params_map();

    let token = move || {
        let p = params.get();
        let code = p.get("code").unwrap_or_default().to_uppercase();
        get_token(&code)
    };

    view! {
        <div style="max-width: 1000px; margin: 0 auto; padding: 20px;">
            {move || match token() {
                None => view! {
                    <div>
                        <a href="/" class="back-link">"← BACK TO TABLE"</a>
                        <h1 style="color: var(--cyber-red); margin-top: 40px;">"TOKEN NOT FOUND"</h1>
                    </div>
                }.into_any(),
                Some(t) => {
                    let code = t.code.clone();
                    let name = t.name.clone();
                    let price = t.price_fmt();
                    let cap = t.cap_fmt();
                    let num_countries = t.countries.len();
                    let countries = t.countries.clone();
                    let total_supply = t.total_supply_b_usd;

                    view! {
                        <div>
                            <a href="/" class="back-link">"← BACK TO TABLE"</a>

                            // Hero
                            <div class="country-hero" style="margin-top: 20px;">
                                <div style="font-size: 14px; letter-spacing: 3px; color: #444; margin-bottom: 8px;">"TOKEN"</div>
                                <h1 style="font-size: clamp(40px, 8vw, 72px); font-weight: 700; color: var(--cyber-yellow); margin: 0;">
                                    {code.clone()}
                                </h1>
                                <div style="font-size: 18px; color: #888; margin-top: 8px;">
                                    {name}
                                </div>
                            </div>

                            // Stats
                            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 12px; margin-top: 30px;">
                                <div class="stat-card">
                                    <div class="stat-label">"PRICE (USD)"</div>
                                    <div class="stat-value" style="color: var(--cyber-orange);">{price}</div>
                                </div>
                                <div class="stat-card">
                                    <div class="stat-label">"MARKET CAP"</div>
                                    <div class="stat-value" style="color: var(--cyber-yellow);">{cap}</div>
                                </div>
                                <div class="stat-card">
                                    <div class="stat-label">"STATES USING"</div>
                                    <div class="stat-value" style="color: var(--cyber-cyan);">{num_countries}</div>
                                </div>
                            </div>

                            // Countries using this token
                            <div style="margin-top: 40px;">
                                <div style="font-size: 11px; letter-spacing: 3px; color: var(--cyber-yellow); margin-bottom: 16px;">
                                    "STATES USING THIS TOKEN"
                                </div>
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
