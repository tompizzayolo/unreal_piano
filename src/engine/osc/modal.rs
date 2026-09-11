use crate::engine::math::complex::Complex64;

pub struct ModalOscillator {
    pub angular_frequency: f64,
    pub damping_ratio: f64,
    pub modal_mass: f64,
    pub displacement: f64,
    pub velocity: f64,
    inverse_modal_mass: f64,
    omega_squared: f64,
    velocity_decay_coefficient: f64,
    cached_step_size: f64,
    coefficient_position_propagation: f64,
    coefficient_velocity_to_position: f64,
    coefficient_velocity_propagation: f64,
    coefficient_force_to_displacement: f64,
    coefficient_velocity_position_coupling: f64,
}

impl ModalOscillator {
    #[inline]
    pub fn new(angular_frequency: f64, damping_ratio: f64, modal_mass: f64) -> Self {
        let angular_frequency = angular_frequency.max(1.0e-6);
        let damping_ratio = damping_ratio.clamp(1.0e-6, 0.95);
        let modal_mass = modal_mass.max(1.0e-12);

        ModalOscillator {
            angular_frequency,
            damping_ratio,
            modal_mass,
            displacement: 0.0,
            velocity: 0.0,
            inverse_modal_mass: 1.0 / modal_mass,
            omega_squared: angular_frequency * angular_frequency,
            velocity_decay_coefficient: 2.0 * damping_ratio * angular_frequency,
            cached_step_size: -1.0,
            coefficient_position_propagation: 0.0,
            coefficient_velocity_to_position: 0.0,
            coefficient_velocity_propagation: 0.0,
            coefficient_force_to_displacement: 0.0,
            coefficient_velocity_position_coupling: 0.0,
        }
    }

    fn refresh_step_coefficients(&mut self, step_size: f64) {
        if self.cached_step_size == step_size {
            return;
        }

        let zeta = self.damping_ratio;
        let omega = self.angular_frequency;
        let damped_frequency = omega * (1.0 - zeta * zeta).sqrt();
        let decay_envelope = (-zeta * omega * step_size).exp();
        let (sine_part, cosine_part) = (damped_frequency * step_size).sin_cos();
        let damping_ratio_scaled = zeta * omega / damped_frequency;

        let position_propagation =
            decay_envelope * (cosine_part + damping_ratio_scaled * sine_part);
        let velocity_to_position = decay_envelope * sine_part / damped_frequency;
        let velocity_propagation =
            decay_envelope * (cosine_part - damping_ratio_scaled * sine_part);

        self.coefficient_position_propagation = position_propagation;
        self.coefficient_velocity_to_position = velocity_to_position;
        self.coefficient_velocity_propagation = velocity_propagation;
        self.coefficient_force_to_displacement = (1.0 - position_propagation) / self.omega_squared;
        self.coefficient_velocity_position_coupling = -self.omega_squared * velocity_to_position;
        self.cached_step_size = step_size;
    }

    #[inline]
    pub fn advance(&mut self, external_modal_force: f64, step_size: f64) {
        self.refresh_step_coefficients(step_size);

        let modal_source_acceleration = external_modal_force * self.inverse_modal_mass;
        let current_displacement = self.displacement;
        let current_velocity = self.velocity;

        self.displacement = self.coefficient_position_propagation * current_displacement
            + self.coefficient_velocity_to_position * current_velocity
            + self.coefficient_force_to_displacement * modal_source_acceleration;

        self.velocity = self.coefficient_velocity_position_coupling * current_displacement
            + self.coefficient_velocity_propagation * current_velocity
            + self.coefficient_velocity_to_position * modal_source_acceleration;
    }

    #[inline]
    pub fn acceleration_for_force(&self, external_modal_force: f64) -> f64 {
        external_modal_force * self.inverse_modal_mass
            - self.velocity_decay_coefficient * self.velocity
            - self.omega_squared * self.displacement
    }
}

pub struct ComplexModalMode {
    pub eigenvalue: Complex64,
    pub state: Complex64,
    inverse_eigenvalue: Complex64,
    cached_step_size: f64,
    propagation_factor: Complex64,
    step_response_integral: Complex64,
}

impl ComplexModalMode {
    #[inline]
    pub fn new(angular_frequency: f64, damping_ratio: f64) -> Self {
        let angular_frequency = angular_frequency.max(1.0e-6);
        let damping_ratio = damping_ratio.clamp(1.0e-6, 0.99);
        let damped_frequency = angular_frequency * (1.0 - damping_ratio * damping_ratio).sqrt();

        let eigenvalue = Complex64::new(-damping_ratio * angular_frequency, damped_frequency);

        ComplexModalMode {
            eigenvalue,
            state: Complex64::ZERO,
            inverse_eigenvalue: eigenvalue.inverse(),
            cached_step_size: -1.0,
            propagation_factor: Complex64::ZERO,
            step_response_integral: Complex64::ZERO,
        }
    }

    fn refresh_step_coefficients(&mut self, step_size: f64) {
        if self.cached_step_size == step_size {
            return;
        }

        self.propagation_factor = (self.eigenvalue * step_size).exponential();
        self.step_response_integral =
            (self.propagation_factor - Complex64::ONE) * self.inverse_eigenvalue;
        self.cached_step_size = step_size;
    }

    #[inline]
    pub fn advance(&mut self, source_term: Complex64, step_size: f64) {
        self.refresh_step_coefficients(step_size);
        self.state =
            self.propagation_factor * self.state + self.step_response_integral * source_term;
    }
}
