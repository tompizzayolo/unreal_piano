//! Modal superposition machinery (paper §7.1) and the exact exponential
//! time steppers of §7.2.
//!
//! A subsystem with a second-order ODE (string, soundboard, room material,
//! paper eq. (2.19)) is reduced to decoupled modal oscillators
//!
//!     q_j'' + 2 zeta_j omega_j q_j' + omega_j^2 q_j = f_j / m_j     (7.1.1)
//!
//! A subsystem with a first-order ODE (the air, paper eq. (5.14)) is reduced
//! to decoupled complex modal equations
//!
//!     q_j' = lambda_j q_j + g_j,   lambda_j = -zeta_j omega_j + i omega_d (7.1.2)
//!
//! Both are advanced with their exact linear propagators; coupling forces
//! enter as piecewise-constant source terms evaluated at the (predicted)
//! end of the step, exactly the strategy described in §7.2 ("the rhs source
//! terms of the next time step are used for approximate integration
//! whenever possible").

use super::complex::Complex64;

/// One real second-order modal oscillator.
pub struct ModalOscillator {
    /// Modal angular frequency omega_j [rad/s].
    pub angular_frequency: f64,
    /// Modal damping ratio zeta_j = c_j / (2 m_j omega_j).
    pub damping_ratio: f64,
    /// Modal mass m_j [kg] (m_j = phi_j^T M phi_j, mass-normalized shape).
    pub modal_mass: f64,
    /// Modal coordinate q_j.
    pub displacement: f64,
    /// Time derivative of the modal coordinate.
    pub velocity: f64,

    // Cached stepping coefficients (recomputed only when dt changes).
    cached_step_size: f64,
    /// A(dt): position -> position propagation (see module derivation).
    coefficient_position_propagation: f64,
    /// B(dt): velocity -> position propagation.
    coefficient_velocity_to_position: f64,
    /// B'(dt): velocity -> velocity propagation.
    coefficient_velocity_propagation: f64,
}

impl ModalOscillator {
    pub fn new(angular_frequency: f64, damping_ratio: f64, modal_mass: f64) -> Self {
        ModalOscillator {
            angular_frequency: angular_frequency.max(1.0e-6),
            damping_ratio: damping_ratio.clamp(1.0e-6, 0.95),
            modal_mass: modal_mass.max(1.0e-12),
            displacement: 0.0,
            velocity: 0.0,
            cached_step_size: -1.0,
            coefficient_position_propagation: 0.0,
            coefficient_velocity_to_position: 0.0,
            coefficient_velocity_propagation: 0.0,
        }
    }

    fn refresh_step_coefficients(&mut self, step_size: f64) {
        if self.cached_step_size != step_size {
            let zeta = self.damping_ratio;
            let omega = self.angular_frequency;
            let damped_frequency = omega * (1.0 - zeta * zeta).sqrt();
            let decay_envelope = (-zeta * omega * step_size).exp();
            let (sine_part, cosine_part) = (damped_frequency * step_size).sin_cos();
            let damping_ratio_scaled = zeta * omega / damped_frequency;

            self.coefficient_position_propagation =
                decay_envelope * (cosine_part + damping_ratio_scaled * sine_part);
            self.coefficient_velocity_to_position = decay_envelope * sine_part / damped_frequency;
            self.coefficient_velocity_propagation =
                decay_envelope * (cosine_part - damping_ratio_scaled * sine_part);
            self.cached_step_size = step_size;
        }
    }

    /// Advance one step of length `step_size` under the external modal force
    /// `external_modal_force` (held constant over the step).
    ///
    /// Exact update of the damped oscillator (paper §7.2.1):
    ///     q⁺  = A q + B q' + (1 - A) g / ω²
    ///     q'⁺ = -ω² B q + B' q' + B g          with g = f / m
    pub fn advance(&mut self, external_modal_force: f64, step_size: f64) {
        self.refresh_step_coefficients(step_size);
        let modal_source_acceleration = external_modal_force / self.modal_mass;
        let omega_squared = self.angular_frequency * self.angular_frequency;
        let current_displacement = self.displacement;
        let current_velocity = self.velocity;

        self.displacement = self.coefficient_position_propagation * current_displacement
            + self.coefficient_velocity_to_position * current_velocity
            + (1.0 - self.coefficient_position_propagation) * modal_source_acceleration
                / omega_squared;
        self.velocity =
            -omega_squared * self.coefficient_velocity_to_position * current_displacement
                + self.coefficient_velocity_propagation * current_velocity
                + self.coefficient_velocity_to_position * modal_source_acceleration;
    }

    /// Modal acceleration for the given force (used by the coupling terms).
    pub fn acceleration_for_force(&self, external_modal_force: f64) -> f64 {
        external_modal_force / self.modal_mass
            - 2.0 * self.damping_ratio * self.angular_frequency * self.velocity
            - self.angular_frequency * self.angular_frequency * self.displacement
    }
}

/// One complex modal degree of freedom of a first-order system (the air).
pub struct ComplexModalMode {
    /// Complex eigenvalue lambda_j = -zeta_j omega_j + i omega_d,j.
    pub eigenvalue: Complex64,
    /// Complex modal state q_j (its real part is proportional to pressure).
    pub state: Complex64,

    cached_step_size: f64,
    /// e^(lambda * dt) — free propagation over one step.
    propagation_factor: Complex64,
    /// (e^(lambda * dt) - 1) / lambda — step response to a constant source.
    step_response_integral: Complex64,
}

impl ComplexModalMode {
    pub fn new(angular_frequency: f64, damping_ratio: f64) -> Self {
        let clamped_damping_ratio = damping_ratio.clamp(1.0e-6, 0.99);
        let damped_frequency =
            angular_frequency * (1.0 - clamped_damping_ratio * clamped_damping_ratio).sqrt();
        ComplexModalMode {
            eigenvalue: Complex64::new(
                -clamped_damping_ratio * angular_frequency,
                damped_frequency,
            ),
            state: Complex64::ZERO,
            cached_step_size: -1.0,
            propagation_factor: Complex64::ZERO,
            step_response_integral: Complex64::ZERO,
        }
    }

    fn refresh_step_coefficients(&mut self, step_size: f64) {
        if self.cached_step_size != step_size {
            self.propagation_factor = (self.eigenvalue * step_size).exponential();
            self.step_response_integral =
                (self.propagation_factor - Complex64::ONE) * self.eigenvalue.inverse();
            self.cached_step_size = step_size;
        }
    }

    /// Exact update for a piecewise-constant source (paper §7.2.2):
    ///     q⁺ = e^(lambda dt) q + (e^(lambda dt) - 1)/lambda * g
    pub fn advance(&mut self, source_term: Complex64, step_size: f64) {
        self.refresh_step_coefficients(step_size);
        self.state =
            self.propagation_factor * self.state + self.step_response_integral * source_term;
    }
}
