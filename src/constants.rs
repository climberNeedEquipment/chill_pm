use strum_macros::{Display, EnumIter, EnumString};

#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumString, Display, EnumIter)]
pub enum Interval {
    #[strum(serialize = "1m")]
    Min1,
    #[strum(serialize = "3m")]
    Min3,
    #[strum(serialize = "5m")]
    Min5,
    #[strum(serialize = "15m")]
    Min15,
    #[strum(serialize = "30m")]
    Min30,
    // #[strum(serialize = "1h")]
    // Hour1,
}
