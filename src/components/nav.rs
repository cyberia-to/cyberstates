use leptos::prelude::*;

/// The four destinations, always present, always in this order and color.
/// The current page's button renders as an outline — "you are here" —
/// so the header never changes shape between pages.
#[component]
pub fn SiteNav(active: &'static str) -> impl IntoView {
    const ITEMS: [(&str, &str, &str); 4] = [
        ("STATES", "/", "nav-states"),
        ("TOKENS", "/tokens", "nav-tokens"),
        ("MAP", "/map", "nav-map"),
        ("METHODOLOGY", "/methodology", "nav-method"),
    ];

    view! {
        <div class="map-zone">
            {ITEMS.map(|(label, href, cls)| {
                let here = label == active;
                view! {
                    <a
                        href=href
                        class=format!("nav-btn {}{}", cls, if here { " nav-here" } else { "" })
                    >
                        {label}
                    </a>
                }
            }).collect_view()}
        </div>
    }
}
