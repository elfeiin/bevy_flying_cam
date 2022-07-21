#![allow(clippy::too_many_arguments)]

use bevy::{
   input::mouse::{MouseMotion, MouseWheel},
   prelude::*,
};
use leafwing_input_manager::{prelude::ActionState, Actionlike};
use std::ops::{Div, Mul, Neg};

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum FlyingCamAction {
   AdjustSpeed,
   Back,
   ClickHoldSecondary,
   Down,
   Focus,
   Forward,
   Left,
   Primary,
   Right,
   Secondary,
   Up,
}

/// Struct for customizing camera behavior.
#[derive(Component)]
pub struct MovableCameraParams {
   pub default_speed: f32,
   pub acceleration: f32,
   pub slow_speed: f32,
   pub scroll_snap: f32,
   // pub forward: KeyCode,
   // pub backward: KeyCode,
   // pub left: KeyCode,
   // pub right: KeyCode,
   // pub upward: KeyCode,
   // pub downward: KeyCode,
   // pub change_speed: KeyCode,
   // pub focus: KeyCode,
}

impl Default for MovableCameraParams {
   fn default() -> Self {
      Self {
         default_speed: 1.0,
         acceleration: 1.0,
         slow_speed: 0.1,
         scroll_snap: 1.0,
      }
   }
}

/// Tags an entity as being capable of moving, rotating, and orbiting.
#[derive(Component)]
pub struct MovableCamera {
   pub speed: f32,
   pub angular_speed: f32,
   pub slow: bool,
   pub cursor_pos: Vec2,
   pub focused: bool,
}

impl Default for MovableCamera {
   fn default() -> Self {
      Self {
         speed: MovableCameraParams::default().default_speed,
         angular_speed: MovableCameraParams::default().default_speed,
         slow: false,
         cursor_pos: Vec2::default(),
         focused: false,
      }
   }
}

/// Takes a quaternion as input and clamps it between -tau/4 and tau/4.
pub fn limit_pitch(tq: Quat) -> Quat {
   // Produce new quaternion with zeroed x and z and normalized y and w
   // from the input quaternion.
   // This results in a quaternion that represents the yaw component
   // of the original quaternion.
   // Not tested for quaternions that have roll other than 0.
   let qy = Quat::from_xyzw(0.0, tq.y, 0.0, tq.w).normalize();
   // Remove yaw from input quaternion, leaving pitch
   let qp = qy.inverse().mul(tq);
   // Convert quaternion to Euler angle
   let pitch = Vec3::X.dot(qp.xyz()).asin().mul(2.0);
   // Clamp angle to range
   let quarter_tau = std::f32::consts::TAU / 4.0;
   let clamped_pitch = pitch.clamp(quarter_tau.neg(), quarter_tau);
   // Convert angle back to quaternion
   let qp = Quat::from_rotation_x(clamped_pitch);
   // Multiply yaw quaternion by pitch quaternion to get constrained quaternion
   qy.mul(qp)
}

/// Rotates a camera quat by a linear amount.
pub fn rotate_cam_quat(window_size: Vec2, motion: Vec2, speed: f32, mut tq: Quat) -> Quat {
   let delta_x = motion
      .x
      .div(window_size.x)
      .mul(std::f32::consts::TAU)
      .mul(speed);
   let delta_y = motion
      .y
      .div(window_size.y)
      .mul(std::f32::consts::TAU.div(2.0))
      .mul(speed);
   let delta_yaw = Quat::from_rotation_y(delta_x.neg());
   let delta_pitch = Quat::from_rotation_x(delta_y.neg());
   // note the order of the following multiplications
   tq = delta_yaw.mul(tq); // yaw around GLOBAL y axis
   tq = tq.mul(delta_pitch); // pitch around LOCAL x axis
   limit_pitch(tq)
}

fn get_primary_window_size(windows: &ResMut<Windows>) -> Vec2 {
   let window = windows.get_primary().unwrap();
   Vec2::new(window.width() as f32, window.height() as f32)
}

fn net_movement(
   action_state: &ActionState<FlyingCamAction>,
   negative: FlyingCamAction,
   positive: FlyingCamAction,
) -> f32 {
   match (
      action_state.pressed(negative),
      action_state.pressed(positive),
   ) {
      (true, false) => -1.0,
      (false, true) => 1.0,
      _ => 0.0,
   }
}

/// Prevents the cursor from moving.
pub fn lock_cursor(
   mut windows: ResMut<Windows>,
   action_state: Query<&ActionState<FlyingCamAction>>,
   mut cam: Query<&mut MovableCamera>,
) {
   let action_state = action_state.single();
   let mut cam = cam.single_mut();
   if action_state.just_pressed(FlyingCamAction::Secondary) {
      if let Some(window) = windows.get_primary_mut() {
         window.set_cursor_lock_mode(true);
         if let Some(pos) = window.cursor_position() {
            cam.cursor_pos = pos;
         }
      }
   }

   if action_state.just_released(FlyingCamAction::Secondary) {
      if let Some(window) = windows.get_primary_mut() {
         window.set_cursor_lock_mode(false);
      }
   }

   if action_state.pressed(FlyingCamAction::Secondary) {
      if let Some(window) = windows.get_primary_mut() {
         window.set_cursor_position(cam.cursor_pos);
      }
   }
}

/// Adjusts the camera speed based on user input.
pub fn adjust_cam_speed(
   time: Res<Time>,
   action_state: Query<&ActionState<FlyingCamAction>>,
   cam_params: Res<MovableCameraParams>,
   mut cam: Query<&mut MovableCamera>,
) {
   let action_state = action_state.single();
   let mut cam = cam.single_mut();
   if action_state.just_pressed(FlyingCamAction::AdjustSpeed) {
      cam.slow = !cam.slow;
      if !cam.slow {
         cam.speed = cam_params.default_speed;
         cam.angular_speed = cam_params.default_speed;
      }
   }

   if cam.slow {
      cam.speed = cam_params.slow_speed;
      cam.angular_speed = cam_params.slow_speed;
   } else if action_state.pressed(FlyingCamAction::Forward)
      || action_state.pressed(FlyingCamAction::Back)
      || action_state.pressed(FlyingCamAction::Left)
      || action_state.pressed(FlyingCamAction::Right)
      || action_state.pressed(FlyingCamAction::Up)
      || action_state.pressed(FlyingCamAction::Down)
   {
      cam.speed += cam_params.acceleration.mul(time.delta_seconds());
   } else {
      cam.speed = cam_params.default_speed;
      cam.angular_speed = cam_params.default_speed;
   }
}

/// Move the camera with QWEASD, zoom with wheel, focus at
/// camera pos with F, and rotate/orbit with right mouse button.
pub fn movable_camera(
   windows: ResMut<Windows>,
   time: Res<Time>,
   action_state: Query<&ActionState<FlyingCamAction>>,
   mut motion: EventReader<MouseMotion>,
   mut scroll_evr: EventReader<MouseWheel>,
   cam_params: Res<MovableCameraParams>,
   mut q_child: Query<(
      &Parent,
      &mut Transform,
      &mut MovableCamera,
      &PerspectiveProjection,
   )>,
   mut q_parent: Query<(&mut Transform, &GlobalTransform), Without<PerspectiveProjection>>,
) {
   let action_state = action_state.single();
   for (parent, mut transform_child, mut cam, ..) in q_child.iter_mut() {
      // Focused Camera
      if cam.focused {
         if action_state.pressed(FlyingCamAction::Forward)
            || action_state.pressed(FlyingCamAction::Back)
            || action_state.pressed(FlyingCamAction::Left)
            || action_state.pressed(FlyingCamAction::Right)
            || action_state.pressed(FlyingCamAction::Up)
            || action_state.pressed(FlyingCamAction::Down)
         {
            if let Ok((mut transform_parent, ..)) = q_parent.get_mut(parent.0) {
               let zoom = transform_child.translation.z;
               // Set child transform to parent transform
               *transform_child = *transform_parent;
               // Offset child by its zoom
               transform_child.translation += zoom.mul(transform_parent.back());
               // Set parent transform to origin
               *transform_parent = Transform::default();
            }
            cam.focused = false;
         }
      } else if action_state.just_pressed(FlyingCamAction::Focus) {
         if let Ok((mut transform_parent, ..)) = q_parent.get_mut(parent.0) {
            // Hand off position and orientation information to parent
            *transform_parent = *transform_child;
         }
         *transform_child = Transform::default();
         cam.focused = true;
      }

      let mut rotation_move = Vec2::ZERO;
      let mut scroll = 0.0;

      if action_state.pressed(FlyingCamAction::Secondary) {
         for ev in motion.iter() {
            rotation_move += ev.delta;
         }
      }

      for ev in scroll_evr.iter() {
         scroll += ev.y;
      }

      if cam.focused {
         // Orbit the camera
         if rotation_move.length_squared() > 0.0 {
            if let Ok((mut transform_parent, ..)) = q_parent.get_mut(parent.0) {
               let window_size = get_primary_window_size(&windows);
               transform_parent.rotation = rotate_cam_quat(
                  window_size,
                  rotation_move,
                  cam.angular_speed,
                  transform_parent.rotation,
               );
            }
         }

         // Zoom the camera. Parent has orientation information so just
         // mutate child's z
         if scroll.abs() > 0.0 {
            transform_child.translation -= Vec3::new(0.0, 0.0, 1.0)
               .mul(cam_params.scroll_snap)
               .mul(scroll)
               .mul(cam.speed);
            // Clamp the child's translation so it can't go past focus (the parent)
            transform_child.translation = transform_child.translation.max(Vec3::new(0.0, 0.0, 0.0));
         }
      // Free Camera
      } else {
         // Rotate the camera
         if rotation_move.length_squared() > 0.0 {
            let window_size = get_primary_window_size(&windows);
            transform_child.rotation = rotate_cam_quat(
               window_size,
               rotation_move,
               cam.angular_speed,
               transform_child.rotation,
            );
         }

         // Zoom the camera relative to camera orientation
         if scroll.abs() > 0.0 {
            let transform_clone = *transform_child;
            transform_child.translation += transform_clone
               .forward()
               .mul(cam_params.scroll_snap)
               .mul(scroll)
               .mul(cam.speed);
         }

         let mut translate_move = Vec3::new(
            net_movement(action_state, FlyingCamAction::Right, FlyingCamAction::Left),
            net_movement(action_state, FlyingCamAction::Down, FlyingCamAction::Up),
            net_movement(
               action_state,
               FlyingCamAction::Back,
               FlyingCamAction::Forward,
            ),
         )
         .normalize_or_zero();

         // Translate the camera
         if translate_move.length_squared() > 0.0 {
            translate_move = translate_move.mul(time.delta_seconds()).mul(cam.speed);
            // Clone the child's transform so we can use its immutable methods
            let transform_clone = *transform_child;
            // Translate camera along each of its local axes
            transform_child.translation += transform_clone.left().mul(translate_move.x);
            transform_child.translation += transform_clone.up().mul(translate_move.y);
            transform_child.translation += transform_clone.forward().mul(translate_move.z);
         }
      }
   }
}

/// Spawn a camera like this. Note the extra bundle.
pub fn spawn_camera(mut commands: Commands) {
   let mut cam = PerspectiveCameraBundle {
      transform: Transform::from_xyz(0.0, 3.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
      ..Default::default()
   };
   cam.camera.near = -1.0;
   commands
      .spawn_bundle((
         Transform::from_xyz(0.0, 0.0, 0.0),
         GlobalTransform::default(),
      ))
      .with_children(|parent| {
         parent.spawn_bundle(cam).insert(MovableCamera::default());
      });
}
