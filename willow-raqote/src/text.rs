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

use allsorts::binary::read::ReadScope;
use allsorts::font::{GlyphTableFlags, MatchingPresentation};
use allsorts::font_data::{DynamicFontTableProvider, FontData as AllsortsFontData};
use allsorts::glyph_position::{GlyphLayout, TextDirection};
use allsorts::outline::OutlineBuilder;
use allsorts::Font as AllsortsFont;
use euclid::default::Transform2D;
use raqote::{DrawOptions, DrawTarget, Path, PathBuilder, Source};

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GlyphPosition {
    pub index: u16,
    pub hori_advance: i32,
    pub vert_advance: i32,
    pub xoff: i32,
    pub yoff: i32,
}

#[ouroboros::self_referencing]
pub struct FontData {
    file_buffer: Vec<u8>,
    script: u32,
    direction: TextDirection,
    vertical: bool,

    #[borrows(file_buffer)]
    #[covariant]
    read_scope: ReadScope<'this>,

    #[borrows(read_scope)]
    #[not_covariant]
    font_data: AllsortsFontData<'this>,

    #[borrows(font_data)]
    #[covariant]
    inner: AllsortsFont<DynamicFontTableProvider<'this>>,
}

impl FontData {
    pub fn load(
        script: u32,
        direction: TextDirection,
        vertical: bool,
        file_buffer: Vec<u8>,
    ) -> Self {
        FontDataBuilder {
            file_buffer,
            script,
            direction,
            vertical,
            read_scope_builder: |buffer| ReadScope::new(buffer),
            font_data_builder: |scope| scope.read::<AllsortsFontData<'_>>().unwrap(),
            inner_builder: |font_data| {
                AllsortsFont::new(font_data.table_provider(0).unwrap())
                    .unwrap()
                    .unwrap()
            },
        }
        .build()
    }

    pub fn shape(&mut self, text: &str) -> Vec<GlyphPosition> {
        let presentation = MatchingPresentation::Required;
        let script = *self.borrow_script();
        let lang_tag = None;
        let features = allsorts::gsub::Features::default();
        let kerning = true;
        let direction = *self.borrow_direction();
        let vertical = *self.borrow_vertical();

        self.with_inner_mut(|font| {
            let mapped = font.map_glyphs(text, script, presentation);
            let infos = font
                .shape(mapped, script, lang_tag, &features, kerning)
                .unwrap();
            let mut layout = GlyphLayout::new(font, &infos, direction, vertical);
            let positions = layout.glyph_positions().unwrap();
            let mut glyphs = Vec::with_capacity(positions.len());
            for (glyph, position) in infos.iter().zip(&positions) {
                glyphs.push(GlyphPosition {
                    index: glyph.glyph.glyph_index,
                    hori_advance: position.hori_advance,
                    vert_advance: position.vert_advance,
                    xoff: position.x_offset,
                    yoff: position.y_offset,
                });
            }

            glyphs
        })
    }

    pub fn draw<Backing>(
        &mut self,
        dt: &mut DrawTarget<Backing>,
        text: &str,
        source: &Source,
        options: &DrawOptions,
    ) where
        Backing: AsRef<[u32]> + AsMut<[u32]>,
    {
        let units_per_em =
            self.with_inner_mut(|font| font.head_table().unwrap().unwrap().units_per_em as f32);
        let px_per_unit = 10.0 / units_per_em;

        let mut xcur = 0;
        let mut ycur = 0;
        for position in self.shape(text) {
            let xpos = xcur + position.xoff;
            let ypos = ycur + position.yoff;
            xcur += position.hori_advance;
            ycur += position.vert_advance;

            let path = self.glyph_path(position.index);
            let translate = Transform2D::translation(xpos as f32, ypos as f32);
            let scale = Transform2D::scale(px_per_unit, -px_per_unit);
            let transform = translate.then(&scale);
            let path = path.transform(&transform);
            dt.fill(&path, source, options);
        }
    }

    pub fn glyph_path(&mut self, index: u16) -> Path {
        use allsorts::cff::CFF;
        use allsorts::tables::{glyf::GlyfTable, loca::LocaTable, FontTableProvider, SfntVersion};
        use allsorts::tag;

        self.with_inner_mut(|font| {
            if font.glyph_table_flags.contains(GlyphTableFlags::CFF)
                && font.font_table_provider.sfnt_version() == tag::OTTO
            {
                let cff_data = font.font_table_provider.read_table_data(tag::CFF).unwrap();
                let mut cff = ReadScope::new(&cff_data).read::<CFF<'_>>().unwrap();
                GlyphPathBuilder::build(&mut cff, index)
            } else if font.glyph_table_flags.contains(GlyphTableFlags::GLYF) {
                let loca_data = font.font_table_provider.read_table_data(tag::LOCA).unwrap();
                let loca = ReadScope::new(&loca_data)
                    .read_dep::<LocaTable<'_>>((
                        usize::from(font.maxp_table.num_glyphs),
                        font.head_table().unwrap().unwrap().index_to_loc_format,
                    ))
                    .unwrap();
                let glyf_data = font.font_table_provider.read_table_data(tag::GLYF).unwrap();
                let mut glyf = ReadScope::new(&glyf_data)
                    .read_dep::<GlyfTable<'_>>(&loca)
                    .unwrap();

                GlyphPathBuilder::build(&mut glyf, index)
            } else {
                panic!("no glyf or CFF table");
            }
        })
    }
}

pub struct GlyphPathBuilder {
    pb: PathBuilder,
}

impl allsorts::outline::OutlineSink for GlyphPathBuilder {
    fn move_to(&mut self, to: allsorts::pathfinder_geometry::vector::Vector2F) {
        self.pb.move_to(to.x(), to.y());
    }

    fn line_to(&mut self, to: allsorts::pathfinder_geometry::vector::Vector2F) {
        self.pb.line_to(to.x(), to.y());
    }

    fn quadratic_curve_to(
        &mut self,
        ctrl: allsorts::pathfinder_geometry::vector::Vector2F,
        to: allsorts::pathfinder_geometry::vector::Vector2F,
    ) {
        self.pb.quad_to(ctrl.x(), ctrl.y(), to.x(), to.y());
    }

    fn cubic_curve_to(
        &mut self,
        ctrl: allsorts::pathfinder_geometry::line_segment::LineSegment2F,
        to: allsorts::pathfinder_geometry::vector::Vector2F,
    ) {
        self.pb.cubic_to(
            ctrl.from_x(),
            ctrl.from_y(),
            ctrl.to_x(),
            ctrl.to_y(),
            to.x(),
            to.y(),
        );
    }

    fn close(&mut self) {
        self.pb.close();
    }
}

impl GlyphPathBuilder {
    pub fn build(builder: &mut impl OutlineBuilder, index: u16) -> Path {
        let pb = PathBuilder::new();
        let mut sink = Self { pb };
        builder.visit(index, &mut sink).unwrap();
        sink.pb.finish()
    }
}
