use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::effects::LightingEffectMode;

/// Criterion matching a running game by executable name or `SteamAppId`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameMatch {
    ProcessName(String),
    SteamAppId(u32),
}

impl GameMatch {
    #[must_use]
    pub fn matches(&self, snapshot: &ProcessSnapshot) -> bool {
        match self {
            Self::ProcessName(target) => {
                let target_lower = target.to_ascii_lowercase();
                snapshot
                    .running_processes
                    .iter()
                    .any(|proc_name| proc_name.to_ascii_lowercase().contains(&target_lower))
            }
            Self::SteamAppId(app_id) => snapshot.active_steam_app_id == Some(*app_id),
        }
    }
}

/// User-configured lighting rule automatically activated when a game launches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameProfileRule {
    pub id: String,
    pub name: String,
    pub matcher: GameMatch,
    pub preset_id: String,
    pub effect_mode: LightingEffectMode,
}

/// Snapshot of currently active OS processes and optional Steam game ID.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProcessSnapshot {
    pub running_processes: Vec<String>,
    pub active_steam_app_id: Option<u32>,
}

/// Trait abstracting OS process & SteamOS inspection so profile switching is testable offline.
pub trait ProcessProvider: Send + Sync {
    fn snapshot(&self) -> ProcessSnapshot;
}

/// Minimal runtime provider reading `SteamAppId` from environment variables.
#[derive(Debug, Default)]
pub struct SystemProcessProvider;

impl ProcessProvider for SystemProcessProvider {
    fn snapshot(&self) -> ProcessSnapshot {
        let active_steam_app_id = std::env::var("SteamAppId")
            .ok()
            .and_then(|v| v.trim().parse::<u32>().ok())
            .filter(|id| *id > 0);
        ProcessSnapshot {
            running_processes: Vec::new(),
            active_steam_app_id,
        }
    }
}

/// Deterministic mock [`ProcessProvider`] for unit tests.
#[derive(Debug, Clone, Default)]
pub struct MockProcessProvider {
    state: Arc<Mutex<ProcessSnapshot>>,
}

impl MockProcessProvider {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_snapshot(&self, snapshot: ProcessSnapshot) {
        if let Ok(mut guard) = self.state.lock() {
            *guard = snapshot;
        }
    }
}

impl ProcessProvider for MockProcessProvider {
    fn snapshot(&self) -> ProcessSnapshot {
        self.state
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }
}

/// Current active profile determined by [`GameWatcher`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ActiveProfileState {
    #[default]
    Desktop,
    GameProfile(GameProfileRule),
}

/// Watches running games/SteamAppIds and emits state transitions `Desktop -> GameProfile -> Desktop`.
pub struct GameWatcher<P: ProcessProvider> {
    provider: P,
    rules: Vec<GameProfileRule>,
    current: ActiveProfileState,
}

impl<P: ProcessProvider> GameWatcher<P> {
    #[must_use]
    pub fn new(provider: P, rules: Vec<GameProfileRule>) -> Self {
        Self {
            provider,
            rules,
            current: ActiveProfileState::Desktop,
        }
    }

    #[must_use]
    pub fn current_state(&self) -> &ActiveProfileState {
        &self.current
    }

    /// Evaluates active processes and returns `Some(new_state)` only when the active profile changes.
    pub fn poll_transition(&mut self) -> Option<ActiveProfileState> {
        let snapshot = self.provider.snapshot();
        let next = self
            .rules
            .iter()
            .find(|rule| rule.matcher.matches(&snapshot))
            .cloned()
            .map_or(ActiveProfileState::Desktop, ActiveProfileState::GameProfile);

        if next != self.current {
            self.current = next.clone();
            Some(next)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transitions_cleanly_between_desktop_and_matching_game_profiles() {
        let provider = MockProcessProvider::new();
        let rules = vec![
            GameProfileRule {
                id: "cyberpunk".to_string(),
                name: "Cyberpunk 2077".to_string(),
                matcher: GameMatch::ProcessName("Cyberpunk2077.exe".to_string()),
                preset_id: "cyber-cyan".to_string(),
                effect_mode: LightingEffectMode::Breathing,
            },
            GameProfileRule {
                id: "elden-ring".to_string(),
                name: "Elden Ring".to_string(),
                matcher: GameMatch::SteamAppId(1_245_620),
                preset_id: "gold".to_string(),
                effect_mode: LightingEffectMode::Static,
            },
        ];

        let mut watcher = GameWatcher::new(provider.clone(), rules);
        assert_eq!(watcher.poll_transition(), None);

        provider.set_snapshot(ProcessSnapshot {
            running_processes: vec!["explorer.exe".into(), "cyberpunk2077.exe".into()],
            active_steam_app_id: None,
        });
        assert!(matches!(
            watcher.poll_transition(),
            Some(ActiveProfileState::GameProfile(ref rule)) if rule.id == "cyberpunk"
        ));
        // Repeated poll with same running game emits no duplicate transition
        assert_eq!(watcher.poll_transition(), None);

        // Switching to SteamAppId 1245620 activates Elden Ring rule
        provider.set_snapshot(ProcessSnapshot {
            running_processes: vec!["eldenring.exe".into()],
            active_steam_app_id: Some(1_245_620),
        });
        assert!(matches!(
            watcher.poll_transition(),
            Some(ActiveProfileState::GameProfile(ref rule)) if rule.id == "elden-ring"
        ));

        // Closing game returns to Desktop profile
        provider.set_snapshot(ProcessSnapshot::default());
        assert_eq!(watcher.poll_transition(), Some(ActiveProfileState::Desktop));
    }
}
