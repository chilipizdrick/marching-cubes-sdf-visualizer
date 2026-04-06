mod gui;
mod mesh;
mod raw_loader;
mod sdfs;
mod textures;
mod transforms;
mod uniforms;
mod vertex;

use std::{rc::Rc, sync::Arc, time::Instant};

use bumpalo::Bump;
use bytemuck::bytes_of;
use egui_wgpu::ScreenDescriptor;
use exp_rs::{EvalContext, Expression, error::ExprError};
use glam::{Quat, Vec3A};
use wgpu::{VertexBufferLayout, util::DeviceExt};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{MouseScrollDelta, WindowEvent},
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes, WindowId},
};

use crate::app::{
    gui::{EguiRenderer, SelectedSdf},
    mesh::{Grid, GridBuilder},
    raw_loader::ScalarField,
    textures::{ColorTexture, DepthTexture},
    transforms::{model_transform, projection_transform, view_transform},
    uniforms::Uniforms,
    vertex::{MeshData, Vertex},
};

#[derive(Default)]
pub struct App {
    state: Option<State<'static>>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = WindowAttributes::default();
        let window = event_loop.create_window(window_attributes).unwrap();

        let state = State::new(window);
        self.state = Some(state);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if let Some(state) = &mut self.state {
            state.window_event(event_loop, id, event);
        }
    }
}

impl App {
    pub fn new() -> Self {
        Self { state: None }
    }
}

struct State<'a> {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'a>,
    surface_config: wgpu::SurfaceConfiguration,
    window: Arc<Window>,
    uniforms: Uniforms,
    uniforms_buffer: wgpu::Buffer,
    uniforms_bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    depth_texture: DepthTexture,
    color_texture: ColorTexture,

    time: f32,
    last_update: Instant,
    camera_radius: f32,

    egui: EguiRenderer,

    mesh: MeshData,
}

impl State<'_> {
    fn new(window: Window) -> Self {
        let window_size = window.inner_size();
        let window = Arc::new(window);
        let instance = wgpu_instance();

        let surface = instance.create_surface(Arc::clone(&window)).unwrap();
        let adapter_options = wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            ..Default::default()
        };

        let adapter = pollster::block_on(instance.request_adapter(&adapter_options)).unwrap();
        let device_desc = Default::default();
        let (device, queue) = pollster::block_on(adapter.request_device(&device_desc)).unwrap();

        let surface_capabilities = surface.get_capabilities(&adapter);
        let texture_format = surface_capabilities
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_capabilities.formats[0]);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: texture_format,
            width: window_size.width,
            height: window_size.height,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };

        surface.configure(&device, &surface_config);

        let camera_pos = Vec3A::splat(3.0);

        let model = model_transform(Vec3A::ONE, Vec3A::ZERO, Quat::IDENTITY);
        let view = view_transform(camera_pos, Vec3A::ZERO, Vec3A::Z);
        let aspect_ratio = window_size.width as f32 / window_size.height as f32;
        let proj = projection_transform(std::f32::consts::PI / 2.0, aspect_ratio, 0.1, 100.0);

        let uniforms = Uniforms::new(model, view, proj, camera_pos);
        let uniforms_buffer_desc = wgpu::util::BufferInitDescriptor {
            label: Some("Uniforms Buffer"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        };
        let uniforms_buffer = device.create_buffer_init(&uniforms_buffer_desc);
        let uniforms_bind_group_layout_desc = wgpu::BindGroupLayoutDescriptor {
            label: Some("Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        };
        let uniforms_bind_group_layout =
            device.create_bind_group_layout(&uniforms_bind_group_layout_desc);
        let uniforms_bind_group_desc = wgpu::BindGroupDescriptor {
            label: Some("Bind Group"),
            layout: &uniforms_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms_buffer.as_entire_binding(),
            }],
        };
        let uniforms_bind_group = device.create_bind_group(&uniforms_bind_group_desc);
        let render_pipeline_layout_desc = wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&uniforms_bind_group_layout],
            push_constant_ranges: &[],
        };
        let render_pipeline_layout = device.create_pipeline_layout(&render_pipeline_layout_desc);
        let shader_module_desc = wgpu::include_wgsl!("shader.wgsl");
        let shader_module = device.create_shader_module(shader_module_desc);

        let egui = EguiRenderer::new(&device, texture_format, &window);

        let mesh = if std::env::args().len() > 1 {
            calculate_mesh_from_scalar_field()
        } else {
            calculate_mesh(&egui.state).unwrap()
        };

        let vertex_buffer_desc = wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        };
        let vertex_buffer = device.create_buffer_init(&vertex_buffer_desc);
        let index_buffer_desc = wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        };
        let index_buffer = device.create_buffer_init(&index_buffer_desc);
        let vertex_buffer_layout = VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 => Float32x3,
                1 => Float32x3,
            ],
        };
        let render_pipeline_desc = wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[vertex_buffer_layout],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DepthTexture::DEPTH_TEXTURE_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            // multisample: Default::default(),
            multisample: wgpu::MultisampleState {
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        };
        let render_pipeline = device.create_render_pipeline(&render_pipeline_desc);

        let depth_texture = DepthTexture::new(&device, &surface_config);
        let color_texture = ColorTexture::new(&device, &surface_config);

        let time = 0.0;
        let last_update = Instant::now();

        Self {
            device,
            queue,
            surface,
            surface_config,
            window,
            uniforms,
            uniforms_buffer,
            uniforms_bind_group,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            depth_texture,
            color_texture,

            time,
            last_update,

            camera_radius: 3.0,

            egui,

            mesh,
        }
    }

    fn render(&mut self) {
        let output = self.surface.get_current_texture().unwrap();
        let texture_view_desc = wgpu::TextureViewDescriptor::default();
        let view = output.texture.create_view(&texture_view_desc);

        let command_encoder_desc = wgpu::CommandEncoderDescriptor {
            label: Some("Command Encoder"),
        };
        let mut encoder = self.device.create_command_encoder(&command_encoder_desc);

        if !self.mesh.vertices.is_empty() {
            self.draw_scene(&mut encoder, &view);
        }
        self.draw_ui(&mut encoder, &view);

        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        output.present();
    }

    fn draw_scene(&self, encoder: &mut wgpu::CommandEncoder, resolve_target: &wgpu::TextureView) {
        let render_pass_desc = wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.color_texture.view,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: Some(resolve_target),
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),

            timestamp_writes: None,
            occlusion_query_set: None,
        };

        {
            let mut rpass = encoder.begin_render_pass(&render_pass_desc);
            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_bind_group(0, &self.uniforms_bind_group, &[]);
            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            rpass.draw_indexed(0..(self.mesh.indices.len() as u32), 0, 0..1);
        }
    }

    fn draw_ui(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [self.surface_config.width, self.surface_config.height],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        self.egui.draw(
            &self.device,
            &self.queue,
            encoder,
            &self.window,
            view,
            screen_descriptor,
            self::gui::main_window,
        );
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        use WindowEvent as WE;

        self.update_time();

        // log::info!("Window event: {event:?}");

        match event {
            WE::CloseRequested => event_loop.exit(),
            WE::Resized(size) => self.handle_window_resized(size),
            WE::RedrawRequested => self.render(),
            WE::MouseWheel { delta, .. } => self.handle_mouse_wheel(delta),

            _ => {}
        }

        self.egui.handle_input(&self.window, &event);
        self.update_state();

        self.write_uniforms();
        self.window.request_redraw();
    }

    fn update_time(&mut self) {
        let now = Instant::now();
        self.time += (now - self.last_update).as_secs_f32();
        self.last_update = now;
    }

    fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        let PhysicalSize { height, .. } = self.window.inner_size();
        let multiplier = multiplier_from_mouse_delta(delta, height as f32);
        self.camera_radius *= multiplier;
    }

    fn update_state(&mut self) {
        let camera_pos = Vec3A::new(
            self.camera_radius * self.time.cos(),
            self.camera_radius * self.time.sin(),
            1.5,
        );
        self.uniforms.camera_pos = camera_pos;
        self.uniforms.view = view_transform(camera_pos, Vec3A::ZERO, Vec3A::Z);

        let gui_state = &mut self.egui.state;
        if gui_state.mesh_recalculation_requested {
            gui_state.mesh_recalculation_requested = false;

            match calculate_mesh(gui_state) {
                Ok(mesh) => {
                    self.mesh = mesh;
                    self.update_buffers();
                }
                Err(e) => log::error!("Error calculating mesh: {}", e),
            }
        }
    }

    fn update_buffers(&mut self) {
        let vertex_buffer_desc = wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&self.mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        };
        self.vertex_buffer = self.device.create_buffer_init(&vertex_buffer_desc);
        let index_buffer_desc = wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&self.mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        };
        self.index_buffer = self.device.create_buffer_init(&index_buffer_desc);
    }

    fn handle_window_resized(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.surface_config.width = size.width;
        self.surface_config.height = size.height;
        self.surface.configure(&self.device, &self.surface_config);
        self.depth_texture = DepthTexture::new(&self.device, &self.surface_config);
        self.color_texture = ColorTexture::new(&self.device, &self.surface_config);

        self.uniforms.proj = projection_transform(
            std::f32::consts::PI / 2.0,
            size.width as f32 / size.height as f32,
            0.1,
            100.0,
        );
    }

    fn write_uniforms(&self) {
        let buffer = &self.uniforms_buffer;
        self.queue.write_buffer(buffer, 0, bytes_of(&self.uniforms))
    }
}

fn wgpu_instance() -> wgpu::Instance {
    let backends = wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::VULKAN;
    let flags = wgpu::InstanceFlags::DEBUG
        | wgpu::InstanceFlags::VALIDATION
        | wgpu::InstanceFlags::GPU_BASED_VALIDATION
        | wgpu::InstanceFlags::AUTOMATIC_TIMESTAMP_NORMALIZATION;

    let desc = wgpu::InstanceDescriptor {
        backends,
        flags,
        ..Default::default()
    };

    wgpu::Instance::new(&desc)
}

fn calculate_mesh(gui_state: &gui::State) -> Result<MeshData, ExprError> {
    let mut grid = Grid::builder()
        .x_range(gui_state.x_range)
        .y_range(gui_state.y_range)
        .z_range(gui_state.z_range)
        .xyz_delta(gui_state.delta)
        .build()
        .unwrap();

    let isovalue = gui_state.isovalue;

    let mesh = match gui_state.selected_sdf {
        SelectedSdf::PreDefined(sdf) => {
            let mut sdf_fn = sdf.sdf_fn();
            grid.generate_mesh_from_fn(&mut sdf_fn, isovalue)
        }
        SelectedSdf::Custom => {
            let arena = Bump::new();
            let ctx = Rc::new(EvalContext::new());
            let mut builder = Expression::new(&arena);
            builder.add_parameter("x", 0.0).unwrap();
            builder.add_parameter("y", 0.0).unwrap();
            builder.add_parameter("z", 0.0).unwrap();
            builder.add_expression(&gui_state.sdf_text)?;

            let mut sdf_fn = |x, y, z| {
                builder.set("x", x as f64).unwrap();
                builder.set("y", y as f64).unwrap();
                builder.set("z", z as f64).unwrap();
                builder.eval(&ctx).unwrap();
                builder.get_result(0).unwrap() as f32
            };

            grid.generate_mesh_from_fn(&mut sdf_fn, isovalue)
        }
    };

    log::info!(
        "Generated mesh with {} vertices and {} indices",
        mesh.vertices.len(),
        mesh.indices.len()
    );

    Ok(mesh)
}

fn calculate_mesh_from_scalar_field() -> MeshData {
    let args: Vec<String> = std::env::args().collect();
    let field_x_len = args[2].parse().unwrap();
    let field_y_len = args[3].parse().unwrap();
    let field_z_len = args[4].parse().unwrap();

    let field = ScalarField::read_from_u8_yzx_file_with_size(
        &args[1],
        field_x_len,
        field_y_len,
        field_z_len,
    )
    .unwrap();

    let mut grid = GridBuilder::new()
        .x_range((-1.0, 1.0))
        .y_range((-1.0, 1.0))
        .z_range((-1.0, 1.0))
        .xyz_delta((
            2.0 / (field_x_len - 1) as f32,
            2.0 / (field_y_len - 1) as f32,
            2.0 / (field_z_len - 1) as f32,
        ))
        .build()
        .unwrap();

    grid.generate_mesh_from_scalar_field(field, 0.5)
}

fn multiplier_from_mouse_delta(delta: MouseScrollDelta, window_height: f32) -> f32 {
    const PIXEL_DELTA_SCROLL_SENSITIVITY: f32 = 5.0;

    match delta {
        MouseScrollDelta::LineDelta(_, y) => match y {
            ..0.0 => 0.9,
            0.0.. => 1.1,
            _ => 1.0,
        },
        MouseScrollDelta::PixelDelta(PhysicalPosition { y, .. }) => match y {
            ..0.0 => 1.0 - (y as f32 / window_height * PIXEL_DELTA_SCROLL_SENSITIVITY).abs(),
            0.0.. => 1.0 + (y as f32 / window_height * PIXEL_DELTA_SCROLL_SENSITIVITY).abs(),
            _ => 1.0,
        },
    }
}
