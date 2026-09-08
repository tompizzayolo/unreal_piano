//! Piano string model (paper §4): a prestressed, semi-3D cylindrical string
//! carrying longitudinal (u), horizontal (v) and vertical (w) displacement.
//!
//! The string is reduced to fixed–fixed sine modes φ_n = sin(nπx/L) over the
//! speaking length; the bridge end is *not* fixed but follows the soundboard
//! (§6.2 dynamic coupling).  Writing
//!
//!     w(x,t) = Σ_n q_n(t) sin(nπx/L) + (x/L) w_bridge(t)
//!
//! and projecting the prestressed-beam PDE (2.11) onto φ_n gives, per mode
//! (M = ρA L / 2, the modal mass):
//!
//!     M q_n'' + c_n q_n' + ω_n² M q_n =
//!         F_hammer sin(nπx_h/L) + ρA (-1)^n / k_n · ẅ_bridge
//!
//! with the dispersion relation of the prestressed string
//!
//!     ω_n² = (T/ρA) k_n² + (EI/ρA) k_n⁴,   k_n = nπ/L          (§4 + §A)
//!
//! and ω_n = k_n √(E/ρ) for the longitudinal polarization.  Modal damping
//! implements the paper's stiffness-proportional viscoelastic law C = 2μK
//! (i.e. ζ_n = μ ω_n, eq. (2.20)) plus a small frequency-independent part.
//!
//! The force the string exerts on the bridge (the reaction that drives the
//! soundboard, §6.2) is the end tension/axial force
//!
//!     F_bridge = -T ∂w/∂x|_L  (vertical/horizontal),  -EA ∂u/∂x|_L (longit.)

use super::modal::ModalOscillator;
use super::vector::Vector3;
use std::f64::consts::PI;

/// Frequency-independent + stiffness-proportional damping parts.
#[derive(Clone, Copy)]
pub struct DampingCoefficients {
    /// ζ₀: frequency-independent damping (air, terminations).
    pub constant_part: f64,
    /// μ: stiffness-proportional part (the paper's C = 2μK → ζ_n = μ ω_n).
    pub stiffness_proportional_part: f64,
}

/// Physical design of one string of the unison group.
#[derive(Clone, Copy)]
pub struct PianoStringDesign {
    /// Speaking length L (agraffe -> bridge) [m].
    pub speaking_length: f64,
    /// Core radius r [m].
    pub core_radius: f64,
    /// Material density ρ [kg/m³] (steel).
    pub material_density: f64,
    /// Young's modulus E [Pa].
    pub youngs_modulus: f64,
    /// Static string tension T (the prestress) [N].
    pub static_tension: f64,
    /// Hammer strike position divided by the speaking length.
    pub hammer_strike_ratio: f64,
    /// Highest modeled modal frequency [Hz].
    pub highest_modeled_frequency: f64,
    /// Hard cap on the number of transverse modes per polarization.
    pub maximum_transverse_mode_count: usize,
    /// Damping of the vertical polarization.
    pub vertical_damping: DampingCoefficients,
    /// Damping of the horizontal polarization (lower -> double-decay effect).
    pub horizontal_damping: DampingCoefficients,
    /// Damping of the longitudinal polarization.
    pub longitudinal_damping: DampingCoefficients,
}

fn transverse_mode_angular_frequency(
    wavenumber: f64,
    tuned_tension: f64,
    linear_mass_density: f64,
    bending_stiffness: f64,
) -> f64 {
    let tension_term = (tuned_tension / linear_mass_density) * wavenumber * wavenumber;
    let stiffness_term = (bending_stiffness / linear_mass_density)
        * wavenumber
        * wavenumber
        * wavenumber
        * wavenumber;
    (tension_term + stiffness_term).sqrt()
}

/// One unison string, reduced to modal form.
pub struct PianoString {
    pub design: PianoStringDesign,
    /// Lateral position of this string under the hammer [m] (§6.1.2).
    pub lateral_offset_from_hammer_center: f64,
    /// Linear mass density ρA [kg/m].
    pub linear_mass_density: f64,
    /// Bending stiffness EI [N·m²] (inharmonicity).
    pub bending_stiffness: f64,
    /// Longitudinal stiffness EA [N].
    pub axial_stiffness: f64,
    /// Tension of this (detuned) unison string [N].
    pub tuned_tension: f64,
    /// Vertical transverse modes (w).
    pub vertical_modes: Vec<ModalOscillator>,
    /// Horizontal transverse modes (v).
    pub horizontal_modes: Vec<ModalOscillator>,
    /// Longitudinal modes (u).
    pub longitudinal_modes: Vec<ModalOscillator>,
    /// sin(nπ x_strike / L) for every mode index (shared by all polarizations).
    pub mode_shape_at_strike_point: Vec<f64>,
    /// k_n = nπ / L.
    pub mode_wavenumbers: Vec<f64>,
    /// (-1)^n for every mode index.
    pub alternating_signs: Vec<f64>,
}

impl PianoString {
    /// `relative_detune` shifts this string's tension (hence its pitch) inside
    /// the unison group, producing the characteristic slow beating.
    pub fn new(design: PianoStringDesign, lateral_offset: f64, relative_detune: f64) -> Self {
        let linear_mass_density =
            design.material_density * PI * design.core_radius * design.core_radius;
        let bending_stiffness = design.youngs_modulus * PI * design.core_radius.powi(4) / 4.0;
        let axial_stiffness = design.youngs_modulus * PI * design.core_radius * design.core_radius;
        let tuned_tension = design.static_tension * (1.0 + relative_detune).powi(2);
        let modal_mass = linear_mass_density * design.speaking_length * 0.5;
        let cutoff_angular_frequency = 2.0 * PI * design.highest_modeled_frequency;

        // --- transverse modes (vertical + horizontal) --------------------
        let mut number_of_transverse_modes = 0usize;
        for mode_index in 1..=design.maximum_transverse_mode_count {
            let wavenumber = mode_index as f64 * PI / design.speaking_length;
            let angular_frequency = transverse_mode_angular_frequency(
                wavenumber,
                tuned_tension,
                linear_mass_density,
                bending_stiffness,
            );
            if angular_frequency > cutoff_angular_frequency {
                break;
            }
            number_of_transverse_modes = mode_index;
        }
        number_of_transverse_modes = number_of_transverse_modes.max(1);

        let mut vertical_modes = Vec::with_capacity(number_of_transverse_modes);
        let mut horizontal_modes = Vec::with_capacity(number_of_transverse_modes);
        let mut mode_shape_at_strike_point = Vec::with_capacity(number_of_transverse_modes);
        let mut mode_wavenumbers = Vec::with_capacity(number_of_transverse_modes);
        let mut alternating_signs = Vec::with_capacity(number_of_transverse_modes);

        for mode_index in 1..=number_of_transverse_modes {
            let wavenumber = mode_index as f64 * PI / design.speaking_length;
            let angular_frequency = transverse_mode_angular_frequency(
                wavenumber,
                tuned_tension,
                linear_mass_density,
                bending_stiffness,
            );
            let vertical_damping_ratio = (design.vertical_damping.constant_part
                + design.vertical_damping.stiffness_proportional_part * angular_frequency)
                .clamp(1.0e-6, 0.3);
            let horizontal_damping_ratio = (design.horizontal_damping.constant_part
                + design.horizontal_damping.stiffness_proportional_part * angular_frequency)
                .clamp(1.0e-6, 0.3);
            vertical_modes.push(ModalOscillator::new(
                angular_frequency,
                vertical_damping_ratio,
                modal_mass,
            ));
            horizontal_modes.push(ModalOscillator::new(
                angular_frequency,
                horizontal_damping_ratio,
                modal_mass,
            ));
            mode_shape_at_strike_point
                .push((mode_index as f64 * PI * design.hammer_strike_ratio).sin());
            mode_wavenumbers.push(wavenumber);
            alternating_signs.push(if mode_index % 2 == 0 { 1.0 } else { -1.0 });
        }

        // --- longitudinal modes: ω_n = k_n √(E/ρ) ------------------------
        let longitudinal_wave_speed = (design.youngs_modulus / design.material_density).sqrt();
        let mut longitudinal_modes = Vec::new();
        let mut mode_index = 1usize;
        while mode_index <= number_of_transverse_modes {
            let wavenumber = mode_index as f64 * PI / design.speaking_length;
            let angular_frequency = wavenumber * longitudinal_wave_speed;
            if angular_frequency > cutoff_angular_frequency {
                break;
            }
            let longitudinal_damping_ratio = (design.longitudinal_damping.constant_part
                + design.longitudinal_damping.stiffness_proportional_part * angular_frequency)
                .clamp(1.0e-6, 0.9);
            longitudinal_modes.push(ModalOscillator::new(
                angular_frequency,
                longitudinal_damping_ratio,
                modal_mass,
            ));
            mode_index += 1;
        }

        PianoString {
            design,
            lateral_offset_from_hammer_center: lateral_offset,
            linear_mass_density,
            bending_stiffness,
            axial_stiffness,
            tuned_tension,
            vertical_modes,
            horizontal_modes,
            longitudinal_modes,
            mode_shape_at_strike_point,
            mode_wavenumbers,
            alternating_signs,
        }
    }

    /// String displacement at the strike point (string frame).  Includes the
    /// bridge-following ramp (x/L)·u_bridge so the contact geometry of
    /// eq. (6.1) sees the moving termination.
    pub fn displacement_at_strike_point(&self, bridge_displacement: Vector3) -> Vector3 {
        let strike_ratio = self.design.hammer_strike_ratio;
        let mut displacement = bridge_displacement.scaled(strike_ratio);
        for mode_index in 0..self.vertical_modes.len() {
            let shape_at_strike = self.mode_shape_at_strike_point[mode_index];
            displacement.z += self.vertical_modes[mode_index].displacement * shape_at_strike;
            displacement.y += self.horizontal_modes[mode_index].displacement * shape_at_strike;
            if mode_index < self.longitudinal_modes.len() {
                displacement.x +=
                    self.longitudinal_modes[mode_index].displacement * shape_at_strike;
            }
        }
        displacement
    }

    /// One explicit modal step of the whole string (paper §7.2.1).
    ///
    /// `hammer_force_at_strike_point`: point force at x_strike (string frame).
    /// `bridge_displacement` / `bridge_acceleration`: state of the bridge
    /// point (§6.2 dynamic coupling — displacement/velocity continuity).
    pub fn apply_forces_and_advance(
        &mut self,
        hammer_force_at_strike_point: Vector3,
        // bridge_displacement: Vector3,
        bridge_acceleration: Vector3,
        step_size: f64,
    ) {
        for mode_index in 0..self.vertical_modes.len() {
            let wavenumber = self.mode_wavenumbers[mode_index];
            let alternating_sign = self.alternating_signs[mode_index];
            let shape_at_strike = self.mode_shape_at_strike_point[mode_index];

            // Vertical polarization: hammer force + inertial bridge coupling.
            let vertical_modal_force = hammer_force_at_strike_point.z * shape_at_strike
                + self.linear_mass_density * alternating_sign * bridge_acceleration.z / wavenumber;
            self.vertical_modes[mode_index].advance(vertical_modal_force, step_size);

            // Horizontal polarization.
            let horizontal_modal_force = hammer_force_at_strike_point.y * shape_at_strike
                + self.linear_mass_density * alternating_sign * bridge_acceleration.y / wavenumber;
            self.horizontal_modes[mode_index].advance(horizontal_modal_force, step_size);
        }
        for mode_index in 0..self.longitudinal_modes.len() {
            let wavenumber = self.mode_wavenumbers[mode_index];
            let alternating_sign = self.alternating_signs[mode_index];
            let shape_at_strike = self.mode_shape_at_strike_point[mode_index];
            let longitudinal_modal_force = hammer_force_at_strike_point.x * shape_at_strike
                + self.linear_mass_density * alternating_sign * bridge_acceleration.x / wavenumber;
            self.longitudinal_modes[mode_index].advance(longitudinal_modal_force, step_size);
        }
    }

    /// Force the string exerts on the bridge (§6.2, reaction of the end
    /// tension/axial force), evaluated from the *current* modal state:
    ///     F = (-EA ∂u/∂x|_L, -T ∂v/∂x|_L, -T ∂w/∂x|_L)
    /// where the end slope contains both the modal sum and the ramp w_b/L
    /// (the latter is the string acting as a restoring spring on the board).
    pub fn force_exerted_on_bridge(&self, bridge_displacement: Vector3) -> Vector3 {
        let mut vertical_end_slope = bridge_displacement.z / self.design.speaking_length;
        let mut horizontal_end_slope = bridge_displacement.y / self.design.speaking_length;
        let mut longitudinal_end_slope = bridge_displacement.x / self.design.speaking_length;
        for mode_index in 0..self.vertical_modes.len() {
            let slope_contribution =
                self.mode_wavenumbers[mode_index] * self.alternating_signs[mode_index];
            vertical_end_slope += slope_contribution * self.vertical_modes[mode_index].displacement;
            horizontal_end_slope +=
                slope_contribution * self.horizontal_modes[mode_index].displacement;
        }
        for mode_index in 0..self.longitudinal_modes.len() {
            let slope_contribution =
                self.mode_wavenumbers[mode_index] * self.alternating_signs[mode_index];
            longitudinal_end_slope +=
                slope_contribution * self.longitudinal_modes[mode_index].displacement;
        }
        Vector3::new(
            -self.axial_stiffness * longitudinal_end_slope,
            -self.tuned_tension * horizontal_end_slope,
            -self.tuned_tension * vertical_end_slope,
        )
    }
}
