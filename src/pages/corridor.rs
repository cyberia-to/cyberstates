use crate::components::notyet::value_or_notyet;
use crate::data::*;
use crate::pages::country::SiteHeader;
use leptos::prelude::*;

/// Bilateral corridor: can holders of `from` enter `to`, and the reverse.
/// Canonical URL: /from/{slug}/to/{slug}
#[component]
pub fn CorridorPage() -> impl IntoView {
    let params = leptos_router::hooks::use_params_map();

    // Rewrite legacy /from/us/to/jp → /from/united-states/to/japan
    Effect::new(move |_| {
        let p = params.get();
        let from_raw = p.get("from").unwrap_or_default();
        let to_raw = p.get("to").unwrap_or_default();
        if from_raw.is_empty() || to_raw.is_empty() {
            return;
        }
        let from = find_country(&from_raw);
        let to = find_country(&to_raw);
        match (from, to) {
            (Some(f), Some(t)) => {
                let canonical = format!("/from/{}/to/{}", f.slug, t.slug);
                let current = format!(
                    "/from/{}/to/{}",
                    from_raw.to_lowercase(),
                    to_raw.to_lowercase()
                );
                if current != canonical {
                    crate::pages::map::navigate_client(&canonical);
                } else {
                    document().set_title(&format!(
                        "{} → {} — visa corridor | Cyberstates",
                        f.name, t.name
                    ));
                }
            }
            _ => {}
        }
    });

    view! {
        <div class="page-frame">
            {move || {
                let p = params.get();
                let from_raw = p.get("from").unwrap_or_default();
                let to_raw = p.get("to").unwrap_or_default();
                let from = find_country(&from_raw);
                let to = find_country(&to_raw);

                match (from, to) {
                    (None, _) | (_, None) => view! {
                        <div>
                            <SiteHeader />
                            <h1 style="color: var(--cyber-red); margin-top: 40px;">"CORRIDOR NOT FOUND"</h1>
                            <p style="color: #666;">"Unknown state in /from/…/to/… path."</p>
                        </div>
                    }.into_any(),
                    (Some(f), Some(t)) if f.code == t.code => view! {
                        <div>
                            <SiteHeader />
                            <h1 style="color: var(--cyber-yellow); margin-top: 40px;">"SAME STATE"</h1>
                            <p style="color: #666;">
                                <a href=f.path() style="color: var(--cyber-cyan);">{f.name.clone()}</a>
                                " — pick a different destination."
                            </p>
                        </div>
                    }.into_any(),
                    (Some(f), Some(t)) => {
                        let out = access_from_to(&f, &t);
                        let inn = access_from_to(&t, &f);
                        let reverse = format!("/from/{}/to/{}", t.slug, f.slug);
                        let f_path = f.path();
                        let t_path = t.path();
                        let f_name = f.name.clone();
                        let t_name = t.name.clone();
                        let f_flag = f.flag.clone();
                        let t_flag = t.flag.clone();
                        let f_code = f.code.clone();
                        let t_code = t.code.clone();

                        view! {
                            <div>
                                <SiteHeader />

                                <div class="state-hero" style="margin-top: 24px;">
                                    <div class="state-identity">
                                        <div style="font-size: 11px; letter-spacing: 3px; color: #555; margin-bottom: 12px;">
                                            "VISA CORRIDOR"
                                        </div>
                                        <h1 style="font-size: clamp(22px, 4vw, 40px); font-weight: 700; color: #fff; margin: 0; line-height: 1.15;">
                                            <a href=f_path.clone() style="color: #fff; text-decoration: none;">
                                                {f_flag.clone()}{" "}{f_name.clone()}
                                            </a>
                                            <span style="color: #444; margin: 0 12px;">"→"</span>
                                            <a href=t_path.clone() style="color: #fff; text-decoration: none;">
                                                {t_flag.clone()}{" "}{t_name.clone()}
                                            </a>
                                        </h1>
                                        <div style="margin-top: 10px; font-size: 12px; color: #555; letter-spacing: 2px;">
                                            {f_code.clone()}{" → "}{t_code.clone()}
                                        </div>
                                    </div>
                                </div>

                                <div class="section-label" style="margin-top: 32px;">"OUTBOUND"</div>
                                <div class="stat-grid">
                                    <div class="stat-card">
                                        <div class="stat-label">{format!("{} HOLDER → {}", f_code, t_name)}</div>
                                        <div class="stat-value" style=format!("color: {};", access_type_color(out.as_str()))>
                                            {access_type_label(out.as_str())}
                                        </div>
                                        <div style="font-size: 12px; color: #444; margin-top: 8px;">
                                            "Can a "<a href=f_path.clone() style="color: var(--cyber-cyan);">{f_name.clone()}</a>
                                            " passport enter "<a href=t_path.clone() style="color: var(--cyber-cyan);">{t_name.clone()}</a>"?"
                                        </div>
                                    </div>
                                    <div class="stat-card">
                                        <div class="stat-label">{format!("{} HOLDER → {}", t_code, f_name)}</div>
                                        <div class="stat-value" style=format!("color: {};", access_type_color(inn.as_str()))>
                                            {access_type_label(inn.as_str())}
                                        </div>
                                        <div style="font-size: 12px; color: #444; margin-top: 8px;">
                                            "Reverse leg — "<a href=reverse style="color: var(--cyber-magenta);">"flip corridor"</a>
                                        </div>
                                    </div>
                                </div>

                                <div class="section-label" style="margin-top: 36px;">"STATES"</div>
                                <div class="stat-grid">
                                    <div class="stat-card">
                                        <div class="stat-label">"FROM"</div>
                                        <div class="stat-value" style="font-size: 18px;">
                                            <a href=f_path style="color: #e0e0e0; text-decoration: none;">
                                                {f_flag}{" "}{f_name}
                                            </a>
                                        </div>
                                        <div style="font-size: 12px; color: #444; margin-top: 6px;">
                                            {value_or_notyet(f.cap_fmt())}{" capital · freedom "}
                                            {format!("{:.1}", f.index().freedom)}
                                        </div>
                                    </div>
                                    <div class="stat-card">
                                        <div class="stat-label">"TO"</div>
                                        <div class="stat-value" style="font-size: 18px;">
                                            <a href=t_path style="color: #e0e0e0; text-decoration: none;">
                                                {t_flag}{" "}{t_name}
                                            </a>
                                        </div>
                                        <div style="font-size: 12px; color: #444; margin-top: 6px;">
                                            {value_or_notyet(t.cap_fmt())}{" capital · hospitality "}
                                            {format!("{:.1}", t.index().openness)}
                                        </div>
                                    </div>
                                </div>
                            </div>
                        }.into_any()
                    }
                }
            }}
        </div>
    }
}

/// Visa access type for a holder of `from` entering `to` (by destination name in the matrix).
fn access_from_to(from: &Country, to: &Country) -> String {
    get_visa_outgoing(&from.code)
        .into_iter()
        .find(|e| e.country == to.name)
        .map(|e| e.access_type)
        .unwrap_or_else(|| "unknown".to_string())
}
