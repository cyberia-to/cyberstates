use leptos::prelude::*;
use crate::numeraires::{load_numeraire, Numeraire};
use leptos_router::components::*;
use leptos_router::path;

use crate::pages::home::HomePage;
use crate::pages::country::CountryPage;
use crate::pages::token::TokenPage;
use crate::pages::tokens::TokensPage;
use crate::pages::methodology::MethodologyPage;

/// Publish the sticky chrome's height as --chrome-h on <html>, so table
/// headers and the map know where to stick. Re-measured after every
/// navigation (chrome height differs per page) and on resize.
fn measure_chrome() {
    use wasm_bindgen::JsCast;
    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        if let (Ok(Some(chrome)), Some(root)) = (doc.query_selector(".site-chrome"), doc.document_element()) {
            let h = chrome.unchecked_into::<web_sys::HtmlElement>().offset_height();
            let _ = root.set_attribute("style", &format!("--chrome-h: {}px", h));
        }
    }
}

#[component]
fn ChromeMeter() -> impl IntoView {
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    let location = leptos_router::hooks::use_location();

    // once: keep the var honest across viewport changes
    Effect::new(move |_| {
        let on_resize = Closure::wrap(Box::new(measure_chrome) as Box<dyn FnMut()>);
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback("resize", on_resize.as_ref().unchecked_ref());
        }
        on_resize.forget();
    });

    // per navigation: measure once the new page has rendered
    Effect::new(move |_| {
        let _ = location.pathname.get();
        if let Some(w) = web_sys::window() {
            let cb = Closure::wrap(Box::new(measure_chrome) as Box<dyn FnMut()>);
            let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), 120);
            cb.forget();
        }
    });

    view! { <span style="display: none;"></span> }
}

#[component]
pub fn App() -> impl IntoView {
    // one measure for the whole terminal, alive across pages and reloads
    let numeraire = RwSignal::new(load_numeraire());
    provide_context::<RwSignal<Numeraire>>(numeraire);

    view! {
        <Router>
            <ChromeMeter />
            <Routes fallback=|| view! { <p style="color: var(--cyber-red); padding: 40px;">"404 — NOT FOUND"</p> }>
                <Route path=path!("/") view=HomePage />
                <Route path=path!("/by/:field") view=HomePage />
                <Route path=path!("/by/:field/asc") view=HomePage />
                <Route path=path!("/in/:region") view=HomePage />
                <Route path=path!("/in/:region/by/:field") view=HomePage />
                <Route path=path!("/in/:region/by/:field/asc") view=HomePage />
                <Route path=path!("/map") view=HomePage />
                <Route path=path!("/tokens") view=TokensPage />
                <Route path=path!("/tokens/by/:field") view=TokensPage />
                <Route path=path!("/doctrine") view=MethodologyPage />
                <Route path=path!("/methodology") view=MethodologyPage />
                <Route path=path!("/state/:code") view=CountryPage />
                <Route path=path!("/country/:code") view=CountryPage />
                <Route path=path!("/token/:code") view=TokenPage />
            </Routes>
        </Router>
    }
}
