use crate::app::CurrentUserResource;
use leptos::prelude::*;
use leptos_meta::provide_meta_context;
use leptos_router::components::Outlet;
use leptos_router::hooks::{use_location, use_navigate};
use leptos_router::NavigateOptions;

#[server]
async fn logout() -> Result<(), ServerFnError> {
    use tower_sessions::Session;

    let session: Session = leptos_axum::extract().await?;
    crate::auth::logout(&session).await
}

#[derive(Clone, Debug, PartialEq)]
struct NavOption {
    key: String,
    label: String,
    source: String,
    icon: String
}

#[component]
pub fn NavBar() -> impl IntoView {
    provide_meta_context();

    let current_user = use_context::<CurrentUserResource>();
    let navigate = use_navigate();
    let location = use_location();

    let logout_action = Action::new(move |_: &()| async move { logout().await });

    Effect::new(move |_| {
        if let Some(Ok(())) = logout_action.value().get() {
            if let Some(current_user) = current_user {
                current_user.refetch();
            }
            navigate("/login", NavigateOptions::default());
        }
    });

    let username = move || {
        current_user
            .and_then(|resource| resource.get())
            .and_then(|user| user.ok())
            .flatten()
            .map(|user| user.username)
            .unwrap_or_default()
    };

    // Populate your nav choices
    let nav_options = vec![
        NavOption {
            key: "Characters".to_string(),
            label: "Characters".to_string(),
            source: "/characters".to_string(),
            icon: include_str!("../assets/characters_icon.svg").to_string()
        },
        NavOption {
            key: "Compendium".to_string(),
            label: "Compendium".to_string(),
            source: "/compendium".to_string(),
            icon: include_str!("../assets/book_icon.svg").to_string()
        },
        NavOption {
            key: "Homebrew".to_string(),
            label: "Homebrew".to_string(),
            source: "/homebrew".to_string(),
            icon: include_str!("../assets/potion_icon.svg").to_string()
        },
        NavOption {
            key: "Settings".to_string(),
            label: "Settings".to_string(),
            source: "/settings".to_string(),
            icon: include_str!("../assets/settings_icon.svg").to_string()
        },
    ];

    view! {
        <div class="drawer lg:drawer-open">
          <input id="my-drawer-4" type="checkbox" class="drawer-toggle" />
          <div class="drawer-content">
            // <!-- Navbar -->
            <nav class="navbar w-full bg-base-300 border-b border-base-content/10">
              <label
                for="my-drawer-4"
                aria-label="open sidebar"
                class="btn btn-square btn-ghost drawer-button lg:hidden"
              >
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" stroke-linejoin="round" stroke-linecap="round" stroke-width="2" fill="none" stroke="currentColor" class="size-5">
                  <path d="M4 6l16 0"></path>
                  <path d="M4 12l16 0"></path>
                  <path d="M4 18l16 0"></path>
                </svg>
              </label>
              <a href="/".to_string() class="px-4 flex items-baseline gap-1.5 font-display text-lg font-semibold tracking-tight">
                "Guidance"
                <span class="text-primary text-lg leading-none">"·"</span>
              </a>
              <div class="flex-1"></div>
              <span class="px-4 text-sm opacity-70">
                {move || {
                    let name = username();
                    if name.is_empty() { name } else { format!("Logged in as {name}") }
                }}
              </span>
            </nav>
            // <!-- Page content here -->
            // <div class="p-4">Page Content</div>
            <div class="p-8 grow bg-base-100 text-base-content">
              {/* Leptos will inject your active page view right here! */}
              <Outlet />
            </div>
          </div>

          <div class="drawer-side is-drawer-close:overflow-visible">
            <label for="my-drawer-4" aria-label="close sidebar" class="drawer-overlay"></label>
            <div class="flex min-h-full flex-col items-start bg-base-200 is-drawer-close:w-14 is-drawer-open:w-64">
              <ul class="menu w-full grow">
                <li class="max-lg:hidden">
                  <label
                    for="my-drawer-4"
                    class="is-drawer-close:tooltip is-drawer-close:tooltip-right flex items-center gap-2 w-full cursor-pointer"
                    data-tip="Toggle Menu"
                    aria-label="toggle sidebar"
                  >
                    {/* Sidebar toggle icon */}
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" stroke-linejoin="round" stroke-linecap="round" stroke-width="2" fill="none" stroke="currentColor" class="my-1.5 inline-block size-4">
                      <path d="M4 4m0 2a2 2 0 0 1 2 -2h12a2 2 0 0 1 2 2v12a2 2 0 0 1 -2 2h-12a2 2 0 0 1 -2 -2z"></path>
                      <path d="M9 4v16"></path>
                      <path d="M14 10l2 2l-2 2"></path>
                    </svg>
                    
                    {/* Hidden when collapsed, visible when open */}
                    <span class="is-drawer-close:hidden">"Collapse Menu"</span>
                  </label>
                </li>

                // // <!-- List item -->
                // <li>
                //   <button class="is-drawer-close:tooltip is-drawer-close:tooltip-right" data-tip="Homepage">
                //     // <!-- Home icon -->
                //     <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" stroke-linejoin="round" stroke-linecap="round" stroke-width="2" fill="none" stroke="currentColor" class="my-1.5 inline-block size-4"><path d="M15 21v-8a1 1 0 0 0-1-1h-4a1 1 0 0 0-1 1v8"></path><path d="M3 10a2 2 0 0 1 .709-1.528l7-5.999a2 2 0 0 1 2.582 0l7 5.999A2 2 0 0 1 21 10v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"></path></svg>
                //     <span class="is-drawer-close:hidden">Homepage</span>
                //   </button>
                // </li>

                // // <!-- List item -->
                // <li>
                //   <button class="is-drawer-close:tooltip is-drawer-close:tooltip-right" data-tip="Settings">
                //     // <!-- Settings icon -->
                //     <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" stroke-linejoin="round" stroke-linecap="round" stroke-width="2" fill="none" stroke="currentColor" class="my-1.5 inline-block size-4"><path d="M20 7h-9"></path><path d="M14 17H5"></path><circle cx="17" cy="17" r="3"></circle><circle cx="7" cy="7" r="3"></circle></svg>
                //     <span class="is-drawer-close:hidden">Settings</span>
                //   </button>
                // </li>
                <For
                  each=move || nav_options.clone()
                  key=|option| option.key.clone()
                  children=move |option| {
                      let is_active = {
                          let source = option.source.clone();
                          move || {
                              let path = location.pathname.get();
                              path == source || path.starts_with(&format!("{source}/"))
                          }
                      };
                      view! {
                          <li>
                              <a
                                href=option.source
                                class="nav-tab is-drawer-close:tooltip is-drawer-close:tooltip-right flex items-center gap-3 w-full text-left py-2"
                                class:nav-tab-active=is_active
                                data-tip=option.key
                              >
                              <span
                                  class="my-1.5 inline-flex items-center justify-center size-4 text-current"
                                  inner_html=option.icon
                              />
                              <span class="is-drawer-close:hidden">{option.label}</span>
                              </a>
                          </li>
                      }
                  }
                />
              </ul>

              <ul class="menu w-full border-t border-base-300">
                <li>
                  <button
                    type="button"
                    class="is-drawer-close:tooltip is-drawer-close:tooltip-right flex items-center gap-3 w-full text-left"
                    data-tip="Log out"
                    on:click=move |_| { logout_action.dispatch(()); }
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" stroke-linejoin="round" stroke-linecap="round" stroke-width="2" fill="none" stroke="currentColor" class="my-1.5 inline-block size-4">
                      <path d="M14 8v-2a2 2 0 0 0 -2 -2h-7a2 2 0 0 0 -2 2v12a2 2 0 0 0 2 2h7a2 2 0 0 0 2 -2v-2"></path>
                      <path d="M7 12h14l-3 -3"></path>
                      <path d="M18 15l3 -3"></path>
                    </svg>
                    <span class="is-drawer-close:hidden">"Log out"</span>
                  </button>
                </li>
              </ul>
            </div>
          </div>
        </div>
    }
}