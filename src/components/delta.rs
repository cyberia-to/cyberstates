use leptos::prelude::*;

use crate::data::delta_badge;

/// Day-over-day change badge — ▲/▼ + percentage, colored red/green. Renders
/// nothing until the daily updater has written a prior snapshot (`None`),
/// so a freshly-added state doesn't show a misleading "▲0.0%" on day one.
#[component]
pub fn Delta(pct: Option<f64>) -> impl IntoView {
    match pct {
        None => ().into_any(),
        Some(pct) => {
            let (arrow, text, color) = delta_badge(pct);
            view! {
                <span style:color=color style="font-size: 11px; margin-left: 6px; font-weight: 400;">
                    {arrow}{text}
                </span>
            }
            .into_any()
        }
    }
}
