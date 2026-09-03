#![allow(non_snake_case)]

use crate::common::icons;
use crate::components::button::{ButtonVariant, button};
use crate::components::csrf::CsrfField;
use crate::i18n::loader;
use topcoat::{
    Result,
    icon::icon,
    view::{attributes, component, view},
};

#[component]
pub async fn Nav(
    locale: String,
    signed_in: bool,
    username: Option<String>,
    is_admin: bool,
    csrf: String,
    path_and_query: String,
) -> Result {
    let user_display = username.unwrap_or_default();
    let lang_label = loader::t(&locale, "lang");
    view! {
        <nav class="bg-surface border-b border-border sticky top-0" style="z-index:52">
            <div class="max-w-7xl mx-auto px-4">
                <div class="flex items-center justify-between h-12">
                    <a
                        href=(format!("/{locale}"))
                        class="font-bold text-xl text-blue-600 dark:text-blue-400 no-underline"
                    >
                        (loader::t(&locale, "site_name"))
                    </a>
                    <form
                        action=(format!("/{locale}/products"))
                        method="get"
                        class="flex-1 max-w-md mx-auto"
                    >
                        <input
                            type="search"
                            name="q"
                            placeholder=(loader::t(&locale, "search_placeholder"))
                            class="w-full px-3 py-1.5 text-sm border border-border rounded-full bg-background text-foreground outline-none focus:border-blue-400 transition"
                        >
                    </form>
                    <div class="flex items-center gap-2 text-sm">
                        <a
                            href=(format!("/{locale}/products"))
                            class="text-blue-600 dark:text-blue-400 hover:underline no-underline"
                        >
                            (loader::t(&locale, "nav_products"))
                        </a>
                        <span class="text-muted-foreground">"|"</span>
                        <button
                            class="border-0 bg-transparent cursor-pointer text-sm"
                            onclick="toggleTheme()"
                        >
                            icon(data: icons::THEME_LIGHT_DARK, size: 18)
                        </button>
                        <div class="relative">
                            button(
                                variant: ButtonVariant::Outline,
                                attrs: attributes! {
                                    onclick="event.stopPropagation();document.getElementById('lang-menu').classList.toggle('hidden')"
                                },
                                icon(data: icons::TRANSLATE, size: 16)
                                <span class="hidden sm:inline ml-1">(lang_label)</span>
                                icon(data: icons::CHEVRON_DOWN, size: 12)
                            )
                            <div
                                id="lang-menu"
                                class="hidden absolute top-full right-0 mt-1 border border-border rounded shadow-md py-1 bg-surface whitespace-nowrap z-50"
                            >
                                for lang in loader::menu_langs(&locale) {
                                    <a
                                        href=(loader::swap_locale(&path_and_query, lang))
                                        class=(if lang == locale.as_str() {
                                            "block w-full text-left px-3 py-1 text-sm text-blue-600 dark:text-blue-400 font-medium no-underline hover:bg-muted"
                                        } else {
                                            "block w-full text-left px-3 py-1 text-sm text-foreground no-underline hover:bg-muted"
                                        })
                                    >
                                        (loader::t(lang, "lang"))
                                    </a>
                                }
                            </div>
                        </div>
                        if signed_in {
                            if is_admin {
                                <a
                                    href=(format!("/{locale}/admin/orders"))
                                    class="text-blue-600 dark:text-blue-400 hover:underline no-underline"
                                >
                                    (loader::t(&locale, "nav_admin"))
                                </a>
                                <span class="text-muted-foreground">"|"</span>
                            }
                            <a
                                href=(format!("/{locale}/orders"))
                                class="text-blue-600 dark:text-blue-400 hover:underline no-underline"
                            >
                                (loader::t(&locale, "nav_orders"))
                            </a>
                            <a
                                href=(format!("/{locale}/users/{user_display}"))
                                class="text-blue-600 dark:text-blue-400 hover:underline no-underline"
                            >
                                (user_display)
                            </a>
                            <form
                                method="POST"
                                action=(format!("/{locale}/sign-out"))
                                class="inline"
                            >
                                CsrfField(token: csrf)
                                <button
                                    class="text-blue-600 dark:text-blue-400 hover:underline bg-transparent border-0 cursor-pointer"
                                >
                                    (loader::t(&locale, "sign_out"))
                                </button>
                            </form>
                        } else {
                            <a
                                href=(format!("/{locale}/sign-in"))
                                class="text-blue-600 dark:text-blue-400 hover:underline no-underline"
                            >
                                (loader::t(&locale, "sign_in"))
                            </a>
                            <span class="text-muted-foreground">"|"</span>
                            <a
                                href=(format!("/{locale}/register"))
                                class="text-blue-600 dark:text-blue-400 hover:underline no-underline"
                            >
                                (loader::t(&locale, "register"))
                            </a>
                        }
                    </div>
                </div>
            </div>
        </nav>
        <script>
            "document.addEventListener('click',function(e){var m=document.getElementById('lang-menu');if(m&&!m.classList.contains('hidden')&&!e.target.closest('#lang-menu')&&!e.target.closest('button[onclick*=\"lang-menu\"]'))m.classList.add('hidden')});"
        </script>
    }
}
