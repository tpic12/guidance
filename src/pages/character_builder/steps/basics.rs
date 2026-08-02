use leptos::prelude::*;

pub fn basics_step(name: RwSignal<String>, char_level: Memo<u8>, enforce_prereqs: RwSignal<bool>) -> impl IntoView {
    view! {
        <h2 class="card-title">"Basics"</h2>
        <label class="form-control w-full max-w-sm">
            <span class="label-text mb-1">"Character name"</span>
            <input
                type="text"
                class="input input-bordered"
                placeholder="e.g. Brenna Ironquill"
                prop:value=move || name.get()
                on:input=move |ev| name.set(event_target_value(&ev))
            />
        </label>
        <div class="flex flex-col gap-1">
            <span class="label-text">"Total level"</span>
            <span class="font-mono text-2xl">
                {move || format!("{:02} / 20", char_level.get())}
            </span>
            <span class="text-xs opacity-70">
                "Set on the Class step — add a class there to begin."
            </span>
        </div>
        <div class="flex flex-col gap-1">
            <span class="label-text">"Multiclass ability prerequisites"</span>
            <div role="tablist" class="tabs tabs-box w-fit">
                {[(true, "Enforce"), (false, "Off")]
                    .map(|(value, label)| {
                        view! {
                            <a
                                role="tab"
                                class=move || {
                                    if enforce_prereqs.get() == value {
                                        "tab tab-active"
                                    } else {
                                        "tab"
                                    }
                                }
                                on:click=move |_| enforce_prereqs.set(value)
                            >
                                {label}
                            </a>
                        }
                    })}
            </div>
            <span class="text-xs opacity-70">
                "When enforced, adding or keeping a class requires the ability scores 5e's rules call for (e.g. 13 Strength for Fighter)."
            </span>
        </div>
    }
}
