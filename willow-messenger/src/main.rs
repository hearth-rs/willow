// Copyright (C) 2023 Marceline Cramer
//
// Willow is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// Willow is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with Willow.  If not, see <https://www.gnu.org/licenses/>.

use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    sync::Arc,
};

use ui::MessageContent;
use willow_react::ElementComponent;
use willow_server::glam::Vec2;
use winit::{event::WindowEvent, event_loop::EventLoopProxy};

mod ui;

#[derive(Clone, Debug)]
pub enum AppEvent {
    Message(MessageContent),
}

struct MessengerApp {
    stream: Arc<TcpStream>,
    messages: Vec<MessageContent>,
    nick: String,
    input: String,
}

impl willow_desktop::App for MessengerApp {
    type Event = AppEvent;

    fn with_proxy(&self, proxy: EventLoopProxy<Self::Event>) {
        let stream = self.stream.clone();

        std::thread::spawn(move || loop {
            let mut reader = BufReader::new(stream.as_ref());
            let mut from = String::new();
            let mut body = String::new();
            reader.read_line(&mut from).unwrap();
            reader.read_line(&mut body).unwrap();

            assert!(from.starts_with("FROM "));
            assert!(body.starts_with("BODY "));

            let from = from.split_off(5).trim().to_string();
            let body = body.split_off(5).trim().to_string();

            let message = MessageContent {
                text: body,
                sender: from,
                timestamp: chrono::Utc::now(),
            };

            proxy.send_event(AppEvent::Message(message)).unwrap();
        });
    }

    fn redraw(&mut self, size: Vec2) -> Box<dyn ElementComponent> {
        Box::new(ui::MessengerApp {
            messages: self.messages.clone(),
            size,
            input: self.input.clone(),
        })
    }

    fn on_event(&mut self, event: Self::Event) {
        match event {
            AppEvent::Message(message) => {
                self.messages.push(message);
            }
        }
    }

    fn on_window_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::ReceivedCharacter(char) => match char {
                '\r' => {
                    let message = format!("BODY {}\n", self.input);
                    let mut stream = self.stream.as_ref();
                    stream.write_all(message.as_bytes()).unwrap();
                    stream.flush().unwrap();

                    self.on_event(AppEvent::Message(MessageContent {
                        text: self.input.clone(),
                        sender: self.nick.clone(),
                        timestamp: chrono::Utc::now(),
                    }));

                    self.input.clear();
                }
                '\u{8}' => {
                    self.input.pop();
                }
                char => {
                    self.input.push(char);
                }
            },
            _ => {}
        }
    }
}

impl MessengerApp {
    pub fn new() -> Self {
        let mut args = std::env::args();
        args.next().expect("expected argv[0]");
        let server = args.next().expect("expected server address");
        let nick = args.next().expect("expected nickname");

        let mut stream = TcpStream::connect(server).unwrap();
        stream
            .write_all(format!("FROM {}\n", nick).as_bytes())
            .unwrap();

        Self {
            nick,
            stream: Arc::new(stream),
            messages: Vec::new(),
            input: String::new(),
        }
    }
}

fn main() {
    let app = MessengerApp::new();
    willow_desktop::run_app(app);
}
