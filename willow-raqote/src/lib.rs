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

use std::f32::consts::TAU;

use euclid::Angle;
use raqote::{DrawOptions, DrawTarget, PathBuilder, SolidSource, Source, Transform};
use willow_server::{Operation, Shape, WalkTree};

pub struct RaqoteRenderer<'a, Backing> {
    dt: &'a mut DrawTarget<Backing>,
    stroke_stack: Vec<Source<'static>>,
    transform_stack: Vec<Transform>,
}

impl<'a, Backing> WalkTree for RaqoteRenderer<'a, Backing>
where
    Backing: AsRef<[u32]> + AsMut<[u32]>,
{
    fn on_shape(&mut self, shape: &Shape) {
        let source = self.stroke_stack.last().unwrap();
        let options = DrawOptions::new();

        let current_transform = self.transform_stack.last().unwrap().clone();
        self.dt.set_transform(&current_transform);

        use Shape::*;
        match shape {
            Empty => {}
            Circle { radius } => {
                let mut pb = PathBuilder::new();
                pb.arc(0., 0., *radius, 0., TAU);
                pb.close();

                let path = pb.finish();

                self.dt.fill(&path, &source, &options);
            }
            Rectangle { min, max } => {
                let size = *max - *min;
                self.dt
                    .fill_rect(min.x, min.y, size.x, size.y, &source, &options);
            }
        }
    }

    fn push_operation(&mut self, operation: &Operation) {
        let current_transform = self.transform_stack.last().unwrap().clone();

        use Operation::*;
        match operation {
            Stroke(stroke) => match stroke {
                willow_server::Stroke::Solid { color } => {
                    let color = (*color * 255.0).as_uvec3();
                    let (r, g, b) = (color.x as u8, color.y as u8, color.z as u8);
                    let a = 255;
                    let source = SolidSource { r, g, b, a };
                    self.stroke_stack.push(Source::Solid(source));
                }
            },
            Translate { offset } => {
                let translate = Transform::translation(offset.x, offset.y);
                self.transform_stack.push(translate);
            }
            Rotation { angle } => {
                let rotation = Transform::rotation(Angle { radians: *angle });
                self.transform_stack.push(current_transform.then(&rotation));
            }
            Scale { scale } => {
                let scale = Transform::scale(*scale, *scale);
                self.transform_stack.push(current_transform.then(&scale));
            }
            Opacity { opacity } => self
                .dt
                .push_layer_with_blend(*opacity, raqote::BlendMode::Src),
        }
    }

    fn pop_operation(&mut self, operation: &Operation) {
        use Operation::*;

        match operation {
            Stroke(_) => {
                self.stroke_stack.pop();
            }
            Translate { .. } | Rotation { .. } | Scale { .. } => {
                self.transform_stack.pop();
            }
            Opacity { .. } => self.dt.pop_layer(),
        }
    }
}

impl<'a, Backing> RaqoteRenderer<'a, Backing> {
    pub fn new(dt: &'a mut DrawTarget<Backing>) -> Self {
        let default_stroke = Source::Solid(SolidSource {
            r: 0xff,
            g: 0x00,
            b: 0xff,
            a: 0xff,
        });

        Self {
            dt,
            stroke_stack: vec![default_stroke],
            transform_stack: vec![Transform::identity()],
        }
    }
}
