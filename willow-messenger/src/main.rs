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
    io::{BufRead, BufReader},
    net::TcpStream,
    num::NonZeroU32,
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
    let event_loop = EventLoopBuilder::<AppEvent>::with_user_event().build();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let context = unsafe { softbuffer::Context::new(&window) }.unwrap();
    let mut surface = unsafe { softbuffer::Surface::new(&context, &window) }.unwrap();

    let proxy = event_loop.create_proxy();
    std::thread::spawn(move || run_connection(proxy));

    let mut state = ui::State::new();

    let mut messages = Vec::new();
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
            });

            let mut buffer = surface.buffer_mut().unwrap();
            buffer.fill(0xff000000);
            let mut dt = DrawTarget::from_backing(width as i32, height as i32, buffer.as_mut());
            let mut ren = willow_raqote::RaqoteRenderer::new(&mut dt);
            state.tree.walk(&mut ren);

            buffer.present().unwrap();
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

pub fn run_connection(proxy: EventLoopProxy<AppEvent>) {
    let stream = TcpStream::connect("127.0.0.1:8080").unwrap();

    loop {
        let mut reader = BufReader::new(&stream);
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
            timestamp: "Just now".to_string(),
        };

        proxy.send_event(AppEvent::Message(message)).unwrap();
    }
}
