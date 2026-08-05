use crate::components::brand::BrandChooser;
use crate::components::nav::SiteNav;
use leptos::prelude::*;
use pulldown_cmark::{html, Options, Parser};
use wasm_bindgen::JsCast;

const METHODOLOGY_MD: &str = include_str!("../../doctrine.md");

#[component]
pub fn MethodologyPage() -> impl IntoView {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    // the logo already says Doctrine — the markdown H1 stays for GitHub readers
    let body = METHODOLOGY_MD
        .trim_start()
        .strip_prefix("# Doctrine")
        .unwrap_or(METHODOLOGY_MD);
    let parser = Parser::new_ext(body, opts);
    let mut html_out = String::new();
    html::push_html(&mut html_out, parser);

    Effect::new(move |_| {
        document().set_title("Cyberstates doctrine — the sovereignty terminal");
    });

    view! {
        <div class="page-frame">
            <div class="site-chrome">
            <div class="header-row1">
                <div class="logo-zone">
                    <BrandChooser active="DOCTRINE" />
                    <div class="logo-suffix"></div>
                </div>
                <SiteNav active="DOCTRINE" />
            </div>
            </div>

            <div class="prose-cyber" style="max-width: 860px; margin: 0 auto;" inner_html=html_out></div>

            <div class="search-dock">
                <input
                    type="text"
                    class="search-input"
                    placeholder="Search country, code, or currency..."
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            let target = ev.target().unwrap();
                            let input: web_sys::HtmlInputElement = target.unchecked_into();
                            let q = input.value();
                            crate::pages::map::navigate_client(&format!("/?q={}", q));
                        }
                    }
                />
                            <a href="https://x.com/cyberiacap" target="_blank" rel="noopener" class="dock-credit">
                    "🏴 a "<span style="color: var(--cyber-green);">"cyberia"</span>" project"
                </a>
            </div>
        </div>
    }
}
