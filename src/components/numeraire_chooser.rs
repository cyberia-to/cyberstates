use crate::numeraires::{store_numeraire, Numeraire};
use leptos::prelude::*;

/// Compact numeraire chooser: one button with the current token's symbol,
/// click opens the list. Reads and writes the app-wide measure, so it can
/// live in the header of every page.
#[component]
pub fn NumeraireChooser() -> impl IntoView {
    let numeraire = use_context::<RwSignal<Numeraire>>().expect("numeraire context");
    let (open, set_open) = signal(false);
    view! {
        <div class="num-chooser">
            <button
                class="region-pill num-btn"
                on:click=move |_| set_open.update(|o| *o = !*o)
            >
                <span style:color=move || numeraire.get().color() style="font-weight: 700;">
                    {move || numeraire.get().label()}
                </span>
                <span class="rating-caret" aria-hidden="true"></span>
            </button>
            <div class="num-menu" style:display=move || if open.get() { "flex" } else { "none" }>
                {Numeraire::ALL.map(|n| {
                    view! {
                        <button
                            class=move || if numeraire.get() == n { "region-pill active" } else { "region-pill" }
                            on:click=move |_| {
                                numeraire.set(n);
                                store_numeraire(n);
                                set_open.set(false);
                            }
                        >
                            <span style:color=n.color() style="font-weight: 700;">{n.label()}</span>
                            {format!("  {}", n.name())}
                        </button>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}
