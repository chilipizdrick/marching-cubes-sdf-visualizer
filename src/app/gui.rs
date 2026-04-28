use egui::Context;
use egui::global_theme_preference_switch;
use egui_wgpu::Renderer;
use egui_wgpu::ScreenDescriptor;

use egui_wgpu::wgpu;
use egui_wgpu::wgpu::{CommandEncoder, Device, Queue, TextureFormat, TextureView};
use egui_winit::EventResponse;
use egui_winit::winit::event::WindowEvent;
use egui_winit::winit::window::Window;

use crate::app::State;
use crate::app::marching_cubes::GenerationMethod;
use crate::app::sdf_functions::SdfFuction;
use crate::app::state::VisualizationMode;

pub struct EguiRenderer {
    pub context: Context,
    pub window_state: egui_winit::State,
    pub renderer: Renderer,
}

impl EguiRenderer {
    pub fn new(
        device: &Device,
        output_texture_format: TextureFormat,
        window: &Window,
    ) -> EguiRenderer {
        let context = Context::default();
        let viewport_id = context.viewport_id();
        let window_state =
            egui_winit::State::new(context.clone(), viewport_id, &window, None, None, None);
        let renderer_options = Default::default();
        let renderer = Renderer::new(device, output_texture_format, renderer_options);

        EguiRenderer {
            context,
            window_state,
            renderer,
        }
    }

    pub fn handle_input(&mut self, window: &Window, event: &WindowEvent) -> EventResponse {
        self.window_state.on_window_event(window, event)
    }

    pub fn draw(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        window: &Window,
        window_surface_view: &TextureView,
        screen_descriptor: ScreenDescriptor,
        state: &mut State,
    ) {
        let raw_input = self.window_state.take_egui_input(window);
        let full_output = self.context.run_ui(raw_input, |ctx| {
            run_ui(ctx, state);
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
                label: Some("Egui Render Pass"),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            let mut rpass = rpass.forget_lifetime();
            self.renderer.render(&mut rpass, &tris, &screen_descriptor);
        }

        for texture_id in &full_output.textures_delta.free {
            self.renderer.free_texture(texture_id)
        }
    }
}

fn run_ui(ctx: &Context, state: &mut State) {
    egui::Window::new("SDF Visualizer")
        .vscroll(true)
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                global_theme_preference_switch(ui);
                ui.selectable_value(
                    &mut state.visualization_mode,
                    VisualizationMode::Fn,
                    "Function",
                );
                ui.selectable_value(
                    &mut state.visualization_mode,
                    VisualizationMode::LoadFromFile,
                    "Load from file",
                );
                ui.selectable_value(
                    &mut state.visualization_mode,
                    VisualizationMode::SpinodalDecomposition,
                    "Spinodal Decomposition",
                );
            });

            ui.separator();

            match state.visualization_mode {
                VisualizationMode::Fn => {
                    draw_fn_ui(ui, state);
                }
                VisualizationMode::LoadFromFile => {
                    draw_load_from_file_ui(ui, state);
                }
                VisualizationMode::SpinodalDecomposition => {
                    draw_spinodal_decomposition_ui(ui, state)
                }
            }
        });
}

fn draw_fn_ui(ui: &mut egui::Ui, state: &mut State) {
    ui.heading("Function SDF");

    ui.label("Grid settings");
    ui.horizontal(|ui| {
        ui.label("X from ");
        ui.add(egui::DragValue::new(&mut state.fn_grid_settings.x_range.0));
        ui.label(" to ");
        ui.add(egui::DragValue::new(&mut state.fn_grid_settings.x_range.1));
    });

    ui.horizontal(|ui| {
        ui.label("Y from ");
        ui.add(egui::DragValue::new(&mut state.fn_grid_settings.y_range.0));
        ui.label(" to ");
        ui.add(egui::DragValue::new(&mut state.fn_grid_settings.y_range.1));
    });

    ui.horizontal(|ui| {
        ui.label("Z from ");
        ui.add(egui::DragValue::new(&mut state.fn_grid_settings.z_range.0));
        ui.label(" to ");
        ui.add(egui::DragValue::new(&mut state.fn_grid_settings.z_range.1));
    });

    ui.horizontal(|ui| {
        ui.label("Delta X ");
        ui.add(egui::DragValue::new(&mut state.fn_grid_settings.delta.0));
        ui.label(" Y ");
        ui.add(egui::DragValue::new(&mut state.fn_grid_settings.delta.1));
        ui.label(" Z ");
        ui.add(egui::DragValue::new(&mut state.fn_grid_settings.delta.2));
    });

    ui.separator();

    ui.horizontal(|ui| {
        egui::ComboBox::from_label("SDF")
            .selected_text(state.selected_sdf_fn.to_string())
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut state.selected_sdf_fn,
                    SdfFuction::Sphere,
                    SdfFuction::Sphere.to_string(),
                );
                ui.selectable_value(
                    &mut state.selected_sdf_fn,
                    SdfFuction::Plane,
                    SdfFuction::Plane.to_string(),
                );
                ui.selectable_value(
                    &mut state.selected_sdf_fn,
                    SdfFuction::Octahedron,
                    SdfFuction::Octahedron.to_string(),
                );
                ui.selectable_value(
                    &mut state.selected_sdf_fn,
                    SdfFuction::CubeRingFrame,
                    SdfFuction::CubeRingFrame.to_string(),
                );
            });
    });

    ui.horizontal(|ui| {
        ui.label("Isovalue: ");
        ui.add(egui::DragValue::new(&mut state.isovalue));
    });

    if ui.button("Generate mesh").clicked() {
        state.construct_sdf_from_fn()
    }
}

fn draw_load_from_file_ui(ui: &mut egui::Ui, state: &mut State) {
    ui.heading("Load SDF from file");
    ui.horizontal(|ui| {
        ui.label("Grid Size (X,Y,Z)");
        ui.add(egui::DragValue::new(&mut state.selected_grid_dims.x));
        ui.add(egui::DragValue::new(&mut state.selected_grid_dims.y));
        ui.add(egui::DragValue::new(&mut state.selected_grid_dims.z));
    });

    ui.horizontal(|ui| {
        ui.label("Isovalue");
        ui.add(egui::Slider::new(&mut state.isovalue, 0.0..=1.0));
    });

    if ui.button("Select file").clicked()
        && let Some(path) = rfd::FileDialog::new().pick_file()
    {
        state.select_sdf_file_path(path);
    }

    if let Some(path) = &state.selected_sdf_file_path {
        ui.label(format!(
            "Selected file:\n{}",
            path.to_str().unwrap_or("<could not display path>")
        ));
        if ui.button("Generate Mesh").clicked()
            && let Err(err) = state.construct_sdf_from_file()
        {
            log::error!("Error loading sdf from file: {err}");
            state.generate_mesh(state.isovalue);
        }
    }
}

fn draw_spinodal_decomposition_ui(ui: &mut egui::Ui, state: &mut State) {
    ui.heading("Spinodal Decomposition Demo");

    ui.horizontal(|ui| {
        ui.label("Grid Size (X,Y,Z)");
        ui.add(egui::DragValue::new(&mut state.selected_grid_dims.x));
        ui.add(egui::DragValue::new(&mut state.selected_grid_dims.y));
        ui.add(egui::DragValue::new(&mut state.selected_grid_dims.z));
    });
    ui.horizontal(|ui| {
        ui.label("Mesh Generation Method");
        ui.radio_value(
            &mut state.mesh_generation_method,
            GenerationMethod::WithBorders,
            "With Borders",
        );
        ui.radio_value(
            &mut state.mesh_generation_method,
            GenerationMethod::Standard,
            "Standard",
        );
    });

    if ui.button("Set grid size").clicked() {
        state.update_grid_dims();
        state.randomize_grid_and_generate_mesh();
    }

    ui.horizontal_wrapped(|ui| {
        ui.label("dx");
        ui.add(egui::Slider::new(&mut state.spin_decomp.dx, 0.01..=100.0).logarithmic(true));
    });
    ui.horizontal_wrapped(|ui| {
        ui.label("dt");
        ui.add(egui::Slider::new(&mut state.spin_decomp.dt, 1e-5..=0.1).logarithmic(true));
    });
    ui.horizontal_wrapped(|ui| {
        ui.label("gamma");
        ui.add(egui::Slider::new(&mut state.spin_decomp.gamma, 0.01..=100.0).logarithmic(true));
    });

    ui.horizontal(|ui| {
        if ui.button("Randomize grid").clicked() {
            state.randomize_grid_and_generate_mesh();
        }

        let start_stop_button_label = if state.simulation_running {
            "Stop simulation"
        } else {
            "Start simulation"
        };
        if ui.button(start_stop_button_label).clicked() {
            state.simulation_running = !state.simulation_running;
        }
    });
}
