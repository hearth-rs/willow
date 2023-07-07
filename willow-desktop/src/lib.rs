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

use std::num::NonZeroU32;

use raqote::DrawTarget;
use willow_react::{Element, ElementComponent, Hooks};
use willow_server::{
    glam::{vec2, Vec2},
    Operation,
};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{EventLoopBuilder, EventLoopProxy},
    window::WindowBuilder,
};

pub use willow_react;

pub trait App: 'static {
    type Event;

    fn with_proxy(&self, proxy: EventLoopProxy<Self::Event>);

    fn redraw(&mut self, size: Vec2) -> Box<dyn ElementComponent>;

    fn on_event(&mut self, event: Self::Event);

    fn on_window_event(&mut self, event: WindowEvent);
}

struct ScalingElement {
    scale: f32,
    inner: Option<Box<dyn ElementComponent + 'static>>,
}

impl ElementComponent for ScalingElement {
    fn render(&mut self, _hooks: &mut Hooks) -> Element {
        Element::Operation {
            operation: Operation::Scale { scale: self.scale },
            child: Element::Component {
                component: self.inner.take().unwrap(),
            }
            .into(),
        }
    }
}

pub fn run_app<T: App>(mut app: T) -> ! {
    let event_loop = EventLoopBuilder::<T::Event>::with_user_event().build();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let context = unsafe { softbuffer::Context::new(&window) }.unwrap();
    let mut surface = unsafe { softbuffer::Surface::new(&context, &window) }.unwrap();

    let proxy = event_loop.create_proxy();
    app.with_proxy(proxy);

    let mut state = willow_react::State::new();

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

            let scale = window.scale_factor() as f32;
            let size = vec2(width as f32, height as f32);
            let inner = Some(app.redraw(size / scale));
            let el = ScalingElement { scale, inner };
            state.set_root(Box::new(el));

            let aabb = willow_server::Aabb {
                min: willow_server::glam::Vec2::ZERO,
                max: size,
            };

            let mut buffer = surface.buffer_mut().unwrap();
            buffer.fill(0xff000000);
            let mut dt = DrawTarget::from_backing(width as i32, height as i32, buffer.as_mut());
            let mut ren = willow_raqote::RaqoteRenderer::new(&mut dt);
            state.tree.walk(&mut ren, &aabb);

            buffer.present().unwrap();
        }
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            control_flow.set_exit();
        }
        Event::WindowEvent { event, .. } => {
            app.on_window_event(event);
            window.request_redraw();
        }
        Event::UserEvent(event) => {
            app.on_event(event);
            window.request_redraw();
        }
        _ => {}
    });
}
