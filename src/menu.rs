//! # See also
//! + `tauri::menu`
use crate::{core, window};
use serde::{ser::SerializeStruct, Deserialize, Serialize};
use std::collections::HashMap;

type Rid = usize;

#[derive(Debug)]
pub struct Menu {
    rid: Rid,
    id: MenuId,
    channel: Option<core::Channel<Message<String>>>,
}

impl Menu {
    pub async fn with_id(id: impl Into<MenuId>) -> Self {
        #[derive(Serialize)]
        struct Args {
            kind: String,
            options: MenuOptions,
            handler: ChannelId,
        }

        let options = MenuOptions {
            id: Some(id.into()),
        };

        let channel = core::Channel::new();

        let (rid, id) = core::invoke::<(Rid, String)>(
            "plugin:menu|new",
            Args {
                kind: ItemKind::Menu.as_str().to_string(),
                options,
                handler: ChannelId::from(&channel),
            },
        )
        .await;

        Self {
            rid,
            id: id.into(),
            channel: Some(channel),
        }
    }

    pub async fn default() -> Self {
        let (rid, id) = core::invoke::<(Rid, String)>("plugin:menu|create_default", ()).await;
        Self {
            rid,
            id: id.into(),
            channel: None,
        }
    }
}

impl Menu {
    pub fn rid(&self) -> Rid {
        self.rid
    }

    pub fn kind() -> &'static str {
        ItemKind::Menu.as_str()
    }
}

impl Menu {
    pub async fn append_item(&self, item: &item::MenuItem) -> Result<(), ()> {
        core::invoke_result(
            "plugin:menu|append",
            AppendItemArgs {
                rid: self.rid,
                kind: Self::kind().to_string(),
                items: vec![(item.rid(), item::MenuItem::kind().to_string())],
            },
        )
        .await
    }

    /// Popup this menu as a context menu on the specified window.
    /// If the position, is provided, it is relative to the window's top-left corner.
    pub async fn popup(&self) -> Result<(), ()> {
        #[derive(Serialize)]
        struct Position {
            x: isize,
            y: isize,
        }

        #[derive(Serialize)]
        struct Args {
            rid: Rid,
            kind: String,
            window: Option<window::WindowLabel>,
            at: Option<HashMap<String, Position>>,
        }

        core::invoke_result(
            "plugin:menu|popup",
            Args {
                rid: self.rid,
                kind: Self::kind().to_string(),
                window: None,
                at: None,
            },
        )
        .await
    }
}

impl Menu {
    pub fn listen(&mut self) -> Option<&mut core::Channel<Message<String>>> {
        self.channel.as_mut()
    }
}

#[derive(Serialize)]
struct AppendItemArgs {
    rid: Rid,
    kind: String,
    items: Vec<(Rid, String)>,
}

#[derive(Serialize, Clone, derive_more::From, Debug)]
#[serde(transparent)]
pub struct MenuId(pub String);

impl From<&'static str> for MenuId {
    fn from(value: &'static str) -> Self {
        Self(value.to_string())
    }
}

#[derive(derive_more::Deref, Deserialize, Debug)]
pub struct Message<T> {
    id: usize,

    #[deref]
    message: T,
}

impl<T> Message<T> {
    pub fn id(&self) -> usize {
        self.id
    }
}

#[derive(Serialize)]
struct MenuOptions {
    id: Option<MenuId>,
}

enum ItemKind {
    MenuItem,
    Predefined,
    Check,
    Icon,
    Submenu,
    Menu,
}

impl ItemKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MenuItem => "MenuItem",
            Self::Predefined => "Predefined",
            Self::Check => "Check",
            Self::Icon => "Icon",
            Self::Submenu => "Submenu",
            Self::Menu => "Menu",
        }
    }
}

struct ChannelId {
    id: usize,
}

impl ChannelId {
    pub fn from<T>(channel: &core::Channel<T>) -> Self {
        Self { id: channel.id() }
    }
}

impl Serialize for ChannelId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_struct("ChannelId", 2)?;
        map.serialize_field("__TAURI_CHANNEL_MARKER__", &true)?;
        map.serialize_field("id", &self.id)?;
        map.end()
    }
}

pub mod item {
    use super::{ChannelId, ItemKind, MenuId, Message, Rid};
    use crate::core;
    use futures::{Stream, StreamExt};
    use serde::Serialize;

    pub struct MenuItem {
        rid: Rid,
        id: MenuId,
        channel: core::Channel<Message<String>>,
    }

    impl MenuItem {
        pub async fn with_id(text: impl Into<String>, id: impl Into<MenuId>) -> Self {
            let mut options = MenuItemOptions::new(text);
            options.set_id(id);

            Self::with_options(options).await
        }

        pub async fn with_options(options: MenuItemOptions) -> Self {
            #[derive(Serialize)]
            struct Args {
                kind: String,
                options: MenuItemOptions,
                handler: ChannelId,
            }

            let channel = core::Channel::new();

            let (rid, id) = core::invoke::<(Rid, String)>(
                "plugin:menu|new",
                Args {
                    kind: ItemKind::MenuItem.as_str().to_string(),
                    options,
                    handler: ChannelId::from(&channel),
                },
            )
            .await;

            Self {
                rid,
                id: id.into(),
                channel,
            }
        }
    }

    impl MenuItem {
        pub fn rid(&self) -> Rid {
            self.rid
        }

        pub fn kind() -> &'static str {
            ItemKind::MenuItem.as_str()
        }
    }

    impl MenuItem {
        // pub fn listen(&mut self) -> impl Stream<Item = Message<String>> {
        //     self.channel.map(|message| message.message)
        // }

        pub fn listen(&mut self) -> &mut core::Channel<Message<String>> {
            &mut self.channel
        }
    }

    #[derive(Serialize)]
    pub struct MenuItemOptions {
        /// Specify an id to use for the new menu item.
        id: Option<MenuId>,

        /// The text of the new menu item.
        text: String,

        /// Whether the new menu item is enabled or not.
        enabled: Option<bool>,

        /// Specify an accelerator for the new menu item.
        accelerator: Option<String>,
    }

    impl MenuItemOptions {
        pub fn new(text: impl Into<String>) -> Self {
            Self {
                id: None,
                text: text.into(),
                enabled: None,
                accelerator: None,
            }
        }

        pub fn set_id(&mut self, id: impl Into<MenuId>) -> &mut Self {
            let _ = self.id.insert(id.into());
            self
        }

        pub fn set_enabled(&mut self, enabled: bool) -> &mut Self {
            let _ = self.enabled.insert(enabled);
            self
        }

        pub fn set_accelerator(&mut self, accelerator: impl Into<String>) -> &mut Self {
            let _ = self.accelerator.insert(accelerator.into());
            self
        }
    }
}

mod inner {
    use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

    #[wasm_bindgen(module = "/src/menu.js")]
    extern "C" {
        #[wasm_bindgen(js_name = "getCurrent")]
        pub fn get_current() -> JsValue;
    }
}