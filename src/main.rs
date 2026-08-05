mod app;
mod components;
mod data;
mod numeraires;
mod pages;

fn main() {
    console_error_panic_hook::set_once();
    // Prerender leaves crawlable markup for bots; strip it before the
    // terminal mounts so static SEO never paints under the SPA.
    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        if let Some(seo) = doc.get_element_by_id("seo-static") {
            let _ = seo.remove();
        }
        if let Some(body) = doc.body() {
            while let Some(child) = body.first_child() {
                let _ = body.remove_child(&child);
            }
        }
    }
    leptos::mount::mount_to_body(app::App);
}
