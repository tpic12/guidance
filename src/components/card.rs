use leptos::prelude::*;

#[component]
pub fn Card(label: String, source: String, icon: String, code: &'static str) -> impl IntoView {
  view! {
    <a href=source class="ledger-tile rounded-box flex items-stretch gap-4 w-full h-36 p-5">
      <div
          class="flex items-center justify-center w-14 shrink-0 text-primary opacity-90"
          inner_html=icon
      />
      <div class="flex flex-col justify-center gap-1.5 min-w-0">
        <span class="font-mono text-[0.7rem] tracking-[0.2em] text-primary/80">{code}</span>
        <h2 class="font-display text-2xl font-semibold leading-tight truncate">{label}</h2>
      </div>
    </a>
  }
}