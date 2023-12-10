use log::{info, error};
use std::time::Instant;

use vulkano_util::{
    context::{VulkanoConfig, VulkanoContext},
    window::VulkanoWindows,
};

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};
use vulkano::{sync::GpuFuture, command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents}, pipeline::PipelineBindPoint};
use crate::framework::graphics::GraphicsContext;

pub struct Application {}

impl Application {
    pub fn new() -> anyhow::Result<Application> {
        Ok(Application {})
    }

    pub fn run(&self) -> anyhow::Result<()> {
        info!("Running Application");

        let context = VulkanoContext::new(VulkanoConfig::default());

        info!(
            "Using device: {} (type: {:?})",
            context.device_name(),
            context.device_type()
        );

        let event_loop = EventLoop::new();
        let mut vulkano_windows = VulkanoWindows::default();
        let _id1 =
            vulkano_windows.create_window(&event_loop, &context, &Default::default(), |_| {});

        let graphics_context =
            GraphicsContext::new(&context, &vulkano_windows.get_primary_renderer().unwrap())?;

        let mut previous_instant = Instant::now();
        let max_frame_time: f64 = 0.1;
        let mut accumulated_time: f64 = 0.0;
        let fixed_time_step: f64 = 1.0 / 240.0;

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => *control_flow = ControlFlow::Exit,
                Event::RedrawRequested(_) => {
                    let current_instant = Instant::now();
                    let mut elapsed = current_instant
                        .duration_since(previous_instant)
                        .as_secs_f64();
                    if elapsed > max_frame_time {
                        elapsed = max_frame_time;
                    }
                    accumulated_time += elapsed;

                    while accumulated_time >= fixed_time_step {
                        Self::update();
                        accumulated_time -= fixed_time_step;
                    }

                    let blending_factor = accumulated_time / fixed_time_step;
                    let renderer = vulkano_windows.get_primary_renderer_mut().unwrap();

                    let future = match renderer.acquire() {
                        Err(e) => {
                            error!("{e}");
                            return;
                        }
                        Ok(future) => future,
                    };

                    let mut builder = AutoCommandBufferBuilder::primary(
                        &command_buffer_allocator,
                        queue.queue_family_index(),
                        CommandBufferUsage::OneTimeSubmit,
                    )
                    .unwrap();
                    builder
                        .begin_render_pass(
                            RenderPassBeginInfo {
                                clear_values: vec![
                                    Some([0.0, 0.0, 1.0, 1.0].into()),
                                    Some(1f32.into()),
                                ],
                                ..RenderPassBeginInfo::framebuffer(
                                    framebuffers[image_index as usize].clone(),
                                )
                            },
                            SubpassContents::Inline,
                        )
                        .unwrap()
                        .bind_pipeline_graphics(pipeline.clone())
                        .bind_descriptor_sets(
                            PipelineBindPoint::Graphics,
                            pipeline.layout().clone(),
                            0,
                            set,
                        )
                        .bind_vertex_buffers(0, (vertex_buffer.clone(), normals_buffer.clone()))
                        .bind_index_buffer(index_buffer.clone())
                        .draw_indexed(index_buffer.len() as u32, 1, 0, 0, 0)
                        .unwrap()
                        .end_render_pass()
                        .unwrap();
                    let command_buffer = builder.build().unwrap();

                    future.then_execute(renderer.graphics_queue(), command_buffer);

                    Self::render(blending_factor, &graphics_context);

                    renderer.present(future, false);

                    previous_instant = current_instant;
                }
                Event::MainEventsCleared => {
                    vulkano_windows
                        .get_primary_window()
                        .unwrap()
                        .request_redraw();
                }
                _ => {}
            }
        })
    }

    fn update() {
        info!("update");
    }

    fn render(blending_factor: f64, graphics_context: &GraphicsContext) {

        

        info!("render: {}", blending_factor);
    }
}
