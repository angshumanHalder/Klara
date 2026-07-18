use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{layout::Rect, pane::Pane, terminal::grid::Color as TermColor};
use bytemuck::{Pod, Zeroable};
use glyphon::{
    Attrs, Buffer as TextBuffer, Cache, Color as GColor, Family, FontSystem, Metrics, Resolution,
    Shaping, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport, fontdb,
};
use wgpu::{
    BlendState, BufferAddress, BufferDescriptor, BufferUsages, ColorTargetState, ColorWrites,
    CommandEncoder, Device, FragmentState, MultisampleState, PipelineLayoutDescriptor,
    PrimitiveState, Queue, RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderSource,
    TextureFormat, TextureView, VertexBufferLayout, VertexState, vertex_attr_array,
};

const FONT_SIZE: f32 = 14.0;
pub const LINE_HEIGHT: f32 = 17.0;
pub const CELL_W: f32 = 8.5;
const MAX_CELLS: u64 = 220 * 60;

fn indexed_col(i: u8) -> [u8; 3] {
    match i {
        0 => [0x16, 0x16, 0x1d],  // black
        1 => [0xc3, 0x42, 0x3f],  // red
        2 => [0x76, 0x94, 0x6a],  // green
        3 => [0xc0, 0xa3, 0x6e],  // yellow
        4 => [0x7e, 0x9c, 0xd8],  // blue
        5 => [0x95, 0x7f, 0xb8],  // magenta
        6 => [0x6a, 0x9e, 0x89],  // cyan
        7 => [0xc8, 0xc0, 0x93],  // white
        8 => [0x72, 0x72, 0x62],  // bright black
        9 => [0xe8, 0x27, 0x26],  // bright red
        10 => [0x98, 0xbb, 0x6c], // bright green
        11 => [0xe6, 0xc3, 0x84], // bright yellow
        12 => [0x7f, 0xb4, 0xca], // bright blue
        13 => [0x93, 0x8a, 0xa9], // bright magenta
        14 => [0x7a, 0xa8, 0x9f], // bright cyan
        15 => [0xdc, 0xd7, 0xba], // bright white
        16..=231 => {
            let i = i - 16;
            let b = i % 6;
            let g = (i / 6) % 6;
            let r = i / 36;
            let c = |v: u8| if v == 0 { 0 } else { v * 40 + 55 };
            [c(r), c(g), c(b)]
        }
        232..=255 => {
            let v = (i - 232) * 10 + 8;
            [v, v, v]
        }
    }
}

fn term_to_gcolor(c: &TermColor) -> GColor {
    match c {
        TermColor::Default => GColor::rgb(0xcd, 0xd6, 0xf4),
        TermColor::Indexed(i) => {
            let [r, g, b] = indexed_col(*i);
            GColor::rgb(r, g, b)
        }
        TermColor::Rgb(r, g, b) => GColor::rgb(*r, *g, *b),
    }
}

fn term_to_rgba(c: &TermColor) -> Option<[f32; 4]> {
    match c {
        TermColor::Default => None,
        TermColor::Indexed(i) => {
            let [r, g, b] = indexed_col(*i);
            Some([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0])
        }
        TermColor::Rgb(r, g, b) => {
            Some([*r as f32 / 255.0, *g as f32 / 255.0, *b as f32 / 255.0, 1.0])
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct BgVertex {
    pos: [f32; 2],
    color: [f32; 4],
}

struct CachedRow {
    buf: TextBuffer,
    bg_verts: Vec<BgVertex>,
}

pub struct Renderer {
    font_system: FontSystem,
    swash_cache: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    bg_pipeline: wgpu::RenderPipeline,
    bg_buf: wgpu::Buffer,
    screen_w: f32,
    screen_h: f32,
    font_id: fontdb::ID,
    pane_cache: HashMap<usize, Vec<CachedRow>>,
}

impl Renderer {
    pub fn new(
        device: &Device,
        queue: &Queue,
        format: TextureFormat,
        screen_w: u32,
        screen_h: u32,
    ) -> Self {
        let font_system = FontSystem::new();
        let font_id = font_system
            .db()
            .query(&fontdb::Query {
                families: &[fontdb::Family::Name("JetBrains Mono")],
                ..Default::default()
            })
            .expect("JetBrains Mono not found in system fonts");
        let swash_cache = SwashCache::new();

        let cache = Cache::new(device);
        let mut viewport = Viewport::new(device, &cache);
        viewport.update(
            queue,
            Resolution {
                width: screen_w,
                height: screen_h,
            },
        );
        let mut atlas = TextAtlas::new(device, queue, &cache, format);
        let text_renderer =
            TextRenderer::new(&mut atlas, device, MultisampleState::default(), None);

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(include_str!("bg.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let bg_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[VertexBufferLayout {
                    array_stride: std::mem::size_of::<BgVertex>() as BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &vertex_attr_array![0 => Float32x2, 1 => Float32x4],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let bg_buf = device.create_buffer(&BufferDescriptor {
            label: None,
            size: MAX_CELLS * 6 * std::mem::size_of::<BgVertex>() as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            font_system,
            swash_cache,
            viewport,
            atlas,
            text_renderer,
            bg_pipeline,
            bg_buf,
            font_id,
            screen_w: screen_w as f32,
            screen_h: screen_h as f32,
            pane_cache: HashMap::new(),
        }
    }

    pub fn resize(&mut self, queue: &Queue, w: u32, h: u32) {
        self.screen_w = w as f32;
        self.screen_h = h as f32;
        self.viewport.update(
            queue,
            Resolution {
                width: w,
                height: h,
            },
        );
    }

    pub fn draw(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        pane_layouts: &[(Arc<Mutex<Pane>>, Rect)],
    ) {
        let active_panes: std::collections::HashSet<_> = pane_layouts
            .iter()
            .map(|(p, _)| Arc::as_ptr(p) as usize)
            .collect();
        self.pane_cache.retain(|id, _| active_panes.contains(id));

        let mut bg_verts: Vec<BgVertex> = Vec::new();

        let screen_w = self.screen_w;
        let screen_h = self.screen_h;

        let font_system = &mut self.font_system;
        let pane_cache = &mut self.pane_cache;

        // Pass 1: update dirty rows. Mutable borrows of pane_cache/cache
        // live only within this loop.
        for (pane_arc, rect) in pane_layouts {
            let pane_id = Arc::as_ptr(pane_arc) as usize;
            let pane = pane_arc.lock().unwrap();
            let mut grid = pane.grid.lock().unwrap();

            let cache = pane_cache.entry(pane_id).or_insert_with(Vec::new);
            while cache.len() < grid.rows {
                cache.push(CachedRow {
                    buf: TextBuffer::new(&mut *font_system, Metrics::new(FONT_SIZE, LINE_HEIGHT)),
                    bg_verts: Vec::new(),
                });
            }

            for row_idx in 0..grid.rows {
                let y = rect.y + row_idx as f32 * LINE_HEIGHT;
                let cached_row = &mut cache[row_idx];

                if grid.dirty[row_idx] {
                    cached_row.bg_verts.clear();
                    let mut text = String::with_capacity(grid.cols);
                    let mut fg = Vec::new();

                    let mut last_color = None;
                    let mut span_start = 0;

                    for col in 0..grid.cols {
                        let cell = grid.cell(row_idx, col);
                        let start = text.len();
                        text.push(cell.ch);
                        if let Some(c) = term_to_rgba(&cell.bg) {
                            let x = rect.x + col as f32 * CELL_W;
                            cached_row.bg_verts.extend_from_slice(&Self::bg_quad(
                                screen_w,
                                screen_h,
                                x,
                                y,
                                CELL_W,
                                LINE_HEIGHT,
                                c,
                            ));
                        }
                        let current_color = term_to_gcolor(&cell.fg);
                        if Some(current_color) != last_color {
                            if let Some(c) = last_color {
                                fg.push((span_start..start, c));
                            }
                            span_start = start;
                            last_color = Some(current_color);
                        }
                    }

                    if let Some(c) = last_color {
                        fg.push((span_start..text.len(), c));
                    }

                    cached_row
                        .buf
                        .set_size(font_system, Some(rect.width), Some(LINE_HEIGHT));

                    let default_attrs = Attrs::new().family(Family::Name("JetBrains Mono"));
                    let spans: Vec<_> = fg
                        .iter()
                        .map(|(range, c)| (&text[range.clone()], default_attrs.color(*c)))
                        .collect();

                    cached_row.buf.set_rich_text(
                        font_system,
                        spans.into_iter(),
                        default_attrs,
                        Shaping::Basic,
                    );
                    cached_row.buf.shape_until_scroll(font_system, false);
                    grid.dirty[row_idx] = false;
                }

                bg_verts.extend_from_slice(&cached_row.bg_verts);
            }
        }

        let mut text_areas: Vec<TextArea> = Vec::new();
        for (pane_arc, rect) in pane_layouts {
            let pane_id = Arc::as_ptr(pane_arc) as usize;
            let cache = &pane_cache[&pane_id];
            for (row_idx, cached_row) in cache.iter().enumerate() {
                let y = rect.y + row_idx as f32 * LINE_HEIGHT;
                text_areas.push(TextArea {
                    buffer: &cached_row.buf,
                    left: rect.x,
                    top: y,
                    scale: 1.0,
                    bounds: TextBounds {
                        left: rect.x as i32,
                        top: y as i32,
                        right: (rect.x + rect.width) as i32,
                        bottom: (y + LINE_HEIGHT) as i32,
                    },
                    default_color: GColor::rgb(0xdc, 0xd7, 0xba),
                    custom_glyphs: &[],
                });
            }
        }

        self.text_renderer
            .prepare(
                device,
                queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                text_areas,
                &mut self.swash_cache,
            )
            .unwrap();

        if !bg_verts.is_empty() {
            queue.write_buffer(&self.bg_buf, 0, bytemuck::cast_slice(&bg_verts));
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        if !bg_verts.is_empty() {
            pass.set_pipeline(&self.bg_pipeline);
            pass.set_vertex_buffer(0, self.bg_buf.slice(..));
            pass.draw(0..bg_verts.len() as u32, 0..1);
        }

        self.text_renderer
            .render(&self.atlas, &self.viewport, &mut pass)
            .unwrap();
    }

    fn ndc(screen_w: f32, screen_h: f32, x: f32, y: f32) -> [f32; 2] {
        [x / screen_w * 2.0 - 1.0, 1.0 - y / screen_h * 2.0]
    }

    fn bg_quad(
        screen_w: f32,
        screen_h: f32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        c: [f32; 4],
    ) -> [BgVertex; 6] {
        let [x1, y1] = Self::ndc(screen_w, screen_h, x, y);
        let [x2, y2] = Self::ndc(screen_w, screen_h, x + w, y + h);
        let v = |px, py| BgVertex {
            pos: [px, py],
            color: c,
        };
        [
            v(x1, y1),
            v(x2, y1),
            v(x2, y2),
            v(x1, y1),
            v(x2, y2),
            v(x1, y2),
        ]
    }
}
