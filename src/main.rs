use iced::{executor, time, Application, Command, Element, Settings, Clipboard, Container, Subscription, Text};
use clipboard::ClipboardProvider;
use clipboard::ClipboardContext;
use std::collections::HashMap;
use std::io::prelude::*;
use std::thread;
use std::net::{TcpListener, TcpStream};

pub fn main() -> iced::Result {
    thread::spawn(|| start_server);
    Hello::run(Settings::default())

}

#[tokio::main]
async fn start_server() {
    let listener = TcpListener::bind("127.0.0.1:80").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
            }
            Err(e) => { /* connection failed */ }
        }
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
    Receive(),
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
            Message::Receive() => {

                // 从当前设备剪切板读取数据
                // 从远端读取数据
                // 将当前设备数据推送到远端
                // 合并数据，记录到当前设备数据库

                // let mut stream = TcpStream::connect("127.0.0.1:34254")?;
                //
                // stream.write(&[1])?;
                // stream.read(&mut [0; 128])?;

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
            .map(|_| Message::Receive())
    }
}