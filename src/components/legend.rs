use leptos::prelude::*;
use crate::data::SortField;

/// The color scale as an instrument, not a caption: click a color to keep
/// only the states at least that good under the active rating — the culled
/// end of the bar dims, a mark shows the threshold, a chip clears it.
/// Clicking near the mark again also clears.
#[component]
pub fn RatingLegend(
    #[prop(into)] field: Signal<SortField>,
    cut: RwSignal<Option<f64>>,
) -> impl IntoView {
    let bar_ref = NodeRef::<leptos::html::Div>::new();
    // hover preview: a ghost mark and a "top N%" hint ride the pointer
    let (hover, set_hover) = signal(None::<f64>);
    let frac_at = move |ev: &web_sys::MouseEvent| -> Option<f64> {
        let el = bar_ref.get_untracked()?;
        let r = el.get_bounding_client_rect();
        if r.width() > 0.0 {
            Some(((ev.client_x() as f64 - r.left()) / r.width()).clamp(0.0, 1.0))
        } else {
            None
        }
    };
    view! {
        <div class="legend-zone">
            <span class="legend-end">
                {move || if field.get().lower_is_better() { "HIGH" } else { "LOW" }}
            </span>
            <div class="legend-barwrap">
            <div
                class="legend-bar"
                node_ref=bar_ref
                on:mousemove=move |ev: web_sys::MouseEvent| {
                    if let Some(g) = frac_at(&ev) {
                        set_hover.set(Some(g));
                    }
                }
                on:mouseleave=move |_| set_hover.set(None)
                on:click=move |ev: web_sys::MouseEvent| {
                    if let Some(g) = frac_at(&ev) {
                        match cut.get_untracked() {
                            Some(old) if (old - g).abs() < 0.05 => cut.set(None),
                            _ => cut.set(Some(g)),
                        }
                    }
                }
            >
                {move || cut.get().map(|g| view! {
                    <div class="legend-dim" style:width=format!("{:.1}%", g * 100.0)></div>
                    <div class="legend-mark" style:left=format!("{:.1}%", g * 100.0)></div>
                })}
                {move || hover.get().map(|g| view! {
                    <div class="legend-ghost" style:left=format!("{:.1}%", g * 100.0)></div>
                })}
            </div>
            {move || hover.get().map(|g| view! {
                <div class="legend-hint" style:left=format!("{:.1}%", g * 100.0)>
                    {format!("top {:.0}%", (1.0 - g) * 100.0)}
                </div>
            })}
            </div>
            <span class="legend-end">
                {move || if field.get().lower_is_better() { "LOW" } else { "HIGH" }}
            </span>
            {move || cut.get().map(|g| view! {
                <button class="legend-clear" on:click=move |_| cut.set(None)>
                    {format!("top {:.0}% ✕", (1.0 - g) * 100.0)}
                </button>
            })}
        </div>
    }
}

/// How many of n ranked rows survive a goodness threshold — shared by the
/// table retain, the map paint and the dock count so they never disagree.
pub fn goodness_keep(i: usize, n: usize, lower_is_better: bool, g: f64) -> bool {
    if n <= 1 {
        return true;
    }
    // rows are sorted descending by metric: row 0 is the highest value,
    // which is the good end everywhere except lower-is-better ratings
    let rank = i as f64 / (n - 1) as f64;
    let good = if lower_is_better { rank } else { 1.0 - rank };
    good + 1e-9 >= g
}
