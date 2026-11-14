pub mod audio_sync;
pub mod engine;
pub mod screen_sync;

pub use audio_sync::{AudioBeatDetector, AudioBeatFrame};
pub use engine::{
    compute_audio_beat_color, compute_battery_smart_color, compute_breathing_color,
    compute_input_reactive_color, compute_spectrum_color, evaluate_effect_frame, EffectEngine,
    EffectFrameInput, LightingEffectMode,
};
pub use screen_sync::ScreenAmbilightSampler;
