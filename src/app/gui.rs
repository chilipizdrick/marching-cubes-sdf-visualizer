use std::fmt::Display;

use egui::epaint::Shadow;
use egui::{Context, Visuals};
use egui_wgpu::ScreenDescriptor;
use egui_wgpu::{Renderer, RendererOptions};

use egui_wgpu::wgpu;
use egui_wgpu::wgpu::{CommandEncoder, Device, Queue, TextureFormat, TextureView};
use egui_winit::winit::event::WindowEvent;
use egui_winit::winit::window::Window;

use crate::app::sdfs::SdfFuction;

pub struct EguiRenderer {
    context: Context,
    window_state: egui_winit::State,
    renderer: Renderer,
    pub state: State,
}

impl EguiRenderer {
    pub fn new(
        device: &Device,
        output_texture_format: TextureFormat,
        window: &Window,
    ) -> EguiRenderer {
        let context = Context::default();
        let viewport_id = context.viewport_id();

        let visuals = Visuals {
            dark_mode: true,
            window_shadow: Shadow::NONE,
            ..Default::default()
        };

        context.set_visuals(visuals);

        let window_state =
            egui_winit::State::new(context.clone(), viewport_id, &window, None, None, None);
        let state = State::default();

        let renderer_options = RendererOptions::default();
        let renderer = Renderer::new(device, output_texture_format, renderer_options);

        EguiRenderer {
            context,
            window_state,
            renderer,
            state,
        }
    }

    pub fn handle_input(&mut self, window: &Window, event: &WindowEvent) {
        let _ = self.window_state.on_window_event(window, event);
    }

    pub fn draw(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        window: &Window,
        window_surface_view: &TextureView,
        screen_descriptor: ScreenDescriptor,
        mut run_ui: impl FnMut(&Context, &mut State),
    ) {
        // self.state.set_pixels_per_point(window.scale_factor() as f32);
        let raw_input = self.window_state.take_egui_input(window);
        let full_output = self.context.run(raw_input, |ctx| {
            run_ui(ctx, &mut self.state);
        });

        self.window_state
            .handle_platform_output(window, full_output.platform_output);

        let tris = self
            .context
            .tessellate(full_output.shapes, full_output.pixels_per_point);
        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }

        self.renderer
            .update_buffers(device, queue, encoder, &tris, &screen_descriptor);

        {
            let rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: window_surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                label: Some("Egui Main Render Pass"),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            let mut rpass = rpass.forget_lifetime();
            self.renderer.render(&mut rpass, &tris, &screen_descriptor);
        }

        for texture_id in &full_output.textures_delta.free {
            self.renderer.free_texture(texture_id)
        }
    }
}

#[derive(Debug, Clone)]
pub struct State {
    pub x_range: (f32, f32),
    pub y_range: (f32, f32),
    pub z_range: (f32, f32),
    pub delta: (f32, f32, f32),
    pub isovalue: f32,
    pub selected_sdf: SelectedSdf,
    pub sdf_text: String,
    pub mesh_recalculation_requested: bool,
}

// const EXAMPLE_SDF: &str = "((x*x+y*y-0.852)^2+(z*z -1.0)^2)*((y*y+z*z-0.852)^2+(x*x-1.0)^2)*((z*z+x*x-0.852)^2+(y*y-1.0)^2)-0.001";

impl Default for State {
    fn default() -> Self {
        Self {
            x_range: (-1.1, 1.1),
            y_range: (-1.1, 1.1),
            z_range: (-1.1, 1.1),
            delta: (0.1, 0.1, 0.1),
            isovalue: 0.0,
            selected_sdf: Default::default(),
            sdf_text: "x^2 + y^2 + z^2 - 1".to_string(),
            mesh_recalculation_requested: false,
        }
    }
}

pub fn main_window(ui: &Context, state: &mut State) {
    egui::Window::new("SDF Visualizer")
        .default_open(true)
        .vscroll(true)
        .max_width(500.0)
        .max_height(800.0)
        .default_width(300.0)
        .resizable(true)
        .show(ui, |ui| {
            ui.heading("Grid Settings");

            ui.horizontal_wrapped(|ui| {
                ui.label("X from");
                ui.add(egui::DragValue::new(&mut state.x_range.0).speed(0.1));
                ui.label("to");
                ui.add(egui::DragValue::new(&mut state.x_range.1).speed(0.1));
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Y from");
                ui.add(egui::DragValue::new(&mut state.y_range.0).speed(0.1));
                ui.label("to");
                ui.add(egui::DragValue::new(&mut state.y_range.1).speed(0.1));
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Z from");
                ui.add(egui::DragValue::new(&mut state.z_range.0).speed(0.1));
                ui.label("to");
                ui.add(egui::DragValue::new(&mut state.z_range.1).speed(0.1));
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Delta: X");
                ui.add(egui::DragValue::new(&mut state.delta.0).speed(0.1));
                ui.label("Y");
                ui.add(egui::DragValue::new(&mut state.delta.1).speed(0.1));
                ui.label("Z");
                ui.add(egui::DragValue::new(&mut state.delta.2).speed(0.1));
            });

            ui.separator();

            ui.heading("SDF Settings");

            ui.horizontal_wrapped(|ui| {
                egui::ComboBox::from_label("SDF")
                    .selected_text(state.selected_sdf.to_string())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut state.selected_sdf,
                            SelectedSdf::PreDefined(SdfFuction::Sphere),
                            SdfFuction::Sphere.to_string(),
                        );
                        ui.selectable_value(
                            &mut state.selected_sdf,
                            SelectedSdf::PreDefined(SdfFuction::Plane),
                            SdfFuction::Plane.to_string(),
                        );
                        ui.selectable_value(
                            &mut state.selected_sdf,
                            SelectedSdf::PreDefined(SdfFuction::Octahedron),
                            SdfFuction::Octahedron.to_string(),
                        );
                        ui.selectable_value(
                            &mut state.selected_sdf,
                            SelectedSdf::PreDefined(SdfFuction::CubeRingFrame),
                            SdfFuction::CubeRingFrame.to_string(),
                        );
                        ui.selectable_value(
                            &mut state.selected_sdf,
                            SelectedSdf::Custom,
                            "Custom".to_string(),
                        );
                    });
            });

            ui.horizontal_wrapped(|ui| {
                ui.label("Isovalue:");
                ui.add(egui::DragValue::new(&mut state.isovalue).speed(0.1))
            });

            if state.selected_sdf == SelectedSdf::Custom {
                ui.label("Custom SDF:");
                ui.text_edit_multiline(&mut state.sdf_text);
            }

            if ui.button("Recalculate mesh").clicked() {
                state.mesh_recalculation_requested = true;
            };
        });
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectedSdf {
    PreDefined(SdfFuction),
    Custom,
}

impl Default for SelectedSdf {
    fn default() -> Self {
        Self::PreDefined(Default::default())
    }
}

impl Display for SelectedSdf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelectedSdf::PreDefined(sdf) => write!(f, "{}", sdf),
            SelectedSdf::Custom => write!(f, "Custom"),
        }
    }
}
