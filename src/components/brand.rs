use leptos::prelude::*;

/// The cyberia flag — seven lights in a ring — leads the wordmark.
/// Rebuilt as crisp vectors from x.com/cyberiacap, colors sampled exact.
const FLAG_SVG: &str = include_str!("../../assets/cyberia-flag.svg");

/// The logo doubles as the view switcher: CYBERSTATES opens a menu of the
/// two listing views. The caret shows only where space is scarce.
#[component]
pub fn BrandChooser(#[prop(optional)] active: &'static str) -> impl IntoView {
    let (open, set_open) = signal(false);
    view! {
        <div class="brand-chooser">
            <h1 class="logo" on:click=move |_| set_open.update(|o| *o = !*o) style="cursor: pointer;">
                <span class="brand-flag" inner_html=FLAG_SVG></span>
                <span style="color: var(--cyber-green);">"cyber"</span>
                <span style="color: var(--cyber-green); margin: 0 1px;">"•"</span>
                <span class="brand-word" style="color: #fff;">{match active {
                    "TOKENS" => "Tokens",
                    "DOCTRINE" => "Doctrine",
                    _ => "States",
                }}</span>
                <span class="brand-caret">" ▾"</span>
            </h1>
            <div class="brand-menu" style:display=move || if open.get() { "flex" } else { "none" }>
                <a class=if active == "STATES" { "region-pill active" } else { "region-pill" } href="/">"STATES"</a>
                <a class=if active == "TOKENS" { "region-pill active" } else { "region-pill" } href="/tokens">"TOKENS"</a>
                <a class=if active == "DOCTRINE" { "region-pill active" } else { "region-pill" } href="/doctrine">"DOCTRINE"</a>
            </div>
        </div>
    }
}
