pub mod engine;

pub use engine::{
    compute_audio_beat_color, compute_battery_smart_color, compute_breathing_color,
    compute_input_reactive_color, compute_spectrum_color, evaluate_effect_frame, EffectEngine,
    EffectFrameInput, LightingEffectMode,
};
