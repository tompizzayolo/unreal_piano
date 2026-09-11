use crate::engine::{
    math::{complex::Complex64, vector::Vector3},
    osc::modal::ComplexModalMode,
};
use std::f64::consts::PI;

pub struct AirRoomDesign {
    pub room_dimensions: Vector3,
    pub speed_of_sound: f64,
    pub air_density: f64,
    pub highest_modeled_frequency: f64,
    pub soundboard_position: Vector3,
    pub listener_left_position: Vector3,
    pub listener_right_position: Vector3,
}

pub struct AirRoom {
    pub modal_states: Vec<ComplexModalMode>,
    pub mode_shape_at_soundboard: Vec<f64>,
    pub mode_shape_at_listener_left: Vec<f64>,
    pub mode_shape_at_listener_right: Vec<f64>,
    source_gain_per_volume_velocity: Vec<Complex64>,
    soundboard_pressure_coefficients: Vec<f64>,
    listener_pressure_coefficients_left: Vec<f64>,
    listener_pressure_coefficients_right: Vec<f64>,
}

#[inline]
fn simplified_mode_shape(index: usize, normalized_pos: f64) -> f64 {
    (index as f64 * PI * normalized_pos).cos()
}

impl AirRoom {
    pub fn new(design: &AirRoomDesign) -> Self {
        let num_modes = 64;

        let mut modal_states = Vec::with_capacity(num_modes);
        let mut mode_shape_at_soundboard = Vec::with_capacity(num_modes);
        let mut mode_shape_at_listener_left = Vec::with_capacity(num_modes);
        let mut mode_shape_at_listener_right = Vec::with_capacity(num_modes);
        let mut source_gain_per_volume_velocity = Vec::with_capacity(num_modes);
        let mut soundboard_pressure_coefficients = Vec::with_capacity(num_modes);
        let mut listener_pressure_coefficients_left = Vec::with_capacity(num_modes);
        let mut listener_pressure_coefficients_right = Vec::with_capacity(num_modes);

        let norm_pos_sb = design.soundboard_position.x / design.room_dimensions.x;
        let norm_pos_lis_left = design.listener_left_position.x / design.room_dimensions.x;
        let norm_pos_lis_right = design.listener_right_position.x / design.room_dimensions.x;

        let base_frequency = design.speed_of_sound / (2.0 * design.room_dimensions.x);
        let density_c2 = design.air_density * design.speed_of_sound * design.speed_of_sound;

        for i in 1..=num_modes {
            let inharmonicity = 0.0005;
            let freq = base_frequency * i as f64 * (1.0 + inharmonicity * i as f64 * i as f64);
            let angular_frequency = 2.0 * PI * freq;
            let damping_ratio = (0.01 + 0.0005 * i as f64).clamp(1.0e-4, 0.5);

            let shape_sb = simplified_mode_shape(i, norm_pos_sb);
            let shape_lis_left = simplified_mode_shape(i, norm_pos_lis_left);
            let shape_lis_right = simplified_mode_shape(i, norm_pos_lis_right);

            modal_states.push(ComplexModalMode::new(angular_frequency, damping_ratio));

            mode_shape_at_soundboard.push(shape_sb);
            mode_shape_at_listener_left.push(shape_lis_left);
            mode_shape_at_listener_right.push(shape_lis_right);

            soundboard_pressure_coefficients.push(2.0 * shape_sb);
            listener_pressure_coefficients_left.push(2.0 * shape_lis_left);
            listener_pressure_coefficients_right.push(2.0 * shape_lis_right);

            let damped_frequency = angular_frequency * (1.0 - damping_ratio * damping_ratio).sqrt();
            let complex_eigenvalue =
                Complex64::new(-damping_ratio * angular_frequency, damped_frequency);

            let projection_denominator = Complex64::new(2.0, 0.0)
                + complex_eigenvalue * (2.0 * damping_ratio / angular_frequency);

            let pressure_source_scale = density_c2 / design.room_dimensions.x * shape_sb;

            source_gain_per_volume_velocity.push(
                projection_denominator
                    .inverse()
                    .scaled(pressure_source_scale),
            );
        }

        AirRoom {
            modal_states,
            mode_shape_at_soundboard,
            mode_shape_at_listener_left,
            mode_shape_at_listener_right,
            source_gain_per_volume_velocity,
            soundboard_pressure_coefficients,
            listener_pressure_coefficients_left,
            listener_pressure_coefficients_right,
        }
    }

    #[inline]
    pub fn mode_count(&self) -> usize {
        self.modal_states.len()
    }

    #[inline]
    pub fn advance(&mut self, soundboard_volume_velocity: f64, step_size: f64) {
        for (mode, source_gain) in self
            .modal_states
            .iter_mut()
            .zip(self.source_gain_per_volume_velocity.iter())
        {
            mode.advance(source_gain.scaled(soundboard_volume_velocity), step_size);
        }
    }

    #[inline]
    pub fn pressure_at_soundboard(&self) -> f64 {
        let mut pressure = 0.0;
        for (mode, coefficient) in self
            .modal_states
            .iter()
            .zip(self.soundboard_pressure_coefficients.iter())
        {
            pressure += coefficient * mode.state.real_part;
        }
        pressure
    }

    #[inline]
    pub fn pressure_at_listener_left(&self) -> f64 {
        let mut pressure = 0.0;
        for (mode, coefficient) in self
            .modal_states
            .iter()
            .zip(self.listener_pressure_coefficients_left.iter())
        {
            pressure += coefficient * mode.state.real_part;
        }
        pressure
    }

    #[inline]
    pub fn pressure_at_listener_right(&self) -> f64 {
        let mut pressure = 0.0;
        for (mode, coefficient) in self
            .modal_states
            .iter()
            .zip(self.listener_pressure_coefficients_right.iter())
        {
            pressure += coefficient * mode.state.real_part;
        }
        pressure
    }

    pub fn reset(&mut self) {
        for mode in &mut self.modal_states {
            mode.state = Complex64::ZERO;
        }
    }
}
