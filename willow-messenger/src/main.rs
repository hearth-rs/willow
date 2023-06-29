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
    num::NonZeroU32,
    sync::Arc,
};

use raqote::DrawTarget;
use ui::MessageContent;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{EventLoopBuilder, EventLoopProxy},
    window::WindowBuilder,
};

mod ui;

#[derive(Clone, Debug)]
pub enum AppEvent {
    Message(MessageContent),
}

fn main() {
    let mut args = std::env::args();
    args.next().expect("expected argv[0]");
    let server = args.next().expect("expected server address");
    let nick = args.next().expect("expected nickname");

    let event_loop = EventLoopBuilder::<AppEvent>::with_user_event().build();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let context = unsafe { softbuffer::Context::new(&window) }.unwrap();
    let mut surface = unsafe { softbuffer::Surface::new(&context, &window) }.unwrap();

    let mut stream = TcpStream::connect(server).unwrap();
    stream
        .write_all(format!("FROM {}\n", nick).as_bytes())
        .unwrap();

    let stream = Arc::new(stream);
    let proxy = event_loop.create_proxy();
    std::thread::spawn({
        let stream = stream.clone();
        move || run_connection(proxy, stream.clone())
    });

    let mut state = ui::State::new();
    let mut messages = Vec::new();
    let mut input = String::new();
    let proxy = event_loop.create_proxy();
    event_loop.run(move |event, _, control_flow| match event {
        Event::RedrawRequested(_) => {
            let (width, height) = {
                let size = window.inner_size();
                (size.width, size.height)
            };

            surface
                .resize(
                    NonZeroU32::new(width).unwrap(),
                    NonZeroU32::new(height).unwrap(),
                )
                .unwrap();

            state.set_root(ui::MessengerApp {
                messages: messages.clone(),
                size: willow_server::glam::vec2(width as f32, height as f32),
                input: input.clone(),
            });

            let mut buffer = surface.buffer_mut().unwrap();
            buffer.fill(0xff000000);
            let mut dt = DrawTarget::from_backing(width as i32, height as i32, buffer.as_mut());
            let mut ren = willow_raqote::RaqoteRenderer::new(&mut dt);
            state.tree.walk(&mut ren);

            buffer.present().unwrap();
        }
        Event::WindowEvent {
            event: WindowEvent::ReceivedCharacter(char),
            ..
        } => {
            match char {
                '\r' => {
                    let message = format!("BODY {}\n", input);
                    stream.as_ref().write_all(message.as_bytes()).unwrap();
                    stream.as_ref().flush().unwrap();

                    proxy
                        .send_event(AppEvent::Message(MessageContent {
                            text: input.clone(),
                            sender: nick.clone(),
                            timestamp: chrono::Utc::now(),
                        }))
                        .unwrap();

                    input.clear();
                }
                '\u{8}' => {
                    input.pop();
                }
                char => {
                    input.push(char);
                }
            }

            window.request_redraw();
        }
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            control_flow.set_exit();
        }
        Event::UserEvent(event) => match event {
            AppEvent::Message(message) => {
                messages.push(message);
                window.request_redraw();
            }
        },
        _ => {}
    });
}

pub fn run_connection(proxy: EventLoopProxy<AppEvent>, stream: Arc<TcpStream>) {
    let mut reader = BufReader::new(stream.as_ref());

    loop {
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
    }
}
