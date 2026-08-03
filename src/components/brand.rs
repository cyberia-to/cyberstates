use leptos::prelude::*;

/// The logo doubles as the view switcher: CYBERSTATES opens a menu of the
/// two listing views. The caret shows only where space is scarce.
#[component]
pub fn BrandChooser(#[prop(optional)] active: &'static str) -> impl IntoView {
    let (open, set_open) = signal(false);
    view! {
        <div class="brand-chooser">
            <h1 class="logo" on:click=move |_| set_open.update(|o| *o = !*o) style="cursor: pointer;">
                <span style="color: var(--cyber-green);">"CYBER"</span>
                <span style="color: #fff;">"STATES"</span>
                <span class="brand-caret">" ▾"</span>
            </h1>
            <div class="brand-menu" style:display=move || if open.get() { "flex" } else { "none" }>
                <a class=if active == "STATES" { "region-pill active" } else { "region-pill" } href="/">"STATES"</a>
                <a class=if active == "TOKENS" { "region-pill active" } else { "region-pill" } href="/tokens">"TOKENS"</a>
            </div>
        </div>
    }
}
