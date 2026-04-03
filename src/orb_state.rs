#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum OrbState {
    #[default]
    Idle,
    Listening,
    Thinking,
    Speaking,
    Error,
}
