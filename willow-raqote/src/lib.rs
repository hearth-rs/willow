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

use raqote::{DrawOptions, DrawTarget, PathBuilder, SolidSource};
use willow_server::{Operation, Shape, WalkTree};

pub struct RaqoteRenderer<'a, Backing> {
    dt: &'a mut DrawTarget<Backing>,
}

impl<'a, Backing> WalkTree for RaqoteRenderer<'a, Backing>
where
    Backing: AsRef<[u32]> + AsMut<[u32]>,
{
    fn on_shape(&mut self, shape: &Shape) {
        let source = raqote::Source::Solid(SolidSource::from_unpremultiplied_argb(
            0xff, 0xff, 0x00, 0xff,
        ));

        let options = DrawOptions::new();

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
        todo!()
    }

    fn pop_operation(&mut self, operation: &Operation) {}
}

impl<'a, Backing> RaqoteRenderer<'a, Backing> {
    pub fn new(dt: &'a mut DrawTarget<Backing>) -> Self {
        Self { dt }
    }
}
