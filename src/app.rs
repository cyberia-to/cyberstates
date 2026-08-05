use leptos::prelude::*;
use crate::numeraires::{load_numeraire, Numeraire};
use leptos_router::components::*;
use leptos_router::path;

use crate::pages::home::HomePage;
use crate::pages::country::CountryPage;
use crate::pages::token::TokenPage;
use crate::pages::tokens::TokensPage;
use crate::pages::methodology::MethodologyPage;
use crate::pages::listing::ListingPage;
use crate::pages::solar_map::SolarMapPage;

/// Publish the sticky chrome's height as --chrome-h on <html>, so table
/// headers and the map know where to stick. Re-measured after every
/// navigation (chrome height differs per page) and on resize.
fn measure_chrome() {
    crate::pages::map::measure_chrome_height();
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

    // once: intercept internal links at capture phase, so every navigation
    // goes through navigate_client and its view-transition cross-fade —
    // the router's own instant handler never sees the click
    Effect::new(move |_| {
        let handler = Closure::wrap(Box::new(move |ev: web_sys::MouseEvent| {
            if ev.default_prevented()
                || ev.button() != 0
                || ev.meta_key() || ev.ctrl_key() || ev.shift_key() || ev.alt_key()
            {
                return;
            }
            let Some(el) = ev.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return };
            let Ok(Some(a)) = el.closest("a[href]") else { return };
            // internal links are root-relative; anything else is not ours
            let href = a.get_attribute("href").unwrap_or_default();
            if !href.starts_with('/') || a.get_attribute("target").is_some() {
                return;
            }
            ev.prevent_default();
            ev.stop_propagation();
            let current = web_sys::window()
                .map(|w| {
                    let l = w.location();
                    format!("{}{}", l.pathname().unwrap_or_default(), l.search().unwrap_or_default())
                })
                .unwrap_or_default();
            if href != current {
                crate::pages::map::navigate_client(&href);
            }
        }) as Box<dyn FnMut(web_sys::MouseEvent)>);
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            let _ = doc.add_event_listener_with_callback_and_bool(
                "click",
                handler.as_ref().unchecked_ref(),
                true,
            );
        }
        handler.forget();
    });

    // per navigation: measure after the dissolve has settled.
    // VT path also measures on transition.finished; this covers
    // back/forward and any hop that skipped startViewTransition.
    Effect::new(move |_| {
        let _ = location.pathname.get();
        if let Some(w) = web_sys::window() {
            let cb = Closure::wrap(Box::new(measure_chrome) as Box<dyn FnMut()>);
            let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                360,
            );
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
                <Route path=path!("/listing") view=ListingPage />
                <Route path=path!("/solar") view=SolarMapPage />
                <Route path=path!("/methodology") view=MethodologyPage />
                <Route path=path!("/state/:code") view=CountryPage />
                <Route path=path!("/country/:code") view=CountryPage />
                <Route path=path!("/token/:code") view=TokenPage />
            </Routes>
        </Router>
    }
}
