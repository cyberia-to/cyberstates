use leptos::prelude::*;
use crate::data::*;
use crate::pages::country::SiteHeader;

/// /listing — where every "not yet" points. A quote that reads not yet
/// marks a territory listed before its token: the ground is real, the
/// first rule is unwritten.
#[component]
pub fn ListingPage() -> impl IntoView {
    Effect::new(move |_| {
        document().set_title("Not yet listed — the sovereignty terminal");
    });

    let pending: Vec<Country> = load_countries()
        .into_iter()
        .filter(|c| c.token_price_usd <= 0.0)
        .collect();

    view! {
        <div class="page-frame">
            <SiteHeader />
            <div class="prose-cyber" style="max-width: 860px; margin: 0 auto;">
                <h2>"NOT YET"</h2>
                <p>
                    "A quote that reads " <span class="notyet" style="cursor: default;">"not yet"</span>
                    " marks a listing before its token."
                </p>
                <p>
                    "Every state answers one question before any other — "
                    <em>"what is money here"</em>
                    " (" <a href="/doctrine">"Doctrine"</a> ", thesis 2). The territories below hold
                    ground under treaty, but none has given its answer: no mint, no price,
                    no supply, no capital. The terminal lists them anyway, because existence
                    precedes recognition — a market can see an asset long before it trades."
                </p>
                <p>
                    "When rules arrive and a token binds, the quote begins: price first,
                    capital the moment the world agrees to hold it. Until then the row
                    stands open — the visible supply curve of unmonetized space."
                </p>
                <div class="pending-grid">
                    {pending.into_iter().map(|c| {
                        let href = format!("/state/{}", c.code.to_lowercase());
                        view! {
                            <a href=href class="region-pill pending-pill">
                                <span style="margin-right: 6px;">{c.flag.clone()}</span>
                                {c.name.clone()}
                            </a>
                        }
                    }).collect_view()}
                </div>
                <p style="color: #555; font-size: 12px;">
                    "A few terrestrial states read not yet on capital alone: their token
                    trades, but no reliable money-stock measurement exists — a data gap,
                    not a missing mint (see the Doctrine's Appendix B)."
                </p>
            </div>
        </div>
    }
}
