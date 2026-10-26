//! Lazy TTF metrics and individual outlines; never preprocess an entire font.
use geometry::lyon_tessellation::{
    FillRule,
    math::point,
    path::{Path, path::Builder},
};
use lumaglass_vector_geometry::{self as geometry, Vertex};
use skrifa::{
    DrawMemory, Font as FontData, PathSink, ScalerFont,
    raw::{TableProvider, tables::kern::SubtableKind},
};

pub struct Font {
    scaler: ScalerFont,
}

pub struct LineMetrics {
    pub ascent: f32,
    pub descent: f32,
}

pub struct Metrics {
    pub advance_width: f32,
}

impl Font {
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, String> {
        let face = FontData::new(bytes, 0).ok_or("Invalid font data")?;
        // Generated assets are SFNT fonts. Do not accept the model's synthetic
        // metrics for a missing/invalid head or fonts outside our u16 glyph API.
        let metrics = face.metrics();
        if !(16..=16384).contains(&metrics.raw_units_per_em)
            || face.num_glyphs() == 0
            || face.num_glyphs() > u32::from(u16::MAX)
            || metrics.h_line().is_none()
            || face.tables().hmtx().is_err()
        {
            return Err("Invalid font metrics".into());
        }
        let scaler = ScalerFont::unhinted(face, None);
        if scaler.outline_source().is_none() {
            return Err("Font has no supported outlines".into());
        }
        Ok(Self { scaler })
    }

    pub fn lookup_glyph_index(&self, character: char) -> u16 {
        self.scaler
            .font()
            .charmap()
            .map_unicode(character)
            .and_then(|id| u16::try_from(id.to_u32()).ok())
            .unwrap_or(0)
    }

    pub fn horizontal_line_metrics(&self, size: f32) -> Option<LineMetrics> {
        let face = self.scaler.font();
        let line = face.metrics().h_line()?;
        let scale = size / f32::from(face.units_per_em());
        Some(LineMetrics {
            ascent: line.ascender.to_f32() * scale,
            descent: line.descender.to_f32() * scale,
        })
    }

    pub fn metrics(&self, character: char, size: f32) -> Metrics {
        let face = self.scaler.font();
        let glyph = face.charmap().map_unicode(character).unwrap_or_default();
        Metrics {
            advance_width: face.glyph_metrics().h_advance(glyph) * size
                / f32::from(face.units_per_em()),
        }
    }

    pub fn horizontal_kern(&self, left: char, right: char, size: f32) -> Option<f32> {
        let face = self.scaler.font();
        let left = face.charmap().map_unicode(left)?;
        let right = face.charmap().map_unicode(right)?;
        for table in face.tables().kern().ok()?.subtables().flatten() {
            if !table.is_horizontal() || table.is_cross_stream() {
                continue;
            }
            // Preserve the existing first matching horizontal legacy-kern rule.
            // GPOS shaping is a separate change, not part of this parser migration.
            let Ok(kind) = table.kind() else {
                continue;
            };
            let value = match kind {
                SubtableKind::Format0(table) => table.kerning(left, right),
                SubtableKind::Format2(table) => table.kerning(left, right),
                SubtableKind::Format3(table) => table.kerning(left, right),
                SubtableKind::Format1(_) => None,
            };
            if let Some(value) = value {
                return Some(value as f32 * size / f32::from(face.units_per_em()));
            }
        }
        None
    }

    pub fn geometry(&self, character: char) -> Result<Vec<Vertex>, String> {
        let face = self.scaler.font();
        let mut outline = GlyphOutline {
            builder: Path::builder(),
            scale: 1. / f32::from(face.units_per_em()),
            open: false,
        };
        let glyph = face.charmap().map_unicode(character).unwrap_or_default();
        let Some(glyph) = self.scaler.outline(glyph) else {
            return Ok(Vec::new());
        };
        glyph
            .draw(DrawMemory::Scratch, &mut outline)
            .map_err(|error| format!("Glyph outline: {error}"))?;
        if outline.open {
            outline.builder.end(true);
        }
        // Flatten at subpixel precision for glyphs up to 256 physical px/em,
        // covering the current 1080p and 4K typography with the same geometry.
        geometry::tessellate(&outline.builder.build(), 0.15 / 256., FillRule::NonZero)
    }
}

struct GlyphOutline {
    builder: Builder,
    scale: f32,
    open: bool,
}

impl PathSink for GlyphOutline {
    fn move_to(&mut self, x: f32, y: f32) {
        if self.open {
            self.builder.end(true);
        }
        self.builder.begin(point(x * self.scale, -y * self.scale));
        self.open = true;
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.builder.line_to(point(x * self.scale, -y * self.scale));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.builder.quadratic_bezier_to(
            point(x1 * self.scale, -y1 * self.scale),
            point(x * self.scale, -y * self.scale),
        );
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.builder.cubic_bezier_to(
            point(x1 * self.scale, -y1 * self.scale),
            point(x2 * self.scale, -y2 * self.scale),
            point(x * self.scale, -y * self.scale),
        );
    }
    fn close(&mut self) {
        if self.open {
            self.builder.end(true);
            self.open = false;
        }
    }
}
