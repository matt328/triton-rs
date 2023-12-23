use std::{collections::HashMap, sync::Arc};

use gilrs::Axis;
use specs::{Builder, Dispatcher, DispatcherBuilder, World, WorldExt};
use tracing::{span, Level};
use vulkano::instance::InstanceExtensions;
use winit::{dpi::PhysicalSize, event::Event, keyboard::KeyCode, window::Window};

use crate::graphics::{Renderer, CUBE_INDICES, CUBE_VERTICES};

use super::{
    components::{
        render::{RenderSystem, Renderable},
        transform::{Transform, TransformSystem},
        ActiveCamera, BlendFactor, Camera, CameraSystem, ResizeEvents,
    },
    input::{
        ActionDescriptor, ActionKind, ActionMap, ActionState, GamepadSource, InputSystem,
        MouseAxis, MouseSource, Source, SystemEvent,
    },
};

#[derive(Default)]
pub struct InputStateResource(pub HashMap<String, ActionState>);

pub struct Context<'a, 'b> {
    input_system: InputSystem,
    world: World,
    fixed_update_dispatcher: Dispatcher<'a, 'b>,
    render_dispatcher: Dispatcher<'a, 'b>,
}

impl<'a, 'b> Context<'a, 'b> {
    pub fn new(
        required_extensions: InstanceExtensions,
        window: Arc<Window>,
    ) -> anyhow::Result<Self> {
        let extent: [f32; 2] = window.inner_size().into();

        let mut renderer = Renderer::new(required_extensions, window.clone())?;

        let mut world = World::new();

        world.insert(ResizeEvents(Vec::new()));
        world.insert(InputStateResource(HashMap::new()));

        let mesh_id = renderer.create_mesh(CUBE_VERTICES.into(), CUBE_INDICES.into())?;

        let mut fixed_update_dispatcher = DispatcherBuilder::new()
            .with(TransformSystem, "transform_system", &[])
            .with(CameraSystem, "camera_system", &[])
            .build();

        let mut render_dispatcher = DispatcherBuilder::new()
            .with_thread_local(RenderSystem::new(renderer))
            .build();

        fixed_update_dispatcher.setup(&mut world);
        render_dispatcher.setup(&mut world);

        world
            .create_entity()
            .with(Transform {
                position: [0.0, 0.0, 0.0].into(),
                rotation: [1.0, 0.0, 0.0, 0.0].into(),
                scale: [1.0, 1.0, 1.0].into(),
            })
            .with(Renderable { mesh_id })
            .build();

        world
            .create_entity()
            .with(Transform {
                position: [5.0, 0.0, 0.0].into(),
                rotation: [1.0, 0.0, 0.0, 0.0].into(),
                scale: [1.0, 1.0, 1.0].into(),
            })
            .with(Renderable { mesh_id })
            .build();

        let cam = world
            .create_entity()
            .with(Camera {
                aspect_ratio: extent[0] / extent[1],
                ..Default::default()
            })
            .build();

        world.insert(ActiveCamera(cam));

        world
            .write_resource::<ResizeEvents>()
            .0
            .push(window.inner_size());

        let walk_forward_action = "walk_forward";
        let walk_backward_action = "walk_backward";
        let strafe_right_action = "strafe_right";
        let strafe_left_action = "strafe_left";
        let move_up_action = "move_up";
        let move_down_action = "move_down";
        let look_vertical_action = "look_vertical_action";
        let look_horizontal_action = "look_horizontal_action";

        let input_system = InputSystem::new()
            .add_action(
                walk_forward_action,
                ActionDescriptor {
                    kind: ActionKind::Button,
                },
            )
            .add_action(
                walk_backward_action,
                ActionDescriptor {
                    kind: ActionKind::Button,
                },
            )
            .add_action(
                strafe_right_action,
                ActionDescriptor {
                    kind: ActionKind::Button,
                },
            )
            .add_action(
                strafe_left_action,
                ActionDescriptor {
                    kind: ActionKind::Button,
                },
            )
            .add_action(
                look_vertical_action,
                ActionDescriptor {
                    kind: ActionKind::Axis,
                },
            )
            .add_action(
                look_horizontal_action,
                ActionDescriptor {
                    kind: ActionKind::Axis,
                },
            )
            .add_action(
                move_up_action,
                ActionDescriptor {
                    kind: ActionKind::Button,
                },
            )
            .add_action(
                move_down_action,
                ActionDescriptor {
                    kind: ActionKind::Button,
                },
            )
            .add_action_map(
                "main",
                ActionMap::new()
                    .bind(Source::Keyboard(KeyCode::KeyW), walk_forward_action)
                    .bind(Source::Keyboard(KeyCode::ArrowUp), walk_forward_action)
                    .bind(
                        Source::Gamepad(GamepadSource::Axis(Axis::LeftStickY)),
                        walk_forward_action,
                    )
                    .bind(Source::Keyboard(KeyCode::KeyS), walk_backward_action)
                    .bind(Source::Keyboard(KeyCode::ArrowDown), walk_backward_action)
                    .bind(Source::Keyboard(KeyCode::KeyA), strafe_left_action)
                    .bind(Source::Keyboard(KeyCode::ArrowLeft), strafe_left_action)
                    .bind(Source::Keyboard(KeyCode::KeyD), strafe_right_action)
                    .bind(Source::Keyboard(KeyCode::ArrowRight), strafe_right_action)
                    .bind(Source::Keyboard(KeyCode::KeyQ), move_up_action)
                    .bind(Source::Keyboard(KeyCode::KeyZ), move_down_action)
                    .bind(
                        Source::Mouse(MouseSource::Move(MouseAxis::MouseY)),
                        look_vertical_action,
                    )
                    .bind(
                        Source::Mouse(MouseSource::Move(MouseAxis::MouseX)),
                        look_horizontal_action,
                    ),
            );

        Ok(Context {
            world,
            fixed_update_dispatcher,
            render_dispatcher,
            input_system,
        })
    }

    pub fn process_winit_event(&mut self, event: &Event<()>, mouse_captured: bool) -> bool {
        self.input_system.process_winit_event(event, mouse_captured)
    }

    pub fn pre_update(&mut self) {
        self.input_system.update_gamepads();
        self.world.insert(InputStateResource(
            self.input_system.get_action_state_map().clone(),
        ));
        // I think we should clear out the action states after we've cloned them into the ECS Resource
        self.input_system.update();
    }

    pub fn update(&mut self) {
        let _span = span!(Level::INFO, "fixed_update").entered();
        self.fixed_update_dispatcher.dispatch(&self.world);
    }

    pub fn render(&mut self, blending_factor: f32) -> anyhow::Result<()> {
        let _span = span!(Level::INFO, "render").entered();
        self.world.insert(BlendFactor(blending_factor));
        self.render_dispatcher.dispatch(&self.world);
        Ok(())
    }

    pub fn window_resized(&mut self, new_size: PhysicalSize<u32>) {
        self.world.write_resource::<ResizeEvents>().0.push(new_size);
    }
}
