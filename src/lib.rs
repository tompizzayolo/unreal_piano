mod engine;
mod instrument;
mod params;
mod presets;
mod ui;
mod voice;

pub use params::UnrealPianoParams;

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use nice_plug::prelude::*;
use nice_plug_iced::iced::PollSubNotifier;
use nice_plug_iced::{
    IcedEditor, IcedEditorState, IcedNiceSettings, application, create_iced_editor,
};

use crate::engine::hammer::HammerGeometry;
use crate::engine::rng::DeterministicRandom;
use crate::instrument::{
    Instrument, LiveVoicingControls, SIMULATION_RATE_HZ, SIMULATION_STEP_SIZE,
    StrikeVoicingControls, note_design, shared_hammer_geometry,
};
use crate::ui::{EditorSharedState, INITIAL_SCALE_FACTOR, INITIAL_WINDOW_SIZE, RESIZE_HINT};
use crate::voice::{PianoVoice, SynthVoice};

/// Number of simultaneous voices.  The main CPU dial: each extra voice costs
/// roughly 3–6% of a core, the shared resonators ~10%.
const NUM_VOICES: usize = 8;
/// Maximum audio block size (blocks are also split on note events).
const MAX_BLOCK_SIZE: usize = 64;
/// A voice is freed once its string energy drops below this [J].
const VOICE_ENERGY_EPSILON: f64 = 1.0e-9;
/// Time for the peak meter to decay by 12 dB in silence [ms].
const PEAK_METER_DECAY_MS: f64 = 150.0;
/// Output-pressure ring size.
const PRESSURE_RING_SIZE: u64 = 8;

pub struct UnrealPiano {
    params: Arc<UnrealPianoParams>,

    editor_state: Arc<IcedEditorState>,

    /// The shared physical instrument (soundboard, air, room).
    instrument: Instrument,
    /// Hammer geometry shared by every note (only the felt law differs).
    shared_hammer_geometry: HammerGeometry,

    voices: [Option<SynthVoice>; NUM_VOICES],
    next_internal_voice_id: u64,

    /// Keys currently held (for the sustain pedal).
    held_keys: [bool; 128],
    sustain_pedal_down: bool,

    /// Output resampling state: each output sample maps to a fractional
    /// position on the engine's time grid.
    engine_steps_rendered: u64,
    output_samples_rendered: u64,
    engine_steps_per_output_sample: f64,
    pressure_ring: Vec<f64>,

    peak_meter: Arc<AtomicF32>,
    peak_meter_decay_weight: f32,
    active_voice_count: Arc<AtomicU32>,
    notifier: PollSubNotifier,
}

impl Default for UnrealPiano {
    fn default() -> Self {
        Self {
            params: Arc::new(UnrealPianoParams::default()),

            editor_state: IcedEditorState::from_size(INITIAL_WINDOW_SIZE, INITIAL_SCALE_FACTOR),

            instrument: Instrument::new(20240903),
            shared_hammer_geometry: shared_hammer_geometry(),
            voices: [0; NUM_VOICES].map(|_| None),
            next_internal_voice_id: 0,

            held_keys: [false; 128],
            sustain_pedal_down: false,

            engine_steps_rendered: 0,
            output_samples_rendered: 0,
            engine_steps_per_output_sample: 1.0, // corrected in activate()
            pressure_ring: vec![0.0; PRESSURE_RING_SIZE as usize],

            peak_meter: Arc::new(AtomicF32::new(util::MINUS_INFINITY_DB)),
            peak_meter_decay_weight: 1.0,
            active_voice_count: Arc::new(AtomicU32::new(0)),
            notifier: PollSubNotifier::new(),
        }
    }
}

impl Plugin for UnrealPiano {
    const NAME: &'static str = "Unreal Piano";
    const VENDOR: &'static str = "UnrealPiano";
    const URL: &'static str = "https://example.com/unreal-piano";
    const EMAIL: &'static str = "info@example.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type Editor = IcedEditor;
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Self::Editor> {
        create_iced_editor(
            self.editor_state.clone(),
            EditorSharedState {
                params: self.params.clone(),
                peak_meter: self.peak_meter.clone(),
                active_voice_count: self.active_voice_count.clone(),
            },
            self.notifier.clone(),
            IcedNiceSettings::new().with_resize_hint(RESIZE_HINT),
            |editor_state, nice_ctx| {
                Ok(application(
                    editor_state,
                    nice_ctx,
                    ui::PianoGui::new,
                    ui::PianoGui::update,
                    ui::PianoGui::view,
                )
                .theme(ui::PianoGui::theme)
                .scale_factor(ui::PianoGui::scale_factor)
                .subscription(ui::PianoGui::subscription)
                .run())
            },
        )
    }

    fn reset(&mut self) {
        self.instrument.reset();
        for voice_slot in self.voices.iter_mut() {
            *voice_slot = None;
        }
        self.next_internal_voice_id = 0;
        self.held_keys.fill(false);
        self.sustain_pedal_down = false;
        self.engine_steps_rendered = 0;
        self.output_samples_rendered = 0;
        self.pressure_ring.fill(0.0);
        self.peak_meter
            .store(util::MINUS_INFINITY_DB, Ordering::Relaxed);
    }

    fn activate(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl ActivateContext<Self>,
    ) -> bool {
        self.engine_steps_per_output_sample = SIMULATION_RATE_HZ / buffer_config.sample_rate as f64;
        self.peak_meter_decay_weight = 0.25f64
            .powf((buffer_config.sample_rate as f64 * PEAK_METER_DECAY_MS / 1000.0).recip())
            as f32;
        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Live voicing controls, read once per process call.
        self.instrument.live_voicing = LiveVoicingControls {
            sympathetic_resonance: self.params.sympathetic_resonance.value() as f64,
            duplex_scale: self.params.duplex_scale_resonance.value() as f64,
            blooming_energy: self.params.blooming_energy.value() as f64,
            blooming_inertia: self.params.blooming_inertia.value() as f64,
        };

        let num_samples = buffer.samples();
        let output = buffer.as_slice();

        let mut next_event = context.next_event();
        let mut block_start: usize = 0;
        let mut block_end: usize = MAX_BLOCK_SIZE.min(num_samples);

        while block_start < num_samples {
            // Handle all events at the start of the block, and cut the block
            // short when another event occurs inside of it.
            'events: loop {
                match next_event {
                    Some(event) if (event.timing() as usize) <= block_start => {
                        match event {
                            NoteEvent::NoteOn {
                                timing,
                                voice_id,
                                channel,
                                note,
                                velocity,
                            } => {
                                self.start_voice(
                                    context, timing, voice_id, channel, note, velocity,
                                );
                            }
                            NoteEvent::NoteOff {
                                voice_id,
                                channel,
                                note,
                                ..
                            } => {
                                self.start_release_for_voices(voice_id, channel, note);
                            }
                            NoteEvent::Choke {
                                timing,
                                voice_id,
                                channel,
                                note,
                            } => {
                                self.choke_voices(context, timing, voice_id, channel, note);
                            }
                            // Sustain pedal (MIDI CC 64).  `value` is an f32
                            // normalized to 0..=1; the else-branch also
                            // tolerates a raw 0..=127 encoding just in case.
                            NoteEvent::MidiCC {
                                timing: _,
                                channel: _,
                                cc,
                                value,
                            } if cc == 64 => {
                                let pedal_down = if value <= 1.0 {
                                    value >= 0.5
                                } else {
                                    value >= 64.0
                                };
                                self.set_sustain_pedal(pedal_down);
                            }
                            _ => (),
                        };
                        next_event = context.next_event();
                    }
                    Some(event) if (event.timing() as usize) < block_end => {
                        block_end = event.timing() as usize;
                        break 'events;
                    }
                    _ => break 'events,
                }
            }

            // Render this block by pulling the engine forward one output
            // sample at a time.
            for sample_index in block_start..block_end {
                let engine_sample = self.render_one_output_sample();
                let gain = self.params.output_gain.smoothed.next();
                let output_sample = engine_sample as f32 * gain;

                output[0][sample_index] = output_sample;
                output[1][sample_index] = output_sample;

                // Peak metering (only while the GUI is open).
                if self.editor_state.is_open() {
                    let amplitude = output_sample.abs();
                    let current_peak = self.peak_meter.load(Ordering::Relaxed);
                    let new_peak = if amplitude > current_peak {
                        amplitude
                    } else {
                        current_peak * self.peak_meter_decay_weight
                            + amplitude * (1.0 - self.peak_meter_decay_weight)
                    };
                    let clamped_peak = if new_peak < 0.0001 { 0.0 } else { new_peak };
                    if current_peak != clamped_peak {
                        self.peak_meter.store(clamped_peak, Ordering::Relaxed);
                        self.notifier.notify();
                    }
                }
            }

            // Free voices that have decayed into silence.
            for voice_slot in self.voices.iter_mut() {
                let terminate = match voice_slot {
                    Some(voice) => voice.engine.is_silent(VOICE_ENERGY_EPSILON),
                    None => false,
                };
                if terminate {
                    if let Some(voice) = voice_slot.take() {
                        context.send_event(NoteEvent::VoiceTerminated {
                            timing: block_end as u32,
                            voice_id: Some(voice.voice_id),
                            channel: voice.channel,
                            note: voice.note,
                        });
                    }
                }
            }

            // Keep the GUI's voice counter up to date.
            if self.editor_state.is_open() {
                let active_count =
                    self.voices.iter().filter(|voice| voice.is_some()).count() as u32;
                if self.active_voice_count.load(Ordering::Relaxed) != active_count {
                    self.active_voice_count
                        .store(active_count, Ordering::Relaxed);
                    self.notifier.notify();
                }
            }

            block_start = block_end;
            block_end = (block_start + MAX_BLOCK_SIZE).min(num_samples);
        }

        ProcessStatus::Normal
    }
}

impl UnrealPiano {
    /// Render one output sample: advance the engine far enough to cover the
    /// fractional engine position of this sample, then linearly interpolate
    /// the listener pressure.
    fn render_one_output_sample(&mut self) -> f64 {
        let engine_position =
            self.output_samples_rendered as f64 * self.engine_steps_per_output_sample;
        let base_step = engine_position.floor() as u64;
        let fraction = engine_position - base_step as f64;

        // Render ahead so that `base_step + 1` exists in the ring.
        while self.engine_steps_rendered < base_step + 2 {
            let pressure = self.instrument.step(&mut self.voices);
            let ring_index = (self.engine_steps_rendered % PRESSURE_RING_SIZE) as usize;
            self.pressure_ring[ring_index] = pressure;
            self.engine_steps_rendered += 1;
        }

        let sample_at_base_step = self.pressure_ring[(base_step % PRESSURE_RING_SIZE) as usize];
        let sample_at_base_step_plus_one =
            self.pressure_ring[((base_step + 1) % PRESSURE_RING_SIZE) as usize];
        self.output_samples_rendered += 1;
        sample_at_base_step + (sample_at_base_step_plus_one - sample_at_base_step) * fraction
    }

    fn start_voice(
        &mut self,
        context: &mut impl ProcessContext<Self>,
        sample_offset: u32,
        voice_id: Option<i32>,
        channel: u8,
        note: u8,
        velocity: f32,
    ) {
        // MIDI velocity (0..1) maps linearly onto the hammer contact speed.
        let minimum_strike_velocity = self.params.minimum_strike_velocity.value() as f64;
        let maximum_strike_velocity = self.params.maximum_strike_velocity.value() as f64;
        let hammer_velocity = (minimum_strike_velocity
            + (maximum_strike_velocity - minimum_strike_velocity) * velocity as f64)
            .clamp(0.05, 12.0);
        // Position of this strike inside the dynamic range (0..1), used for
        // the piano/mezzo/forte hardness curve.
        let velocity_blend = ((hammer_velocity - minimum_strike_velocity)
            / (maximum_strike_velocity - minimum_strike_velocity).max(1.0e-3))
        .clamp(0.0, 1.0);

        let noise_seed = self
            .next_internal_voice_id
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ ((note as u64) << 32);
        let mut strike_randomness = DeterministicRandom::new(noise_seed);
        let (noise_minimum, noise_maximum) = ordered_range(
            self.params.hammer_noise_min.value() as f64,
            self.params.hammer_noise_max.value() as f64,
        );
        let (unison_minimum, unison_maximum) = ordered_range(
            self.params.unison_width_min.value() as f64,
            self.params.unison_width_max.value() as f64,
        );
        let hammer_noise_level =
            noise_minimum + (noise_maximum - noise_minimum) * strike_randomness.unit_interval();
        let unison_width_level =
            unison_minimum + (unison_maximum - unison_minimum) * strike_randomness.unit_interval();

        let strike_controls = StrikeVoicingControls {
            hammer_hardness: self.params.hammer_hardness.value() as f64,
            hammer_hardness_piano: self.params.hammer_hardness_piano.value() as f64,
            hammer_hardness_mezzo: self.params.hammer_hardness_mezzo.value() as f64,
            hammer_hardness_forte: self.params.hammer_hardness_forte.value() as f64,
            hammer_noise: hammer_noise_level,
            hammer_tone: self.params.hammer_tone.value() as f64,
            soft_pedal: self.params.soft_pedal.value() as f64,
            unison_width: unison_width_level,
            string_length_scale: self.params.string_length.value() as f64,
            strike_point_ratio: self.params.strike_point.value() as f64,
        };

        let design = note_design(note, &strike_controls, velocity_blend);
        let mut engine_voice = PianoVoice::new(&design, self.shared_hammer_geometry);

        // A re-strike physically hits the *same* ringing strings: inherit
        // the string state of any existing voice on this key, then retire
        // that voice (one string group per key).
        for existing_slot in self.voices.iter_mut() {
            let is_same_key = match existing_slot {
                Some(existing) => existing.note == note && existing.channel == channel,
                None => false,
            };
            if is_same_key {
                if let Some(existing) = existing_slot.take() {
                    engine_voice.adopt_string_state_from(&existing.engine);
                    context.send_event(NoteEvent::VoiceTerminated {
                        timing: sample_offset,
                        voice_id: Some(existing.voice_id),
                        channel: existing.channel,
                        note: existing.note,
                    });
                }
            }
        }

        engine_voice.strike(hammer_velocity, noise_seed);

        let new_voice = SynthVoice {
            voice_id: voice_id.unwrap_or_else(|| compute_fallback_voice_id(note, channel)),
            internal_voice_id: self.next_internal_voice_id,
            channel,
            note,
            releasing: false,
            engine: engine_voice,
        };
        self.next_internal_voice_id = self.next_internal_voice_id.wrapping_add(1);
        self.held_keys[(note as usize).min(127)] = true;

        match self.voices.iter().position(|voice| voice.is_none()) {
            Some(free_voice_index) => {
                self.voices[free_voice_index] = Some(new_voice);
            }
            None => {
                // Steal the oldest voice and inform the host.
                let oldest_voice = self.voices.iter_mut().min_by_key(|voice| {
                    voice
                        .as_ref()
                        .map_or(u64::MAX, |candidate| candidate.internal_voice_id)
                });
                if let Some(oldest_slot) = oldest_voice {
                    if let Some(oldest_voice) = oldest_slot.take() {
                        context.send_event(NoteEvent::VoiceTerminated {
                            timing: sample_offset,
                            voice_id: Some(oldest_voice.voice_id),
                            channel: oldest_voice.channel,
                            note: oldest_voice.note,
                        });
                    }
                }
                if let Some(free_voice_index) = self.voices.iter().position(|voice| voice.is_none())
                {
                    self.voices[free_voice_index] = Some(new_voice);
                }
            }
        }
    }

    fn start_release_for_voices(&mut self, voice_id: Option<i32>, channel: u8, note: u8) {
        self.held_keys[(note as usize).min(127)] = false;
        // With the sustain pedal down the damper stays off the string.
        if self.sustain_pedal_down {
            return;
        }
        let release_seconds = self.params.damper_release_ms.value() as f64 / 1000.0;
        for voice in self.voices.iter_mut().flatten() {
            let is_target = voice_id == Some(voice.voice_id)
                || (channel == voice.channel && note == voice.note);
            if is_target {
                voice.releasing = true;
                voice
                    .engine
                    .start_release(release_seconds, SIMULATION_STEP_SIZE);
                if voice_id.is_some() {
                    return;
                }
            }
        }
    }

    fn set_sustain_pedal(&mut self, pedal_down: bool) {
        if pedal_down == self.sustain_pedal_down {
            return;
        }
        self.sustain_pedal_down = pedal_down;
        if pedal_down {
            return;
        }
        // Pedal lifted: damp every voice whose key is no longer held.
        let release_seconds = self.params.damper_release_ms.value() as f64 / 1000.0;
        for voice in self.voices.iter_mut().flatten() {
            if !self.held_keys[(voice.note as usize).min(127)] {
                voice.releasing = true;
                voice
                    .engine
                    .start_release(release_seconds, SIMULATION_STEP_SIZE);
            }
        }
    }

    fn choke_voices(
        &mut self,
        context: &mut impl ProcessContext<Self>,
        sample_offset: u32,
        voice_id: Option<i32>,
        channel: u8,
        note: u8,
    ) {
        for voice_slot in self.voices.iter_mut() {
            let is_target = match voice_slot {
                Some(voice) => {
                    voice_id == Some(voice.voice_id)
                        || (channel == voice.channel && note == voice.note)
                }
                None => false,
            };
            if is_target {
                if let Some(voice) = voice_slot.take() {
                    context.send_event(NoteEvent::VoiceTerminated {
                        timing: sample_offset,
                        voice_id: Some(voice.voice_id),
                        channel: voice.channel,
                        note: voice.note,
                    });
                }
                if voice_id.is_some() {
                    return;
                }
            }
        }
    }
}

/// Order a min/max parameter pair (so a reversed pair still works).
fn ordered_range(first: f64, second: f64) -> (f64, f64) {
    if first <= second {
        (first, second)
    } else {
        (second, first)
    }
}

const fn compute_fallback_voice_id(note: u8, channel: u8) -> i32 {
    note as i32 | ((channel as i32) << 16)
}

impl ClapPlugin for UnrealPiano {
    const CLAP_ID: &'static str = "com.unrealpiano.unreal-piano";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("A physical-modeling piano after arXiv:2409.03481");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for UnrealPiano {
    const VST3_CLASS_ID: [u8; 16] = *b"UnrealPianoClap1";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

nice_export_clap!(UnrealPiano);
nice_export_vst3!(UnrealPiano);
