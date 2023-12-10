use log::info;
use std::time::Instant;

use vulkano_util::{
    context::{VulkanoConfig, VulkanoContext},
    window::VulkanoWindows,
};

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

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
                        Application::update();
                        accumulated_time -= fixed_time_step;
                    }

                    let blending_factor = accumulated_time / fixed_time_step;
                    Application::render(blending_factor);

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

    fn render(blending_factor: f64) {
        info!("render: {}", blending_factor);
    }
}
