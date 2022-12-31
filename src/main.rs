use iced::executor;
use iced::widget::{
    column, container, scrollable, text, Column,
};
use iced::{
    self, Application, Color, Command, Element, Length, Subscription, Theme,
};
use iced::event::{self, Event};
use iced::keyboard;
use iced::subscription;
use iced_graphics;

use once_cell::sync::Lazy;
use once_cell::sync::OnceCell;
use cli_clipboard::{ClipboardContext, ClipboardProvider};

use tauri_hotkey;
use std::sync::{Arc, Mutex};
use project_root;

pub fn main() -> iced::Result {

    let mut project_root_path = String::from("");
    let mut windows_selected_path = Arc::new(String::from(""));
    let mut os_type = 0;
    let mut current_copy_string = Arc::new(String::from(""));
    
    match project_root::get_project_root() {
        Ok(p) => {
            match p.to_str() {
                Some(pt) => {
                    project_root_path = pt.to_string();

                    windows_selected_path = Arc::new(pt.to_string()+"\\deps\\fetch_selected_text.exe");
                }
                None => ()
            }
        },
        Err(_) => ()
    };


    if cfg!(target_os = "linux") {
        os_type = 1
    }else if cfg!(target_os = "macos"){
        os_type = 2
    }else if cfg!(target_os = "windows"){
        os_type = 3
    }

    let mut hotkey = tauri_hotkey::HotkeyManager::new();
   
    let alt_c = tauri_hotkey::Hotkey {
        keys: vec![tauri_hotkey::Key::C],
        modifiers: vec![tauri_hotkey::Modifier::ALT],
    };
   
    
    hotkey.register(alt_c, move || {
        
        if os_type == 3 {
            let mut command = std::process::Command::new(windows_selected_path.to_string());
         
            let output = command.output().unwrap();
         
            match output.status.code() {
                Some(code) => {
                    if code == 0 {
                        match String::from_utf8(output.stdout) {
                            Ok(v) => {
                                current_copy_string = Arc::new(v);
                                println!("copy {}", current_copy_string);
                            }
                            Err(_) => ()
                        }
                        
                    }
                }
                None => ()
            }
        }
        })
        .unwrap();
    
        
    ClipboardShare::run(settings())
}

#[derive(Default)]
struct ClipboardShare {
    messages: Vec<String>,
}

#[derive(Debug, Clone)]
enum Message {
    Send(String),
    Copy(u8),
    None
}

impl Application for ClipboardShare {
    type Message = Message;
    type Theme = Theme;
    type Flags = ();
    type Executor = executor::Default;

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        (
            Self::default(),
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("剪切板")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Send(msg) => {
                self.messages.push(msg);
                Command::none()
            }
            Message::Copy(idx) => {
                let mut clipboard_ctx = ClipboardContext::new().unwrap();

                clipboard_ctx.set_contents(String::from("Good Morning!")).unwrap();
                Command::none()
            }
            Message::None => Command::none()
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        subscription::events_with(|event, status| match (event, status) {
            (
                Event::Keyboard(keyboard::Event::KeyPressed {
                    key_code: keyboard::KeyCode::Key1,
                    modifiers: keyboard::Modifiers::ALT,
                    ..
                }),
                event::Status::Ignored,
            ) => {
                Some(Message::Copy(1))
            },
            _ => None,
        })
    }

    fn view(&self) -> Element<Message> {
        let message_log: Element<_> = if self.messages.is_empty() {
            container(
                text("剪切板内容")
                    .style(Color::from_rgb8(0x88, 0x88, 0x88)),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
        } else {
            scrollable(
                Column::with_children(
                    self.messages
                        .iter()
                        .cloned()
                        .map(text)
                        .map(Element::from)
                        .collect(),
                )
                .width(Length::Fill)
                .spacing(10),
            )
            .id(MESSAGE_LOG.clone())
            .height(Length::Fill)
            .into()
        };

        column![message_log]
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .spacing(10)
            .into()
    }
}

static MESSAGE_LOG: Lazy<scrollable::Id> = Lazy::new(scrollable::Id::unique);


// 在 lib.rs 内
pub fn settings() -> iced::Settings<()> {
    iced::Settings {
        default_font:font(),
        ..Default::default()
    }
 }
 // OnceCell可以帮助我们安全的定义一个生存期为 static的全局变量
 static FONT: OnceCell<Option<Vec<u8>>> = OnceCell::new();
 
 fn font() -> Option<&'static [u8]> {
    FONT.get_or_init(|| {
        // 需要添加iced_graphics这个crate
        use iced_graphics::font::Family;
        let source = iced_graphics::font::Source::new();
        source
            .load(&[
                Family::Title("PingFang SC".to_owned()),
                Family::Title("Hiragino Sans GB".to_owned()),
                Family::Title("Heiti SC".to_owned()),
                Family::Title("Microsoft YaHei".to_owned()),
                Family::Title("WenQuanYi Micro Hei".to_owned()),
                Family::Title("Microsoft YaHei".to_owned()),
                Family::Title("Helvetica".to_owned()),
                Family::Title("Tahoma".to_owned()),
                Family::Title("Arial".to_owned()),
                Family::SansSerif,
            ])
            .ok()
    })
    .as_ref()
    .map(|f| f.as_slice())
 }