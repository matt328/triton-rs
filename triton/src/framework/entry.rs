use anyhow::Context;
use game_loop::game_loop;
use log::info;
use std::sync::Arc;
use vulkano::{
    instance::{Instance, InstanceCreateInfo},
    swapchain::Surface,
    VulkanLibrary,
};
use vulkano_win::create_surface_from_winit;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

use super::graphics::GraphicsContext;

pub struct Game {
    graphics_context: GraphicsContext,
}

impl Game {
    pub fn new(instance: Arc<Instance>, surface: Arc<Surface>) -> anyhow::Result<Game> {
        Ok(Game {
            graphics_context: GraphicsContext::new(instance.clone(), surface.clone())
                .context("Game creating graphics context")?,
        })
    }

    pub fn update(&self) {
        info!("update");
    }

    pub fn render(&self, blending_factor: f64) {
        info!("Render: {}", blending_factor);
    }

    pub fn window_handler(&self, event: &Event<()>) -> bool {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                return false;
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                // self.recreate_swapchain = true;
                return true;
            }
            _ => true,
        }
    }
}

pub struct Application {}

impl Application {
    pub fn new() -> anyhow::Result<Application> {
        Ok(Application {})
    }

    pub fn run(&self) -> anyhow::Result<()> {
        info!("Running Application");

        let library = VulkanLibrary::new().context("No Vulkan lib or dll was found")?;

        let required_extensions = vulkano_win::required_extensions(&library);

        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                enabled_extensions: required_extensions,
                enumerate_portability: true,
                ..Default::default()
            },
        )
        .context("Failed to create Instance")?;

        let event_loop = EventLoop::new();

        let window = Arc::new(
            WindowBuilder::new()
                .with_title("Triton Window")
                .build(&event_loop)
                .context("Error creating window")?,
        );

        let surface = create_surface_from_winit(window.clone(), instance.clone())
            .context("Error creating surface")?;

        let game = Arc::new(
            Game::new(instance.clone(), surface.clone()).context("Failed to create Game")?,
        );

        game_loop(
            event_loop,
            window.clone(),
            game,
            240,
            0.1,
            |g| {
                g.game.update();
            },
            |g| {
                g.game.render(g.blending_factor());
            },
            |g, event| {
                if !g.game.window_handler(event) {
                    g.exit();
                }
            },
        )
    }
}
