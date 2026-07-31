use crate::models::class::{ability_label, ClassSummary};
use leptos::prelude::*;

#[server]
pub async fn get_classes() -> Result<Vec<ClassSummary>, ServerFnError> {
    use sqlx::SqlitePool;

    let pool = expect_context::<SqlitePool>();
    crate::db::list_classes(&pool)
        .await
        .map_err(|err| ServerFnError::new(err.to_string()))
}

#[component]
pub fn ClassesPage() -> impl IntoView {
    let classes = Resource::new(|| (), |_| async move { get_classes().await });

    view! {
        <div class="flex flex-col gap-4">
            <a href="/compendium" class="btn btn-ghost btn-sm w-fit gap-2">
                "← Back to Compendium"
            </a>
            <h1 class="text-3xl">"Classes"</h1>

            <Suspense fallback=move || {
                view! { <p>"Loading classes..."</p> }
            }>
                {move || Suspend::new(async move {
                    match classes.await {
                        Ok(classes) if classes.is_empty() => {
                            view! {
                                <p class="opacity-70">
                                    "No classes imported yet. Run the seed script to load fixtures."
                                </p>
                            }
                                .into_any()
                        }
                        Ok(classes) => {
                            view! {
                                <div class="w-full grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-4">
                                    {classes.into_iter().map(class_card).collect_view()}
                                </div>
                            }
                                .into_any()
                        }
                        Err(err) => {
                            view! {
                                <p class="text-error">{format!("Failed to load classes: {err}")}</p>
                            }
                                .into_any()
                        }
                    }
                })}
            </Suspense>
        </div>
    }
}

fn class_card(class: ClassSummary) -> impl IntoView {
    let saves = class
        .saving_throws
        .iter()
        .map(|code| ability_label(code))
        .collect::<Vec<_>>()
        .join(", ");

    view! {
        <a
            href=format!("/compendium/classes/{}", class.id)
            class="card card-border bg-neutral text-neutral-content w-full shadow-sm hover:border-current"
        >
            <div class="card-body">
                <h2 class="card-title">
                    {class.name}
                    <span class="badge badge-outline badge-sm font-normal">{class.source}</span>
                </h2>
                <ul class="text-sm space-y-1">
                    <li>
                        <span class="font-semibold">"Hit Die: "</span>
                        {format!("d{}", class.hit_die)}
                    </li>
                    <li>
                        <span class="font-semibold">"Saving Throws: "</span>
                        {saves}
                    </li>
                    <li>
                        <span class="font-semibold">"Subclasses: "</span>
                        {class.subclass_count}
                    </li>
                </ul>
            </div>
        </a>
    }
}
