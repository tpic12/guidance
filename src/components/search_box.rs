use leptos::prelude::*;

#[component]
pub fn SearchBox(value: RwSignal<String>, placeholder: &'static str) -> impl IntoView {
    view! {
        <input
            type="text"
            class="input input-bordered w-full max-w-sm"
            placeholder=placeholder
            prop:value=move || value.get()
            on:input=move |ev| value.set(event_target_value(&ev))
        />
    }
}
