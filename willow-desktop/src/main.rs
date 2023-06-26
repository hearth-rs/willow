use std::num::NonZeroU32;

use raqote::DrawTarget;
use willow_server::Tree;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let context = unsafe { softbuffer::Context::new(&window) }.unwrap();
    let mut surface = unsafe { softbuffer::Surface::new(&context, &window) }.unwrap();

    let mut tree = Tree::new();
    tree.update_node(willow_server::NodeUpdate {
        target: 0,
        content: willow_server::NodeContent::Shape(willow_server::Shape::Circle { radius: 250.0 }),
    })
    .unwrap();

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

            let mut buffer = surface.buffer_mut().unwrap();
            buffer.fill(0xff000000);
            let mut dt = DrawTarget::from_backing(width as i32, height as i32, buffer.as_mut());
            let mut ren = willow_raqote::RaqoteRenderer::new(&mut dt);
            tree.walk(&mut ren);

            buffer.present().unwrap();
        }
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            control_flow.set_exit();
        }
        _ => {}
    });
}
