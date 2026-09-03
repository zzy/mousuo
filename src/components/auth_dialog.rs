#![allow(non_snake_case)]

use crate::common::icons;
use crate::components::dialog::{dialog, dialog_content};
use topcoat::{
    Result,
    icon::icon,
    view::{View, attributes, component, view},
};

/// 鉴权弹层宽度档位：注册恰为签入的 2 倍
#[derive(Clone, Copy)]
pub enum AuthDialogWidth {
    SignIn,
    Register,
}

impl AuthDialogWidth {
    /// 宽度类；`!` 后缀压过 dialog_content 内置 max-w-lg
    /// （Tailwind 层叠按字典序，普通 max-w-* 类会被内置值压死）
    fn classes(self) -> &'static str {
        match self {
            Self::SignIn => "max-w-md!",
            Self::Register => "max-w-4xl!",
        }
    }
}

/// 鉴权页居中弹层：官方 dialog 基准之上加项目定制（宽度档位 + 悬浮关闭钮）。
/// dialog.rs 保持与 registry 逐字一致，组件更新不影响本包装。
#[component]
pub async fn auth_dialog(
    locale: String,
    width: AuthDialogWidth,
    #[default] child: View,
) -> Result {
    view! {
        dialog(
            open: true,
            dialog_content(
                attrs: attributes! { class=(width.classes()) },
                <a
                    href=(format!("/{locale}"))
                    class="absolute -top-4 -right-4 z-10 flex size-9 items-center justify-center rounded-full border border-border bg-surface text-muted-foreground no-underline shadow-xs hover:bg-muted hover:text-foreground"
                >
                    icon(data: icons::CLOSE)
                </a>
                (child)
            )
        )
    }
}
