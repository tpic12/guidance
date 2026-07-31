use leptos::prelude::*;

/// One toggleable pill — the building block every filter category's chip row
/// is made of. Plain function (not `#[component]`) since `on_toggle` needs to
/// be a generic closure, which the `#[component]` macro doesn't take cleanly.
pub fn filter_chip(label: String, selected: Signal<bool>, on_toggle: impl Fn() + Send + Sync + 'static) -> impl IntoView {
    view! {
        <button
            type="button"
            aria-pressed=move || selected.get().to_string()
            class=move || {
                if selected.get() {
                    "btn btn-sm btn-primary"
                } else {
                    "btn btn-sm btn-outline"
                }
            }
            on:click=move |_| on_toggle()
        >
            {label}
        </button>
    }
}

/// Where a `filter_chip_group`'s option list comes from — a compile-time
/// list (`School::ALL`) needs nothing special, but a server-fetched list
/// (spell sources, linked classes, ...) is backed by a `Resource` and MUST
/// be read inside a `<Suspense>` or it hydrates with mismatched server/client
/// HTML and silently breaks the page's interactivity. Baking that branch in
/// here — rather than leaving each page to remember its own `<Suspense>`
/// wrapper — is what lets any number of filter categories, static or async,
/// go through the exact same call shape.
pub enum FilterOptions<T: Send + Sync + 'static> {
    Static(Vec<T>),
    Async(Resource<Result<Vec<T>, ServerFnError>>),
}

/// One filter-dialog section: a title, Select all/Reset controls, and a
/// wrapping row of `filter_chip`s — the "checkbox list" pattern every
/// compendium page's filter dialog used to hand-roll separately.
pub fn filter_chip_group<T, L>(title: &'static str, options: FilterOptions<T>, label: L, draft: RwSignal<Vec<T>>) -> impl IntoView
where
    T: PartialEq + Clone + Send + Sync + 'static,
    L: Fn(&T) -> String + Send + Sync + 'static,
{
    let label = StoredValue::new(label);
    match options {
        FilterOptions::Static(values) => chip_group_body(title, values, label, draft).into_any(),
        FilterOptions::Async(resource) => {
            view! {
                <Suspense fallback=move || view! { <p class="text-sm opacity-60">"Loading..."</p> }>
                    {move || Suspend::new(async move {
                        match resource.await {
                            Ok(values) => chip_group_body(title, values, label, draft).into_any(),
                            Err(err) => {
                                view! {
                                    <p class="text-sm text-error">{format!("Failed to load: {err}")}</p>
                                }
                                    .into_any()
                            }
                        }
                    })}
                </Suspense>
            }
                .into_any()
        }
    }
}

/// The actual chip-row markup, shared by both `FilterOptions` branches once
/// each has its concrete `Vec<T>` in hand (immediately for `Static`, after
/// the `Resource` resolves for `Async`).
fn chip_group_body<T, L>(title: &'static str, values: Vec<T>, label: StoredValue<L>, draft: RwSignal<Vec<T>>) -> impl IntoView
where
    T: PartialEq + Clone + Send + Sync + 'static,
    L: Fn(&T) -> String + Send + Sync + 'static,
{
    let values_for_select_all = values.clone();
    let toggle = move |value: T| {
        draft.update(|values| match values.iter().position(|v| *v == value) {
            Some(pos) => {
                values.remove(pos);
            }
            None => values.push(value),
        });
    };

    view! {
        <div>
            <div class="border border-secondary rounded-box p-3 flex items-center justify-between mb-2">
                <h4 class="font-mono text-xs tracking-[0.15em] uppercase text-secondary">{title}</h4>
                <div class="flex gap-1">
                    <button
                        type="button"
                        class="btn btn-ghost btn-xs"
                        on:click=move |_| draft.set(values_for_select_all.clone())
                    >
                        "Select all"
                    </button>
                    <button type="button" class="btn btn-ghost btn-xs" on:click=move |_| draft.set(Vec::new())>
                        "Reset"
                    </button>
                </div>
            </div>
            <div class="flex flex-wrap gap-2 rounded-box p-3">
                {values
                    .into_iter()
                    .map(|value| {
                        let selected_value = value.clone();
                        let selected = Signal::derive(move || draft.get().contains(&selected_value));
                        let toggle_value = value.clone();
                        let text = label.with_value(|f| f(&value));
                        filter_chip(text, selected, move || toggle(toggle_value.clone()))
                    })
                    .collect_view()}
            </div>
        </div>
    }
}

/// Modal shell for a filter dialog: title, a responsive grid of
/// `filter_chip_group` sections, and Cancel/Save actions. The page still
/// owns the `dialog_ref` (it needs it to call `show_modal()` from its own
/// "Filters" button) and the draft/applied signal bookkeeping behind
/// `on_cancel`/`on_save` — this only owns the modal's structure and styling.
pub fn filter_dialog(
    dialog_ref: NodeRef<leptos::html::Dialog>,
    title: &'static str,
    on_cancel: impl Fn() + Send + Sync + 'static,
    on_save: impl Fn() + Send + Sync + 'static,
    sections: Vec<AnyView>,
) -> impl IntoView {
    view! {
        <dialog node_ref=dialog_ref class="modal">
            <div class="modal-box max-w-4xl">
                <h3 class="font-bold text-lg mb-4">{title}</h3>
                <div class="flex flex-col gap-5">{sections}</div>
                <div class="modal-action">
                    <button type="button" class="btn btn-ghost" on:click=move |_| on_cancel()>
                        "Cancel"
                    </button>
                    <button type="button" class="btn btn-primary" on:click=move |_| on_save()>
                        "Save"
                    </button>
                </div>
            </div>
            <form method="dialog" class="modal-backdrop">
                <button>"close"</button>
            </form>
        </dialog>
    }
}

/// The "Filters" / "Filters (N)" button that opens a `filter_dialog`.
pub fn filter_button(active_count: impl Fn() -> usize + Send + Sync + 'static, on_click: impl Fn() + Send + Sync + 'static) -> impl IntoView {
    view! {
        <button type="button" class="btn btn-outline btn-sm" on:click=move |_| on_click()>
            {move || {
                let count = active_count();
                if count == 0 { "Filters".to_string() } else { format!("Filters ({count})") }
            }}
        </button>
    }
}
