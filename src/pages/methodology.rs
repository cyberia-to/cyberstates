use leptos::prelude::*;
use pulldown_cmark::{html, Options, Parser};

const METHODOLOGY_MD: &str = include_str!("../../methodology.md");

#[component]
pub fn MethodologyPage() -> impl IntoView {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    let parser = Parser::new_ext(METHODOLOGY_MD, opts);
    let mut html_out = String::new();
    html::push_html(&mut html_out, parser);

    Effect::new(move |_| {
        document().set_title("Cyberstates methodology — Global Visa Openness Analytics");
    });

    view! {
        <div style="max-width: 860px; margin: 0 auto; padding: 20px;">
            <div class="header-row1">
                <div class="logo-zone">
                    <a href="/" style="text-decoration: none;">
                        <h1 class="logo">
                            <span style="color: var(--cyber-green);">"CYBER"</span>
                            <span style="color: #fff;">"STATES"</span>
                        </h1>
                    </a>
                    <div class="logo-suffix">"methodology"</div>
                </div>
                <div class="map-zone">
                    <a href="/" class="tokens-btn">"STATES"</a>
                    <a href="/map" class="map-btn">"MAP"</a>
                </div>
            </div>

            <div class="prose-cyber" inner_html=html_out></div>
        </div>
    }
}
