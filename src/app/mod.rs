mod gui;
mod marching_cubes;
mod mesh;
mod scalar_field;
mod sdf_functions;
mod spinoidal_decomposition;
mod state;
mod textures;
mod uniforms;

use std::sync::Arc;

use bytemuck::bytes_of;
use egui_wgpu::ScreenDescriptor;
use wgpu::util::DeviceExt;
use winit::{
    event::{KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{Key, NamedKey},
    window::{CursorGrabMode, Window, WindowId},
};

use crate::app::{
    gui::EguiRenderer,
    mesh::Vertex,
    state::State,
    textures::{DepthTexture, MSAATexture},
};

pub struct AppState<'a> {
    instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'a>,
    surface_config: wgpu::SurfaceConfiguration,
    window: Arc<Window>,
    uniforms_buffer: wgpu::Buffer,
    uniforms_bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    depth_texture: DepthTexture,
    msaa_texture: MSAATexture,

    egui: EguiRenderer,

    state: State,
}

impl AppState<'_> {
    pub fn new(window: Window) -> anyhow::Result<Self> {
        let window_size = window.inner_size();
        let window = Arc::new(window);
        let instance = wgpu_instance();

        let surface = instance.create_surface(Arc::clone(&window))?;
        let adapter_options = wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            ..Default::default()
        };

        let adapter = pollster::block_on(instance.request_adapter(&adapter_options))?;
        let device_desc = Default::default();
        let (device, queue) = pollster::block_on(adapter.request_device(&device_desc))?;

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

        let aspect_ratio = window_size.width as f32 / window_size.height as f32;
        let state = State::new(aspect_ratio);

        let uniforms_buffer_desc = wgpu::util::BufferInitDescriptor {
            label: Some("Uniforms Buffer"),
            contents: bytemuck::bytes_of(&state.uniforms),
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
            bind_group_layouts: &[Some(&uniforms_bind_group_layout)],
            immediate_size: 0,
        };
        let render_pipeline_layout = device.create_pipeline_layout(&render_pipeline_layout_desc);
        let shader_module_desc = wgpu::include_wgsl!("shader.wgsl");
        let shader_module = device.create_shader_module(shader_module_desc);

        let vertex_buffer_desc = wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&state.mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        };
        let vertex_buffer = device.create_buffer_init(&vertex_buffer_desc);
        let index_buffer_desc = wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&state.mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        };
        let index_buffer = device.create_buffer_init(&index_buffer_desc);
        let vertex_buffer_layout = wgpu::VertexBufferLayout {
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
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
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
            multiview_mask: None,
            cache: None,
        };
        let render_pipeline = device.create_render_pipeline(&render_pipeline_desc);

        let depth_texture = DepthTexture::new(&device, &surface_config);
        let msaa_texture = MSAATexture::new(&device, &surface_config);

        let egui = EguiRenderer::new(&device, texture_format, &window);

        Ok(Self {
            instance,
            device,
            queue,
            surface,
            surface_config,
            window,
            uniforms_buffer,
            uniforms_bind_group,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            depth_texture,
            msaa_texture,

            egui,

            state,
        })
    }

    pub fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        use WindowEvent as WE;

        // log::debug!("Window event: {event:?}");

        let response = self.egui.handle_input(&self.window, &event);

        // log::debug!("egui response: {response:?}");

        if !response.consumed {
            match &event {
                WE::CloseRequested => self.handle_close_requested(event_loop),
                WE::Resized(size) => self.handle_window_resized(size),
                WE::KeyboardInput {
                    event: key_event, ..
                } => self.handle_key_event(key_event),
                _ => {}
            }
        }

        self.state.update();

        if self.state.mesh_changed {
            self.update_vertex_index_buffers();
        }

        self.write_uniforms();

        if event == WE::RedrawRequested {
            self.render();
        }

        self.window.request_redraw();
    }

    pub fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        event: winit::event::DeviceEvent,
    ) {
        // log::info!("Device event: {event:?}");
        if self.state.camera_mode_on {
            use winit::event::DeviceEvent as DE;
            if let DE::MouseMotion { delta } = event {
                self.state.handle_mouse_motion(delta);
            }
        }
    }

    fn handle_close_requested(&self, event_loop: &ActiveEventLoop) {
        event_loop.exit();
    }

    fn handle_window_resized(&mut self, size: &winit::dpi::PhysicalSize<u32>) {
        self.surface_config.width = size.width;
        self.surface_config.height = size.height;
        self.surface.configure(&self.device, &self.surface_config);
        self.depth_texture = DepthTexture::new(&self.device, &self.surface_config);
        self.msaa_texture = MSAATexture::new(&self.device, &self.surface_config);

        self.state.camera.aspect_ratio = size.width as f32 / size.height as f32;
        self.state.uniforms.view_proj = self.state.camera.view_proj_matrix();
    }

    fn handle_key_event(&mut self, key_event: &KeyEvent) {
        if key_event.logical_key == Key::Named(NamedKey::Escape) && key_event.state.is_pressed() {
            self.state.camera_mode_on = !self.state.camera_mode_on;
            if self.state.camera_mode_on {
                self.window.set_cursor_grab(CursorGrabMode::Locked).unwrap();
                self.window.set_cursor_visible(false);
            } else {
                self.window.set_cursor_grab(CursorGrabMode::None).unwrap();
                self.window.set_cursor_visible(true);
            }
        }

        self.state.handle_key_event(key_event)
    }

    fn render(&mut self) {
        use wgpu::CurrentSurfaceTexture as CST;

        let output = self.surface.get_current_texture();
        match output {
            CST::Success(output) => self.draw_frame(output),
            CST::Suboptimal(output) => {
                self.surface.configure(&self.device, &self.surface_config);
                self.draw_frame(output);
            }
            CST::Outdated => {
                self.surface.configure(&self.device, &self.surface_config);
            }
            CST::Timeout | CST::Occluded | CST::Validation => {}
            CST::Lost => match self.instance.create_surface(self.window.clone()) {
                Ok(surface) => {
                    self.surface = surface;
                    self.surface.configure(&self.device, &self.surface_config);
                }
                Err(e) => log::error!("Error creating surface: {e}"),
            },
        }
    }

    fn draw_frame(&mut self, output: wgpu::SurfaceTexture) {
        let texture_view_desc = wgpu::TextureViewDescriptor::default();
        let view = output.texture.create_view(&texture_view_desc);

        let command_encoder_desc = wgpu::CommandEncoderDescriptor {
            label: Some("Command Encoder"),
        };
        let mut encoder = self.device.create_command_encoder(&command_encoder_desc);

        // if !self.state.mesh.vertices.is_empty() {
        //     self.draw_mesh(&mut encoder, &mut rpass);
        // }
        self.draw_mesh(&mut encoder, &view);

        if !self.state.camera_mode_on {
            self.draw_ui(&mut encoder, &view);
        }

        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        output.present();
    }

    fn write_uniforms(&self) {
        let buffer = &self.uniforms_buffer;
        self.queue
            .write_buffer(buffer, 0, bytes_of(&self.state.uniforms))
    }

    fn draw_mesh(&self, encoder: &mut wgpu::CommandEncoder, resolve_target: &wgpu::TextureView) {
        let render_pass_desc = wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.msaa_texture.view,
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
            multiview_mask: None,
        };

        let mut rpass = encoder.begin_render_pass(&render_pass_desc);
        if !self.state.mesh.vertices.is_empty() {
            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_bind_group(0, &self.uniforms_bind_group, &[]);
            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            rpass.draw_indexed(0..(self.state.mesh.indices.len() as u32), 0, 0..1);
        }
    }

    fn draw_ui(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let window_size = self.window.inner_size();
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [window_size.width, window_size.height],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        self.egui.draw(
            &self.device,
            &self.queue,
            encoder,
            &self.window,
            view,
            screen_descriptor,
            &mut self.state,
        )
    }

    fn update_vertex_index_buffers(&mut self) {
        let vertex_buffer_desc = wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&self.state.mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        };
        self.vertex_buffer = self.device.create_buffer_init(&vertex_buffer_desc);
        let index_buffer_desc = wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&self.state.mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        };
        self.index_buffer = self.device.create_buffer_init(&index_buffer_desc);

        self.state.mesh_changed = false;
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
        memory_budget_thresholds: Default::default(),
        backend_options: Default::default(),
        display: None,
    };

    wgpu::Instance::new(desc)
}
