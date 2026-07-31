use crate::models::character::CharacterSummary;
use leptos::prelude::*;

#[server]
pub async fn list_characters() -> Result<Vec<CharacterSummary>, ServerFnError> {
    use sqlx::SqlitePool;
    use tower_sessions::Session;

    let pool = expect_context::<SqlitePool>();
    let session: Session = leptos_axum::extract().await?;
    let user = crate::auth::require_login(&pool, &session).await?;
    crate::db::list_characters(&pool, &user.id)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))
}

#[server]
pub async fn delete_character(id: String) -> Result<(), ServerFnError> {
    use sqlx::SqlitePool;
    use tower_sessions::Session;

    let pool = expect_context::<SqlitePool>();
    let session: Session = leptos_axum::extract().await?;
    let user = crate::auth::require_login(&pool, &session).await?;
    crate::db::delete_character(&pool, &id, &user.id)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))
}

#[component]
pub fn CharactersPage() -> impl IntoView {
    let characters = Resource::new(|| (), |_| async move { list_characters().await });
    let pending_delete = RwSignal::new(None::<CharacterSummary>);
    let confirm_dialog = NodeRef::<leptos::html::Dialog>::new();

    let delete = Action::new(|id: &String| {
        let id = id.clone();
        async move { delete_character(id).await }
    });
    Effect::new(move |_| {
        if let Some(Ok(())) = delete.value().get() {
            characters.refetch();
        }
    });

    let request_delete = move |summary: CharacterSummary| {
        pending_delete.set(Some(summary));
        if let Some(dialog) = confirm_dialog.get() {
            let _ = dialog.show_modal();
        }
    };
    let close_confirm = move || {
        if let Some(dialog) = confirm_dialog.get() {
            dialog.close();
        }
    };

    view! {
        <div class="flex flex-col gap-4">
            <div class="flex items-center justify-between">
                <h1 class="text-3xl">"Characters"</h1>
                <a href="/characters/new" class="btn btn-primary">
                    "New Character"
                </a>
            </div>

            <dialog node_ref=confirm_dialog class="modal">
                <div class="modal-box">
                    <h3 class="font-bold text-lg">
                        {move || {
                            let name = pending_delete
                                .get()
                                .map(|summary| summary.name)
                                .unwrap_or_default();
                            format!("Delete {name}?")
                        }}
                    </h3>
                    <p class="py-2 text-sm opacity-70">"This can't be undone."</p>
                    <div class="modal-action">
                        <button type="button" class="btn btn-ghost" on:click=move |_| close_confirm()>
                            "Cancel"
                        </button>
                        <button
                            type="button"
                            class="btn btn-error"
                            on:click=move |_| {
                                if let Some(summary) = pending_delete.get() {
                                    delete.dispatch(summary.id);
                                }
                                close_confirm();
                            }
                        >
                            "Delete"
                        </button>
                    </div>
                </div>
                <form method="dialog" class="modal-backdrop">
                    <button>"close"</button>
                </form>
            </dialog>

            <Suspense fallback=move || view! { <p>"Loading characters..."</p> }>
                {move || Suspend::new(async move {
                    match characters.await {
                        Ok(rows) if rows.is_empty() => {
                            view! {
                                <p class="opacity-70">
                                    "No characters yet — create your first one."
                                </p>
                            }
                                .into_any()
                        }
                        Ok(rows) => {
                            view! {
                                <div class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-4">
                                    {rows
                                        .into_iter()
                                        .map(|summary| character_card(summary, request_delete))
                                        .collect_view()}
                                </div>
                            }
                                .into_any()
                        }
                        Err(err) => {
                            view! {
                                <p class="text-error">
                                    {format!("Failed to load characters: {err}")}
                                </p>
                            }
                                .into_any()
                        }
                    }
                })}
            </Suspense>
        </div>
    }
}

fn character_card(
    summary: CharacterSummary,
    request_delete: impl Fn(CharacterSummary) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let subtitle = {
        let class_name = summary.class_name.clone().unwrap_or_else(|| "Unknown class".to_string());
        let species_name =
            summary.species_name.clone().unwrap_or_else(|| "Unknown species".to_string());
        format!("Level {} {class_name} · {species_name}", summary.level)
    };
    let sheet_href = format!("/characters/{}", summary.id);
    let edit_href = format!("/characters/{}/edit", summary.id);
    let delete_summary = summary.clone();

    let level_badge = format!("LV {:02}", summary.level);

    view! {
        <div class="card ledger-tile rounded-box flex flex-col gap-3 p-5">
            <div class="flex items-start justify-between gap-2">
                <h2 class="font-display text-xl font-semibold leading-tight">{summary.name.clone()}</h2>
                <span class="font-mono text-[0.7rem] tracking-[0.15em] text-primary/80 shrink-0 pt-1">
                    {level_badge}
                </span>
            </div>
            <p class="text-sm opacity-70">{subtitle}</p>
            <div class="flex justify-end gap-2 mt-2">
                <a href=sheet_href class="btn btn-primary btn-sm">
                    "View Sheet"
                </a>
                <a href=edit_href class="btn btn-outline btn-sm">
                    "Edit"
                </a>
                <button
                    type="button"
                    class="btn btn-ghost btn-sm text-error"
                    on:click=move |_| request_delete(delete_summary.clone())
                >
                    "Delete"
                </button>
            </div>
        </div>
    }
}
