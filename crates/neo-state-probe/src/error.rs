#[derive(Debug)]
pub enum StateReadError {
    InvalidField,
    DuplicateBinding,
    MissingBinding,
    UnknownTweak,
    StatePlan,
    Read,
    UnsupportedPlatform,
}
