use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;

use crate::pages::home::HomePage;
use crate::pages::country::CountryPage;
use crate::pages::token::TokenPage;
use crate::pages::tokens::TokensPage;
use crate::pages::methodology::MethodologyPage;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=|| view! { <p style="color: var(--cyber-red); padding: 40px;">"404 — NOT FOUND"</p> }>
                <Route path=path!("/") view=HomePage />
                <Route path=path!("/by/:field") view=HomePage />
                <Route path=path!("/by/:field/asc") view=HomePage />
                <Route path=path!("/in/:region") view=HomePage />
                <Route path=path!("/in/:region/by/:field") view=HomePage />
                <Route path=path!("/in/:region/by/:field/asc") view=HomePage />
                <Route path=path!("/map") view=HomePage />
                <Route path=path!("/tokens") view=TokensPage />
                <Route path=path!("/methodology") view=MethodologyPage />
                <Route path=path!("/state/:code") view=CountryPage />
                <Route path=path!("/country/:code") view=CountryPage />
                <Route path=path!("/token/:code") view=TokenPage />
            </Routes>
        </Router>
    }
}
