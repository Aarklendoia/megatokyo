mod daemon_client;
mod push;
mod storage;

use leptos::prelude::*;
use leptos::task::spawn_local;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let initial = storage::load();
    let (base_url, set_base_url) = signal(initial.base_url);
    let (token, set_token) = signal(initial.token);
    let (chapters, set_chapters) = signal(None::<Result<Vec<String>, String>>);
    let (notifications_status, set_notifications_status) = signal(None::<Result<(), String>>);

    let save_settings = move |_| {
        storage::save(&storage::DaemonLink {
            base_url: base_url.get(),
            token: token.get(),
        });
    };

    let enable_notifications = move |_| {
        let base_url = base_url.get();
        let token = token.get();
        set_notifications_status.set(None);
        spawn_local(async move {
            let result = push::subscribe(&base_url, &token).await;
            set_notifications_status.set(Some(result));
        });
    };

    let load_chapters = move |_| {
        let base_url = base_url.get();
        let token = token.get();
        set_chapters.set(None);
        spawn_local(async move {
            let result = daemon_client::fetch_chapters(&base_url, &token)
                .await
                .map(|chs| {
                    chs.into_iter()
                        .map(|c| format!("#{} — {}", c.number, c.title))
                        .collect()
                });
            set_chapters.set(Some(result));
        });
    };

    view! {
        <main>
            <h1>"Megatokyo"</h1>
            <section>
                <h2>"Daemon settings"</h2>
                <label>
                    "Base URL "
                    <input
                        type="text"
                        placeholder="http://127.0.0.1:8420"
                        prop:value=move || base_url.get()
                        on:input=move |ev| set_base_url.set(event_target_value(&ev))
                    />
                </label>
                <label>
                    "Token "
                    <input
                        type="password"
                        prop:value=move || token.get()
                        on:input=move |ev| set_token.set(event_target_value(&ev))
                    />
                </label>
                <button on:click=save_settings>"Save"</button>
                <button on:click=load_chapters>"Load chapters"</button>
                <button on:click=enable_notifications>"Enable notifications"</button>
                {move || match notifications_status.get() {
                    None => ().into_any(),
                    Some(Ok(())) => view! { <p>"Notifications enabled."</p> }.into_any(),
                    Some(Err(err)) => view! { <p class="error">{err}</p> }.into_any(),
                }}
            </section>
            <section>
                <h2>"Chapters"</h2>
                {move || match chapters.get() {
                    None => view! { <p>"Not loaded yet."</p> }.into_any(),
                    Some(Ok(list)) => view! {
                        <ul>
                            {list.into_iter().map(|c| view! { <li>{c}</li> }).collect_view()}
                        </ul>
                    }.into_any(),
                    Some(Err(err)) => view! { <p class="error">{err}</p> }.into_any(),
                }}
            </section>
        </main>
    }
}
