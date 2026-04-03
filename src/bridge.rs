//! Cross-thread signaling for orb visuals and TTS amplitude.

use parking_lot::Mutex;
use std::sync::Arc;

use crate::orb_state::OrbState;

#[derive(Debug, Clone)]
pub struct ConfirmReply {
    pub id: uuid::Uuid,
    pub approved: bool,
}

#[derive(Clone, Default)]
pub struct OrbBus {
    pub state: Arc<Mutex<OrbState>>,
    pub amplitude: Arc<Mutex<f32>>,
}
