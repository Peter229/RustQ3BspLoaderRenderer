use cgmath::*;
use winit::event::*;
use winit::dpi::PhysicalPosition;
use std::time::Duration;
use std::f32::consts::FRAC_PI_2;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

#[derive(Debug)]
pub struct Camera {
    pub position: Point3<f32>,
    yaw: Rad<f32>,
    pitch: Rad<f32>,
    pub view: Matrix4<f32>,
}

impl Camera {
    
    pub fn new() -> Self {
        Self {
            position: Point3::new(0.0, 0.0, 0.0),
            yaw: Rad(0.0),
            pitch: Rad(0.0),
            view: Matrix4::look_at_dir(
                Point3::new(0.0, 0.0, 0.0),
                Vector3::new(
                    Rad(0.0).cos() * Rad(0.0).sin(),
                    Rad(0.0).sin(),
                    Rad(0.0).sin() * Rad(0.0).cos(),
                ).normalize(),
                Vector3::unit_y(),
            ),
        }
    }

    /*pub fn calc_matrix(&self) -> Matrix4<f32> {

        Matrix4::look_at_dir(
            self.position,
            Vector3::new(
                self.yaw.0.cos() * self.pitch.0.sin(),
                self.pitch.0.sin(),
                self.yaw.0.sin() * self.pitch.0.cos(),
            ).normalize(),
            Vector3::unit_y(),
        )
    }*/
}

pub struct Projection {
    aspect: f32,
    fovy: Rad<f32>,
    znear: f32,
    zfar: f32,
}

impl Projection {
    
    pub fn new<F: Into<Rad<f32>>>(
        width: u32,
        height: u32,
        fovy: F,
        znear: f32,
        zfar: f32,
    ) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy: fovy.into(),
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

#[derive(Debug)]
pub struct CameraController {
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    scroll: f32,
    speed: f32,
    sensitivity: f32,
}

impl CameraController {

    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            scroll: 0.0,
            speed,
            sensitivity,
        }
    }

    pub fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        
        let amount = if state == ElementState::Pressed { 1.0 } else { 0.0 };
        match key {
            VirtualKeyCode::W => {
                self.amount_forward = amount;
                true
            }
            VirtualKeyCode::S => {
                self.amount_backward = amount;
                true
            }
            VirtualKeyCode::A => {
                self.amount_left = amount;
                true
            }
            VirtualKeyCode::D => {
                self.amount_right = amount;
                true
            }
            VirtualKeyCode::Space => {
                self.amount_up = amount;
                true
            }
            VirtualKeyCode::LShift => {
                self.amount_down = amount;
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, mouse_dx: f32, mouse_dy: f32, camera: &mut Camera) {

        let mut x_offset = mouse_dx * self.sensitivity;
        let mut y_offset = mouse_dy * self.sensitivity;
        self.rotate_horizontal -= x_offset;
        self.rotate_vertical += y_offset;
        self.rotate_horizontal %= 360.0_f32;

        let max_look_up: f32 = 89.0_f32.to_radians();
        if self.rotate_vertical > max_look_up {
            self.rotate_vertical = max_look_up;
        }else if self.rotate_vertical < -max_look_up {
            self.rotate_vertical = -max_look_up;
        }

        camera.yaw = Rad(self.rotate_horizontal);
        camera.pitch = Rad(self.rotate_vertical);
    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        /*self.scroll = match delta {
            MouseScrollDelta::LineDelta(_, scroll) => -scroll * 100.0,
            MouseScrollDelta::PixelDelta(PhysicalPosition {
                y: scroll,
                ..
            }) => -*scroll as f32,
        };*/
    }

    pub fn update_camera(&mut self, camera: &mut Camera) {

        //let (yaw_sin, yaw_cos) = camera.yaw.0.sin_cos();
        //let (pitch_sin, pitch_cos) = camera.pitch.0.sin_cos();
        let forward = Vector3::new(self.rotate_horizontal.cos() * self.rotate_vertical.cos(), self.rotate_horizontal.sin() * self.rotate_vertical.cos(), self.rotate_vertical.sin()).normalize();
        let right = forward.cross(Vector3::unit_z()).normalize();
        let up = right.cross(forward);
        camera.position += forward * (self.amount_forward - self.amount_backward) * self.speed;
        camera.position += right * (self.amount_right - self.amount_left) * self.speed;
        camera.view = Matrix4::look_at_dir(camera.position, forward, up);
        //camera.position.y += (self.amount_up - self.amount_down) * self.speed;
    }
}