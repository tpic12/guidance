use crate::app::CurrentUserResource;
use crate::models::user::CurrentUser;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;

#[server]
pub async fn get_current_user() -> Result<Option<CurrentUser>, ServerFnError> {
    use sqlx::SqlitePool;
    use tower_sessions::Session;

    // This resource lives at the App level, above routing, so it's also
    // polled for requests that never reach a matched page (e.g. the 404
    // fallback) — those don't go through `leptos_routes_with_context`'s
    // pool-providing closure, so treat a missing pool as logged-out rather
    // than panicking and dropping the connection.
    let Some(pool) = use_context::<SqlitePool>() else {
        return Ok(None);
    };
    let session: Session = leptos_axum::extract().await?;
    crate::auth::current_user(&pool, &session).await
}

#[server]
pub async fn login(username: String, password: String) -> Result<(), ServerFnError> {
    use sqlx::SqlitePool;
    use tower_sessions::Session;

    let pool = expect_context::<SqlitePool>();
    let session: Session = leptos_axum::extract().await?;

    let user_id = crate::db::authenticate(&pool, &username, &password)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))?
        .ok_or_else(|| ServerFnError::new("Invalid username or password"))?;

    crate::auth::login(&session, user_id).await
}

#[component]
pub fn LoginPage() -> impl IntoView {
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let navigate = use_navigate();

    let current_user = use_context::<CurrentUserResource>();

    let login_action = Action::new(move |(username, password): &(String, String)| {
        let username = username.clone();
        let password = password.clone();
        async move { login(username, password).await }
    });

    Effect::new(move |_| {
        if let Some(Ok(())) = login_action.value().get() {
            let Some(current_user) = current_user else { return };
            // Wait for the refetch to actually resolve before navigating —
            // navigating immediately races the still-stale "logged out"
            // resource value, which bounces ProtectedParentRoute straight
            // back to /login.
            if matches!(current_user.get(), Some(Ok(Some(_)))) {
                navigate("/", NavigateOptions::default());
            } else {
                current_user.refetch();
            }
        }
    });

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        login_action.dispatch((username.get(), password.get()));
    };

    view! {
        <div class="min-h-screen flex items-center justify-center">
            <div class="max-w-sm w-full card bg-base-100 border border-base-300 shadow-xl">
                <div class="card-body">
                    <h2 class="card-title text-primary">"Sign in to Guidance"</h2>
                    <form class="flex flex-col gap-3" on:submit=on_submit>
                        <label class="form-control">
                            <span class="label-text mb-1">"Username"</span>
                            <input
                                type="text"
                                class="input input-bordered w-full"
                                autocomplete="username"
                                prop:value=move || username.get()
                                on:input=move |ev| username.set(event_target_value(&ev))
                            />
                        </label>
                        <label class="form-control">
                            <span class="label-text mb-1">"Password"</span>
                            <input
                                type="password"
                                class="input input-bordered w-full"
                                autocomplete="current-password"
                                prop:value=move || password.get()
                                on:input=move |ev| password.set(event_target_value(&ev))
                            />
                        </label>
                        {move || {
                            match login_action.value().get() {
                                Some(Err(err)) => {
                                    view! {
                                        <div class="alert alert-error text-sm py-2">
                                            {err.to_string()}
                                        </div>
                                    }
                                        .into_any()
                                }
                                _ => ().into_any(),
                            }
                        }}
                        <button
                            type="submit"
                            class="btn btn-primary mt-2"
                            disabled=move || login_action.pending().get()
                        >
                            "Log in"
                        </button>
                    </form>
                </div>
            </div>
        </div>
    }
}
