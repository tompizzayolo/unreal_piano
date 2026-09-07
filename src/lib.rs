mod engine;
mod params;
mod ui;

use nice_plug::prelude::*;
use nice_plug_iced::iced::PollSubNotifier;
use nice_plug_iced::{IcedEditor, IcedEditorState};
use std::sync::Arc;

use engine::soundboard::Soundboard;
use engine::voice::Voice;
use params::PianoParams;

const NUM_VOICES: u32 = 16;
const MAX_BLOCK_SIZE: usize = 64;
const DIRECT_RADIATION_MIX: f32 = 0.15;

struct UnrealPiano {
    params: Arc<PianoParams>,
    voices: [Option<Voice>; NUM_VOICES as usize],
    soundboard: Soundboard,
    next_internal_voice_id: u64,

    editor_state: Arc<IcedEditorState>,
    notifier: PollSubNotifier,
}

impl Default for UnrealPiano {
    fn default() -> Self {
        Self {
            params: Arc::new(PianoParams::default()),
            voices: [0; NUM_VOICES as usize].map(|_| None),
            soundboard: Soundboard::new(),
            next_internal_voice_id: 0,

            editor_state: IcedEditorState::from_size(ui::MIN_WINDOW_SIZE, ui::INITIAL_SCALE_FACTOR),
            notifier: PollSubNotifier::new(),
        }
    }
}

impl Plugin for UnrealPiano {
    const NAME: &'static str = "Unreal Piano";
    const VENDOR: &'static str = "Tom Pizza";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
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
        ui::create_editor(
            self.params.clone(),
            self.editor_state.clone(),
            self.notifier.clone(),
        )
    }

    fn reset(&mut self) {
        self.voices.fill(None);
        self.soundboard.reset();
        self.next_internal_voice_id = 0;
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let num_samples = buffer.samples();
        let sample_rate = context.transport().sample_rate;
        let dt = 1.0 / sample_rate;
        let output = buffer.as_slice();

        let mut next_event = context.next_event();
        let mut block_start: usize = 0;
        let mut block_end: usize = MAX_BLOCK_SIZE.min(num_samples);

        while block_start < num_samples {
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
                                let frequency = util::midi_note_to_freq(note);
                                let inharmonicity = self.params.inharmonicity.value();
                                let voice = self.start_voice(
                                    context,
                                    timing,
                                    voice_id,
                                    channel,
                                    note,
                                    frequency,
                                    velocity,
                                    inharmonicity,
                                );
                                let _ = voice;
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
                            _ => (),
                        }
                        next_event = context.next_event();
                    }
                    Some(event) if (event.timing() as usize) < block_end => {
                        block_end = event.timing() as usize;
                        break 'events;
                    }
                    _ => break 'events,
                }
            }

            output[0][block_start..block_end].fill(0.0);
            output[1][block_start..block_end].fill(0.0);

            for sample_idx in block_start..block_end {
                let params = self.params.next_snapshot();

                let mut total_bridge_force = 0.0_f32;
                let mut direct_left = 0.0_f32;
                let mut direct_right = 0.0_f32;

                for voice_slot in self.voices.iter_mut() {
                    if let Some(voice) = voice_slot.as_mut() {
                        let voice_output = voice.process(dt, &params);
                        total_bridge_force += voice_output.bridge_force;
                        let pan_left = 0.5 - voice.pan;
                        let pan_right = 0.5 + voice.pan;
                        direct_left += voice_output.direct_signal * pan_left;
                        direct_right += voice_output.direct_signal * pan_right;
                    }
                }

                let soundboard_signal =
                    self.soundboard
                        .process(total_bridge_force, dt, params.soundboard_decay);

                let left =
                    (soundboard_signal + direct_left * DIRECT_RADIATION_MIX) * params.output_gain;
                let right =
                    (soundboard_signal + direct_right * DIRECT_RADIATION_MIX) * params.output_gain;

                output[0][sample_idx] = left.tanh();
                output[1][sample_idx] = right.tanh();
            }

            for voice_slot in self.voices.iter_mut() {
                match voice_slot {
                    Some(voice) if voice.is_dead() => {
                        context.send_event(NoteEvent::VoiceTerminated {
                            timing: block_end as u32,
                            voice_id: Some(voice.voice_id),
                            channel: voice.channel,
                            note: voice.note,
                        });
                        *voice_slot = None;
                    }
                    _ => (),
                }
            }

            block_start = block_end;
            block_end = (block_start + MAX_BLOCK_SIZE).min(num_samples);
        }

        ProcessStatus::Normal
    }
}

impl UnrealPiano {
    fn get_voice_idx(&self, voice_id: i32) -> Option<usize> {
        self.voices
            .iter()
            .position(|voice| matches!(voice, Some(v) if v.voice_id == voice_id))
    }

    fn start_voice(
        &mut self,
        context: &mut impl ProcessContext<Self>,
        sample_offset: u32,
        voice_id: Option<i32>,
        channel: u8,
        note: u8,
        frequency: f32,
        velocity: f32,
        inharmonicity: f32,
    ) -> &mut Voice {
        let resolved_voice_id =
            voice_id.unwrap_or_else(|| compute_fallback_voice_id(note, channel));
        let internal_id = self.next_internal_voice_id;
        self.next_internal_voice_id = self.next_internal_voice_id.wrapping_add(1);

        let new_voice = Voice::new(
            resolved_voice_id,
            channel,
            note,
            internal_id,
            frequency,
            velocity,
            inharmonicity,
        );

        match self.voices.iter().position(|v| v.is_none()) {
            Some(free_idx) => {
                self.voices[free_idx] = Some(new_voice);
                self.voices[free_idx].as_mut().unwrap()
            }
            None => {
                let oldest_slot = unsafe {
                    self.voices
                        .iter_mut()
                        .min_by_key(|v| v.as_ref().unwrap_unchecked().internal_voice_id)
                        .unwrap_unchecked()
                };
                {
                    let old = oldest_slot.as_ref().unwrap();
                    context.send_event(NoteEvent::VoiceTerminated {
                        timing: sample_offset,
                        voice_id: Some(old.voice_id),
                        channel: old.channel,
                        note: old.note,
                    });
                }
                *oldest_slot = Some(new_voice);
                oldest_slot.as_mut().unwrap()
            }
        }
    }

    fn start_release_for_voices(&mut self, voice_id: Option<i32>, channel: u8, note: u8) {
        for voice_slot in self.voices.iter_mut() {
            match voice_slot {
                Some(voice)
                    if voice_id == Some(voice.voice_id)
                        || (channel == voice.channel && note == voice.note) =>
                {
                    voice.note_off();
                    if voice_id.is_some() {
                        return;
                    }
                }
                _ => (),
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
            match voice_slot {
                Some(voice)
                    if voice_id == Some(voice.voice_id)
                        || (channel == voice.channel && note == voice.note) =>
                {
                    context.send_event(NoteEvent::VoiceTerminated {
                        timing: sample_offset,
                        voice_id: Some(voice.voice_id),
                        channel,
                        note,
                    });
                    *voice_slot = None;
                    if voice_id.is_some() {
                        return;
                    }
                }
                _ => (),
            }
        }
    }
}

const fn compute_fallback_voice_id(note: u8, channel: u8) -> i32 {
    note as i32 | ((channel as i32) << 16)
}

impl ClapPlugin for UnrealPiano {
    const CLAP_ID: &'static str = "com.modal-systems.physical-piano";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Real-time modal piano synthesis derived from physical modeling.");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
    ];
}

nice_export_clap!(UnrealPiano);
