use leptos::prelude::*;
use pulldown_cmark::{html, Options, Parser};
use wasm_bindgen::JsCast;
use crate::components::nav::SiteNav;

const METHODOLOGY_MD: &str = include_str!("../../doctrine.md");

#[component]
pub fn MethodologyPage() -> impl IntoView {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    let parser = Parser::new_ext(METHODOLOGY_MD, opts);
    let mut html_out = String::new();
    html::push_html(&mut html_out, parser);

    Effect::new(move |_| {
        document().set_title("Cyberstates doctrine — the sovereignty terminal");
    });

    view! {
        <div style="max-width: 1440px; margin: 0 auto; padding: 20px;">
            <div class="header-row1">
                <div class="logo-zone">
                    <a href="/" style="text-decoration: none;">
                        <h1 class="logo">
                            <span style="color: var(--cyber-green);">"CYBER"</span>
                            <span style="color: #fff;">"STATES"</span>
                        </h1>
                    </a>
                    <div class="logo-suffix">"doctrine"</div>
                </div>
                <input
                    type="text"
                    class="search-input"
                    placeholder="Search country, code, or currency..."
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            let target = ev.target().unwrap();
                            let input: web_sys::HtmlInputElement = target.unchecked_into();
                            let q = input.value();
                            if let Some(w) = web_sys::window() {
                                let _ = w.location().set_href(&format!("/?q={}", q));
                            }
                        }
                    }
                />
                <SiteNav active="DOCTRINE" />
            </div>

            <div class="prose-cyber" style="max-width: 860px; margin: 0 auto;" inner_html=html_out></div>
        </div>
    }
}
