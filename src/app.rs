use crate::components::card::Card;
use crate::components::nav_bar::NavBar;
use crate::models::user::CurrentUser;
use crate::pages::{
    backgrounds::BackgroundsPage,
    character_builder::CharacterBuilderPage,
    character_sheet::CharacterSheetPage,
    characters::CharactersPage,
    compendium::CompendiumPage,
    homebrew::HomebrewPage,
    login::{get_current_user, LoginPage},
    settings::SettingsPage,
    class_detail::ClassDetailPage,
    classes::ClassesPage,
    feats::FeatsPage,
    items::ItemsPage,
    optional_features::OptionalFeaturesPage,
    species::SpeciesPage,
    spells::SpellsPage
};
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{ProtectedParentRoute, Route, Router, Routes},
    path,
};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <link rel="icon" type="image/svg+xml" href="/favicon.svg"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

/// The logged-in user (or `None`), shared via context so the route guard,
/// `NavBar`, and `SettingsPage` all read the same in-flight/resolved value
/// instead of each independently re-fetching "who am I".
pub type CurrentUserResource = Resource<Result<Option<CurrentUser>, ServerFnError>>;

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    let current_user: CurrentUserResource =
        Resource::new(|| (), |_| async move { get_current_user().await });
    provide_context(current_user);

    // Effects only run client-side after hydration, so this marks the moment
    // event handlers are live — the e2e tests wait on it before interacting.
    Effect::new(|_| {
        if let Some(body) = document().body() {
            let _ = body.set_attribute("data-hydrated", "");
        }
    });

    // Client-side only (like the effect above), so logged-out/pre-hydration
    // views fall back to the `--default` theme configured in style/main.css.
    Effect::new(move |_| {
        if let Some(Ok(Some(user))) = current_user.get() {
            if let Some(html) = document().document_element() {
                let _ = html.set_attribute("data-theme", &user.theme);
            }
        }
    });

    view! {
        <Stylesheet id="leptos" href="/pkg/guidance.css"/>

        <Title text="Guidance"/>

        <Router>
            <Routes fallback=|| "Page not found.".into_view()>
                <Route path=path!("login") view=LoginPage/>
                <ProtectedParentRoute
                    path=path!("")
                    view=NavBar
                    condition=move || current_user.get().map(|user| matches!(user, Ok(Some(_))))
                    redirect_path=|| "/login"
                >
                    <Route path=path!("") view=HomePage/>
                    <Route path=path!("compendium") view=CompendiumPage/>
                    <Route path=path!("compendium/spells") view=SpellsPage/>
                    <Route path=path!("compendium/items") view=ItemsPage/>
                    <Route path=path!("compendium/classes") view=ClassesPage/>
                    <Route path=path!("compendium/classes/:id") view=ClassDetailPage/>
                    <Route path=path!("compendium/backgrounds") view=BackgroundsPage/>
                    <Route path=path!("compendium/feats") view=FeatsPage/>
                    <Route path=path!("compendium/optional-features") view=OptionalFeaturesPage/>
                    <Route path=path!("compendium/species") view=SpeciesPage/>
                    <Route path=path!("characters") view=CharactersPage/>
                    <Route path=path!("characters/new") view=CharacterBuilderPage/>
                    <Route path=path!("characters/:id") view=CharacterSheetPage/>
                    <Route path=path!("characters/:id/edit") view=CharacterBuilderPage/>
                    <Route path=path!("homebrew") view=HomebrewPage/>
                    <Route path=path!("settings") view=SettingsPage/>
                </ProtectedParentRoute>
            </Routes>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    let current_user = use_context::<CurrentUserResource>();
    let username = move || {
        current_user
            .and_then(|resource| resource.get())
            .and_then(|user| user.ok())
            .flatten()
            .map(|user| user.username)
    };

    let greeting = move || match username() {
        Some(name) => format!("Welcome back, {name}"),
        None => "Welcome back".to_string(),
    };

    view! {
        <div class="flex flex-col gap-8">
            <div>
                <p class="font-mono text-xs tracking-[0.25em] text-primary/80 mb-2">"AT THE TABLE"</p>
                <h1 class="font-display text-3xl font-semibold mb-1">{greeting}</h1>
                <p class="text-sm opacity-70">"Pick up a character, or browse what's on the shelf."</p>
            </div>
            <div class="w-full grid grid-cols-1 sm:grid-cols-2 gap-4 max-w-2xl">
                <Card
                    label="Characters".to_string()
                    source="/characters".to_string()
                    icon=include_str!("assets/characters_icon.svg").to_string()
                    code="CHR"
                />
                <Card
                    label="Compendium".to_string()
                    source="/compendium".to_string()
                    icon=include_str!("assets/book_icon.svg").to_string()
                    code="COM"
                />
            </div>
        </div>
    }
}
