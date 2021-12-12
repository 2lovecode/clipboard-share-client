use iced::{executor, time, Application, Command, Element, Settings, Clipboard, Container, Subscription, Text};
use clipboard::ClipboardProvider;
use clipboard::ClipboardContext;
use std::collections::HashMap;

pub fn main() -> iced::Result {
    read_data_from_remote();

    Hello::run(Settings::default())
}

async fn get_data_from_remote() -> Result<HashMap<String, String>, reqwest::Error>{
    Ok(reqwest::get("https://httpbin.org/ip").await?.json::<HashMap<String, String>>().await?)
}

#[tokio::main]
async fn read_data_from_remote() {
    if let Ok(resp) = get_data_from_remote().await {
        println!("{:#?}", resp);
    }
}
// fn send_data_to_remote() {

// }

#[derive(Default)]
struct Hello {
    t_value : String,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(),
}


impl Application for Hello {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (Hello, Command<Self::Message>) {
        (Hello{t_value: String::from(""), ..Self::default()}, Command::none())
    }

    fn title(&self) -> String {
        String::from("Clipboard Share Client")
    }

    fn update(&mut self, _messsage: Self::Message, _clipboard: &mut Clipboard) -> Command<Self::Message> {
        match _messsage {
            Message::Tick() => {
                let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
                let ss = ctx.get_contents().unwrap();
                if self.t_value != ss {
                    // println!("{}", local_time.to_string());
                    self.t_value = ss;
                }
                
            }
        }
        Command::none()
    }

    fn view(&mut self) -> Element<Self::Message> {
// String::from_utf8_lossy(v: &[u8])
        let input = Text::new(self.t_value.to_string());
        Container::new(input).into()
    }

    fn subscription(&self) -> Subscription<Message> {
        time::every(std::time::Duration::from_millis(2000))
            .map(|_| Message::Tick())
    }
}