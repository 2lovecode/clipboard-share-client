use iced::executor;
use iced::widget::{
    column, container, scrollable, text, Column,
};
use iced::{
    self, Application, Color, Command, Element, Length, Subscription, Theme,
};
use once_cell::sync::Lazy;
use once_cell::sync::OnceCell;
use cli_clipboard::{ClipboardContext, ClipboardProvider};
use iced_graphics;
use tauri_hotkey;
use std::time;
use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::channel;

pub fn main() -> iced::Result {

    let mut hotkey = tauri_hotkey::HotkeyManager::new();
   
    let (sender, receiver) = channel();

    let macos_ctrlc = tauri_hotkey::Hotkey {
        keys: vec![tauri_hotkey::Key::C],
        modifiers: vec![tauri_hotkey::Modifier::SUPER],
    };
   
    
    let mut text_pool = Arc::new(Mutex::new(Vec::<String>::new()));
    let text_pool_clone = Arc::clone(&text_pool);
    let mut current_text = String::from("Good Morning");

    hotkey.register(macos_ctrlc, move || {
        sender.send(1);
       
        })
        .unwrap();
    
    thread::spawn(move || {
        let aa = receiver.recv().unwrap();
        println!("{}", aa);
    });
    // let mut clipboard_ctx = ClipboardContext::new().unwrap();
    // match clipboard_ctx.get_contents() {
    //     Ok(content) => {
    //         if !content.is_empty() {
    //             println!("clipboard {}", content);
    //             text_pool.lock().unwrap().push(content);
    //         }
    //     }
    //     Err(_) => ()
    // };
        
    ClipboardShare::run(settings())
}

#[derive(Default)]
struct ClipboardShare {
    messages: Vec<String>,
}

#[derive(Debug, Clone)]
enum Message {
    Send(String),
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
            Message::None => Command::none()
        }
    }

    // fn subscription(&self) -> Subscription<Message> {
    //     iced::time::every(std::time::Duration::from_millis(500)).map(|_| {
            
    //     })
    // }

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