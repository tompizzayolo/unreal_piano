use super::complex::Complex64;
use super::modal::ComplexModalMode;
use super::vector::Vector3;
use std::f64::consts::PI;

pub struct AirRoomDesign {
    pub room_dimensions: Vector3,
    pub speed_of_sound: f64,
    pub air_density: f64,
    pub highest_modeled_frequency: f64,
    pub soundboard_position: Vector3,
    pub listener_position: Vector3,
}

pub struct AirRoom {
    pub modal_states: Vec<ComplexModalMode>,
    pub mode_shape_at_soundboard: Vec<f64>,
    pub mode_shape_at_listener: Vec<f64>,
    source_gain_per_volume_velocity: Vec<Complex64>,
    soundboard_pressure_coefficients: Vec<f64>,
    listener_pressure_coefficients: Vec<f64>,
}

#[inline]
fn mode_shape_at_position(
    index_x: usize,
    index_y: usize,
    index_z: usize,
    position: Vector3,
    inverse_room_x: f64,
    inverse_room_y: f64,
    inverse_room_z: f64,
) -> f64 {
    (index_x as f64 * PI * position.x * inverse_room_x).cos()
        * (index_y as f64 * PI * position.y * inverse_room_y).cos()
        * (index_z as f64 * PI * position.z * inverse_room_z).cos()
}

impl AirRoom {
    pub fn new(design: &AirRoomDesign) -> Self {
        let room = design.room_dimensions;
        let room_volume = room.x * room.y * room.z;
        let inverse_room_x = 1.0 / room.x;
        let inverse_room_y = 1.0 / room.y;
        let inverse_room_z = 1.0 / room.z;

        let maximum_wavenumber =
            2.0 * PI * design.highest_modeled_frequency / design.speed_of_sound;

        let maximum_index_x = (maximum_wavenumber * room.x / PI).ceil() as usize;
        let maximum_index_y = (maximum_wavenumber * room.y / PI).ceil() as usize;
        let maximum_index_z = (maximum_wavenumber * room.z / PI).ceil() as usize;

        let estimated_capacity =
            (maximum_index_x + 1) * (maximum_index_y + 1) * (maximum_index_z + 1);

        let mut modal_states = Vec::with_capacity(estimated_capacity);
        let mut mode_shape_at_soundboard = Vec::with_capacity(estimated_capacity);
        let mut mode_shape_at_listener = Vec::with_capacity(estimated_capacity);
        let mut source_gain_per_volume_velocity = Vec::with_capacity(estimated_capacity);
        let mut soundboard_pressure_coefficients = Vec::with_capacity(estimated_capacity);
        let mut listener_pressure_coefficients = Vec::with_capacity(estimated_capacity);

        let density_times_sound_speed_squared =
            design.air_density * design.speed_of_sound * design.speed_of_sound;

        for index_x in 0..=maximum_index_x {
            for index_y in 0..=maximum_index_y {
                for index_z in 0..=maximum_index_z {
                    if index_x + index_y + index_z == 0 {
                        continue;
                    }

                    let wavenumber_x = index_x as f64 * PI * inverse_room_x;
                    let wavenumber_y = index_y as f64 * PI * inverse_room_y;
                    let wavenumber_z = index_z as f64 * PI * inverse_room_z;

                    let wavenumber_squared = wavenumber_x * wavenumber_x
                        + wavenumber_y * wavenumber_y
                        + wavenumber_z * wavenumber_z;

                    let wavenumber = wavenumber_squared.sqrt();

                    if wavenumber > maximum_wavenumber {
                        continue;
                    }

                    let angular_frequency = design.speed_of_sound * wavenumber;

                    let nonzero_index_count =
                        (index_x > 0) as usize + (index_y > 0) as usize + (index_z > 0) as usize;

                    let modal_volume = room_volume / (1usize << nonzero_index_count) as f64;

                    let frequency = angular_frequency / (2.0 * PI);
                    let reverberation_time = 0.30 + 0.55 * (-frequency / 160.0).exp();

                    let damping_ratio =
                        (6.9 / (angular_frequency * reverberation_time)).clamp(1.0e-4, 0.5);

                    let shape_at_soundboard = mode_shape_at_position(
                        index_x,
                        index_y,
                        index_z,
                        design.soundboard_position,
                        inverse_room_x,
                        inverse_room_y,
                        inverse_room_z,
                    );

                    let shape_at_listener = mode_shape_at_position(
                        index_x,
                        index_y,
                        index_z,
                        design.listener_position,
                        inverse_room_x,
                        inverse_room_y,
                        inverse_room_z,
                    );

                    modal_states.push(ComplexModalMode::new(angular_frequency, damping_ratio));

                    mode_shape_at_soundboard.push(shape_at_soundboard);
                    mode_shape_at_listener.push(shape_at_listener);

                    soundboard_pressure_coefficients.push(2.0 * shape_at_soundboard);
                    listener_pressure_coefficients.push(2.0 * shape_at_listener);

                    let damped_frequency =
                        angular_frequency * (1.0 - damping_ratio * damping_ratio).sqrt();

                    let complex_eigenvalue =
                        Complex64::new(-damping_ratio * angular_frequency, damped_frequency);

                    let projection_denominator = Complex64::new(2.0, 0.0)
                        + complex_eigenvalue * (2.0 * damping_ratio / angular_frequency);

                    let pressure_source_scale =
                        density_times_sound_speed_squared / modal_volume * shape_at_soundboard;

                    source_gain_per_volume_velocity.push(
                        projection_denominator
                            .inverse()
                            .scaled(pressure_source_scale),
                    );
                }
            }
        }

        AirRoom {
            modal_states,
            mode_shape_at_soundboard,
            mode_shape_at_listener,
            source_gain_per_volume_velocity,
            soundboard_pressure_coefficients,
            listener_pressure_coefficients,
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
    pub fn pressure_at_listener(&self) -> f64 {
        let mut pressure = 0.0;

        for (mode, coefficient) in self
            .modal_states
            .iter()
            .zip(self.listener_pressure_coefficients.iter())
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
