use leptos::prelude::*;

/// The pre-listing mark. Where a quote would read N/A, the terminal says
/// what it means: not yet — the territory is listed, its token is not
/// bound. Links to /listing, where the state of the listing is explained.
#[component]
pub fn NotYet() -> impl IntoView {
    view! {
        <a href="/listing" class="notyet">"not yet"</a>
    }
}

/// Render a preformatted value, or the pre-listing mark where the
/// formatter said N/A.
pub fn value_or_notyet(text: String) -> AnyView {
    if text == "N/A" {
        view! { <NotYet /> }.into_any()
    } else {
        view! { <span>{text}</span> }.into_any()
    }
}
