pub use jiff::SignedDuration;

#[derive(Clone, Copy, Debug)]
pub struct Base64();

#[derive(Clone, Debug)]
pub enum CUType {
    Individual,
    Group,
    Resource,
    Room,
    Unknown(Option<String>),
}

#[derive(Clone, Debug)]
pub enum Display {
    Badge(Option<String>),
    Graphic,
    Fullsize,
    Thumbnail,
}

#[derive(Clone, Debug)]
pub enum FBType {
    Free,
    Busy(Option<String>),
    BusyUnavailable,
    BusyTentative,
}

#[derive(Clone, Debug)]
pub enum Feature {
    Audio,
    Chat,
    Feed,
    Moderator,
    Phone,
    Screen,
    Video,
    Other(String),
}

#[derive(Clone, Debug)]
pub enum PartStat {
    NeedsAction(Option<String>),
    Accepted,
    Declined,
    Tentative,
    Delegated,
    Completed,
    InProcess,
}
#[derive(Clone, Copy, Debug)]
pub enum Related {
    Start,
    End,
}

#[derive(Clone, Debug)]
pub enum RelType {
    Parent(Option<String>),
    Child,
    Sibling,
}

#[derive(Clone, Debug)]
pub enum Role {
    Chair,
    ReqParticipant(Option<String>),
    OptParticipant,
    NonParticipant,
}

#[derive(Clone, Debug)]
pub enum ScheduleAgent {
    Server,
    Client,
    None(Option<String>),
}

#[derive(Clone, Debug)]
pub enum ScheduleForceSend {
    Request,
    Reply,
    Unknown(Option<String>),
}

#[derive(Clone, Copy, Debug)]
pub struct ThisAndFuture();

#[derive(Clone, Debug)]
pub enum Value {
    Binary,
    Boolean,
    CalAddress,
    Date,
    DateTime,
    Duration,
    Float,
    Integer,
    Period,
    Recur,
    Text,
    Time,
    Uid,
    Uri,
    UtcOffset,
    XmlReference,
    Other(String),
}

pub type UriString = String; // FIXME: this type can't contain CONTROL, DQUOTE, ";", ":", ","
pub type ParamText = String; // FIXME: this type can't contain CONTROL, DQUOTE, ";", ":", ","
pub type FmtType = String; // FIXME: must be a media type the media type [RFC4288]
pub type Language = String; // FIXME: must be as defined in [RFC5646].
pub type ScheduleStatus = Vec<String>; // FIXME: must be at least one dot-separated pair or triplet of integers, like "3.1" or "3.1.1"
pub type CalAddress = String; // FIXME: must be mailto: uri
