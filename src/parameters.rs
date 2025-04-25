use crate::names::NameIds;
pub use jiff::SignedDuration;
use litemap::LiteMap;
use std::num::NonZeroUsize;
use xmacro::xmacro;

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
pub struct UriString(String);
impl UriString {
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
    #[must_use]
    pub fn new(s: String) -> Self {
        Self(s)
    }
}

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

pub type ParamText = String; // FIXME: this type can't contain CONTROL, DQUOTE, ";", ":", ","
pub type FmtType = String; // FIXME: must be a media type the media type [RFC4288]
pub type Language = String; // FIXME: must be as defined in [RFC5646].
pub type ScheduleStatus = Vec<String>; // FIXME: must be at least one dot-separated pair or triplet of integers, like "3.1" or "3.1.1"
pub type CalAddress = String; // FIXME: must be mailto: uri
#[derive(Clone, Debug)]
pub enum ParameterValue {
    Boolean(bool),
    CUType(CUType),
    Display(Display),
    Duration(SignedDuration),
    Encoding(Option<Base64>),
    FBType(FBType),
    Feature(Feature),
    FmtType(FmtType),
    Language(Language),
    Order(NonZeroUsize),
    ParamText(ParamText),
    PartStat(PartStat),
    Range(Option<ThisAndFuture>),
    RelType(RelType),
    Related(Related),
    Role(Role),
    ScheduleAgent(ScheduleAgent),
    ScheduleForceSend(ScheduleForceSend),
    ScheduleStatus(ScheduleStatus),
    SentBy(CalAddress),
    Size(u64),
    Text(String),
    Tzid(String),
    Uri(UriString),
    UriList(Vec<UriString>),
    Value(Value),
}
pub struct Parameters(LiteMap<usize, ParameterValue>);

xmacro! {
    $(
        RFC: CONST: method: tag: variant: typ: doc:
        "[RFC5545, Section 3.2.1](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.1)\
        " ALTREP altrep "ALTREP" Uri UriString ""

        "[RFC5545, Section 3.2.2](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.2)\
        " CN cn "CN" Text String ""

        "[RFC5545, Section 3.2.3](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.3)\
        " CUTYPE cutype "CUTYPE" CUType CUType ""

        "[RFC5545, Section 3.2.4](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.4)\
        " DELEGATED_FROM delegated_from "DELEGATED-FROM" UriList (Vec<UriString>) ""

        "[RFC5545, Section 3.2.5](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.5)\
        " DELEGATED_TO delegated_to "DELEGATED-TO" UriList (Vec<UriString>) ""

        "[RFC9073, Section 5.3](https://datatracker.ietf.org/doc/html/rfc9073#section-5.3)\
        " DERIVED derived "DERIVED" Boolean bool ""

        "[RFC5545, Section 3.2.6](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.6)\
        " DIR dir "DIR" Uri UriString ""

        "[RFC7986, Section 6.1](https://datatracker.ietf.org/doc/html/rfc7986#section-6.1)\
        " DISPLAY display "DISPLAY" Display Display ""

        "[RFC7986, Section 6.2](https://datatracker.ietf.org/doc/html/rfc7986#section-6.2)\
        " EMAIL email "EMAIL" Text String ""

        "[RFC5545, Section 3.2.7](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.7)\
        " ENCODING encoding "ENCODING" Encoding (Option<Base64>) "\n\
            RFC 5545 gives values of 8BIT or BASE64, but the effect of an 8BIT value is the same as
            having no ENCODING parameter, so we use the single-valued Base64 type."

        "[RFC5545, Section 3.2.9](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.9)\
        " FBTYPE fbtype "FBTYPE" FBType FBType ""

        "[RFC7986, Section 6.3](https://datatracker.ietf.org/doc/html/rfc7986#section-6.3)\
        " FEATURE feature "FEATURE" Feature Feature ""

        "[RFC8607, Section 4.2](https://datatracker.ietf.org/doc/html/rfc8607#section-4.2)\
        " FILENAME filename "FILENAME" ParamText ParamText ""

        "[RFC5545, Section 3.2.8](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.8)\
        " FMTTYPE fmttype "FMTTYPE" FmtType FmtType ""

        "[RFC9253, Section 6.2](https://datatracker.ietf.org/doc/html/rfc9253#section-6.2)\
        " GAP gap "GAP" Duration SignedDuration ""

        "[RFC7986, Section 6.4](https://datatracker.ietf.org/doc/html/rfc7986#section-6.4)\
        " LABEL label "LABEL" Text String ""

        "[RFC5545, Section 3.2.10](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.10)\
        " LANGUAGE language "LANGUAGE" Language Language ""

        "[RFC9253, Section 6.1](https://datatracker.ietf.org/doc/html/rfc9253#section-6.1)\
        " LINKREL linkrel "LINKREL" Uri UriString ""

        "[RFC8607, Section 4.3](https://datatracker.ietf.org/doc/html/rfc8607#section-4.3)\
        " MANAGED_ID managed_id "MANAGED-ID" ParamText ParamText ""

        "[RFC5545, Section 3.2.11](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.11)\
        " MEMBER member "MEMBER" UriList (Vec<UriString>) ""

        "[RFC9073, Section 5.1](https://datatracker.ietf.org/doc/html/rfc9073#section-5.1)\
        " ORDER order "ORDER" Order NonZeroUsize ""

        "[RFC5545, Section 3.2.12](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.12)\
        " PARTSTAT partstat "PARTSTAT" PartStat PartStat ""

        "[RFC5545, Section 3.2.13](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.13)\
        " RANGE range "RANGE" Range (Option<ThisAndFuture>) "\n\
            RFC 5545 says the only valid value for RANGE is THISANDFUTURE, so we have another \
            single-valued type"

        "[RFC5545, Section 3.2.14](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.14)\
        " RELATED related "RELATED" Related Related ""

        "[RFC5545, Section 3.2.15](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.15)\
        " RELTYPE reltype "RELTYPE" Related Related ""

        "[RFC5545, Section 3.2.16](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.16)\
        " ROLE role "ROLE" Role Role ""

        "[RFC5545, Section 3.2.17](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.17)\
        " RSVP rsvp "RSVP" Boolean bool ""

        "[RFC6638, Section 7.1](https://datatracker.ietf.org/doc/html/rfc6638#section-7.1)\
        " SCHEDULE_AGENT schedule_agent "SCHEDULE-AGENT" ScheduleAgent ScheduleAgent ""

        "[RFC6638, Section 7.2](https://datatracker.ietf.org/doc/html/rfc6638#section-7.2)\
        " SCHEDULE_FORCE_SEND schedule_force_send "SCHEDULE-FORCE-SEND" ScheduleForceSend ScheduleForceSend ""

        "[RFC6638, Section 7.3](https://datatracker.ietf.org/doc/html/rfc6638#section-7.3)\
        " SCHEDULE_STATUS schedule_status "SCHEDULE-STATUS" ScheduleStatus ScheduleStatus ""

        "[RFC9073, Section 5.2](https://datatracker.ietf.org/doc/html/rfc9073#section-5.2)\
        " SCHEMA schema "SCHEMA" Uri UriString ""

        "[RFC5545, Section 3.2.18](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.18)\
        " SENT_BY sent_by "SENT-BY" SentBy CalAddress ""

        "[RFC8607, Section 4.1](https://datatracker.ietf.org/doc/html/rfc8607#section-4.1)\
        " SIZE size "SIZE" Size u64 ""

        "[RFC5545, Section 3.2.19](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.19)\
        " TZID tzid "TZID" Tzid String ""

        "[RFC5545, Section 3.2.20](https://datatracker.ietf.org/doc/html/rfc5545#section-3.2.20)\
        " VALUE value "VALUE" Value Value ""
    )
    ${
        const $CONST: usize = $?0;
    }
    impl Parameters {
        pub fn default() -> Self {
            Self(LiteMap::new())
        }
        ${
            #[doc = $RFC]
            #[doc = ""]
            #[doc = "Get the value of the"]
            #[doc = $tag]
            #[doc = "parameter."]
            #[doc = $doc]
            #[must_use]
            pub fn $method(&self) -> Option<&$typ> {
                match self.0.get(&$CONST) {
                    None => None,
                    Some(ParameterValue::$variant(value)) => Some(value),
                    _ => panic!(concat!("Unexpected type for ", $tag)),
                }
            }
            #[doc = "Set the value of the"]
            #[doc = $tag]
            #[doc = "parameter."]
            pub fn $+set_$method(&mut self, value: $typ) {
                self.0.insert($CONST, ParameterValue::$variant(value));
            }
        }
    }
    fn param_ids() -> NameIds {
        const NAMES: [&'static str; $#tag] = [${ $tag, }];
        NameIds::known_ids(NAMES)
    }
}
#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;

    #[rustfmt::skip]
    const PARAMETER_IDS: [usize; 35] = [
        ALTREP, CN, CUTYPE, DELEGATED_FROM, DELEGATED_TO, DERIVED, DIR, DISPLAY,
        EMAIL, ENCODING, FBTYPE, FEATURE, FILENAME, FMTTYPE, GAP, LABEL,
        LANGUAGE, LINKREL, MANAGED_ID, MEMBER, ORDER, PARTSTAT, RANGE, RELATED,
        RELTYPE, ROLE, RSVP, SCHEDULE_AGENT, SCHEDULE_FORCE_SEND, SCHEDULE_STATUS,
        SCHEMA, SENT_BY, SIZE, TZID, VALUE
        ];
    #[rustfmt::skip]
    const PARAMETER_NAMES: [&str; 35] = [
        "ALTREP", "CN", "CUTYPE", "DELEGATED-FROM", "DELEGATED-TO", "DERIVED", "DIR", "DISPLAY",
        "EMAIL", "ENCODING", "FBTYPE", "FEATURE", "FILENAME", "FMTTYPE", "GAP", "LABEL",
        "LANGUAGE", "LINKREL", "MANAGED-ID", "MEMBER", "ORDER", "PARTSTAT", "RANGE", "RELATED",
        "RELTYPE", "ROLE", "RSVP", "SCHEDULE-AGENT", "SCHEDULE-FORCE-SEND", "SCHEDULE-STATUS",
        "SCHEMA", "SENT-BY", "SIZE", "TZID", "VALUE"
        ];

    #[test]
    fn parameter_ids_remain_in_order() {
        let expected: Vec<_> = (ALTREP..=VALUE).collect();
        assert_eq!(Vec::from(PARAMETER_IDS), expected);
    }
    #[test]
    fn parameter_names_are_sorted() {
        let mut sorted = PARAMETER_NAMES;
        sorted.sort_unstable();
    }
    #[test]
    fn parameter_names_correspond_to_parameter_ids() {
        let mut ids = param_ids();
        let names_from_ids: Vec<_> =
            PARAMETER_IDS.into_iter().map(|id| ids.name(id).unwrap().to_string()).collect();
        assert_eq!(names_from_ids, Vec::from(PARAMETER_NAMES));
    }
}
