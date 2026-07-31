use crate::app::CurrentUserResource;
use crate::models::user::{Permission, UserSummary, THEMES};
use leptos::prelude::*;

#[server]
async fn change_username(new_username: String) -> Result<(), ServerFnError> {
    use sqlx::SqlitePool;
    use tower_sessions::Session;

    let pool = expect_context::<SqlitePool>();
    let session: Session = leptos_axum::extract().await?;
    let user = crate::auth::require_login(&pool, &session).await?;

    crate::db::update_username(&pool, &user.id, &new_username)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))
}

#[server]
async fn change_password(new_password: String) -> Result<(), ServerFnError> {
    use sqlx::SqlitePool;
    use tower_sessions::Session;

    let pool = expect_context::<SqlitePool>();
    let session: Session = leptos_axum::extract().await?;
    let user = crate::auth::require_login(&pool, &session).await?;

    crate::db::update_password(&pool, &user.id, &new_password)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))
}

#[server]
async fn change_theme(theme: String) -> Result<(), ServerFnError> {
    use sqlx::SqlitePool;
    use tower_sessions::Session;

    if !THEMES.contains(&theme.as_str()) {
        return Err(ServerFnError::new("Unknown theme"));
    }

    let pool = expect_context::<SqlitePool>();
    let session: Session = leptos_axum::extract().await?;
    let user = crate::auth::require_login(&pool, &session).await?;

    crate::db::update_theme(&pool, &user.id, &theme)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))
}

#[server]
async fn list_users_admin() -> Result<Vec<UserSummary>, ServerFnError> {
    use sqlx::SqlitePool;
    use tower_sessions::Session;

    let pool = expect_context::<SqlitePool>();
    let session: Session = leptos_axum::extract().await?;
    crate::auth::require_permission(&pool, &session, Permission::ManageUsers).await?;

    crate::db::list_users(&pool).await.map_err(|err| ServerFnError::new(err.to_string()))
}

#[server]
async fn create_user_admin(username: String, password: String) -> Result<(), ServerFnError> {
    use sqlx::SqlitePool;
    use tower_sessions::Session;

    let pool = expect_context::<SqlitePool>();
    let session: Session = leptos_axum::extract().await?;
    crate::auth::require_permission(&pool, &session, Permission::ManageUsers).await?;

    crate::db::create_user(&pool, &username, &password, false)
        .await
        .map(|_| ())
        .map_err(|err| ServerFnError::new(err.to_string()))
}

#[server]
async fn grant_permission_admin(user_id: String, permission: Permission) -> Result<(), ServerFnError> {
    use sqlx::SqlitePool;
    use tower_sessions::Session;

    let pool = expect_context::<SqlitePool>();
    let session: Session = leptos_axum::extract().await?;
    let admin = crate::auth::require_permission(&pool, &session, Permission::ManageUsers).await?;

    crate::db::grant_permission(&pool, &user_id, permission, &admin.id)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))
}

#[server]
async fn revoke_permission_admin(user_id: String, permission: Permission) -> Result<(), ServerFnError> {
    use sqlx::SqlitePool;
    use tower_sessions::Session;

    let pool = expect_context::<SqlitePool>();
    let session: Session = leptos_axum::extract().await?;
    crate::auth::require_permission(&pool, &session, Permission::ManageUsers).await?;

    crate::db::revoke_permission(&pool, &user_id, permission)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    Account,
    Users,
}

#[component]
pub fn SettingsPage() -> impl IntoView {
    let current_user = expect_context::<CurrentUserResource>();
    let active_tab = RwSignal::new(SettingsTab::Account);

    let is_admin = move || {
        current_user
            .get()
            .and_then(|user| user.ok())
            .flatten()
            .map(|user| user.is_admin())
            .unwrap_or(false)
    };

    view! {
        <div class="w-[70%] min-w-[40rem]">
            <div role="tablist" class="tabs tabs-lift mb-4">
                <a
                    role="tab"
                    class="tab"
                    class:tab-active=move || active_tab.get() == SettingsTab::Account
                    on:click=move |_| active_tab.set(SettingsTab::Account)
                >
                    "Account"
                </a>
                <Show when=is_admin fallback=|| ()>
                    <a
                        role="tab"
                        class="tab"
                        class:tab-active=move || active_tab.get() == SettingsTab::Users
                        on:click=move |_| active_tab.set(SettingsTab::Users)
                    >
                        "Users"
                    </a>
                </Show>
            </div>

            <Show when=move || active_tab.get() == SettingsTab::Account fallback=|| ()>
                <YourAccountCard current_user=current_user/>
            </Show>
            <Show when=move || is_admin() && active_tab.get() == SettingsTab::Users fallback=|| ()>
                <ManageUsersCard/>
            </Show>
        </div>
    }
}

#[component]
fn YourAccountCard(current_user: CurrentUserResource) -> impl IntoView {
    let new_username = RwSignal::new(String::new());
    let new_password = RwSignal::new(String::new());

    Effect::new(move |_| {
        if let Some(Ok(Some(user))) = current_user.get() {
            new_username.set(user.username);
        }
    });

    let change_username_action = Action::new(move |new_username: &String| {
        let new_username = new_username.clone();
        async move { change_username(new_username).await }
    });
    let change_password_action = Action::new(move |new_password: &String| {
        let new_password = new_password.clone();
        async move { change_password(new_password).await }
    });
    let change_theme_action = Action::new(move |theme: &String| {
        let theme = theme.clone();
        async move { change_theme(theme).await }
    });

    Effect::new(move |_| {
        if let Some(Ok(())) = change_username_action.value().get() {
            current_user.refetch();
        }
    });
    Effect::new(move |_| {
        if let Some(Ok(())) = change_password_action.value().get() {
            new_password.set(String::new());
        }
    });
    Effect::new(move |_| {
        if let Some(Ok(())) = change_theme_action.value().get() {
            current_user.refetch();
        }
    });

    let current_theme = move || {
        current_user
            .get()
            .and_then(|user| user.ok())
            .flatten()
            .map(|user| user.theme)
            .unwrap_or_else(|| "guidance".to_string())
    };

    view! {
        <div class="card bg-base-100 shadow-xl">
            <div class="card-body gap-4">
                <h2 class="card-title text-primary">"Your account"</h2>

                <form
                    class="flex flex-wrap items-end gap-2"
                    on:submit=move |ev| {
                        ev.prevent_default();
                        change_username_action.dispatch(new_username.get());
                    }
                >
                    <label class="form-control">
                        <span class="label-text mb-1">"Username"</span>
                        <input
                            type="text"
                            class="input input-bordered"
                            prop:value=move || new_username.get()
                            on:input=move |ev| new_username.set(event_target_value(&ev))
                        />
                    </label>
                    <button type="submit" class="btn btn-primary">"Update username"</button>
                    {move || {
                        match change_username_action.value().get() {
                            Some(Err(err)) => {
                                view! { <span class="text-error text-sm">{err.to_string()}</span> }
                                    .into_any()
                            }
                            _ => ().into_any(),
                        }
                    }}
                </form>

                <form
                    class="flex flex-wrap items-end gap-2"
                    on:submit=move |ev| {
                        ev.prevent_default();
                        change_password_action.dispatch(new_password.get());
                    }
                >
                    <label class="form-control">
                        <span class="label-text mb-1">"New password"</span>
                        <input
                            type="password"
                            class="input input-bordered"
                            autocomplete="new-password"
                            prop:value=move || new_password.get()
                            on:input=move |ev| new_password.set(event_target_value(&ev))
                        />
                    </label>
                    <button type="submit" class="btn btn-primary">"Update password"</button>
                    {move || {
                        match change_password_action.value().get() {
                            Some(Ok(())) => {
                                view! { <span class="text-success text-sm">"Password updated."</span> }
                                    .into_any()
                            }
                            Some(Err(err)) => {
                                view! { <span class="text-error text-sm">{err.to_string()}</span> }
                                    .into_any()
                            }
                            None => ().into_any(),
                        }
                    }}
                </form>

                <div class="flex flex-wrap items-end gap-2">
                    <label class="form-control">
                        <span class="label-text mb-1">"Theme"</span>
                        <select
                            class="select select-bordered"
                            prop:value=current_theme
                            on:change=move |ev| {
                                change_theme_action.dispatch(event_target_value(&ev));
                            }
                        >
                            {THEMES
                                .into_iter()
                                .map(|theme| {
                                    view! { <option value=theme>{theme_label(theme)}</option> }
                                })
                                .collect_view()}
                        </select>
                    </label>
                    {move || {
                        match change_theme_action.value().get() {
                            Some(Err(err)) => {
                                view! { <span class="text-error text-sm">{err.to_string()}</span> }
                                    .into_any()
                            }
                            _ => ().into_any(),
                        }
                    }}
                </div>
            </div>
        </div>
    }
}

fn theme_label(theme: &str) -> String {
    let mut chars = theme.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[component]
fn ManageUsersCard() -> impl IntoView {
    let users = Resource::new(|| (), |_| async move { list_users_admin().await });

    let new_user_username = RwSignal::new(String::new());
    let new_user_password = RwSignal::new(String::new());

    let create_user_action = Action::new(
        move |(username, password): &(String, String)| {
            let username = username.clone();
            let password = password.clone();
            async move { create_user_admin(username, password).await }
        },
    );
    let toggle_permission_action = Action::new(
        move |(user_id, permission, currently_granted): &(String, Permission, bool)| {
            let user_id = user_id.clone();
            let permission = *permission;
            let currently_granted = *currently_granted;
            async move {
                if currently_granted {
                    revoke_permission_admin(user_id, permission).await
                } else {
                    grant_permission_admin(user_id, permission).await
                }
            }
        },
    );

    Effect::new(move |_| {
        if let Some(Ok(())) = create_user_action.value().get() {
            new_user_username.set(String::new());
            new_user_password.set(String::new());
            users.refetch();
        }
    });
    Effect::new(move |_| {
        if let Some(Ok(())) = toggle_permission_action.value().get() {
            users.refetch();
        }
    });

    view! {
        <div class="card bg-base-100 shadow-xl">
            <div class="card-body gap-4">
                <h2 class="card-title text-primary">"Manage users"</h2>

                <div class="overflow-auto border border-base-300 rounded-box">
                    <table class="table">
                        <thead>
                            <tr>
                                <th>"Username"</th>
                                {Permission::ALL
                                    .into_iter()
                                    .map(|permission| view! { <th>{permission.label()}</th> })
                                    .collect_view()}
                            </tr>
                        </thead>
                        <tbody>
                            <Suspense fallback=move || {
                                view! {
                                    <tr>
                                        <td colspan="5">"Loading users..."</td>
                                    </tr>
                                }
                            }>
                                {move || Suspend::new(async move {
                                    match users.await {
                                        Ok(rows) => {
                                            rows.into_iter()
                                                .map(|user| view! { <UserRow user=user toggle_permission=toggle_permission_action/> })
                                                .collect_view()
                                                .into_any()
                                        }
                                        Err(err) => {
                                            view! {
                                                <tr>
                                                    <td colspan="5">
                                                        {format!("Failed to load users: {err}")}
                                                    </td>
                                                </tr>
                                            }
                                                .into_any()
                                        }
                                    }
                                })}
                            </Suspense>
                        </tbody>
                    </table>
                </div>

                <form
                    class="flex flex-wrap items-end gap-2"
                    on:submit=move |ev| {
                        ev.prevent_default();
                        create_user_action.dispatch((new_user_username.get(), new_user_password.get()));
                    }
                >
                    <label class="form-control">
                        <span class="label-text mb-1">"New user's username"</span>
                        <input
                            type="text"
                            class="input input-bordered"
                            prop:value=move || new_user_username.get()
                            on:input=move |ev| new_user_username.set(event_target_value(&ev))
                        />
                    </label>
                    <label class="form-control">
                        <span class="label-text mb-1">"New user's password"</span>
                        <input
                            type="password"
                            class="input input-bordered"
                            prop:value=move || new_user_password.get()
                            on:input=move |ev| new_user_password.set(event_target_value(&ev))
                        />
                    </label>
                    <button type="submit" class="btn btn-primary">"Add user"</button>
                    {move || {
                        match create_user_action.value().get() {
                            Some(Err(err)) => {
                                view! { <span class="text-error text-sm">{err.to_string()}</span> }
                                    .into_any()
                            }
                            _ => ().into_any(),
                        }
                    }}
                </form>
            </div>
        </div>
    }
}

#[component]
fn UserRow(
    user: UserSummary,
    toggle_permission: Action<(String, Permission, bool), Result<(), ServerFnError>>,
) -> impl IntoView {
    let user_id = user.id.clone();

    view! {
        <tr>
            <td>{user.username.clone()}</td>
            {if user.is_instance_owner {
                view! {
                    <td colspan="4">
                        <span class="badge badge-primary">"Owner (all permissions)"</span>
                    </td>
                }
                    .into_any()
            } else {
                Permission::ALL
                    .into_iter()
                    .map(|permission| {
                        let user_id = user_id.clone();
                        let granted = user.permissions.contains(&permission);
                        view! {
                            <td>
                                <input
                                    type="checkbox"
                                    class="checkbox checkbox-sm"
                                    prop:checked=granted
                                    on:change=move |_| {
                                        toggle_permission.dispatch((user_id.clone(), permission, granted));
                                    }
                                />
                            </td>
                        }
                    })
                    .collect_view()
                    .into_any()
            }}
        </tr>
    }
}
