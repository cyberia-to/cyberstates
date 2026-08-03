use leptos::prelude::*;
use crate::numeraires::Numeraire;

/// Compact numeraire chooser: one button with the current token's flag,
/// click opens the list. Lives in the header's right zone without
/// widening it.
#[component]
pub fn NumeraireChooser(
    numeraire: ReadSignal<Numeraire>,
    set_numeraire: WriteSignal<Numeraire>,
) -> impl IntoView {
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
                " ▾"
            </button>
            <div class="num-menu" style:display=move || if open.get() { "flex" } else { "none" }>
                {Numeraire::ALL.map(|n| {
                    view! {
                        <button
                            class=move || if numeraire.get() == n { "region-pill active" } else { "region-pill" }
                            on:click=move |_| {
                                set_numeraire.set(n);
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
